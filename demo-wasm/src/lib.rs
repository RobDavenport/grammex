//! WASM bindings for the grammex interactive demo.

use wasm_bindgen::prelude::*;

/// Placeholder: generate a dungeon graph and return JSON.
#[wasm_bindgen]
pub fn generate_demo(_config_json: &str) -> String {
    r#"{"status":"not_implemented"}"#.to_string()
}

/// Placeholder: step-by-step rewriter.
#[wasm_bindgen]
pub struct StepRewriter {
    // TODO
}

#[wasm_bindgen]
impl StepRewriter {
    #[wasm_bindgen(constructor)]
    pub fn new(_config_json: &str) -> Self {
        Self {}
    }

    /// Perform one rewrite step. Returns JSON event.
    pub fn step(&mut self) -> String {
        r#"{"type":"not_implemented"}"#.to_string()
    }

    /// Get the current graph as JSON (nodes + edges).
    pub fn graph_json(&self) -> String {
        r#"{"nodes":[],"edges":[]}"#.to_string()
    }
}
