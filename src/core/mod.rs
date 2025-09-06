//! Core types & traits: domain-agnostic contracts for tools and protocol.

pub mod error;
pub mod tool;
pub mod content;
pub mod mcp;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_module_compiles() {
        // Smoke test to ensure module wiring is valid
        let _ = ();
    }
}


