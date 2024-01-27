//! The basic trait for handler.

use std::net::SocketAddr;
use async_trait::async_trait;
use tcp_handler::common::AesCipher;
use tcp_handler::bytes::{Buf, BytesMut};
use tcp_handler::flate2::Compression;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::mutable_cipher::MutableCipher;
use crate::network::{NetworkError, recv, send};

/// A wrapper of the stream with some common methods.
pub struct IOStream<R, W> where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
    pub(crate) receiver: R,
    pub(crate) sender: W,
    pub(crate) cipher: MutableCipher,
    addr: SocketAddr,
    version: String,
}

impl<R, W> IOStream<R, W> where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
    pub(crate) fn new(receiver: R, sender: W, cipher: AesCipher, addr: SocketAddr, version: String) -> Self {
        Self { receiver, sender, cipher: MutableCipher::new(cipher), addr, version }
    }

    /// Get the peer addr.
    pub fn get_addr(&self) -> &SocketAddr {
        &self.addr
    }

    /// Get the version of the client.
    pub fn get_version(&self) -> &str {
        &self.version
    }

    /// Send a message to the client.
    pub async fn send<B: Buf + Send>(&mut self, message: &mut B) -> Result<(), NetworkError> {
        let (cipher, guard) = self.cipher.get().await?;
        let cipher = send(&mut self.sender, message, cipher, Compression::default()).await?;
        self.cipher.reset(guard, cipher);
        Ok(())
    }

    /// Recv a message from the client.
    pub async fn recv(&mut self) -> Result<BytesMut, NetworkError> {
        let (cipher, guard) = self.cipher.get().await?;
        let (response, cipher) = recv(&mut self.receiver, cipher).await?;
        self.cipher.reset(guard, cipher);
        Ok(response)
    }

    /// A shortcut of send and recv message.
    pub async fn send_recv<B: Buf + Send>(&mut self, message: &mut B) -> Result<BytesMut, NetworkError> {
        self.send(message).await?;
        self.recv().await
    }
}

/// The basic trait for handler.
#[async_trait]
pub trait FuncHandler<R, W>: Send where R: AsyncReadExt + Unpin + Send, W: AsyncWriteExt + Unpin + Send {
    /// Define your handler here.
    async fn handle(&self, stream: &mut IOStream<R, W>) -> anyhow::Result<()>;
}

/// Conveniently define a handler.
///
/// The struct is zero-sized.
#[macro_export]
macro_rules! func_handler {
    ($vi: vis $name: ident, |$stream: ident| $block: expr) => {
        $vi struct $name;
        #[$crate::async_trait::async_trait]
        impl<R: $crate::tokio::io::AsyncReadExt + Unpin + Send, W: $crate::tokio::io::AsyncWriteExt + Unpin + Send> $crate::handler_base::FuncHandler<R, W> for $name {
            async fn handle(&self, $stream: &mut $crate::handler_base::IOStream<R, W>) -> $crate::anyhow::Result<()> {
                $block
            }
        }
    };
}
