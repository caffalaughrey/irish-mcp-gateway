use crate::core::tool::Tool;
use crate::tools::spellcheck::{SpellcheckLocalBackend, SpellcheckRemoteBackend};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub struct Registry(pub Arc<HashMap<&'static str, Arc<dyn Tool>>>);

pub fn build_registry() -> Registry {
    let mut map: HashMap<&'static str, Arc<dyn Tool>> = HashMap::new();

    // Always include spellcheck placeholder (local)
    let spellcheck: Arc<dyn Tool> = Arc::new(SpellcheckLocalBackend::default());
    map.insert("gael.spellcheck.v1", spellcheck);

    // Conditionally include remote spellcheck if configured
    if let Ok(base) = std::env::var("SPELLCHECK_BASE_URL") {
        if !base.trim().is_empty() {
            let remote_spellcheck: Arc<dyn Tool> = Arc::new(SpellcheckRemoteBackend::new(base));
            map.insert("gael.spellcheck.v1", remote_spellcheck);
        }
    }

    Registry(Arc::new(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_includes_spellcheck_when_configured() {
        std::env::set_var("SPELLCHECK_BASE_URL", "http://example");
        let reg = build_registry();
        assert!(reg.0.contains_key("gael.spellcheck.v1"));
        std::env::remove_var("SPELLCHECK_BASE_URL");
    }
}
