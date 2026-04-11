//! Tests for `#[tool(...)]` attribute metadata and `ToolCollection<M>`.
//!
//! Each integration-test file is its own binary with its own `inventory`,
//! so the tools declared here don't leak into other tests.

use serde::Deserialize;
use tools_core::{
    validate_tool_attrs, validate_tool_attrs_for, MetaValidationError, NoMeta, ToolCollection,
    ToolError,
};
use tools_rs::tool;

// ---------- tools under test ----------

#[tool(requires_approval = true, cost_tier = 3)]
/// Deletes a file
async fn delete_file(path: String) -> String {
    format!("deleted {path}")
}

#[tool(requires_approval = true)]
/// Writes a file
async fn write_file(path: String) -> String {
    format!("wrote {path}")
}

#[tool]
/// Reads a file (no metadata declared)
async fn read_file(path: String) -> String {
    format!("read {path}")
}

#[tool(experimental)]
/// Bare-flag attribute → `{"experimental": true}`
async fn flaky_tool() -> String {
    "ok".into()
}

// ---------- metadata schemas ----------

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(default)]
struct Policy {
    requires_approval: bool,
    cost_tier: u8,
    #[serde(default)]
    experimental: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct StrictPolicy {
    requires_approval: bool,
}

// ---------- untyped baseline ----------

#[tokio::test]
async fn untyped_collection_works_for_all_tools() {
    let tools: ToolCollection<NoMeta> = ToolCollection::collect_tools().unwrap();

    for name in ["delete_file", "write_file", "read_file", "flaky_tool"] {
        assert!(tools.get(name).is_some(), "missing tool: {name}");
    }
}

// ---------- typed happy path ----------

#[test]
fn typed_collection_reads_attrs() {
    let tools = ToolCollection::<Policy>::collect_tools().expect("policy collect");

    let delete = tools.meta("delete_file").unwrap();
    assert!(delete.requires_approval);
    assert_eq!(delete.cost_tier, 3);

    let read = tools.meta("read_file").unwrap();
    assert!(!read.requires_approval, "default fills missing field");
    assert_eq!(read.cost_tier, 0);

    let flaky = tools.meta("flaky_tool").unwrap();
    assert!(flaky.experimental, "bare flag becomes true");
}

#[test]
fn iter_yields_all_entries_with_meta() {
    let tools = ToolCollection::<Policy>::collect_tools().unwrap();
    let approval_required: Vec<&str> = tools
        .iter()
        .filter(|(_, e)| e.meta.requires_approval)
        .map(|(name, _)| name)
        .collect();
    assert!(approval_required.contains(&"delete_file"));
    assert!(approval_required.contains(&"write_file"));
    assert!(!approval_required.contains(&"read_file"));
}

// ---------- strict schema rejects unknown attrs ----------

#[test]
fn strict_policy_rejects_unknown_fields() {
    // `delete_file` declared `cost_tier`, which `StrictPolicy` does not know.
    let result = ToolCollection::<StrictPolicy>::collect_tools();
    let err = match result {
        Ok(_) => panic!("strict policy should reject cost_tier"),
        Err(e) => e,
    };
    match err {
        ToolError::BadMeta { tool, error } => {
            assert!(
                tool == "delete_file" || tool == "flaky_tool",
                "unexpected failing tool: {tool}",
            );
            assert!(error.contains("unknown field"), "error was: {error}");
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

// ---------- accumulating validators ----------

#[test]
fn validate_tool_attrs_accumulates_strict_failures() {
    let errors = validate_tool_attrs::<StrictPolicy>().unwrap_err();
    // delete_file (cost_tier) and flaky_tool (experimental) both violate strict.
    assert!(errors.len() >= 2, "expected at least 2 failures, got {errors:?}");
    let names: Vec<&str> = errors.iter().map(|e| e.tool.as_ref()).collect();
    assert!(names.contains(&"delete_file"));
    assert!(names.contains(&"flaky_tool"));
}

#[test]
fn validate_tool_attrs_passes_for_permissive() {
    validate_tool_attrs::<Policy>().expect("permissive policy validates");
}

#[test]
fn validate_tool_attrs_for_validates_subset() {
    // Only check write_file and read_file against StrictPolicy — both have
    // no extra attrs → pass.
    validate_tool_attrs_for::<StrictPolicy>(&["write_file", "read_file"])
        .expect("subset is clean against strict");
}

#[test]
fn validate_tool_attrs_for_errors_on_unknown_name() {
    let errors = validate_tool_attrs_for::<Policy>(&["read_file", "no_such_tool"])
        .expect_err("unknown name must error");
    let missing: Vec<&MetaValidationError> = errors
        .iter()
        .filter(|e| e.error.contains("no tool with this name"))
        .collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].tool.as_ref(), "no_such_tool");
}

// ---------- NoMeta default generic ----------

#[test]
fn no_meta_is_the_default_generic() {
    // No turbofish → defaults to ToolCollection<NoMeta>.
    let _: ToolCollection = ToolCollection::collect_tools().unwrap();
}
