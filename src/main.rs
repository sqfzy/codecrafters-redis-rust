mod db;
mod error;
mod frame;

use crate::{db::Db, error::RedisResult, frame::*};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("Fail to connect");

    let mut db = Db::new();

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("accepted new connection from {addr}");
                if let Err(e) = handle(stream, &mut db).await {
                    println!("error: {}", e);
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

async fn handle(mut stream: TcpStream, db: &mut Db) -> RedisResult<()> {
    loop {
        let array_len = stream.get_array_len().await?;
        if array_len == 0 {
            return Ok(());
        }
        let cmd_name_len = stream.get_bulk_len().await? as usize;
        let cmd_name = stream.get_exact(cmd_name_len).await?;

        dbg!(array_len, cmd_name_len, cmd_name.clone());

        match &cmd_name.to_ascii_lowercase()[..] {
            // *2\r\n$4\r\necho\r\n$3\r\nhey\r\n
            // return: $3\r\nhey\r\n
            b"echo" => {
                let len = stream.get_bulk_len().await? as usize;
                let content = stream.get_exact(len).await?;
                stream.send_frame(Frame::Bulk(&content)).await?;
            }
            // *1\r\n$4\r\nping\r\n
            // return: +PONG\r\n
            b"ping" => stream.send_frame(Frame::Simple("PONG")).await?,
            // *2\r\n$3\r\nget\r\n$3\r\nkey\r\n
            // return(the key exesits): $5\r\nvalue\r\n
            // return(the key doesn't exesit): $-1\r\n
            b"get" => {
                let len = stream.get_bulk_len().await? as usize;
                let key = stream.get_exact(len).await?;
                dbg!(key.clone());
                match db.get(&key) {
                    Some(value) => stream.send_frame(Frame::Bulk(&value)).await?,
                    None => stream.send_frame(Frame::Null).await?,
                }
            }
            // *3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
            // return: +OK\r\n
            b"set" => {
                let len = stream.get_bulk_len().await? as usize;
                let key = stream.get_exact(len).await?;
                let len = stream.get_bulk_len().await? as usize;
                let value = stream.get_exact(len).await?;
                dbg!(key.clone(), value.clone());
                db.set(key, value);
                if db.get(b"a").is_none() {
                    println!("yew");
                }
                stream.send_frame(Frame::Simple("OK")).await?;
            }
            _ => {}
        }
    }
    // Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn test_ping() {}
}
