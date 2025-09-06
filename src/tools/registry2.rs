use std::collections::HashMap;
use std::sync::Arc;

use crate::core::tool::{Tool, ToolSpec};
use crate::tools::grammar_new::{GrammarLocalBackend, GrammarRemoteBackend};

#[derive(Clone)]
pub struct ToolRegistry {
    by_name: Arc<HashMap<&'static str, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { by_name: Arc::new(HashMap::new()) }
    }

    pub fn with_tools<I, T>(iter: I) -> Self
    where
        I: IntoIterator<Item = Arc<T>>,
        T: Tool + 'static,
    {
        let mut map: HashMap<&'static str, Arc<dyn Tool>> = HashMap::new();
        for t in iter.into_iter() {
            map.insert(t.name(), t);
        }
        Self { by_name: Arc::new(map) }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: Arc<T>) {
        let mut_map = Arc::get_mut(&mut self.by_name).expect("no other clones when registering");
        mut_map.insert(tool.name(), tool);
    }

    pub fn list(&self) -> Vec<ToolMeta> {
        self
            .by_name
            .values()
            .map(|t| ToolMeta {
                name: t.name(),
                description: t.description(),
                input_schema: t.input_schema(),
            })
            .collect()
    }

    pub async fn call(&self, name: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let t = self
            .by_name
            .get(name)
            .ok_or_else(|| format!("unknown tool: {name}"))?;
        t.call(args).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolMeta {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: serde_json::Value,
}

/// Build a registry v2 from environment, selecting grammar backend.
pub fn build_registry_v2_from_env() -> ToolRegistry {
    let mut map: HashMap<&'static str, Arc<dyn Tool>> = HashMap::new();
    if let Ok(base) = std::env::var("GRAMADOIR_BASE_URL") {
        if !base.trim().is_empty() {
            map.insert("gael.grammar_check.v2", Arc::new(GrammarRemoteBackend::new(base)));
        } else {
            map.insert("gael.grammar_check.v2", Arc::new(GrammarLocalBackend::default()));
        }
    } else {
        map.insert("gael.grammar_check.v2", Arc::new(GrammarLocalBackend::default()));
    }
    ToolRegistry { by_name: Arc::new(map) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct Echo;

    impl ToolSpec for Echo {
        fn name(&self) -> &'static str { "test.echo2" }
        fn description(&self) -> &'static str { "echo tool" }
        fn input_schema(&self) -> serde_json::Value { serde_json::json!({"type":"object"}) }
    }

    #[async_trait]
    impl Tool for Echo {
        async fn call(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(args.clone())
        }
    }

    #[tokio::test]
    async fn registry_registers_lists_and_calls() {
        let t = Arc::new(Echo);
        let reg = ToolRegistry::with_tools([t]);
        let metas = reg.list();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].name, "test.echo2");
        let out = reg.call("test.echo2", &serde_json::json!({"x": 2})).await.unwrap();
        assert_eq!(out["x"], 2);
    }

    #[tokio::test]
    async fn registry_v2_builds_with_local_fallback() {
        std::env::remove_var("GRAMADOIR_BASE_URL");
        let reg = build_registry_v2_from_env();
        let metas = reg.list();
        assert!(metas.iter().any(|m| m.name == "gael.grammar_check.v2"));
        let out = reg.call("gael.grammar_check.v2", &serde_json::json!({"text":"x"})).await.unwrap();
        assert!(out["issues"].is_array());
    }
}


