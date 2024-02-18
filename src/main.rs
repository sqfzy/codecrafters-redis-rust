mod cmd;
mod db;
mod error;
mod frame;
mod stream;

use crate::{
    db::Db,
    error::{RedisError, RedisResult},
    frame::Frame,
    stream::FrameHandler,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

fn init() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }
    tracing_subscriber::fmt::init();
}

#[tokio::main]
async fn main() {
    // client_test().await;
    init();

    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("Fail to connect");

    let db = Db::new();

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("accepted new connection from {addr}");
                let db = db.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle(stream, db).await {
                        match e {
                            RedisError::EndofStream => {}
                            _ => {
                                println!("error: {}", e);
                            }
                        }
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

async fn handle(mut stream: TcpStream, mut db: Db) -> RedisResult<()> {
    // server_test(&mut stream).await;

    loop {
        let frames = stream.read_frame().await?;
        let mut cmd = frames.parse_cmd()?;
        let res = (*cmd).execute(&mut db).await?;
        stream.write_frame(res).await?;
    }
}

async fn client_test() {
    let mut stream = TcpStream::connect("127.0.0.1:6379").await.unwrap();
    stream
        .write_all(b"*2\r\n$7\r\nCOMMAND\r\n$4\r\nDOCS\r\n")
        .await
        .unwrap();
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    println!("{:?}", String::from_utf8(buf[0..n].to_vec()).unwrap());
}

async fn server_test(stream: &mut TcpStream) {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    println!("{:?}", String::from_utf8(buf[0..n].to_vec()).unwrap());
}

#[cfg(test)]
mod test {
    #[test]
    fn test_ping() {}
}
