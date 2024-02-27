use super::CmdExecutor;
use crate::{db::Db, error::RedisResult, Frame, CONFIG};

pub struct Replconf;

#[async_trait::async_trait]
impl CmdExecutor for Replconf {
    async fn execute(self: Box<Self>, _db: &mut Db) -> RedisResult<Frame> {
        Ok(Frame::Simple("OK".to_string()))
    }
}

pub struct Psync;

#[async_trait::async_trait]
impl CmdExecutor for Psync {
    async fn execute(self: Box<Self>, _db: &mut Db) -> RedisResult<Frame> {
        Ok(Frame::Simple(format!(
            "+FULLRESYNC {} 0\r\n",
            CONFIG.replid
        )))
    }
}
