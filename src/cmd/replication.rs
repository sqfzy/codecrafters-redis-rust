use super::CmdExecutor;
use crate::{db::Db, error::RedisResult, Frame, CONFIG};

pub struct Replconf;

impl CmdExecutor for Replconf {
    async fn execute(self, _db: Db) -> RedisResult<Frame> {
        Ok(Frame::Simple("OK".to_string()))
    }
}

pub struct Psync;

impl CmdExecutor for Psync {
    async fn execute(self, _db: Db) -> RedisResult<Frame> {
        Ok(Frame::Simple(format!(
            "+FULLRESYNC {} 0\r\n",
            CONFIG.replid
        )))
    }
}
