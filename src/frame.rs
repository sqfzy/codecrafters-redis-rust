use crate::error::{RedisError, RedisResult};
use bytes::{BufMut, Bytes};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[derive(Clone, Debug)]
pub enum Frame<'a> {
    Simple(&'a str),       // +
    Error(&'a str),        // -
    Integer(u64),          // :
    Bulk(&'a [u8]),        // $
    Null,                  // $-
    Array(Vec<Frame<'a>>), // *
}

pub trait FrameHandler {
    async fn get_array_len(&mut self) -> RedisResult<u64>;
    async fn get_bulk_len(&mut self) -> RedisResult<u64>;
    async fn get_exact(&mut self, n: usize) -> RedisResult<Bytes>;
    async fn send_frame(&mut self, frame: Frame<'_>) -> RedisResult<()>;
}

impl FrameHandler for TcpStream {
    async fn get_array_len(&mut self) -> RedisResult<u64> {
        match self.read_u8().await {
            Ok(byte) => {
                if byte != b'*' {
                    return Err(RedisError::InvaildFrame);
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
            let mut byte = self.read_u8().await?;
            if byte != b'\r' {
                len_buf.put_u8(byte);
                continue;
            }

            byte = self.read_u8().await?;
            if byte != b'\n' {
                len_buf.put_u8(b'\r');
                len_buf.put_u8(byte);
                continue;
            }

            let len = String::from_utf8(len_buf)
                .map_err(|_| RedisError::InvaildFrame)?
                .parse::<u64>()
                .map_err(|_| RedisError::InvaildFrame)?;

            // not allow 0 length
            if len == 0 {
                return Err(RedisError::InvaildFrame);
            }

            return Ok(len);
        }
    }

    async fn get_bulk_len(&mut self) -> RedisResult<u64> {
        if self.read_u8().await? != b'$' {
            return Err(RedisError::InvaildFrame);
        }

        // Vail command format:
        // $12\r\n
        // *12\r\n
        // ...
        // Invail command format:
        // $12, $12\r, $12\n
        let mut len_buf = vec![];
        loop {
            let mut byte = self.read_u8().await?;
            if byte != b'\r' {
                len_buf.put_u8(byte);
                continue;
            }

            byte = self.read_u8().await?;
            if byte != b'\n' {
                len_buf.put_u8(b'\r');
                len_buf.put_u8(byte);
                continue;
            }

            let len = String::from_utf8(len_buf)
                .map_err(|_| RedisError::InvaildFrame)?
                .parse::<u64>()
                .map_err(|_| RedisError::InvaildFrame)?;

            // not allow 0 length
            if len == 0 {
                return Err(RedisError::InvaildFrame);
            }

            return Ok(len);
        }
    }

    async fn get_exact(&mut self, n: usize) -> RedisResult<Bytes> {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf).await?;

        let mut new_line = [0u8; 2];
        self.read_exact(&mut new_line).await?;
        if new_line != "\r\n".as_bytes() {
            return Err(RedisError::InvaildFrame);
        }

        Ok(buf.into())
    }

    async fn send_frame(&mut self, frame: Frame<'_>) -> RedisResult<()> {
        match frame {
            // *<len>\r\n<Frame>...
            Frame::Array(frames) => {
                let header = format!("*{}\r\n", frames.len());
                self.write_all(header.as_bytes()).await?;

                for frame in frames {
                    send_value(self, frame).await?;
                }
            }
            _ => send_value(self, frame).await?,
        }

        Ok(())
    }
}

async fn send_value(stream: &mut TcpStream, frame: Frame<'_>) -> RedisResult<()> {
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
            stream.write_all(b).await?;
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

// pub async fn get_array_len(stream: &mut TcpStream) -> RedisResult<u64> {
//     match stream.read_u8().await {
//         Ok(byte) => {
//             if byte != b'*' {
//                 return Err(RedisError::InvaildFrame);
//             }
//         }
//         Err(e) => {
//             if e.kind() == std::io::ErrorKind::UnexpectedEof {
//                 return Ok(0);
//             }
//         }
//     }
//
//     // Vail command format:
//     // $12\r\n
//     // *12\r\n
//     // ...
//     // Invail command format:
//     // $12, $12\r, $12\n
//     let mut len_buf = vec![];
//     loop {
//         let mut byte = stream.read_u8().await?;
//         if byte != b'\r' {
//             len_buf.put_u8(byte);
//             continue;
//         }
//
//         byte = stream.read_u8().await?;
//         if byte != b'\n' {
//             len_buf.put_u8(b'\r');
//             len_buf.put_u8(byte);
//             continue;
//         }
//
//         let len = String::from_utf8(len_buf)
//             .map_err(|_| RedisError::InvaildFrame)?
//             .parse::<u64>()
//             .map_err(|_| RedisError::InvaildFrame)?;
//
//         // not allow 0 length
//         if len == 0 {
//             return Err(RedisError::InvaildFrame);
//         }
//
//         return Ok(len);
//     }
// }
//
// pub async fn get_bulk_len(stream: &mut TcpStream) -> RedisResult<u64> {
//     if stream.read_u8().await? != b'$' {
//         return Err(RedisError::InvaildFrame);
//     }
//
//     // Vail command format:
//     // $12\r\n
//     // *12\r\n
//     // ...
//     // Invail command format:
//     // $12, $12\r, $12\n
//     let mut len_buf = vec![];
//     loop {
//         let mut byte = stream.read_u8().await?;
//         if byte != b'\r' {
//             len_buf.put_u8(byte);
//             continue;
//         }
//
//         byte = stream.read_u8().await?;
//         if byte != b'\n' {
//             len_buf.put_u8(b'\r');
//             len_buf.put_u8(byte);
//             continue;
//         }
//
//         let len = String::from_utf8(len_buf)
//             .map_err(|_| RedisError::InvaildFrame)?
//             .parse::<u64>()
//             .map_err(|_| RedisError::InvaildFrame)?;
//
//         // not allow 0 length
//         if len == 0 {
//             return Err(RedisError::InvaildFrame);
//         }
//
//         return Ok(len);
//     }
// }
//
// pub async fn get_exact(stream: &mut TcpStream, n: usize) -> RedisResult<Bytes> {
//     let mut buf = vec![0u8; n];
//     stream.read_exact(&mut buf).await?;
//
//     let mut new_line = [0u8; 2];
//     stream.read_exact(&mut new_line).await?;
//     if new_line != "\r\n".as_bytes() {
//         return Err(RedisError::InvaildFrame);
//     }
//
//     Ok(buf.into())
// }
//
// // +<content>\r\n
// pub async fn send_simple(stream: &mut TcpStream, content: &str) -> RedisResult<()> {
//     let msg = format!("+{}\r\n", content);
//     stream.write_all(msg.as_bytes()).await?;
//     stream.flush().await?;
//     Ok(())
// }
//
// // -<content>\r\n
// pub async fn send_error(stream: &mut TcpStream, content: &str) -> RedisResult<()> {
//     let msg = format!("+{}\r\n", content);
//     stream.write_all(msg.as_bytes()).await?;
//     stream.flush().await?;
//     Ok(())
// }
//
// // :<u64>\r\n
// pub async fn send_integer(stream: &mut TcpStream, num: u64) -> RedisResult<()> {
//     let msg = format!(":{}\r\n", num);
//     stream.write_all(msg.as_bytes()).await?;
//     stream.flush().await?;
//     Ok(())
// }
//
// // $<len>\r\n<content>\r\n
// pub async fn send_bulk(stream: &mut TcpStream, content: &[u8]) -> RedisResult<()> {
//     let header = format!("${}\r\n", content.len());
//     stream.write_all(header.as_bytes()).await?;
//     stream.write_all(content).await?;
//     stream.write_all(b"\r\n").await?;
//     stream.flush().await?;
//     Ok(())
// }
//
// // $-1\r\n
// pub async fn send_null(stream: &mut TcpStream) -> RedisResult<()> {
//     stream.write_all(b"$-1\r\n").await?;
//     stream.flush().await?;
//     Ok(())
// }
//
// pub async fn send_array(stream: &mut TcpStream, frames: &[Bytes]) -> RedisResult<()> {
//     let header = format!("*{}\r\n", frames.len());
//     stream.write_all(header.as_bytes()).await?;
//     for frame in frames {
//         send_bulk(stream, frame).await?;
//     }
//     stream.flush().await?;
//     Ok(())
// }
