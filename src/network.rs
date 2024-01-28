//! Some network utility functions.

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;
use log::{debug, error, info, trace};
use tcp_handler::bytes::{Buf, BufMut, BytesMut};
use tcp_handler::common::{AesCipher, PacketError, StarterError};
use tcp_handler::compress_encrypt::{server_init, server_start};
use tcp_handler::flate2::Compression;
use tcp_handler::variable_len_reader::{VariableReader, VariableWriter};
use thiserror::Error;
use tokio::signal::ctrl_c;
use tokio::time::timeout;
use tokio::net::{TcpListener, TcpStream};
use tokio::{select, spawn};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use crate::config::{get_addr, get_connect_sec, get_idle_sec};
use crate::handler_base::IOStream;
use crate::Server;

/// Error in send/recv message.
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Sending/receiving timeout. See [`tcp_server::config::get_idle_sec`].
    #[error("Network timeout: {} after {1} sec.", match .0 { 1 => "Sending", 2 => "Receiving", _ => "Connecting" })]
    Timeout(u8, u64),

    /// During init protocol. From [`tcp_handler`][crate::tcp_handler].
    #[error("During io packet: {0:?}")]
    StarterError(#[from] StarterError),

    /// During io packet. From [`tcp_handler`][crate::tcp_handler].
    #[error("During io packet: {0:?}")]
    PacketError(#[from] PacketError),

    /// During read/write data from [`bytes`][crate::bytes].
    #[error("During read/write data: {0:?}")]
    BufError(#[from] std::io::Error),

    /// Broken cipher. This is a fatal error.
    ///
    /// When another error returned during send/recv, the stream is broken because no [`AesCipher`] received.
    /// In order not to panic, the stream marks as broken and this error is returned.
    #[error("Broken client.")]
    BrokenCipher(),
}

#[inline]
pub(crate) async fn send<W: AsyncWriteExt + Unpin + Send, B: Buf + Send>(stream: &mut W, message: &mut B, cipher: AesCipher, level: Compression) -> Result<AesCipher, NetworkError> {
    let idle = get_idle_sec();
    timeout(Duration::from_secs(idle), tcp_handler::compress_encrypt::send(stream, message, cipher, level)).await
        .map_err(|_| NetworkError::Timeout(1, idle))?.map_err(|e| e.into())
}

#[inline]
pub(crate) async fn recv<R: AsyncReadExt + Unpin + Send>(stream: &mut R, cipher: AesCipher) -> Result<(BytesMut, AesCipher), NetworkError> {
    let idle = get_idle_sec();
    timeout(Duration::from_secs(idle), tcp_handler::compress_encrypt::recv(stream, cipher)).await
        .map_err(|_| NetworkError::Timeout(2, idle))?.map_err(|e| e.into())
}

pub(super) async fn start_server<S: Server + Sync + ?Sized>(s: &'static S) -> std::io::Result<()> {
    let cancel_token = CancellationToken::new();
    let canceller = cancel_token.clone();
    spawn(async move {
        if let Err(e) = ctrl_c().await {
            error!("Failed to listen for shutdown signal: {}", e);
        } else {
            canceller.cancel();
        }
    });
    let server = TcpListener::bind(get_addr()).await?;
    info!("Listening on {}.", server.local_addr()?);
    let tasks = TaskTracker::new();
    select! {
        _ = cancel_token.cancelled() => {
            info!("Shutting down the server gracefully...");
        }
        _ = async { loop {
            let (client, address) = match server.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };
            let canceller = cancel_token.clone();
            tasks.spawn(async move {
                trace!("TCP stream connected from {}.", address);
                if let Err(e) = handle_client(s, client, address, canceller).await {
                    error!("Failed to handle connection. address: {}, err: {}", address, e);
                }
                trace!("TCP stream disconnected from {}.", address);
            });
        } } => {}
    }
    tasks.close();
    tasks.wait().await;
    Ok(())
}

async fn handle_client<S: Server + Sync + ?Sized>(server: &S, client: TcpStream, address: SocketAddr, cancel_token: CancellationToken) -> Result<(), NetworkError> {
    let (receiver, sender)= client.into_split();
    let mut receiver = BufReader::new(receiver);
    let mut sender = BufWriter::new(sender);
    let mut version = None;
    let connect = get_connect_sec();
    let cipher = match select! {
        _ = cancel_token.cancelled() => { return Ok(()); },
        c = timeout(Duration::from_secs(connect), async {
            let init = server_init(&mut receiver, server.get_identifier(), |v| {
                version = Some(v.to_string());
                server.check_version(v)
            }).await;
            server_start(&mut sender, init).await
        }) => c.map_err(|_| NetworkError::Timeout(3, connect))?,
    } { Ok(cipher) => cipher, Err(e) => {
        if let StarterError::IO(ref e) = e {
            if e.kind() == ErrorKind::UnexpectedEof {
                return Ok(()); // Ignore 'early eof'.
            }
        }
        return Err(e.into());
    } };
    let version = version.unwrap();
    debug!("Client connected from {}. version: {}", address, version);
    let mut stream = IOStream::new(receiver, sender, cipher, address, version);
    loop {
        let receiver = &mut stream.receiver;
        let sender = &mut stream.sender;
        let (mut cipher, mut guard) = stream.cipher.get().await?;
        let mut data = match select! {
            _ = cancel_token.cancelled() => { return Ok(()); },
            d = tcp_handler::compress_encrypt::recv(receiver, cipher) => d, // No timeout here.
        } {
            Ok((d, c)) => { cipher = c; d.reader() },
            Err(e) => {
                if let PacketError::IO(ref e) = e {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        return Ok(()); // Ignore 'early eof'.
                    }
                }
                return Err(e.into());
            }
        };
        let func = data.read_string()?;
        let function = server.get_function(&func);
        let mut writer = BytesMut::new().writer();
        writer.write_bool(function.is_some())?;
        cipher = send(sender, &mut writer.into_inner(), cipher, Compression::fast()).await?;
        (*guard).replace(cipher);
        drop(guard);
        if let Some(function) = function {
            if let Err(error) = function.handle(&mut stream).await {
                server.handle_error(&func, error, &mut stream).await?;
            }
        }
    }
}
