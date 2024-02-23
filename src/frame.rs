use crate::{
    cmd::{self, CmdExecutor, Section},
    error::{RedisError, RedisResult},
};
use bytes::{Buf, Bytes, BytesMut};
use std::{io::Read, time::Duration, usize};
use tracing::debug;

#[derive(Clone, Debug, Default, PartialEq)]
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

impl From<Vec<Bytes>> for Frame {
    fn from(value: Vec<Bytes>) -> Self {
        Frame::Array(value.into_iter().map(Frame::Bulk).collect())
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
            b"set" => return Ok(Box::new(cmd::Set::try_from(bulks)?) as Box<dyn CmdExecutor>),
            b"info" => return Ok(Box::new(cmd::Info::try_from(bulks)?) as Box<dyn CmdExecutor>),
            b"replconf" => return Ok(Box::new(cmd::Replconf)),
            b"psync" => return Ok(Box::new(cmd::Psync)),
            _ => {}
        }

        Err(RedisError::SyntaxErr)
    }
}

impl TryFrom<Vec<Bytes>> for cmd::Set {
    type Error = RedisError;
    fn try_from(bulks: Vec<Bytes>) -> Result<Self, Self::Error> {
        debug!("parsing to Set");

        let len = bulks.len();
        if len >= 3 {
            let key = bulks[1].clone();
            let value = bulks[2].clone();

            if len == 3 {
                return Ok(cmd::Set {
                    key,
                    value,
                    expire: None,
                    keep_ttl: false,
                });
            }
            if len == 4 {
                match bulks[4].to_ascii_lowercase().as_slice() {
                    b"keepttl" => {
                        return Ok(cmd::Set {
                            key,
                            value,
                            expire: None,
                            keep_ttl: true,
                        });
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
                        return Ok(cmd::Set {
                            key,
                            value,
                            expire: Some(Duration::from_secs(expire)),
                            keep_ttl: false,
                        });
                    }
                    b"px" => {
                        return Ok(cmd::Set {
                            key,
                            value,
                            expire: Some(Duration::from_millis(expire)),
                            keep_ttl: false,
                        });
                    }
                    _ => {}
                }
            }
        }

        Err(RedisError::SyntaxErr)
    }
}

impl TryFrom<Vec<Bytes>> for cmd::Info {
    type Error = RedisError;
    fn try_from(value: Vec<Bytes>) -> Result<Self, Self::Error> {
        debug!("parsing to Info");

        let len = value.len();
        if len == 1 {
            return Ok(cmd::Info {
                sections: Section::Default,
            });
        }
        if len == 2 {
            let section = value[1].clone();
            return Ok(cmd::Info {
                sections: section.try_into()?,
            });
        }
        if len > 2 && len <= 14 {
            let sections = value[1..].to_vec();
            return Ok(cmd::Info {
                sections: sections.try_into()?,
            });
        }

        Err(RedisError::SyntaxErr)
    }
}
