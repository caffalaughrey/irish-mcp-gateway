use crate::core::tool::Tool;
use crate::tools::spellcheck::{SpellcheckLocalBackend, SpellcheckRemoteBackend};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub struct Registry(pub Arc<HashMap<&'static str, Arc<dyn Tool>>>);

pub fn build_registry() -> Registry {
    let mut map: HashMap<&'static str, Arc<dyn Tool>> = HashMap::new();

    // Always include spellcheck placeholder (local)
    let spellcheck: Arc<dyn Tool> = Arc::new(SpellcheckLocalBackend);
    map.insert("spell.check", spellcheck);

    // Conditionally include remote spellcheck if configured
    if let Ok(base) = std::env::var("SPELLCHECK_BASE_URL") {
        if !base.trim().is_empty() {
            let remote_spellcheck: Arc<dyn Tool> = Arc::new(SpellcheckRemoteBackend::new(base));
            map.insert("spell.check", remote_spellcheck);
        }
    }

    Registry(Arc::new(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn it_includes_spellcheck_when_configured() {
        std::env::set_var("SPELLCHECK_BASE_URL", "http://example");
        let reg = build_registry();
        assert!(reg.0.contains_key("spell.check"));
        std::env::remove_var("SPELLCHECK_BASE_URL");
    }
}
