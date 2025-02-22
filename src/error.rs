use openai_api_rs::v1::chat_completion::ChatCompletionResponse;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Notify error: {0}")]
    NotifyError(#[from] notify::Error),

    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Serde YAML error: {0}")]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error("Stream closed")]
    StreamClosedError,

    #[error("No API key provided")]
    NoApiKeyError,

    #[error("Api error: {0:?}")]
    ApiError(#[from] openai_api_rs::v1::error::APIError),

    #[error("Document cannot be processed: {0:?}")]
    DoesNotProcessError(Option<ChatCompletionResponse>),

    #[error("File type not supported: {0:?}")]
    UnsupportedFileTypeError(PathBuf),

    #[error("Unexpected: {0}")]
    UnexpectedError(String),

    #[error("EncodingError")]
    EncodingError,

    #[error("Error reading metadata: {0}")]
    MetadataInError(String),

    #[error("Error writing metadata: {0}")]
    MetadataOutError(String),

    #[error("RedirectIOError")]
    RedirectIOError,

    #[error("Not a valid PDF")]
    NotValidPdfError,

    #[error("File disappeared: {0:?}")]
    FileDisappearedError(PathBuf),

    #[error("Cannot convert PDF: {0}")]
    PdfConversionError(String),

    #[error("Dependency missing: {0}")]
    DependencyMissingError(String),

    #[error("File already present: {0:?}")]
    FileExists(PathBuf),

    #[error("Skeleton error")]
    SkelError,

    #[error("Channel send error: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<()>),

    #[error("Join task error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Timeout: {0}")]
    TimeoutError(#[from] tokio::time::error::Elapsed),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
