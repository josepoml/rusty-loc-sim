use std::{error::Error, fmt};

use rustls_pki_types::InvalidDnsNameError;

//MESSAGE OPERATION ERROR
#[derive(Debug, thiserror::Error)]
pub enum MessageOperationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plist error: {0}")]
    Plist(#[from] plist::Error),
    #[error("UsbmuxSock error: {0}")]
    UsbmuxSockError(#[from] UsbmuxSockError),
    #[error("Invalid or no response")]
    MissingStream,
    #[error("Invalid or no response")]
    ResponseError,
}

// USBMUX OPERATION ERROR
#[derive(Debug, thiserror::Error)]
pub enum UsbmuxOperationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("MessageOperation error: {0}")]
    MessageOperationError(#[from] MessageOperationError),
    #[error("UsbmuxSock error: {0}")]
    UsbmuxSockError(#[from] UsbmuxSockError),
    #[error("Plist error: {0}")]
    Plist(#[from] plist::Error),
    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Ssl sock error: {0}")]
    SslError(#[from] SslError),
    #[error("Missing arguments: {0}")]
    MissingArguments(&'static str),
    #[error("Parse error")]
    ParseError,
    #[error("Error: {0}")]
    Error(String),
}

// USBMUX SOCK ERROR
#[derive(Debug, thiserror::Error)]
pub enum UsbmuxSockError {
    #[error("Failed to connect to USBMUXD socket: {0}")]
    Io(#[from] std::io::Error),
}

// SSL ERROR
#[derive(Debug, thiserror::Error)]
pub enum SslError {
    #[error("SSL error: {0}")]
    Ssl(#[from] rustls::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UsbmuxSock error: {0}")]
    UsbmuxSockError(#[from] UsbmuxSockError),
    #[error("Invalid certificate: {0}")]
    PemError(#[from] rustls_pki_types::pem::Error),
    #[error("Invalid sock configuration: {0}")]
    SslSockConfigError(#[from] InvalidDnsNameError),
}
