use crate::{
    error::{RedisError, RedisResult},
    frame::Frame,
};
use bytes::{BufMut, Bytes};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{debug, field::debug};

pub trait FrameHandler {
    async fn read_frame(&mut self) -> RedisResult<Frame>;
    async fn write_frame(&mut self, frame: Frame) -> RedisResult<()>;
}

impl FrameHandler for TcpStream {
    async fn read_frame(&mut self) -> RedisResult<Frame> {
        // match self.read_u8().await? {
        //     b'*' => {
        //         debug!("reading array");
        //
        //         let len = get_len(self, b'*').await? as usize;
        //         let mut frames: Vec<Frame> = Vec::with_capacity(len);
        //
        //         for _ in 0..len {
        //             println!("debug1");
        //             let frame = read_value(self).await?;
        //             frames.push(frame);
        //         }
        //
        //         debug!(?frames);
        //
        //         Ok(Frame::Array(frames))
        //     }
        //     prefix @ b'+' | prefix @ b'-' | prefix @ b':' | prefix @ b'$' => {
        //         read_value(self, prefix).await
        //     }
        //     _ => Err(RedisError::SyntaxErr),
        // }
        todo!()
    }

    async fn write_frame(&mut self, frame: Frame) -> RedisResult<()> {
        match frame {
            // *<len>\r\n<Frame>...
            Frame::Array(frames) => {
                let header = format!("*{}\r\n", frames.len());
                self.write_all(header.as_bytes()).await?;

                for frame in frames {
                    write_value(self, frame).await?;
                }
            }
            _ => write_value(self, frame).await?,
        }

        Ok(())
    }
}

async fn read_line(stream: &mut TcpStream) -> RedisResult<Bytes> {
    // match stream.read_u8().await {
    //     Ok(byte) => {
    //         if byte != prefix {
    //             return Err(RedisError::SyntaxErr);
    //         }
    //     }
    //     Err(e) => {
    //         if e.kind() == std::io::ErrorKind::UnexpectedEof {
    //             return Ok(0);
    //         }
    //     }
    // }
    
    let mut buf = vec![];
    loop {
        let byte = stream.read_u8().await?;
        if byte == b'\r' {
            let byte = stream.read_u8().await?;
            if byte == b'\n' {
                break;
            }
            buf.put_u8(b'\r');
            buf.put_u8(byte);
        }
        buf.put_u8(byte);
    }

    // Ok(buf.into())
    todo!()
}

async fn get_len(stream: &mut TcpStream, prefix: u8) -> RedisResult<u64> {
    match stream.read_u8().await {
        Ok(byte) => {
            if byte != prefix {
                return Err(RedisError::SyntaxErr);
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return Ok(0);
            }
        }
    }

    let len = read_line(stream).await?;
    let len = String::from_utf8(len.into())
        .map_err(|_| RedisError::SyntaxErr)?
        .parse::<u64>()
        .map_err(|_| RedisError::SyntaxErr)?;

    // not allow 0 length
    if len == 0 {
        return Err(RedisError::SyntaxErr);
    }

    return Ok(len);
}

async fn get_exact(stream: &mut TcpStream, n: usize) -> RedisResult<Bytes> {
    let mut buf = vec![0u8; n];
    stream.read_exact(&mut buf).await?;

    let mut new_line = [0u8; 2];
    stream.read_exact(&mut new_line).await?;
    if new_line != "\r\n".as_bytes() {
        return Err(RedisError::SyntaxErr);
    }

    Ok(buf.into())
}

async fn write_value(stream: &mut TcpStream, frame: Frame) -> RedisResult<()> {
    match frame {
        // +<str>\r\n
        Frame::Simple(s) => {
            let msg = format!("+{}\r\n", s);
            stream.write_all(msg.as_bytes()).await?;
            stream.flush().await?;
        }
        // -<err>\r\n
        Frame::Error(e) => {
            let msg = format!("+{}\r\n", e);
            stream.write_all(msg.as_bytes()).await?;
            stream.flush().await?;
        }
        // :<num>\r\n
        Frame::Integer(n) => {
            let msg = format!("+{}\r\n", n);
            stream.write_all(msg.as_bytes()).await?;
            stream.flush().await?;
        }
        // $<len>\r\n<bytes>\r\n
        Frame::Bulk(b) => {
            let header = format!("${}\r\n", b.len());
            stream.write_all(header.as_bytes()).await?;
            stream.write_all(&b).await?;
            stream.write_all(b"\r\n").await?;
            stream.flush().await?;
        }
        // $-1\r\n
        Frame::Null => {
            stream.write_all(b"$-1\r\n").await?;
            stream.flush().await?;
        }
        Frame::Array(_) => unreachable!(),
    }

    Ok(())
}

// async fn read_value(stream: &mut TcpStream) -> RedisResult<Frame> {
//     match stream.read_u8().await? {
//         b'+' => read_value(stream, b'+').await,
//         b'-' => read_value(stream, b'-').await,
//         b':' => read_value(stream, b':').await,
//         b'$' => read_value(stream, b'$').await,
//         b'*' => read_value(stream, b'*').await,
//         _ => Err(RedisError::SyntaxErr),
//     }
// }

async fn read_value(stream: &mut TcpStream, prefix: u8) -> RedisResult<Frame> {
    match prefix {
        b'+' => {
            debug!("reading simple");

            let mut buf = vec![];
            loop {
                let byte = stream.read_u8().await?;
                if byte == b'\r' {
                    let byte = stream.read_u8().await?;
                    if byte == b'\n' {
                        break;
                    }
                }
                buf.push(byte);
            }
            let res = Frame::Simple(String::from_utf8(buf).map_err(|_| RedisError::SyntaxErr)?);

            debug!(?res);

            Ok(res)
        }
        b'-' => {
            let mut buf = vec![];
            loop {
                let byte = stream.read_u8().await?;
                if byte == b'\r' {
                    let byte = stream.read_u8().await?;
                    if byte == b'\n' {
                        break;
                    }
                }
                buf.push(byte);
            }
            let res = Frame::Error(String::from_utf8(buf).map_err(|_| RedisError::SyntaxErr)?);

            debug!(?res);

            Ok(res)
        }
        b':' => {
            let mut buf = vec![];
            loop {
                let byte = stream.read_u8().await?;
                if byte == b'\r' {
                    let byte = stream.read_u8().await?;
                    if byte == b'\n' {
                        break;
                    }
                }
                buf.push(byte);
            }
            let res = Frame::Integer(
                String::from_utf8(buf)
                    .map_err(|_| RedisError::SyntaxErr)?
                    .parse::<u64>()
                    .map_err(|_| RedisError::SyntaxErr)?,
            );

            debug!(?res);

            Ok(res)
        }
        b'$' => {
            let len = get_len(stream, b'$').await? as usize;
            let bytes = get_exact(stream, len).await?;
            let res = Frame::Bulk(bytes);

            debug!(?res);

            Ok(res)
        }
        b'*' => unreachable!(),
        _ => Err(RedisError::SyntaxErr),
    }
}
