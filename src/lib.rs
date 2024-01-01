pub mod configuration;
mod network;

// extern for macro.
pub extern crate anyhow;
pub extern crate async_trait;
pub extern crate tokio;
pub extern crate tokio_util;
pub extern crate tcp_handler;
#[cfg(feature = "serde")]
pub extern crate serde;

use std::net::SocketAddr;
use anyhow::Result;
use async_trait::async_trait;
use tcp_handler::common::AesCipher;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::network::start_server;

pub use network::{send, recv};

#[async_trait]
pub trait FuncHandler<R, W>: Send where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
    async fn handle(&self, receiver: &mut R, sender: &mut W, cipher: AesCipher, addr: SocketAddr) -> Result<AesCipher>;
}

#[macro_export]
macro_rules! func_handler {
    ($vi: vis $name: ident, |$receiver: ident, $sender: ident, $cipher: ident, $addr: ident| $block: expr) => {
        $vi struct $name;
        #[$crate::async_trait::async_trait]
        impl<R: $crate::tokio::io::AsyncReadExt + Unpin + Send, W: $crate::tokio::io::AsyncWriteExt + Unpin + Send> $crate::FuncHandler<R, W> for $name {
            async fn handle(&self, $receiver: &mut R, $sender: &mut W, $cipher: $crate::tcp_handler::common::AesCipher, $addr: std::net::SocketAddr) -> $crate::anyhow::Result<$crate::tcp_handler::common::AesCipher> {
                $block
            }
        }
    };
}

#[async_trait]
pub trait Server {
    fn get_identifier(&self) -> &'static str;

    fn check_version(&self, version: &str) -> bool;

    fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>>
        where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send;

    async fn start(&'static self) -> Result<()> {
        start_server(self, self.get_identifier()).await
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
