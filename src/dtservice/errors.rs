#[derive(Debug, thiserror::Error)]
pub enum DtServiceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
