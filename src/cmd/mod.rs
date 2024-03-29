mod command;
mod replication;

use crate::db::Db;
use crate::frame::Frame;
pub use command::*;
pub use replication::*;

#[async_trait::async_trait]
pub trait CmdExecutor: Send {
    async fn execute(self: Box<Self>, db: &mut Db) -> anyhow::Result<Frame>;
}
