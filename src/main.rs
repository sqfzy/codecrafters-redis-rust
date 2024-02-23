mod cli;
mod cmd;
mod config;
mod db;
mod error;
mod frame;
mod stream;

use std::{str::FromStr, sync::Arc};

use crate::{
    db::Db,
    error::{RedisError, RedisResult},
    frame::Frame,
    stream::FrameHandler,
};
use config::RedisConfig;
use once_cell::sync::Lazy;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::Level;

static CONFIG: Lazy<Arc<config::RedisConfig>> = Lazy::new(|| Arc::new(RedisConfig::new()));

fn init() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }

    let level = Level::from_str(&std::env::var("RUST_LOG").unwrap()).unwrap();
    tracing_subscriber::fmt()
        // .pretty()
        .with_max_level(level)
        .init();
}

#[tokio::main]
async fn main() {
    // client_test("*2\r\n$4\r\ninfo\r\n$11\r\nreplication\r\n").await;
    // return;
    init();

    let config = CONFIG.clone();
    config.may_replicaof().await.unwrap();

    let listener = TcpListener::bind(format!("localhost:{}", CONFIG.port))
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
    // return Ok(());

    loop {
        if let Some(frame) = stream.read_frame().await? {
            let mut cmd = frame.parse_cmd()?;
            let res = (*cmd).execute(&mut db).await?;
            stream.write_frame(res).await?;
        }
    }
}

async fn client_test(cmd: &'static str) {
    let mut stream = TcpStream::connect("127.0.0.1:6379").await.unwrap();
    stream.write_all(cmd.as_bytes()).await.unwrap();
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
