# Tcp-Server

[![Crate](https://img.shields.io/crates/v/tcp-server.svg)](https://crates.io/crates/tcp-server)
![Crates.io License](https://img.shields.io/crates/l/tcp-server)

**Read this in other languages: [English](README.md), [简体中文](README_zh.md).**

# Description

Convenient server-side TCP service.
Also see [tcp-client](https://crates.io/crates/tcp-client) for client-side.

Based on [tcp-handler](https://crates.io/crates/tcp-handler).

With complete API [document](https://docs.rs/tcp-server/).


# Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
tcp-server = "~0.2"
```


# Example

```rust,no_run
use tcp_server::{func_handler, Server};
use tcp_server::anyhow::anyhow;
use tcp_server::handler_base::FuncHandler;
use tcp_server::tcp_handler::bytes::{Buf, BufMut, BytesMut};
use tcp_server::tcp_handler::variable_len_reader::{VariableReader, VariableWriter};
use tcp_server::tokio::io::{AsyncReadExt, AsyncWriteExt};

struct MyServer;

impl Server for MyServer {
    fn get_identifier(&self) -> &'static str {
        "MyTcpApplication"
    }
    
    fn check_version(&self, version: &str) -> bool {
        version == env!("CARGO_PKG_VERSION")
    }
    
    fn get_function<R, W>(&self, func: &str) -> Option<Box<dyn FuncHandler<R, W>>>
        where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
        match func {
            // define your route here.
            // example:
            "hello" => Some(Box::new(HelloHandler)),
            _ => None,
        }
    }
}

// define your method here.
// example:
func_handler!(HelloHandler, |stream| {
    let mut reader = stream.recv().await?.reader();
    if "hello server" != reader.read_string()? {
        return Err(anyhow!("Invalid message."));
    }
    let mut writer = BytesMut::new().writer();
    writer.write_string("hello client")?;
    stream.send(&mut writer.into_inner()).await?;
    Ok(())
});

#[tokio::main]
async fn main() {
    MyServer.start().await.unwrap();
}
```


# Version map

Versions map to [tcp-client](https://crates.io/crates/tcp-client) with the same protocol.
(Recommended for use in conjunction, otherwise unexpected bugs may occur.)

| client version | server version |
|----------------|----------------|
| \>=0.1.0       | \>=0.2.0       |
| <0.1.0         | <0.2.0         |
