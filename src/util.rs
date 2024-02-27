use bytes::Bytes;

use crate::error::{RedisError, RedisResult};

pub fn bytes_to_string(bytes: Bytes) -> RedisResult<String> {
    String::from_utf8(bytes.into()).map_err(|_| RedisError::syntax_err("invaild cmd format"))
}

pub fn bytes_to_u64(bytes: Bytes) -> RedisResult<u64> {
    String::from_utf8(bytes.into())
        .map_err(|_| RedisError::syntax_err("invaild cmd format"))?
        .parse::<u64>()
        .map_err(|_| RedisError::syntax_err("invaild cmd format"))
}
