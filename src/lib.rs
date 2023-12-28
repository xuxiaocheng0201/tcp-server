pub mod configuration;

// extern for macro.
pub extern crate anyhow;
pub extern crate async_trait;
pub extern crate tokio;
pub extern crate tokio_util;
pub extern crate tcp_handler;
#[cfg(feature = "serde")]
pub extern crate serde;

use std::net::SocketAddr;
use std::ops::Add;
use std::time::Duration;
use anyhow::Result;
use async_trait::async_trait;
use log::{debug, error, info, trace};
use tcp_handler::bytes::{Buf, Bytes, BytesMut};
use tcp_handler::common::{AesCipher, PacketError};
use tcp_handler::compress_encrypt::{server_init, server_start};
use tcp_handler::flate2::Compression;
use tcp_handler::variable_len_reader::asynchronous::AsyncVariableWritable;
use tcp_handler::variable_len_reader::VariableReadable;
use tokio::signal::ctrl_c;
use tokio::time::{Instant, sleep};
use tokio::net::{TcpListener, TcpStream};
use tokio::{select, spawn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use crate::configuration::{get_addr, get_connect_sec, get_idle_sec};

#[async_trait]
pub trait FuncHandler<R, W>: Send where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
    async fn handle(&self, receiver: &mut R, sender: &mut W, cipher: AesCipher) -> Result<AesCipher>;
}

#[macro_export]
macro_rules! func_handler {
    ($vi: vis $name: ident, |$receiver: ident, $sender: ident, $cipher: ident| $block: expr) => {
        $vi struct $name;
        #[$crate::async_trait::async_trait]
        impl<R: $crate::tokio::io::AsyncReadExt + Unpin + Send, W: $crate::tokio::io::AsyncWriteExt + Unpin + Send> $crate::FuncHandler<R, W> for $name {
            async fn handle(&self, $receiver: &mut R, $sender: &mut W, $cipher: $crate::tcp_handler::common::AesCipher) -> $crate::anyhow::Result<AesCipher> {
                $block
            }
        }
    };
}

#[async_trait]
pub trait Server {
    fn check_version(&self, version: &str) -> bool;

    fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>>
        where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send;

    async fn start(&'static self) -> Result<()> {
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
                    if let Err(e) = handle(self, client, address, canceller).await {
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
}

#[inline]
pub async fn send<W: AsyncWriteExt + Unpin + Send>(stream: &mut W, message: &Bytes, cipher: AesCipher, level: Compression) -> std::result::Result<AesCipher, PacketError> {
    tcp_handler::compress_encrypt::send(stream, message, cipher, level).await
}

#[inline]
pub async fn recv<R: AsyncReadExt + Unpin + Send>(stream: &mut R, cipher: AesCipher, time: Duration) -> Option<std::result::Result<(BytesMut, AesCipher), PacketError>> {
    select! {
        c = tcp_handler::compress_encrypt::recv(stream, cipher) => Some(c),
        _ = sleep(time) => None,
    }
}

async fn handle(server: &'static (impl Server + ?Sized), client: TcpStream, address: SocketAddr, cancel_token: CancellationToken) -> Result<()> {
    let (mut receiver, mut sender)= client.into_split();
    let mut version = None;
    let connect_sec = get_connect_sec();
    let mut cipher = match select! {
        _ = cancel_token.cancelled() => { Err(()) },
        _ = sleep(Duration::from_secs(connect_sec)) => {
            debug!("Connection timeout: {}, {} secs.", address, connect_sec);
            Err(())
        },
        c = async {
            let init = server_init(&mut receiver, &"Wlist-server", |v| {
                version = Some(v.to_string());
                server.check_version(v)
            }).await;
            server_start(&mut sender, init).await.map_err(|e| {
                trace!("Error connection client. address: {}, err: {:?}", address, e)
            })
        } => c,
    } { Ok(c) => c, Err(_) => return Ok(()), };
    let version = version.unwrap();
    debug!("Client connected from {}. version: {}", address, version);
    let mut last_time = Instant::now();
    loop {
        let idle_sec = get_idle_sec();
        let mut data = select! {
            _ = cancel_token.cancelled() => { return Ok(()); },
            d = recv(&mut receiver, cipher, Instant::now().duration_since(last_time).add(Duration::from_secs(idle_sec))) => match d {
                Some(d) => match d {
                    Ok((d, c)) => { cipher = c; d.reader() },
                    Err(e) => { trace!("Error receiving data. address: {}, err: {:?}", address, e); return Ok(()); }
                },
                None => {
                    debug!("Read timeout: {}. duration: {} secs.", address, idle_sec);
                    return Ok(());
                }
            },
        };
        if let Some(func) = server.get_function(&data.read_string()?) {
            sender.write_bool(true).await?;
            sender.flush().await?;
            cipher = func.handle(&mut receiver, &mut sender, cipher).await?;
            last_time = Instant::now();
        } else {
            sender.write_bool(false).await?;
            sender.flush().await?;
        }
    }
}


#[cfg(test)]
mod tests {
    use anyhow::Result;

    #[tokio::test]
    async fn test() -> Result<()> {
        env_logger::builder().is_test(true).try_init()?;

        Ok(())
    }
}
