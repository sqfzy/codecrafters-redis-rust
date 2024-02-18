use crate::{
    db::DbManipulate,
    error::{RedisError, RedisResult},
    Frame,
};
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

pub struct Info {
    pub sections: Section,
}

pub enum Section {
    Array(Vec<Section>),
    // all: Return all sections (excluding module generated ones)
    All,
    // default: Return only the default set of sections
    Default,
    // everything: Includes all and modules
    Everything,
    // server: General information about the Redis server
    Server,
    // clients: Client connections section
    Clients,
    // memory: Memory consumption related information
    Memory,
    // persistence: RDB and AOF related information
    Persistence,
    // stats: General statistics
    Stats,
    // replication: Master/replica replication information
    Replication,
    // cpu: CPU consumption statistics
    Cpu,
    // commandstats: Redis command statistics
    CommandStats,
    // latencystats: Redis command latency percentile distribution statistics
    LatencyStats,
    // sentinel: Redis Sentinel section (only applicable to Sentinel instances)
    Sentinel,
    // cluster: Redis Cluster section
    Cluster,
    // modules: Modules section
    Modules,
    // keyspace: Database related statistics
    Keyspace,
    // errorstats: Redis error statistics
    ErrorStats,
}
impl TryFrom<Bytes> for Section {
    type Error = RedisError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let value = value.to_ascii_lowercase();
        match value.as_slice() {
            b"replication" => Ok(Section::Replication),
            // TODO:
            _ => Err(RedisError::SyntaxErr),
        }
    }
}
impl TryFrom<Vec<Bytes>> for Section {
    type Error = RedisError;

    fn try_from(value: Vec<Bytes>) -> Result<Self, Self::Error> {
        let mut sections = Vec::with_capacity(value.len());
        for section in value {
            sections.push(section.try_into()?);
        }
        Ok(Section::Array(sections))
    }
}

#[async_trait]
impl CmdExecutor for Info {
    async fn execute(&mut self, db: &mut dyn DbManipulate) -> RedisResult<Frame> {
        match self.sections {
            Section::Replication => Ok(Frame::Bulk(Bytes::from_static(
                b"# Replication\r\nrole:master\r\n",
            ))),
            // TODO:
            _ => Err(RedisError::InComplete),
        }
    }
}
