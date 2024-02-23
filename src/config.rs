use crate::{cli::Cli, error::RedisResult, frame::Frame, stream::FrameHandler};
use bytes::Bytes;
use clap::Parser;
use rand::Rng;
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[derive(Debug, Default)]
pub struct RedisConfig {
    pub port: u16,
    pub replicaof: Option<String>,
    pub replid: String, // random 40 bytes
    pub repl_offset: u64,
}

impl RedisConfig {
    pub fn new() -> Self {
        let cli = Cli::parse();

        let replid: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(40)
            .map(char::from) // 将u8转换为char
            .collect(); // 直接收集到String中

        RedisConfig {
            port: cli.port,
            replicaof: cli.replicaof.map(|addr| addr.to_string()),
            replid,
            repl_offset: 0,
        }
    }

    pub async fn may_replicaof(&self) -> RedisResult<()> {
        if let Some(repl) = self.replicaof.as_ref() {
            let mut to_master = TcpStream::connect(repl).await?;

            // send {PING}
            to_master
                .write_frame(Frame::Array(vec![Frame::Bulk(Bytes::from_static(b"PING"))]))
                .await?;
            // recv {PONG}
            if to_master.read_frame().await? != Some(Frame::Simple("PONG".to_string())) {
                panic!("Master server responds invaildly");
            }

            // send {REPLCONF listening-port <PORT>}
            to_master
                .write_frame(
                    vec![
                        "REPLCONF".into(),
                        "listening-port".into(),
                        self.port.to_string().into(),
                    ]
                    .into(),
                )
                .await?;
            // recv {OK}
            if to_master.read_frame().await? != Some(Frame::Simple("OK".to_string())) {
                panic!("Master server responds invaildly");
            }

            // send {REPLCONF capa psync2}
            to_master
                .write_frame(vec!["REPLCONF".into(), "capa".into(), "psync2".into()].into())
                .await?;
            // recv {OK}
            if to_master.read_frame().await? != Some(Frame::Simple("OK".to_string())) {
                panic!("Master server responds invaildly");
            }

            // send {PSYNC ? -1}
            to_master
                .write_frame(vec!["PSYNC".into(), "?".into(), "-1".into()].into())
                .await?;
            // recv {FULLRESYNC <REPL_ID> 0}
            if let Some(Frame::Simple(s)) = to_master.read_frame().await? {
                tracing::info!(
                    "Successfully replicaof {}, repl_id is {}",
                    self.replicaof.as_ref().expect("Replicaof should be exist"),
                    s
                );
            }

            to_master.shutdown().await?;
        }
        Ok(())
    }
}
