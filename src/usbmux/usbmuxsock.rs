use tokio::{io::AsyncWriteExt, net::TcpStream};

use byteorder::ReadBytesExt;

use super::errors::UsbmuxSockError;

/// Represents a connection to the usbmuxd socket.
///
/// This struct manages a TCP connection to the usbmuxd service,
/// typically running on localhost:27015. It provides methods to
/// create a new connection and to reset (reconnect) the socket.
pub struct UsbmuxSock {
    /// The underlying asynchronous TCP stream.
    pub sock: TcpStream,
}

impl UsbmuxSock {
    /// Creates a new `UsbmuxSock` by connecting to the usbmuxd socket.
    ///
    /// # Errors
    ///
    /// Returns a [`UsbmuxSockError`] if the connection fails.
    pub async fn new() -> Result<Self, UsbmuxSockError> {
        let mut stream = TcpStream::connect("127.0.0.1:27015").await?;
        Ok(UsbmuxSock { sock: stream })
    }

    /// Resets the current socket connection by shutting it down and reconnecting.
    ///
    /// # Errors
    ///
    /// Returns a [`UsbmuxSockError`] if shutting down or reconnecting fails.
    pub async fn reset(&mut self) -> Result<Self, UsbmuxSockError> {
        self.sock.shutdown().await?;
        let new_sock = UsbmuxSock::new().await?;
        Ok(new_sock)
    }
}
