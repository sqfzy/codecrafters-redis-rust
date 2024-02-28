// #![allow(clippy::needless_return)]

mod cli;
mod cmd;
mod config;
mod db;
mod frame;
mod init;
mod server;
mod stream;
mod util;

use config::RedisConfig;
use once_cell::sync::Lazy;
use std::sync::Arc;

static CONFIG: Lazy<Arc<config::RedisConfig>> = Lazy::new(|| Arc::new(RedisConfig::new()));

// TODO:
// 1. more reasonable error handling
// 2. more commands: ttl, exist

// asd asd
// (error) ERR unknown command 'asd', with args beginning with: 'asd'
// get foo a
// (error) ERR wrong number of arguments for 'get' command
// set foo bar bar2
// (error) ERR syntax error

#[tokio::main]
async fn main() {
    init::init();

    server::run().await;
}
