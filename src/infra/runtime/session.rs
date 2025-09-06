use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Minimal session store abstraction for future Redis swap.
#[derive(Clone, Default)]
pub struct InMemorySessionStore {
    inner: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self { Self::default() }

    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.inner.lock().unwrap().get(key).cloned()
    }

    pub fn set(&self, key: impl Into<String>, val: serde_json::Value) {
        self.inner.lock().unwrap().insert(key.into(), val);
    }

    pub fn delete(&self, key: &str) {
        self.inner.lock().unwrap().remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_stores_and_retrieves_values() {
        let store = InMemorySessionStore::new();
        store.set("s1", serde_json::json!({"ready": true}));
        let v = store.get("s1").unwrap();
        assert!(v["ready"].as_bool().unwrap());
        store.delete("s1");
        assert!(store.get("s1").is_none());
    }
}


