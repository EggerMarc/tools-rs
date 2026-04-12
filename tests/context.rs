//! Tests for shared context (`Arc<T>`) injection via `CollectionBuilder`.
//!
//! Each integration-test file has its own `inventory`, so the tools
//! declared here don't leak into the metadata tests or vice-versa.

use std::sync::Arc;

use serde::Deserialize;
use tools_core::{NoMeta, ToolCollection, ToolError};
use tools_rs::{tool, FunctionCall};

// ---------- shared state ----------

struct AppState {
    greeting: String,
}

// ---------- tools ----------

#[tool]
/// Greets someone using the shared AppState.
async fn greet(ctx: AppState, name: String) -> String {
    format!("{}, {}!", ctx.greeting, name)
}

#[tool]
/// A plain tool that doesn't need context.
async fn echo(msg: String) -> String {
    msg
}

// ---------- happy path ----------

#[tokio::test]
async fn builder_injects_context_into_tool() {
    let state = Arc::new(AppState {
        greeting: "Howdy".into(),
    });
    let tools = ToolCollection::<NoMeta>::builder()
        .with_context(state)
        .collect()
        .expect("collect with ctx");

    let resp = tools
        .call(FunctionCall::new(
            "greet".into(),
            serde_json::json!({ "name": "Alice" }),
        ))
        .await
        .expect("call greet");

    assert_eq!(resp.result, serde_json::json!("Howdy, Alice!"));
}

// ---------- mixed collection (ctx + non-ctx) ----------

#[tokio::test]
async fn mixed_collection_ctx_and_non_ctx() {
    let state = Arc::new(AppState {
        greeting: "Hi".into(),
    });
    let tools = ToolCollection::<NoMeta>::builder()
        .with_context(state)
        .collect()
        .expect("mixed collect");

    // ctx tool works
    let resp = tools
        .call(FunctionCall::new(
            "greet".into(),
            serde_json::json!({ "name": "Bob" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.result, serde_json::json!("Hi, Bob!"));

    // non-ctx tool works too
    let resp = tools
        .call(FunctionCall::new(
            "echo".into(),
            serde_json::json!({ "msg": "ping" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.result, serde_json::json!("ping"));
}

// ---------- collect_tools() rejects ctx tools ----------

#[test]
fn collect_tools_rejects_ctx_tools() {
    let result = ToolCollection::<NoMeta>::collect_tools();
    let err = match result {
        Ok(_) => panic!("expected MissingCtx error"),
        Err(e) => e,
    };
    match err {
        ToolError::MissingCtx { tool } => {
            assert_eq!(tool, "greet");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

// ---------- type mismatch ----------

#[test]
fn builder_rejects_wrong_ctx_type() {
    let wrong_ctx: Arc<i32> = Arc::new(42);
    let result = ToolCollection::<NoMeta>::builder()
        .with_context(wrong_ctx)
        .collect();
    let err = match result {
        Ok(_) => panic!("expected CtxTypeMismatch"),
        Err(e) => e,
    };
    match err {
        ToolError::CtxTypeMismatch {
            tool,
            expected,
            got,
        } => {
            assert_eq!(tool, "greet");
            assert!(expected.contains("AppState"), "expected={expected}");
            assert!(got.contains("i32"), "got={got}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

// ---------- programmatic register into ctx collection ----------

#[tokio::test]
async fn register_non_ctx_tool_into_ctx_collection() {
    let state = Arc::new(AppState {
        greeting: "Hey".into(),
    });
    let mut tools = ToolCollection::<NoMeta>::builder()
        .with_context(state)
        .collect()
        .unwrap();

    tools
        .register(
            "double",
            "Doubles a number",
            |n: (i32,)| async move { n.0 * 2 },
            (),
        )
        .unwrap();

    let resp = tools
        .call(FunctionCall::new(
            "double".into(),
            serde_json::json!([7]),
        ))
        .await
        .unwrap();
    assert_eq!(resp.result, serde_json::json!(14));
}

// ---------- JSON schema excludes ctx ----------

#[test]
fn json_schema_excludes_ctx_param() {
    let state = Arc::new(AppState {
        greeting: "Hi".into(),
    });
    let tools = ToolCollection::<NoMeta>::builder()
        .with_context(state)
        .collect()
        .unwrap();

    let entry = tools.get("greet").expect("greet exists");
    let params = &entry.decl.parameters;

    // `name` should be present
    assert!(
        params["properties"]["name"].is_object(),
        "name field missing from schema"
    );
    // `ctx` should NOT be present
    assert!(
        params["properties"].get("ctx").is_none(),
        "ctx leaked into JSON schema: {params}"
    );
}

// ---------- typed metadata + context together ----------

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(default)]
struct Policy {
    requires_approval: bool,
}

#[test]
fn metadata_and_context_work_together() {
    let state = Arc::new(AppState {
        greeting: "Hello".into(),
    });
    let tools = ToolCollection::<Policy>::builder()
        .with_context(state)
        .collect()
        .expect("meta + ctx");

    let greet_meta = tools.meta("greet").unwrap();
    assert!(!greet_meta.requires_approval);

    let echo_meta = tools.meta("echo").unwrap();
    assert!(!echo_meta.requires_approval);
}
