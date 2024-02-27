use bytes::Bytes;
use std::{collections::HashMap, time::Duration};
use tokio::time::Instant;

#[derive(Debug)]
pub struct StringDb {
    entries: HashMap<String, Entry>,
}

impl StringDb {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Entry {
    value: Bytes,
    // when expire_at is None, it means the entry never expire
    expire_at: Option<Instant>,
}

impl super::StringDbManipulator for StringDb {
    async fn get(&self, key: &str) -> Option<Bytes> {
        assert!(!key.is_empty(), "key must not be empty");

        if let Some(value) = self.entries.get(key) {
            if let Some(expire_at) = value.expire_at {
                if expire_at < Instant::now() {
                    // if the entry is expired, remove it and return None
                    self.entries.remove(key);
                    return None;
                }
            }
            return Some(value.value.clone());
        };
        // if the key is not found, return None
        None
    }

    async fn set(&mut self, key: String, value: Bytes, expire: Option<Duration>, keepttl: bool) {
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

        let expire_at = expire.map(|e| Instant::now() + e);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.value = value.clone();
            if !keepttl {
                entry.expire_at = expire_at;
            }
        } else {
            self.entries.insert(key, Entry { value, expire_at });
        }
        // self.entries
        //     .entry(key)
        //     .and_modify(|e| {
        //         e.value = value.clone();
        //         if !keepttl {
        //             e.expire_at = expire_at;
        //         }
        //     })
        //     .or_insert(Entry { value, expire_at });
    }
}
