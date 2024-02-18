use crate::{db::DbManipulate, error::RedisResult, Frame};
use async_trait::async_trait;
use bytes::Bytes;
use std::time::Duration;

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

// *1\r\n$4\r\nping\r\n
// return: +PONG\r\n
pub struct Ping;
#[async_trait]
impl CmdExecutor for Ping {
    async fn execute(&mut self, _db: &mut dyn DbManipulate) -> RedisResult<Frame> {
        Ok(Frame::Simple("PONG".to_string()))
    }
}

// *2\r\n$4\r\necho\r\n$3\r\nhey\r\n
// return: $3\r\nhey\r\n
pub struct Echo {
    pub msg: Bytes,
}
#[async_trait]
impl CmdExecutor for Echo {
    async fn execute(&mut self, _db: &mut dyn DbManipulate) -> RedisResult<Frame> {
        Ok(Frame::Bulk(self.msg.clone()))
    }
}

// https://redis.io/commands/get/
// *2\r\n$3\r\nget\r\n$3\r\nkey\r\n
// return(the key exesits): $5\r\nvalue\r\n
// return(the key doesn't exesit): $-1\r\n
pub struct Get {
    pub key: Bytes,
}
#[async_trait]
impl CmdExecutor for Get {
    async fn execute(&mut self, db: &mut dyn DbManipulate) -> RedisResult<Frame> {
        Ok(match db.get(self.key.clone()).await {
            Some(value) => Frame::Bulk(value),
            None => Frame::Null,
        })
    }
}

pub struct Set {
    pub key: Bytes,
    pub value: Bytes,
    pub expire: Option<Duration>,
    pub keep_ttl: bool,
}
#[async_trait]
impl CmdExecutor for Set {
    async fn execute(&mut self, db: &mut dyn DbManipulate) -> RedisResult<Frame> {
        db.set(
            self.key.clone(),
            self.value.clone(),
            self.expire,
            self.keep_ttl,
        )
        .await;
        Ok(Frame::Simple("OK".to_string()))
    }
}
