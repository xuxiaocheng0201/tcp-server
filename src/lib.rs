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
    async fn handle(&self, receiver: &mut R, sender: &mut W, cipher: AesCipher, addr: SocketAddr, version: &str) -> Result<AesCipher>;
}

#[macro_export]
macro_rules! func_handler {
    ($vi: vis $name: ident, |$receiver: ident, $sender: ident, $cipher: ident, $addr: ident, $version: ident| $block: expr) => {
        $vi struct $name;
        #[$crate::async_trait::async_trait]
        impl<R: $crate::tokio::io::AsyncReadExt + Unpin + Send, W: $crate::tokio::io::AsyncWriteExt + Unpin + Send> $crate::FuncHandler<R, W> for $name {
            async fn handle(&self, $receiver: &mut R, $sender: &mut W, $cipher: $crate::tcp_handler::common::AesCipher, $addr: std::net::SocketAddr, $version: &str) -> $crate::anyhow::Result<$crate::tcp_handler::common::AesCipher> {
                $block
            }
        }
    };
}

#[async_trait]
pub trait Server {
    fn get_identifier(&self) -> &'static str;

    /// # Example
    /// ```ignore
    /// version == env!("CARGO_PKG_VERSION")
    /// ```
    fn check_version(&self, version: &str) -> bool;

    fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>>
        where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send;

    async fn start(&'static self) -> Result<()> {
        start_server(self).await
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use env_logger::Target;
    use tcp_client::ClientFactory;
    use tcp_handler::common::AesCipher;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::spawn;
    use crate::{FuncHandler, Server};
    use crate::configuration::{Configuration, set_config};

    struct TestServer;
    static TEST_SERVER: TestServer = TestServer;

    impl Server for TestServer {
        fn get_identifier(&self) -> &'static str {
            "tester"
        }

        /// ```
        fn check_version(&self, version: &str) -> bool {
            version == env!("CARGO_PKG_VERSION")
        }

        fn get_function<R, W>(&self, _func: &str) -> Option<Box<dyn FuncHandler<R, W>>> where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
            None
        }
    }

    type TestClient = (TcpStream, AesCipher);
    struct TestClientFactory;

    impl ClientFactory<TestClient> for TestClientFactory {
        fn get_identifier(&self) -> &'static str {
            "tester"
        }

        fn get_version(&self) -> &'static str {
            env!("CARGO_PKG_VERSION")
        }
    }

    #[tokio::test]
    async fn test() -> Result<()> {
        env_logger::builder().parse_filters("trace").target(Target::Stderr).try_init()?;
        set_config(Configuration {
            addr: "localhost:25565".to_string(),
            ..Configuration::default()
        });
        let server = spawn(TEST_SERVER.start());
        TestClientFactory.connect("localhost:25565").await?;
        server.abort();
        Ok(())
    }
}
