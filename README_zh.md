# Tcp处理-Server端

[![Crate](https://img.shields.io/crates/v/tcp-server.svg)](https://crates.io/crates/tcp-server)
![Crates.io License](https://img.shields.io/crates/l/tcp-server)

**其他语言版本：[English](README.md)，[简体中文](README_zh.md)。**

# 描述

便捷的TCP服务-服务端。
客户端请见 [tcp-client](https://crates.io/crates/tcp-client)。

基于 [tcp-handler](https://crates.io/crates/tcp-handler)。

API [文档](https://docs.rs/tcp-server/)已完善。


# 用法

将以下内容添加到你的`Cargo.toml`：

```toml
[dependencies]
tcp-server = "~0.2"
```


# 示例

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
            // 在此处定义你的路由
            // 示例：
            "hello" => Some(Box::new(HelloHandler)),
            _ => None,
        }
    }
}

// 在此处定义你的处理逻辑
// 示例：
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


# 版本对照表

与 [tcp-client](https://crates.io/crates/tcp-client) 板条箱具有相同协议的版本。
（推荐配套使用，否则可能会出现意料之外的BUG）

| client version | server version |
|----------------|----------------|
| \>=0.1.0       | \>=0.2.0       |
| <0.1.0         | <0.2.0         |
