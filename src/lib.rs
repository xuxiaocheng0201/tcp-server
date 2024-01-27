#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
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

use anyhow::Error;
use async_trait::async_trait;
use log::error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::handler_base::{FuncHandler, IOStream};
use crate::network::{NetworkError, start_server};

/// The basic trait for a server.
/// # Example
/// ```rust,no_run
/// use tcp_server::{func_handler, Server};
/// use tcp_server::anyhow::anyhow;
/// use tcp_server::handler_base::FuncHandler;
/// use tcp_server::tcp_handler::bytes::{Buf, BufMut, BytesMut};
/// use tcp_server::tcp_handler::variable_len_reader::{VariableReader, VariableWriter};
/// use tcp_server::tokio::io::{AsyncReadExt, AsyncWriteExt};
///
/// struct MyServer;
///
/// impl Server for MyServer {
///     fn get_identifier(&self) -> &'static str {
///         "MyTcpApplication"
///     }
///
///     fn check_version(&self, version: &str) -> bool {
///         version == env!("CARGO_PKG_VERSION")
///     }
///
///     fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>>
///         where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
///         match func {
///             "hello" => Some(Box::new(HelloHandler)),
///             _ => None,
///         }
///     }
/// }
///
/// func_handler!(HelloHandler, |stream| {
///     let mut reader = stream.recv().await?.reader();
///     if "hello server" != reader.read_string()? {
///         return Err(anyhow!("Invalid message."));
///     }
///     let mut writer = BytesMut::new().writer();
///     writer.write_string("hello client")?;
///     stream.send(&mut writer.into_inner()).await?;
///     Ok(())
/// });
///
/// #[tokio::main]
/// async fn main() {
///     MyServer.start().await.unwrap();
/// }
/// ```
#[async_trait]
pub trait Server {
    /// Get the identifier of your application.
    /// # Note
    /// This should be a const.
    fn get_identifier(&self) -> &'static str;

    /// Check the version of the client.
    /// You can reject the client if the version is not supported.
    /// # Note
    /// This should be no side effect.
    /// # Example
    /// ```rust,ignore
    /// version == env!("CARGO_PKG_VERSION")
    /// ```
    fn check_version(&self, version: &str) -> bool;

    /// Return the function handler. See [`Server`] for example.
    /// # Note
    /// This should be no side effect.
    fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>>
        where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send;

    /// Handle the error which returned by the [`FuncHandler`].
    /// Default only print the error message.
    async fn handle_error<R, W>(&self, func: &str, error: Error, _stream: &mut IOStream<R, W>) -> Result<(), NetworkError>
        where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
        error!("Failed to handle in {}: {}", func, error);
        Ok(())
    }

    /// Start the server. This method will **block** the caller thread.
    ///
    /// It **only** will return an error if the server cannot start.
    /// If you want to handle errors returned by [`FuncHandler`], you should use [`Server::handle_error`].
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
    use tcp_client::config::{ClientConfig, set_config as set_client_config};
    use tcp_client::{client_factory, ClientFactory};
    use tcp_client::network::NetworkError;
    use tcp_handler::bytes::{Buf, BufMut, BytesMut};
    use tcp_handler::variable_len_reader::{VariableReader, VariableWriter};
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
                func_handler!(TestHandler, |stream| {
                    let mut reader = stream.recv().await?.reader();
                    assert_eq!("hello server", reader.read_string()?);
                    let mut writer = BytesMut::new().writer();
                    writer.write_string("hello client")?;
                    stream.send(&mut writer.into_inner()).await?;
                    Ok(())
                });
                assert_eq!(std::mem::size_of::<TestHandler>(), 0);
                Some(Box::new(TestHandler))
            } else {
                None
            }
        }
    }

    client_factory!(TestClientFactory, TestClient, "tester");

    impl TestClient {
        async fn test_method(&mut self) -> Result<(), NetworkError> {
            self.check_func("test").await?;
            let mut writer = BytesMut::new().writer();
            writer.write_string("hello server")?;
            let mut reader = self.send_recv(&mut writer.into_inner()).await?.reader();
            assert_eq!("hello client", reader.read_string()?);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test() -> Result<()> {
        env_logger::builder().parse_filters("trace").target(Target::Stderr).try_init()?;
        set_server_config(ServerConfig {
            addr: "localhost:25565".to_string(),
            connect_sec: 10,
            idle_sec: 3,
        });
        set_client_config(ClientConfig {
            connect_sec: 10,
            idle_sec: 3,
        });

        let server = spawn(TestServer.start());
        let mut client = TestClientFactory.connect("localhost:25565").await?;
        client.test_method().await?;
        drop(client);

        sleep(Duration::from_millis(100)).await; // Waiting for all log printing to complete.
        server.abort();
        Ok(())
    }
}
