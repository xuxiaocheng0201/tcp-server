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
    use std::time::Duration;
    use anyhow::Result;
    use env_logger::Target;
    use tcp_client::client_base::ClientBase;
    use tcp_client::configuration::{Configuration as ClientConfiguration, set_config as set_client_config};
    use tcp_client::quickly_connect;
    use tcp_client::mutable_cipher::MutableCipher;
    use tcp_handler::common::AesCipher;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
    use tokio::net::TcpStream;
    use tokio::spawn;
    use tokio::time::sleep;
    use crate::{FuncHandler, Server};
    use crate::configuration::{ServerConfiguration as ServerConfiguration, set_server_config as set_server_config};

    struct TestServer;

    impl Server for TestServer {
        fn get_identifier(&self) -> &'static str {
            "tester"
        }

        fn check_version(&self, version: &str) -> bool {
            version == env!("CARGO_PKG_VERSION")
        }

        fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>> where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
            if func == "test" {
                func_handler!(TestHandler, |_receiver, _sender, cipher, _addr, _version| {
                    Ok(cipher)
                });
                Some(Box::new(TestHandler))
            } else {
                None
            }
        }
    }

    struct TestClient(OwnedReadHalf, OwnedWriteHalf, MutableCipher);

    impl From<(TcpStream, AesCipher)> for TestClient {
        fn from(value: (TcpStream, AesCipher)) -> Self {
            let (receiver, sender) = value.0.into_split();
            Self(receiver, sender, MutableCipher::new(value.1))
        }
    }

    impl ClientBase<OwnedReadHalf, OwnedWriteHalf> for TestClient {
        fn get_receiver<'a>(&'a mut self) -> (&'a mut OwnedReadHalf, &MutableCipher) {
            (&mut self.0, &self.2)
        }

        fn get_sender<'a>(&'a mut self) -> (&'a mut OwnedWriteHalf, &MutableCipher) {
            (&mut self.1, &self.2)
        }
    }

    static TEST_SERVER: TestServer = TestServer;

    #[tokio::test]
    async fn test() -> Result<()> {
        env_logger::builder().parse_filters("trace").target(Target::Stderr).try_init()?;
        set_server_config(ServerConfiguration {
            addr: "localhost:25565".to_string(),
            connect_sec: 10,
            idle_sec: 3,
        });
        set_client_config(ClientConfiguration {
            connect_sec: 10,
            idle_sec: 3,
        });

        let server = spawn(TEST_SERVER.start());
        let mut client: TestClient = quickly_connect("tester", env!("CARGO_PKG_VERSION"), "localhost:25565").await?;

        client.check_func("test").await?;

        drop(client);
        sleep(Duration::from_secs(1)).await; // Waiting for all log printing to complete.

        server.abort();
        Ok(())
    }
}
