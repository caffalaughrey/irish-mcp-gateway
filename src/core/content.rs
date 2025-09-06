//! Structured content model placeholder.

use serde_json::Value as JsonValue;

/// Wrapper to indicate a JSON value intended for structuredContent.
#[derive(Debug, Clone)]
pub struct StructuredJson(pub JsonValue);

impl From<JsonValue> for StructuredJson {
    fn from(v: JsonValue) -> Self { StructuredJson(v) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let val = JsonValue::String("ok".into());
        let s: StructuredJson = val.clone().into();
        assert_eq!(s.0, val);
    }
}


