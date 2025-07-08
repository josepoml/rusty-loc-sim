use core::error;
use std::{error::Error, fmt, num::ParseIntError};

use hex::FromHexError;

#[derive(Debug)]
pub enum XpcError {
    HandshakeError(HandshakeError),
    ParseError(ParseError),
}

impl From<ParseError> for XpcError {
    fn from(value: ParseError) -> Self {
        XpcError::ParseError(value)
    }
}

impl From<ReceiveFrameError> for XpcError {
    fn from(value: ReceiveFrameError) -> Self {
        XpcError::HandshakeError(HandshakeError::ReceiveFrameError(value))
    }
}

impl From<SendFrameError> for XpcError {
    fn from(value: SendFrameError) -> Self {
        XpcError::HandshakeError(HandshakeError::SendFrameError(value))
    }
}

impl fmt::Display for XpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XpcError::HandshakeError(error) => write!(f, "Handshake error: {}", error),
            XpcError::ParseError(error) => write!(f, "ParseError: {}", error),
        }
    }
}

impl Error for XpcError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            XpcError::HandshakeError(error) => Some(error),
            XpcError::ParseError(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum SendFrameError {
    IoError(std::io::Error),
    ParseError(ParseError),
}

impl From<FromHexError> for SendFrameError {
    fn from(value: FromHexError) -> Self {
        SendFrameError::ParseError(ParseError::FromHexError(value))
    }
}

impl From<std::io::Error> for SendFrameError {
    fn from(value: std::io::Error) -> Self {
        SendFrameError::IoError(value)
    }
}

impl fmt::Display for SendFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendFrameError::IoError(error) => write!(f, "Io error: {}", error),
            SendFrameError::ParseError(error) => write!(f, "Parse error: {}", error),
        }
    }
}

impl Error for SendFrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SendFrameError::IoError(error) => Some(error),
            SendFrameError::ParseError(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum ReceiveFrameError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for ReceiveFrameError {
    fn from(value: std::io::Error) -> Self {
        ReceiveFrameError::IoError(value)
    }
}

impl fmt::Display for ReceiveFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReceiveFrameError::IoError(error) => write!(f, "Error receiving frame: {}", error),
        }
    }
}

impl Error for ReceiveFrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ReceiveFrameError::IoError(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum HandshakeError {
    IoError(std::io::Error),
    SendFrameError(SendFrameError),
    ReceiveFrameError(ReceiveFrameError),
}

impl From<ReceiveFrameError> for HandshakeError {
    fn from(value: ReceiveFrameError) -> Self {
        HandshakeError::ReceiveFrameError(value)
    }
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandshakeError::IoError(error) => write!(f, "Io error: {}", error),
            HandshakeError::ReceiveFrameError(error) => {
                write!(f, "Error receiving frames: {}", error)
            }
            HandshakeError::SendFrameError(error) => write!(f, "Error sending frames: {}", error),
        }
    }
}

impl Error for HandshakeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HandshakeError::IoError(error) => Some(error),
            HandshakeError::ReceiveFrameError(error) => Some(error),
            HandshakeError::SendFrameError(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    MatchError(String),
    Utf8(std::str::Utf8Error),
    ParseIntError(ParseIntError),
    FromHexError(FromHexError),
}

impl From<std::str::Utf8Error> for ParseError {
    fn from(value: std::str::Utf8Error) -> Self {
        ParseError::Utf8(value)
    }
}

impl From<FromHexError> for ParseError {
    fn from(value: FromHexError) -> Self {
        ParseError::FromHexError(value)
    }
}

impl From<ParseIntError> for ParseError {
    fn from(value: ParseIntError) -> Self {
        ParseError::ParseIntError(value)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Io(error) => write!(f, "Parse error: {}", error),
            ParseError::MatchError(from) => write!(f, "No matches for: {}", from),
            ParseError::Utf8(error) => write!(f, "utf-8 parse error: {}", error),
            ParseError::ParseIntError(error) => write!(f, "erro parsing int: {error}"),
            ParseError::FromHexError(error) => write!(f, "error parsing from hex: {error}"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseError::Io(error) => Some(error),
            ParseError::MatchError(s) => None,
            ParseError::Utf8(error) => Some(error),
            ParseError::ParseIntError(error) => Some(error),
            ParseError::FromHexError(error) => Some(error),
        }
    }
}
