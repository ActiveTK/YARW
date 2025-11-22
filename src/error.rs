use thiserror::Error;
use std::string::FromUtf8Error;

#[derive(Error, Debug)]
pub enum RsyncError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid option: {0}")]
    InvalidOption(String),

    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),

    #[error("Incompatible protocol version: local={local}, remote={remote}")]
    #[allow(dead_code)]
    IncompatibleProtocol { local: i32, remote: i32 },

    #[error("Path is not valid UTF-8: {0:?}")]
    InvalidPath(std::path::PathBuf),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Remote execution error: {0}")]
    RemoteExec(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Checksum mismatch for file: {0}")]
    #[allow(dead_code)]
    ChecksumMismatch(String),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("General error: {0}")]
    Other(String),

    #[error("Unknown error")]
    #[allow(dead_code)]
    Unknown,
}

impl From<toml::de::Error> for RsyncError {
    fn from(err: toml::de::Error) -> Self {
        RsyncError::Config(err.to_string())
    }
}

impl From<anyhow::Error> for RsyncError {
    fn from(err: anyhow::Error) -> Self {
        RsyncError::Other(err.to_string())
    }
}

impl From<ssh2::Error> for RsyncError {
    fn from(err: ssh2::Error) -> Self {
        RsyncError::Network(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, RsyncError>;