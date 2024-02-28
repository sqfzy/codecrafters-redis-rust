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

#[async_trait::async_trait]
impl super::StringDbManipulator for StringDb {
    async fn get(&mut self, key: &str) -> Option<Bytes> {
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

    async fn set(&mut self, key: String, value: Bytes, expire: Option<Duration>) {
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.value = value.clone();
            match expire {
                Some(e) if e.as_secs() == 0 => {
                    entry.expire_at = None;
                }
                Some(e) => {
                    entry.expire_at = Some(Instant::now() + e);
                }
                None => {}
            }
        } else {
            self.entries.insert(
                key,
                Entry {
                    value,
                    expire_at: expire.map(|e| Instant::now() + e),
                },
            );
        }
    }

    async fn del(&mut self, key: &str) {
        self.entries.remove(key);
    }

    async fn check_exist(&mut self, key: &str) -> bool {
        if let Some(value) = self.entries.get(key) {
            if let Some(expire_at) = value.expire_at {
                if expire_at < Instant::now() {
                    // if the entry is expired, remove it and return false
                    self.entries.remove(key);
                    return false;
                }
            }
            return true;
        }
        // if the key is not found, return false
        false
    }

    async fn get_ttl(&mut self, key: &str) -> Option<Duration> {
        if let Some(value) = self.entries.get(key) {
            if let Some(expire_at) = value.expire_at {
                if expire_at < Instant::now() {
                    // if the entry is expired, remove it and return None
                    self.entries.remove(key);
                    return None;
                }
                return Some(expire_at - Instant::now());
            }
            return Some(Duration::from_secs(0));
        }
        // if the key is not found, return None
        None
    }
}

#[cfg(test)]
mod string_db_test {
    use super::*;
    use crate::db::StringDbManipulator;
    use tokio::time::sleep;

    #[tokio::test]
    async fn set_should_work() {
        // test:
        //  1. can set an element.
        //  2. expire_at should work.

        let mut db = StringDb::new();
        assert_eq!(None, db.get("foo").await); // at first, without "foo" key

        db.set("foo".into(), "bar".into(), None).await; // set "foo" "bar"
        assert_eq!(Some("bar".into()), db.get("foo").await);

        // set with 1 seconds life time
        db.set("foo".into(), "bar".into(), Some(Duration::from_secs(1)))
            .await;
        sleep(Duration::from_secs(1)).await; // make it expire
        assert_eq!(None, db.get("foo").await); // "foo" key has expired
    }

    #[tokio::test]
    async fn del_should_work() {
        let mut db = StringDb::new();
        db.set("foo".into(), "bar".into(), None).await;
        assert_eq!(Some("bar".into()), db.get("foo").await);
        db.del("foo").await;
        assert_eq!(None, db.get("foo").await);
    }

    #[tokio::test]
    async fn check_exist_should_work() {
        let mut db = StringDb::new();
        db.set("foo".into(), "bar".into(), None).await;
        assert!(db.check_exist("foo").await);
    }

    #[tokio::test]
    async fn get_ttl_should_work() {
        let mut db = StringDb::new();
        db.set("foo".into(), "bar".into(), Some(Duration::from_secs(1)))
            .await;
        let ttl = db.get_ttl("foo").await;
        assert!(ttl.is_some());
        assert!(Duration::from_secs(1) - ttl.unwrap() < Duration::from_millis(100));
    }
}
