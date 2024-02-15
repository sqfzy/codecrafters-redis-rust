pub type RedisResult<T> = std::result::Result<T, RedisError>;

#[derive(thiserror::Error, Debug)]
pub enum RedisError {
    #[error("Invaild command format")]
    InvaildFrame,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
