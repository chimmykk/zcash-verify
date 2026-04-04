use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Key error: {0}")]
    Key(String),

    #[error("Scan error: {0}")]
    Scan(String),

    #[error("Proof error: {0}")]
    Proof(String),

    #[error("Verification error: {0}")]
    Verify(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Tonic(#[from] tonic::transport::Error),

    #[error("gRPC status: {0}")]
    TonicStatus(#[from] tonic::Status),
}

pub type Result<T> = std::result::Result<T, Error>;
