//! Tests for PCTX runtime
//!
//! This module contains both unit tests for the Rust MCP client implementation
//! and integration tests that spin up a JavaScript runtime to test the full stack.

mod mcp_registry;
mod runtime_integration;

// Helper function to initialize rustls crypto provider for network tests
pub(crate) fn init_rustls_crypto() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
