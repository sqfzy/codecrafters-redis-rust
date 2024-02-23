use bytes::Bytes;
use clap::{value_parser, Parser};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::info;

use crate::{error::RedisResult, frame::Frame, stream::FrameHandler};

#[derive(Parser)]
pub struct Cli {
    #[clap(short, long, default_value = "6379")]
    pub port: u16,
    #[clap(long, value_parser = value_parser!(SocketAddr))]
    pub replicaof: Option<SocketAddr>,
}

