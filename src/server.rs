use std::net::SocketAddr;

use crate::{db::*, frame::Frame, stream::FrameHandler, CONFIG};
use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing::{debug, error};

pub async fn run() {
    // client_test("*2\r\n$4\r\ninfo\r\n$11\r\nreplication\r\n").await;
    // return;

    let config = CONFIG.clone();
    config.may_replicaof().await.unwrap();

    let listener = TcpListener::bind(format!("localhost:{}", CONFIG.port))
        .await
        .expect("Fail to connect");

    let db = Db::new(Box::new(StringDb::new()));

    loop {
        match listener.accept().await {
            Ok((mut stream, addr)) => {
                debug!("accepted new connection from {addr}");

                let mut db = db.clone();
                tokio::spawn(async move {
                    loop {
                        match handle(&mut stream, &mut db, addr).await {
                            Err(e) => {
                                let _ = stream.write_frame(Frame::Error(e.to_string())).await;
                            }
                            Ok(Some(())) => {}
                            Ok(None) => break,
                        }
                    }
                });
            }
            Err(e) => {
                error!("error: {}", e);
            }
        }
    }
}

async fn handle(stream: &mut TcpStream, db: &mut Db, addr: SocketAddr) -> Result<Option<()>> {
    // server_test(&mut stream).await;
    // return Ok(());

    if let Some(frame) = stream.read_frame().await? {
        let cmd = frame.parse_cmd()?;
        let res = cmd.execute(db).await?;
        stream.write_frame(res).await?;
        Ok(Some(()))
    } else {
        debug!("{addr} turn off connection");
        Ok(None)
    }
}

#[allow(dead_code)]
async fn client_test(cmd: &'static str) {
    let mut stream = TcpStream::connect("127.0.0.1:6379").await.unwrap();
    stream.write_all(cmd.as_bytes()).await.unwrap();
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    println!("{:?}", String::from_utf8(buf[0..n].to_vec()).unwrap());
}

#[allow(dead_code)]
async fn server_test(stream: &mut TcpStream) {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    println!("{:?}", String::from_utf8(buf[0..n].to_vec()).unwrap());
}
