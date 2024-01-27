#![doc = include_str!("../README.md")]
// #![warn(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod config;
pub mod network;
mod mutable_cipher;
pub mod handler_base;

pub extern crate anyhow;
pub extern crate async_trait;
pub extern crate tokio;
pub extern crate tcp_handler;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::handler_base::FuncHandler;
use crate::network::start_server;

#[async_trait]
pub trait Server {
    fn get_identifier(&self) -> &'static str;

    /// # Example
    /// ```rust,ignore
    /// version == env!("CARGO_PKG_VERSION")
    /// ```
    fn check_version(&self, version: &str) -> bool;

    fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>>
        where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send;

    async fn start(&'static self) -> std::io::Result<()> {
        start_server(self).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use anyhow::Result;
    use env_logger::Target;
    use tcp_client::client_base::ClientBase;
    use tcp_client::config::{ClientConfig as ClientConfiguration, set_config as set_client_config};
    use tcp_client::{client_factory, ClientFactory};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::spawn;
    use tokio::time::sleep;
    use crate::{func_handler, Server};
    use crate::config::{ServerConfig, set_config as set_server_config};
    use crate::handler_base::FuncHandler;

    struct TestServer;

    impl Server for TestServer {
        fn get_identifier(&self) -> &'static str {
            "tester"
        }

        fn check_version(&self, version: &str) -> bool {
            version == env!("CARGO_PKG_VERSION")
        }

        fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>>
            where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
            if func == "test" {
                func_handler!(TestHandler, |_stream| {
                    Ok(())
                });
                Some(Box::new(TestHandler))
            } else {
                None
            }
        }
    }

    client_factory!(TestClientFactory, TestClient, "tester");

    #[tokio::test]
    async fn test() -> Result<()> {
        env_logger::builder().parse_filters("trace").target(Target::Stderr).try_init()?;
        set_server_config(ServerConfig {
            addr: "localhost:25565".to_string(),
            connect_sec: 10,
            idle_sec: 3,
        });
        set_client_config(ClientConfiguration {
            connect_sec: 10,
            idle_sec: 3,
        });

        let server = spawn(TestServer.start());
        let mut client = TestClientFactory.connect("localhost:25565").await?;
        client.check_func("test").await?;
        drop(client);

        sleep(Duration::from_secs(1)).await; // Waiting for all log printing to complete.
        server.abort();
        Ok(())
    }
}
