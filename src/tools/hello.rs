use async_trait::async_trait;
use serde_json::json;
use crate::domain::{Tool, ToolError};

#[derive(Clone, Default)]
pub struct HelloTool;

#[async_trait]
impl Tool for HelloTool {
    fn name(&self) -> &'static str { "hello.echo" }
    fn description(&self) -> &'static str { "Return a friendly greeting" }
    fn input_schema(&self) -> serde_json::Value {
        json!({ "type":"object", "properties": { "name": { "type":"string" } }, "required": [] })
    }
    async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let name = arguments.get("name").and_then(|v| v.as_str()).unwrap_or("world");
        Ok(json!({ "message": format!("Dia dhuit, {name}!") }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[tokio::test]
    async fn it_knows_hello_defaults_to_world() {
        let t = HelloTool::default();
        let out = t.call(&Value::Null).await.unwrap();
        assert_eq!(out["message"], "Dia dhuit, world!");
    }

    #[tokio::test]
    async fn it_knows_hello_with_name() {
        let t = HelloTool::default();
        let out = t.call(&json!({"name":"Arn"})).await.unwrap();
        assert_eq!(out["message"], "Dia dhuit, Arn!");
    }

    #[test]
    fn it_knows_schema_has_name_prop() {
        let t = HelloTool::default();
        let s = t.input_schema();
        assert_eq!(s["type"], "object");
        assert!(s["properties"]["name"].is_object());
        assert!(s["required"].is_array());
    }
}
