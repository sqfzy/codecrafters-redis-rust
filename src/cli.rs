use clap::{value_parser, Parser};
use std::net::SocketAddr;

#[derive(Parser)]
pub struct Cli {
    #[clap(short, long, default_value = "6379")]
    pub port: u16,
    #[clap(long, value_parser = value_parser!(SocketAddr))]
    pub replicaof: Option<SocketAddr>,
}
