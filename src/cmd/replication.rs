use super::CmdExecutor;
use crate::{db::DbManipulate, error::RedisResult, Frame, CONFIG};
use async_trait::async_trait;

pub struct Replconf;

#[async_trait]
impl CmdExecutor for Replconf {
    async fn execute(&mut self, _db: &mut dyn DbManipulate) -> RedisResult<Frame> {
        Ok(Frame::Simple("OK".to_string()))
    }
}

pub struct Psync;

#[async_trait]
impl CmdExecutor for Psync {
    async fn execute(&mut self, _db: &mut dyn DbManipulate) -> RedisResult<Frame> {
        Ok(Frame::Simple(format!(
            "+FULLRESYNC {} 0\r\n",
            CONFIG.replid
        )))
    }
}
