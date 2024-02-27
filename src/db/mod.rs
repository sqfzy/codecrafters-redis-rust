mod string_db;

use bytes::Bytes;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

pub use string_db::StringDb;

#[derive(Debug, Clone)]
pub struct Db {
    pub inner: Arc<Mutex<DbInner>>,
}

#[derive(Debug)]
pub struct DbInner {
    pub string_db: Box<dyn StringDbManipulator>,
}

#[async_trait::async_trait]
pub trait StringDbManipulator: Send + std::fmt::Debug {
    async fn get(&mut self, key: &str) -> Option<Bytes>;
    async fn set(&mut self, key: String, value: Bytes, expire: Option<Duration>);
    async fn del(&mut self, key: &str);
    async fn check_exist(&mut self, key: &str) -> bool;
    async fn get_ttl(&mut self, key: &str) -> Option<Duration>;
}

impl Db {
    pub fn new(string_db: Box<dyn StringDbManipulator>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(DbInner { string_db })),
        }
    }
}
//
// #[cfg(test)]
// mod db_test {
//     use std::thread::sleep;
//
//     use super::*;
//
//     #[tokio::test]
//     async fn set_test() {
//         // test:
//         //  1. can set an element.
//         //  2. expire_at should work.
//
//         let mut db = Db::new();
//         assert_eq!(None, db.get("foo".into()).await); // at first, without "foo" key
//
//         db.set("foo".into(), "bar".into(), None, false).await; // set "foo" "bar"
//         assert_eq!(Some("bar".into()), db.get("foo".into()).await);
//
//         // set with 1 seconds life time
//         db.set(
//             "foo".into(),
//             "bar".into(),
//             Some(Duration::from_secs(1)),
//             false,
//         )
//         .await;
//         sleep(Duration::from_secs(1)); // make it expire
//         assert_eq!(None, db.get("foo".into()).await); // "foo" key has expired
//     }
// }
