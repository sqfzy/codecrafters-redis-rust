mod command;
mod replication;

use crate::db::Db;
use crate::{error::RedisResult, Frame};
pub use command::*;
pub use replication::*;

pub trait CmdExecutor: Send {
    async fn execute(self, db: Db) -> RedisResult<Frame>;
}
