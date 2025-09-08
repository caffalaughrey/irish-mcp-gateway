use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub trait SessionStore: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: String);
}

#[derive(Default, Clone)]
pub struct InMemorySessionStore(Arc<RwLock<HashMap<String, String>>>);

impl SessionStore for InMemorySessionStore {
    fn get(&self, key: &str) -> Option<String> {
        self.0.read().ok()?.get(key).cloned()
    }
    fn set(&self, key: &str, value: String) {
        if let Ok(mut m) = self.0.write() {
            m.insert(key.to_string(), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn in_memory_store_roundtrip() {
        let store = InMemorySessionStore::default();
        assert!(store.get("k").is_none());
        store.set("k", "v".into());
        assert_eq!(store.get("k").unwrap(), "v");
    }
}
