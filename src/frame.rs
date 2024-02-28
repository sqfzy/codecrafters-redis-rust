use crate::{
    cmd::{self, CmdExecutor, Section},
    util::{bytes_to_string, bytes_to_u64},
};
use anyhow::{anyhow, bail, Error, Result};
use bytes::Bytes;
use std::time::Duration;
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
    type Error = Error;

    fn try_into(self) -> Result<Vec<Bytes>, Error> {
        if let Frame::Array(frames) = self {
            frames
                .into_iter()
                .map(|frame| match frame {
                    Frame::Bulk(bytes) => Ok(bytes),
                    _ => panic!("invaild frame"),
                })
                .collect()
        } else {
            panic!("invaild frame");
        }
    }
}

impl From<Vec<Bytes>> for Frame {
    fn from(value: Vec<Bytes>) -> Self {
        Frame::Array(value.into_iter().map(Frame::Bulk).collect())
    }
}

impl Frame {
    pub fn parse_cmd(self) -> Result<Box<dyn CmdExecutor>> {
        let bulks: Vec<Bytes> = self.try_into()?;
        let len = bulks.len();
        let cmd_name = bytes_to_string(bulks[0].clone())?;
        match cmd_name.to_lowercase().as_str() {
            "command" => return Ok(Box::new(cmd::Command)),
            "ping" => {
                if len == 1 {
                    return Ok(Box::new(cmd::Ping));
                }
            }
            "echo" => {
                if len == 2 {
                    return Ok(Box::new(cmd::Echo {
                        msg: bulks[1].clone(),
                    }));
                }
            }
            "get" => {
                if len == 2 {
                    return Ok(Box::new(cmd::Get {
                        key: bytes_to_string(bulks[1].clone())?,
                    }));
                }
                bail!("ERR wrong number of arguments for 'get' command")
            }
            "set" => return Ok(Box::new(cmd::Set::try_from(bulks)?) as Box<dyn CmdExecutor>),
            "info" => return Ok(Box::new(cmd::Info::try_from(bulks)?) as Box<dyn CmdExecutor>),
            "replconf" => return Ok(Box::new(cmd::Replconf)),
            "psync" => return Ok(Box::new(cmd::Psync)),
            _ => {}
        }

        Err(anyhow!(
            // "ERR unknown command {}, with args beginning with:",
            "ERR unknown command {}",
            cmd_name
        ))
    }
}

impl TryFrom<Vec<Bytes>> for cmd::Set {
    type Error = Error;
    fn try_from(bulks: Vec<Bytes>) -> Result<Self, Self::Error> {
        let len = bulks.len();
        if len >= 3 {
            let key = bytes_to_string(bulks[1].clone())?;
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
                let expire = bytes_to_u64(bulks[4].clone())?;

                if expire == 0 {
                    bail!("ERR invalid expire time in 'set' command")
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

        Err(anyhow!("ERR syntax error"))
    }
}

impl TryFrom<Vec<Bytes>> for cmd::Info {
    type Error = Error;
    fn try_from(value: Vec<Bytes>) -> Result<Self, Self::Error> {
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

        Err(anyhow!("ERR syntax error"))
    }
}
