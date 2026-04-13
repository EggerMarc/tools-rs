//! Python adapter integration tests.
//!
//! Requires `--features python` and a system Python 3.10+.
//! Uses fixture scripts in `tests/fixtures/python/`.

#![cfg(feature = "python")]

use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use tools_rs::{FunctionCall, Language, ToolCollection, ToolsBuilder};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/python")
}

// ============================================================================
// LOADING
// ============================================================================

#[test]
fn single_tool_loads() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir().join("reverse.py").parent().unwrap())
        .collect()
        .unwrap();

    let decls = tools.json().unwrap();
    let arr = decls.as_array().unwrap();
    assert!(
        arr.iter().any(|d| d["name"] == "reverse"),
        "expected 'reverse' tool in declarations: {arr:?}"
    );
}

#[test]
fn single_tool_schema_correct() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let decls = tools.json().unwrap();
    let reverse = decls
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "reverse")
        .expect("reverse tool not found");

    assert_eq!(reverse["description"], "Reverse a string.");
    assert_eq!(
        reverse["parameters"]["properties"]["text"]["type"],
        "string"
    );
    assert_eq!(reverse["parameters"]["required"], json!(["text"]));
}

#[test]
fn multi_tool_loads() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let decls = tools.json().unwrap();
    let names: Vec<&str> = decls
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d["name"].as_str())
        .collect();

    assert!(names.contains(&"add"), "missing 'add': {names:?}");
    assert!(names.contains(&"multiply"), "missing 'multiply': {names:?}");
}

#[test]
fn no_tools_file_produces_no_extra_tools() {
    // no_tools.py has no @tool decorators — should not add any tools
    // beyond what inventory provides. We can't test "exactly zero from
    // this file" easily, but we can verify it doesn't error.
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let decls = tools.json().unwrap();
    let names: Vec<&str> = decls
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d["name"].as_str())
        .collect();

    // helper() from no_tools.py should NOT appear
    assert!(!names.contains(&"helper"));
}

// ============================================================================
// EXECUTION
// ============================================================================

#[tokio::test]
async fn tool_callable() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let resp = tools
        .call(FunctionCall::new(
            "reverse".into(),
            json!({ "text": "hello" }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.result, json!("olleh"));
}

#[tokio::test]
async fn tool_with_numeric_args() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let resp = tools
        .call(FunctionCall::new(
            "add".into(),
            json!({ "a": 17, "b": 25 }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.result, json!(42));
}

// ============================================================================
// TYPE HINTS → SCHEMA
// ============================================================================

#[test]
fn param_types_to_schema() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let decls = tools.json().unwrap();
    let greet = decls
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "greet")
        .expect("greet tool not found");

    assert_eq!(
        greet["parameters"]["properties"]["name"]["type"],
        "string"
    );
    assert_eq!(
        greet["parameters"]["properties"]["times"]["type"],
        "integer"
    );
}

#[test]
fn optional_params_not_required() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let decls = tools.json().unwrap();
    let greet = decls
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "greet")
        .expect("greet tool not found");

    let required = greet["parameters"]["required"]
        .as_array()
        .unwrap();

    assert!(required.contains(&json!("name")));
    assert!(!required.contains(&json!("times")), "times should be optional");
}

#[test]
fn enum_params_schema() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let decls = tools.json().unwrap();
    let convert = decls
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "convert")
        .expect("convert tool not found");

    let unit_schema = &convert["parameters"]["properties"]["unit"];
    assert_eq!(unit_schema["type"], "string");
    assert_eq!(
        unit_schema["enum"],
        json!(["celsius", "fahrenheit"])
    );
}

// ============================================================================
// METADATA
// ============================================================================

#[test]
fn meta_extracted() {
    #[derive(Debug, Default, Deserialize)]
    #[serde(default)]
    struct Policy {
        requires_approval: bool,
        cost_tier: u8,
    }

    let tools = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .with_meta::<Policy>()
        .collect()
        .unwrap();

    let convert_meta = tools.meta("convert").expect("convert tool not found");
    assert!(convert_meta.requires_approval);

    let multiply_meta = tools.meta("multiply").expect("multiply tool not found");
    assert_eq!(multiply_meta.cost_tier, 2);
}

// ============================================================================
// DESCRIPTION
// ============================================================================

#[test]
fn description_extracted() {
    let tools: ToolCollection = ToolsBuilder::new()
        .with_language(Language::Python)
        .from_path(fixtures_dir())
        .collect()
        .unwrap();

    let decls = tools.json().unwrap();
    let add = decls
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "add")
        .expect("add tool not found");

    assert_eq!(add["description"], "Add two numbers.");
}
