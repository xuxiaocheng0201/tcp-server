use std::io::{Error, ErrorKind};
use std::io::ErrorKind::UnexpectedEof;
use std::net::SocketAddr;
use std::ops::Add;
use std::time::Duration;
use log::{debug, error, info, trace};
use tcp_handler::bytes::{Buf, BytesMut};
use tcp_handler::common::{AesCipher, PacketError};
use tcp_handler::compress_encrypt::{server_init, server_start};
use tcp_handler::flate2::Compression;
use tcp_handler::variable_len_reader::asynchronous::AsyncVariableWritable;
use tcp_handler::variable_len_reader::VariableReadable;
use tokio::signal::ctrl_c;
use tokio::time::{Instant, timeout};
use tokio::net::{TcpListener, TcpStream};
use tokio::{select, spawn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use crate::configuration::{get_server_addr, get_server_connect_sec, get_server_idle_sec};
use crate::Server;

#[inline]
pub async fn send<W: AsyncWriteExt + Unpin + Send, B: Buf>(stream: &mut W, message: &mut B, cipher: AesCipher, level: Compression) -> Result<AesCipher, PacketError> {
    tcp_handler::compress_encrypt::send(stream, message, cipher, level).await
}

#[inline]
pub async fn recv<R: AsyncReadExt + Unpin + Send>(stream: &mut R, cipher: AesCipher, time: Option<(SocketAddr, Duration)>) -> Result<(BytesMut, AesCipher), PacketError> {
    if let Some((addr, time)) = time {
        timeout(time, tcp_handler::compress_encrypt::recv(stream, cipher)).await
            .map_err(|_| PacketError::IO(Error::new(ErrorKind::TimedOut, format!("Recv timeout: {}. timeout: {:?}", addr, time))))?
    } else {
        tcp_handler::compress_encrypt::recv(stream, cipher).await
    }
}

pub(super) async fn start_server<S: Server + ?Sized + Sync>(s: &'static S) -> anyhow::Result<()> {
    let cancel_token = CancellationToken::new();
    let canceller = cancel_token.clone();
    spawn(async move {
        if let Err(e) = ctrl_c().await {
            error!("Failed to listen for shutdown signal: {}", e);
        } else {
            canceller.cancel();
        }
    });
    let server = TcpListener::bind(get_server_addr()).await?;
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

async fn handle_client<S: Server + ?Sized>(server: &S, client: TcpStream, address: SocketAddr, cancel_token: CancellationToken) -> anyhow::Result<()> {
    let (mut receiver, mut sender)= client.into_split();
    let mut version = None;
    let connect_sec = get_server_connect_sec();
    let mut cipher = match select! {
        _ = cancel_token.cancelled() => { Err(()) },
        c = timeout(Duration::from_secs(connect_sec), async {
            let init = server_init(&mut receiver, server.get_identifier(), |v| {
                version = Some(v.to_string());
                server.check_version(v)
            }).await;
            server_start(&mut sender, init).await.map_err(|e| {
                trace!("Error connection client. address: {}, err: {:?}", address, e)
            })
        }) => c.unwrap_or_else(|_| {
            debug!("Connection timeout: {}, {} secs.", address, connect_sec);
            Err(())
        }),
    } { Ok(c) => c, Err(_) => return Ok(()) };
    let version = version.unwrap();
    debug!("Client connected from {}. version: {}", address, version);
    let mut last_time = Instant::now();
    loop {
        let idle_sec = get_server_idle_sec();
        let mut data = match select! {
            _ = cancel_token.cancelled() => { return Ok(()); },
            d = recv(&mut receiver, cipher, Some((address, Instant::now().duration_since(last_time).add(Duration::from_secs(idle_sec))))) => d,
        } {
            Ok((d, c)) => { cipher = c; d.reader() },
            Err(e) => {
                if let PacketError::IO(ref e) = e {
                    if e.kind() == UnexpectedEof {
                        return Ok(()); // Ignore 'early eof'.
                    }
                }
                trace!("Error receiving data. address: {}, err: {:?}", address, e);
                return Ok(());
            }
        };
        if let Some(func) = server.get_function(&data.read_string()?) {
            sender.write_bool(true).await?;
            sender.flush().await?;
            cipher = func.handle(&mut receiver, &mut sender, cipher, address, &version).await?;
            last_time = Instant::now();
        } else {
            sender.write_bool(false).await?;
            sender.flush().await?;
        }
    }
}
