use std::{collections::HashMap, sync::Arc};
use crate::domain::Tool;
use super::hello::HelloTool;
use super::grammar::GrammarTool;

#[derive(Clone)]
pub struct Registry(pub Arc<HashMap<&'static str, Arc<dyn Tool>>>);

pub fn build_registry() -> Registry {
    let mut map: HashMap<&'static str, Arc<dyn Tool>> = HashMap::new();

    // Always provide hello
    let hello: Arc<dyn Tool> = Arc::new(HelloTool::default());
    map.insert(hello.name(), hello);

    // Conditionally include Gramadóir (avoid breaking existing flows if not configured)
    if let Ok(base) = std::env::var("GRAMADOIR_BASE_URL") {
        if !base.trim().is_empty() {
            let grammar: Arc<dyn Tool> = Arc::new(GrammarTool::new(base));
            map.insert(grammar.name(), grammar);
        }
    }

    Registry(Arc::new(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_includes_grammar_when_configured() {
        std::env::set_var("GRAMADOIR_BASE_URL", "http://example");
        let reg = build_registry();
        assert!(reg.0.contains_key("gael.grammar_check"));
        std::env::remove_var("GRAMADOIR_BASE_URL");
    }
}
