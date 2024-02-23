use crate::{
    cmd::{self, CmdExecutor, Section},
    error::{RedisError, RedisResult},
};
use bytes::{Buf, Bytes, BytesMut};
use std::{io::Read, time::Duration, usize};
use tokio_util::codec::{AnyDelimiterCodec, Decoder, Encoder, LinesCodec};
use tracing::debug;

pub struct FrameCodec;

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

impl Decoder for FrameCodec {
    type Item = Frame;
    type Error = RedisError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match src[0] {
            b'*' => {
                let len = read_line(src)?;
                let len = String::from_utf8(len.into())
                    .map_err(|_| RedisError::SyntaxErr)?
                    .parse::<u64>()
                    .map_err(|_| RedisError::SyntaxErr)?;

                let mut frames = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    let frame = read_value(src)?;
                    frames.push(frame);
                }

                Ok(Some(Frame::Array(frames)))
            }
            b'+' | b'-' | b':' | b'$' => Ok(Some(read_value(src)?)),
            _ => Err(RedisError::SyntaxErr),
        }
    }
}

fn read_line(src: &mut BytesMut) -> RedisResult<Option<Bytes>> {
    let mut pos = None;
    for i in 0..src.len() {
        if Some("\r\n".as_ref()) == src.get(i..=i + 1) {
            pos = Some(i);
            break;
        }
    }

    let mut line = src.split_to(pos as usize + 1);
    // remove the last \r\n
    line.truncate(line.len() - 2);

    Ok(line.freeze())
}

fn read_value(src: &mut BytesMut) -> RedisResult<Frame> {
    match src.get_u8() {
        b'+' => {
            let line = read_line(src)?;
            let msg = String::from_utf8(line.to_vec()).map_err(|_| RedisError::SyntaxErr)?;
            Ok(Frame::Simple(msg))
        }
        b'-' => {
            let line = read_line(src)?;
            let msg = String::from_utf8(line.to_vec()).map_err(|_| RedisError::SyntaxErr)?;
            Ok(Frame::Error(msg))
        }
        b':' => {
            let line = read_line(src)?;
            let num = String::from_utf8(line.to_vec())
                .map_err(|_| RedisError::SyntaxErr)?
                .parse::<u64>()
                .map_err(|_| RedisError::SyntaxErr)?;
            Ok(Frame::Integer(num))
        }
        b'$' => {
            let len = read_line(src)?;
            let len = String::from_utf8(len.into())
                .map_err(|_| RedisError::SyntaxErr)?
                .parse::<u64>()
                .map_err(|_| RedisError::SyntaxErr)? as usize;

            // // not allow 0 length
            // if len == 0 {
            //     return Err(RedisError::SyntaxErr);
            // }

            let mut buf = BytesMut::with_capacity(len);
            src.reader().read_exact(&mut buf).unwrap();
            Ok(Frame::Bulk(buf.freeze()))
        }
        b'*' => unreachable!(),
        _ => Err(RedisError::SyntaxErr),
    }
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
        Frame::Array(value.into_iter().map(|b| Frame::Bulk(b)).collect())
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
            b"info" => {
                debug!("parsing to Info");

                if len == 1 {
                    return Ok(Box::new(cmd::Info {
                        sections: Section::Default,
                    }));
                }
                if len == 2 {
                    let section = bulks[1].clone();
                    return Ok(Box::new(cmd::Info {
                        sections: section.try_into()?,
                    }));
                }
                if len > 2 && len <= 14 {
                    let sections = bulks[1..].to_vec();
                    return Ok(Box::new(cmd::Info {
                        sections: sections.try_into()?,
                    }));
                }
            }
            _ => {}
        }

        Err(RedisError::SyntaxErr)
    }
}
