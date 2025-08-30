use std::collections::HashMap;
use std::sync::Arc;
use crate::domain::Tool;
use super::hello::HelloTool;

#[derive(Clone)]
pub struct Registry(pub Arc<HashMap<&'static str, Arc<dyn Tool>>>);

pub fn build_registry() -> Registry {
    let mut map: HashMap<&'static str, Arc<dyn Tool>> = HashMap::new();
    let hello: Arc<dyn Tool> = Arc::new(HelloTool::default());
    map.insert(hello.name(), hello);
    Registry(Arc::new(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_knows_registry_contains_hello() {
        let reg = build_registry();
        assert!(reg.0.contains_key("hello.echo"));
    }
}

