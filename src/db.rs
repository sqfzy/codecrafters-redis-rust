use async_trait::async_trait;
use bytes::Bytes;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};

#[async_trait]
pub trait DbManipulate: Send {
    async fn get(&self, key: Bytes) -> Option<Bytes>;
    async fn set(&mut self, key: Bytes, value: Bytes, expire: Option<Duration>, keepttl: bool);
}

// async fn check(&self, key: Bytes)

#[derive(Debug, Clone)]
pub struct Db {
    pub inner: Arc<Mutex<DbInner>>,
}

#[derive(Debug)]
pub struct DbInner {
    entries: HashMap<Bytes, Entry>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DbInner {
                entries: HashMap::new(),
            })),
        }
    }
}

#[derive(Debug)]
pub struct Entry {
    value: Bytes,
    // when expire_at is None, it means the entry never expire
    expire_at: Option<Instant>,
}

#[async_trait]
impl DbManipulate for Db {
    async fn get(&self, key: Bytes) -> Option<Bytes> {
        assert!(!key.is_empty(), "key must not be empty");

        let mut inner = self.inner.lock().await;
        if let Some(value) = inner.entries.get(&key) {
            if let Some(expire_at) = value.expire_at {
                if expire_at < Instant::now() {
                    // if the entry is expired, remove it and return None
                    inner.entries.remove(&key);
                    return None;
                }
            }
            return Some(value.value.clone());
        };
        // if the key is not found, return None
        None
    }

    async fn set(&mut self, key: Bytes, value: Bytes, expire: Option<Duration>, keepttl: bool) {
        assert!(!key.is_empty(), "key must not be empty");
        assert!(!value.is_empty(), "value must not be empty");
        assert!(
            expire.is_none() || expire.unwrap().as_secs() > 0,
            "expire must be positive"
        );
        assert!(
            expire.is_some() || !keepttl,
            "expire and keepttl can't be both None"
        );

        let mut inner = self.inner.lock().await;
        let expire_at = expire.map(|e| Instant::now() + e);
        inner
            .entries
            .entry(key)
            .and_modify(|e| {
                e.value = value.clone();
                if !keepttl {
                    e.expire_at = expire_at;
                }
            })
            .or_insert(Entry { value, expire_at });
    }
}

// #[cfg(test)]
// mod db_test {
//     use super::*;
//
//     #[test]
//     async fn test() {
//         let mut db = Db::new();
//         db.set("foo".as_bytes().into(), "bar".as_bytes().into());
//         let value = db.get("foo".as_bytes().into()).await;
//         assert_eq!(value, Some("bar".as_bytes().into()));
//     }
// }
