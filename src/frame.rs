use std::time::Duration;

use crate::{
    cmd,
    cmd::CmdExecutor,
    error::{RedisError, RedisResult},
};
use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, info};

#[derive(Clone, Debug, Default)]
pub enum Frame {
    Simple(String), // +<str>\r\n
    Error(String),  // -<err>\r\n
    Integer(u64),   // :<num>\r\n
    Bulk(Bytes),    // $<len>\r\n<bytes>\r\n
    #[default]
    Null, // $-1\r\n
    Array(Vec<Frame>), // *<len>\r\n<Frame>...
}

impl TryInto<Vec<Bytes>> for Frame {
    type Error = RedisError;

    fn try_into(self) -> Result<Vec<Bytes>, RedisError> {
        if let Frame::Array(frames) = self {
            frames
                .into_iter()
                .map(|frame| match frame {
                    Frame::Bulk(bytes) => Ok(bytes),
                    _ => Err(RedisError::SyntaxErr),
                })
                .collect()
        } else {
            Err(RedisError::SyntaxErr)
        }
    }
}

impl Frame {
    pub fn parse_cmd(self) -> RedisResult<Box<dyn CmdExecutor>> {
        let bulks: Vec<Bytes> = self.try_into()?;
        let len = bulks.len();

        let cmd_name = bulks[0].to_ascii_lowercase();
        match cmd_name.as_slice() {
            b"command" => return Ok(Box::new(cmd::Command)),
            b"ping" => {
                debug!("parsing to Ping");

                if len == 1 {
                    return Ok(Box::new(cmd::Ping));
                }
            }
            b"echo" => {
                debug!("parsing to Echo");

                if len == 2 {
                    return Ok(Box::new(cmd::Echo {
                        msg: bulks[1].clone(),
                    }));
                }
            }
            b"get" => {
                debug!("parsing to Get");

                if len == 2 {
                    return Ok(Box::new(cmd::Get {
                        key: bulks[1].clone(),
                    }));
                }
            }
            b"set" => {
                debug!("parsing to Set");

                if len >= 3 {
                    let key = bulks[1].clone();
                    let value = bulks[2].clone();

                    if len == 3 {
                        return Ok(Box::new(cmd::Set {
                            key,
                            value,
                            expire: None,
                            keep_ttl: false,
                        }));
                    }
                    if len == 4 {
                        match bulks[4].to_ascii_lowercase().as_slice() {
                            b"keepttl" => {
                                return Ok(Box::new(cmd::Set {
                                    key,
                                    value,
                                    expire: None,
                                    keep_ttl: true,
                                }));
                            }
                            _ => {}
                        }
                    }
                    if len == 5 {
                        let expire_unit = bulks[3].to_ascii_lowercase();
                        let expire = String::from_utf8(bulks[4].to_vec())
                            .map_err(|_| RedisError::SyntaxErr)?
                            .parse::<u64>()
                            .map_err(|_| RedisError::SyntaxErr)?;

                        if expire == 0 {
                            return Err(RedisError::SyntaxErr);
                        }

                        match expire_unit.as_slice() {
                            b"ex" => {
                                return Ok(Box::new(cmd::Set {
                                    key,
                                    value,
                                    expire: Some(Duration::from_secs(expire)),
                                    keep_ttl: false,
                                }));
                            }
                            b"px" => {
                                return Ok(Box::new(cmd::Set {
                                    key,
                                    value,
                                    expire: Some(Duration::from_millis(expire)),
                                    keep_ttl: false,
                                }));
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }

        Err(RedisError::SyntaxErr)
    }
}
