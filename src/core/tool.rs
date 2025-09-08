use async_trait::async_trait;

/// Minimal metadata every tool must expose.
pub trait ToolSpec {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn input_schema(&self) -> serde_json::Value;
}

/// Backend abstraction so a tool can be local or remote.
#[allow(dead_code)]
#[async_trait]
pub trait ToolBackend: Send + Sync {
    async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, String>;
}

/// Tool = Spec + Backend implementation
#[async_trait]
pub trait Tool: ToolSpec + Send + Sync {
    /// Execute the tool with the given arguments, returning a JSON value or a string error.
    async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, String>;

    /// Optional liveness/health probe for the tool.
    /// Defaults to healthy. Remote implementations should override.
    async fn health(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

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
    async fn echo_returns_input_verbatim() {
        let t = Echo;
        let out = t.call(&serde_json::json!({"x":1})).await.unwrap();
        assert_eq!(out["x"], 1);
    }

    struct Failing;

    impl ToolSpec for Failing {
        fn name(&self) -> &'static str {
            "test.failing"
        }
        fn description(&self) -> &'static str {
            "failing tool"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({"type":"object"})
        }
    }

    #[async_trait]
    impl Tool for Failing {
        async fn call(&self, _args: &serde_json::Value) -> Result<serde_json::Value, String> {
            Err("boom".into())
        }
    }

    #[tokio::test]
    async fn failing_propagates_error() {
        let t = Failing;
        let err = t.call(&serde_json::json!({})).await.unwrap_err();
        assert!(err.contains("boom"));
    }

    #[test]
    fn tool_spec_metadata_is_exposed() {
        let e = Echo;
        assert_eq!(e.name(), "test.echo");
        assert!(e.description().contains("echo"));
        let s = e.input_schema();
        assert_eq!(s["type"], "object");
    }

    struct BackendEcho;

    #[async_trait]
    impl ToolBackend for BackendEcho {
        async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(arguments.clone())
        }
    }

    #[tokio::test]
    async fn tool_backend_trait_is_callable() {
        let b = BackendEcho;
        let out = b.call(&serde_json::json!({"y":2})).await.unwrap();
        assert_eq!(out["y"], 2);
    }

    struct BackendFail;

    #[async_trait]
    impl ToolBackend for BackendFail {
        async fn call(&self, _arguments: &serde_json::Value) -> Result<serde_json::Value, String> {
            Err("backend fail".into())
        }
    }

    #[tokio::test]
    async fn tool_backend_error_path() {
        let b = BackendFail;
        let err = b.call(&serde_json::json!({})).await.unwrap_err();
        assert!(err.contains("fail"));
    }

    struct SpecAndBackend;

    impl ToolSpec for SpecAndBackend {
        fn name(&self) -> &'static str {
            "test.combo"
        }
        fn description(&self) -> &'static str {
            "combo tool"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({"type":"object","properties":{}})
        }
    }

    #[async_trait]
    impl Tool for SpecAndBackend {
        async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(arguments.clone())
        }
    }

    #[tokio::test]
    async fn combo_tool_covers_spec_and_call() {
        let t = SpecAndBackend;
        assert_eq!(t.name(), "test.combo");
        assert!(t.description().contains("combo"));
        let s = t.input_schema();
        assert_eq!(s["type"], "object");
        let out = t.call(&serde_json::json!({"n":1})).await.unwrap();
        assert_eq!(out["n"], 1);
    }

    #[test]
    fn tool_specs_have_valid_schemas() {
        let e = Echo;
        let s = e.input_schema();
        assert_eq!(s["type"], "object");
        // Properties may vary per tool; requiring object type suffices here
    }
}
