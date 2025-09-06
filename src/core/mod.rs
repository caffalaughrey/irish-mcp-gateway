//! Core types & traits: domain-agnostic contracts for tools and protocol.

pub mod content;
pub mod error;
pub mod mcp;
pub mod tool;

#[cfg(test)]
mod tests {
    #[test]
    fn core_module_compiles() {
        // Smoke test to ensure module wiring is valid
        let _ = ();
    }
}
