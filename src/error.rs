use tracing::error;

pub type RedisResult<T> = std::result::Result<T, RedisError>;

#[derive(thiserror::Error, Debug)]
pub enum RedisError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Syntax error! {0}")]
    SyntaxErr(String), // return error msg to client
    #[error("This function have not completed yet!")]
    InComplete, // return error msg to client
}
// 返回给客户端的错误消息
// 服务器的日志

impl RedisError {
    pub fn syntax_err(msg: &str) -> Self {
        let err = Self::SyntaxErr(msg.to_string());
        error!("{}", err.to_string());
        err
    }

    pub fn incomplete_err() -> Self {
        // error!("{}", Self::InComplete.to_string());
        Self::InComplete
    }
}
