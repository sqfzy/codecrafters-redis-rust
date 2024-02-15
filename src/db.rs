use bytes::Bytes;
use std::collections::HashMap;

pub struct Db {
    pub entries: HashMap<Bytes, Bytes>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<Bytes> {
        self.entries.get(key).cloned()
    }

    pub fn set(&mut self, key: Bytes, value: Bytes) {
        self.entries
            .entry(key)
            .and_modify(|v| {
                *v = value.clone();
            })
            .or_insert(value);
    }
}

#[cfg(test)]
mod db_test {
    use super::*;

    #[test]
    fn test() {
        let mut db = Db::new();
        db.set("foo".as_bytes().into(), "bar".as_bytes().into());
        let value = db.get("foo".as_bytes());
        assert_eq!(value, Some("bar".as_bytes().into()));
    }
}
