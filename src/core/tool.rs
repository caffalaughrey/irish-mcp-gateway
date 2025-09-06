use async_trait::async_trait;

/// Minimal metadata every tool must expose.
pub trait ToolSpec {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn input_schema(&self) -> serde_json::Value;
}

/// Backend abstraction so a tool can be local or remote.
#[async_trait]
pub trait ToolBackend: Send + Sync {
    async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, String>;
}

/// Tool = Spec + Backend implementation
#[async_trait]
pub trait Tool: ToolSpec + Send + Sync {
    async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo;

    impl ToolSpec for Echo {
        fn name(&self) -> &'static str {
            "test.echo"
        }
        fn description(&self) -> &'static str {
            "echo tool"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({"type":"object"})
        }
    }

    #[async_trait]
    impl Tool for Echo {
        async fn call(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(args.clone())
        }
    }

    #[tokio::test]
    async fn it_runs_echo() {
        let t = Echo;
        let out = t.call(&serde_json::json!({"x":1})).await.unwrap();
        assert_eq!(out["x"], 1);
    }
}
