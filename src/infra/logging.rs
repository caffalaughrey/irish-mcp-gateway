pub fn init() {
    // loud boot even if RUST_LOG not set
    let _ = std::env::var("RUST_LOG");
}
