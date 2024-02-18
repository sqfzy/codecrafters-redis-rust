use crate::{
    error::{RedisError, RedisResult},
    frame::Frame,
};
use async_trait::async_trait;
use bytes::{BufMut, Bytes};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[async_trait]
pub trait FrameHandler {
    async fn read_frame(&mut self) -> RedisResult<Frame>;
    async fn write_frame(&mut self, frame: Frame) -> RedisResult<()>;
}

#[async_trait]
impl FrameHandler for TcpStream {
    async fn read_frame(&mut self) -> RedisResult<Frame> {
        let len = get_len(self, b'*').await? as usize;
        if len == 0 {
            return Err(RedisError::EndofStream);
        }
        let mut frames: Vec<Frame> = Vec::with_capacity(len);

        for _ in 0..len {
            let len = get_len(self, b'$').await? as usize;
            let bytes = get_exact(self, len).await?;
            let frame = Frame::Bulk(bytes);
            frames.push(frame);
        }

        Ok(Frame::Array(frames))
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

    // Vail command format:
    // $12\r\n
    // *12\r\n
    // ...
    // Invail command format:
    // $12, $12\r, $12\n
    let mut len_buf = vec![];
    loop {
        let mut byte = stream.read_u8().await?;
        if byte != b'\r' {
            len_buf.put_u8(byte);
            continue;
        }

        byte = stream.read_u8().await?;
        if byte != b'\n' {
            len_buf.put_u8(b'\r');
            len_buf.put_u8(byte);
            continue;
        }

        let len = String::from_utf8(len_buf)
            .map_err(|_| RedisError::SyntaxErr)?
            .parse::<u64>()
            .map_err(|_| RedisError::SyntaxErr)?;

        // not allow 0 length
        if len == 0 {
            return Err(RedisError::SyntaxErr);
        }

        return Ok(len);
    }
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
