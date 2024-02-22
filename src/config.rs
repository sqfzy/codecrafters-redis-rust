use crate::cli::Cli;
use clap::Parser;
use rand::Rng;

#[derive(Debug)]
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
}
