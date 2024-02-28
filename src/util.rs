use anyhow::{anyhow, Result};
use bytes::Bytes;

pub fn bytes_to_string(bytes: Bytes) -> Result<String> {
    String::from_utf8(bytes.into()).map_err(|_| anyhow!("ERR syntax error"))
}

pub fn bytes_to_u64(bytes: Bytes) -> Result<u64> {
    String::from_utf8(bytes.into())
        .map_err(|_| anyhow!("ERR syntax error"))?
        .parse::<u64>()
        .map_err(|_| anyhow!("ERR syntax error"))
}
