pub type RedisResult<T> = std::result::Result<T, RedisError>;

#[derive(thiserror::Error, Debug)]
pub enum RedisError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Syntax Error")]
    SyntaxErr,
    #[error("End of Stream")]
    EndofStream,
}
