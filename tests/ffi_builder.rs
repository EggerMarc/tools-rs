//! Builder-flow integration tests for the FFI scripted path.
//!
//! Feature-gated: requires `--features python` to run.
//! These test the `ToolsBuilder::new().with_language().from_path().collect()`
//! pipeline against the stub adapters (not yet implemented).

#![cfg(feature = "python")]

use tools_rs::{Language, ToolCollection, ToolsBuilder};

#[test]
fn scripted_no_paths_returns_collection() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .collect()
        .unwrap();

    // Should succeed — no paths means just inventory tools
    let _ = tools.json().unwrap();
}

#[test]
fn scripted_with_path_hits_stub() {
    let err = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path("/nonexistent/script.py")
        .collect()
        .err()
        .expect("should error — adapter not implemented");

    assert!(
        err.to_string().contains("not yet implemented"),
        "unexpected error: {err}"
    );
}

#[test]
fn scripted_from_path_chainable() {
    let err = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path("/first.py")
        .from_path("/second.py")
        .collect()
        .err()
        .expect("should error on first path");

    // Error should mention the first path
    assert!(err.to_string().contains("first.py"));
}

#[test]
fn scripted_collection_includes_inventory_tools() {
    // When no paths are given, inventory tools from #[tool] in this
    // binary should still be collected.
    let plain: ToolCollection = ToolsBuilder::new().collect().unwrap();
    let scripted: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .collect()
        .unwrap();

    let plain_decls = plain.json().unwrap();
    let scripted_decls = scripted.json().unwrap();

    // Same inventory tools in both
    assert_eq!(
        plain_decls.as_array().unwrap().len(),
        scripted_decls.as_array().unwrap().len(),
    );
}

// ============================================================================
// PER-LANGUAGE TEST MATRIX (template for adapter PRs)
// ============================================================================
//
// When implementing an adapter (e.g. ffi_python.rs), copy this matrix
// and test against real fixture scripts in tests/fixtures/<lang>/:
//
// - single_tool_loads        — one tool per file, name/desc/schema correct
// - multi_tool_loads         — multiple tools from one file
// - tool_callable            — loaded tool executes, returns correct result
// - param_types_to_schema    — language type hints → JSON schema types
// - optional_params          — optional params not in `required`
// - enum_params              — union/literal types → `enum` in schema
// - meta_extracted           — meta annotations deserialize correctly
// - no_tools_returns_empty   — file without tool markers → empty vec
// - malformed_tool_errors    — bad annotation/syntax → clear error
// - description_extracted    — docstring/comment → description field
