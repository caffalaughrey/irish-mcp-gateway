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
