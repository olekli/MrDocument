#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Notify error: {0}")]
    NotifyError(#[from] notify::Error),

    #[error("Stream closed")]
    StreamClosedError,

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
