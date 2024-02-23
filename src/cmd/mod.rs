mod command;
mod replication;

use crate::{db::DbManipulate, error::RedisResult, Frame};
use async_trait::async_trait;
pub use command::*;
pub use replication::*;

#[async_trait]
pub trait CmdExecutor: Send {
    async fn execute(&mut self, db: &mut dyn DbManipulate) -> RedisResult<Frame>;
}

// *2\r\n$7\r\nCOMMAND\r\n$4\r\nDOCS\r\n
pub struct Command;
#[async_trait]
impl CmdExecutor for Command {
    async fn execute(&mut self, _db: &mut dyn DbManipulate) -> RedisResult<Frame> {
        Ok(Frame::Array(vec![]))
    }
}
