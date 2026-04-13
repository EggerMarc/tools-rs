//! Adapter-agnostic FFI integration tests.
//!
//! These test the `RawToolDef` → `register_raw` → call pipeline using
//! hand-built tool definitions (no real interpreter). Validates the
//! plumbing that every language adapter will rely on.

use serde::Deserialize;
use serde_json::{json, Value};
use tools_rs::{FunctionCall, RawToolDef, ToolCollection, ToolError, ToolsBuilder};

// ============================================================================
// HELPERS
// ============================================================================

/// Build a simple `RawToolDef` that echoes a `msg` param.
fn echo_tool_def() -> RawToolDef {
    RawToolDef {
        name: "echo".into(),
        description: "Echoes the input message".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "msg": { "type": "string", "description": "Message to echo" }
            },
            "required": ["msg"]
        }),
        meta: json!({}),
        func: Box::new(|args| {
            Box::pin(async move {
                let msg = args
                    .get("msg")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                Ok(Value::String(msg.to_string()))
            })
        }),
    }
}

/// Build a `RawToolDef` that adds two numbers.
fn add_tool_def() -> RawToolDef {
    RawToolDef {
        name: "add".into(),
        description: "Adds two numbers".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "a": { "type": "integer" },
                "b": { "type": "integer" }
            },
            "required": ["a", "b"]
        }),
        meta: json!({}),
        func: Box::new(|args| {
            Box::pin(async move {
                let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                Ok(Value::Number((a + b).into()))
            })
        }),
    }
}

/// Build a `RawToolDef` with custom meta fields.
fn meta_tool_def(meta: Value) -> RawToolDef {
    RawToolDef {
        name: "guarded".into(),
        description: "A tool with metadata".into(),
        parameters: json!({ "type": "object", "properties": {}, "required": [] }),
        meta,
        func: Box::new(|_| Box::pin(async move { Ok(Value::Null) })),
    }
}

/// Build a `RawToolDef` whose closure returns an error.
fn failing_tool_def() -> RawToolDef {
    RawToolDef {
        name: "fail".into(),
        description: "Always fails".into(),
        parameters: json!({ "type": "object", "properties": {}, "required": [] }),
        meta: json!({}),
        func: Box::new(|_| {
            Box::pin(async move {
                Err(ToolError::Runtime("intentional failure".into()))
            })
        }),
    }
}

/// Register a `RawToolDef` into a `ToolCollection`, leaking name/desc.
fn register_def(
    collection: &mut ToolCollection,
    def: RawToolDef,
) -> Result<(), ToolError> {
    let name: &'static str = Box::leak(def.name.into_boxed_str());
    let desc: &'static str = Box::leak(def.description.into_boxed_str());
    let func = def.func;
    collection.register_raw(name, desc, def.parameters, move |v| func(v), ())?;
    Ok(())
}

// ============================================================================
// REGISTRATION
// ============================================================================

#[test]
fn raw_tool_def_registers_into_collection() {
    let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();
    register_def(&mut tools, echo_tool_def()).unwrap();

    let decls = tools.json().unwrap();
    let arr = decls.as_array().unwrap();
    assert!(arr.iter().any(|d| d["name"] == "echo"));
}

#[test]
fn raw_tool_def_schema_appears_in_declarations() {
    let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();
    register_def(&mut tools, echo_tool_def()).unwrap();

    let decls = tools.json().unwrap();
    let echo_decl = decls
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "echo")
        .unwrap();

    assert_eq!(echo_decl["description"], "Echoes the input message");
    assert_eq!(echo_decl["parameters"]["properties"]["msg"]["type"], "string");
    assert_eq!(echo_decl["parameters"]["required"], json!(["msg"]));
}

#[test]
fn multiple_raw_tools_register() {
    let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();
    register_def(&mut tools, echo_tool_def()).unwrap();
    register_def(&mut tools, add_tool_def()).unwrap();

    let decls = tools.json().unwrap();
    let names: Vec<&str> = decls
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d["name"].as_str())
        .collect();

    assert!(names.contains(&"echo"));
    assert!(names.contains(&"add"));
}

#[test]
fn duplicate_name_errors() {
    let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();
    register_def(&mut tools, echo_tool_def()).unwrap();

    let err = register_def(&mut tools, echo_tool_def()).unwrap_err();
    assert!(matches!(err, ToolError::AlreadyRegistered { .. }));
}

// ============================================================================
// EXECUTION
// ============================================================================

#[tokio::test]
async fn raw_tool_callable() {
    let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();
    register_def(&mut tools, echo_tool_def()).unwrap();

    let resp = tools
        .call(FunctionCall::new("echo".into(), json!({ "msg": "hello" })))
        .await
        .unwrap();

    assert_eq!(resp.result, json!("hello"));
}

#[tokio::test]
async fn raw_tool_with_numeric_args() {
    let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();
    register_def(&mut tools, add_tool_def()).unwrap();

    let resp = tools
        .call(FunctionCall::new("add".into(), json!({ "a": 17, "b": 25 })))
        .await
        .unwrap();

    assert_eq!(resp.result, json!(42));
}

#[tokio::test]
async fn raw_tool_error_propagates() {
    let mut tools: ToolCollection = ToolsBuilder::new().collect().unwrap();
    register_def(&mut tools, failing_tool_def()).unwrap();

    let err = tools
        .call(FunctionCall::new("fail".into(), json!({})))
        .await
        .unwrap_err();

    assert!(err.to_string().contains("intentional failure"));
}

// ============================================================================
// METADATA
// ============================================================================

#[test]
fn raw_tool_meta_deserializes_into_typed_collection() {
    #[derive(Debug, Default, Deserialize)]
    #[serde(default)]
    struct Policy {
        requires_approval: bool,
        cost_tier: u8,
    }

    let mut tools = ToolsBuilder::new()
        .with_meta::<Policy>()
        .collect()
        .unwrap();

    let def = meta_tool_def(json!({ "requires_approval": true, "cost_tier": 3 }));
    let name: &'static str = Box::leak(def.name.into_boxed_str());
    let desc: &'static str = Box::leak(def.description.into_boxed_str());
    let meta: Policy = serde_json::from_value(def.meta).unwrap();
    let func = def.func;
    tools
        .register_raw(name, desc, def.parameters, move |v| func(v), meta)
        .unwrap();

    let entry = tools.get("guarded").unwrap();
    assert!(entry.meta.requires_approval);
    assert_eq!(entry.meta.cost_tier, 3);
}

#[test]
fn raw_tool_bad_meta_is_catchable() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct StrictPolicy {
        requires_approval: bool, // required, no default
    }

    let def = meta_tool_def(json!({})); // missing required field
    let result: Result<StrictPolicy, _> = serde_json::from_value(def.meta);
    assert!(result.is_err());
}

// ============================================================================
// LEAK_STRING
// ============================================================================

#[test]
fn leak_string_produces_valid_static_str() {
    let dynamic = String::from("my_tool_name");
    let leaked: &'static str = Box::leak(dynamic.into_boxed_str());
    assert_eq!(leaked, "my_tool_name");
}

#[test]
fn leak_string_works_with_unicode() {
    let dynamic = String::from("wetter_zürich");
    let leaked: &'static str = Box::leak(dynamic.into_boxed_str());
    assert_eq!(leaked, "wetter_zürich");
}
