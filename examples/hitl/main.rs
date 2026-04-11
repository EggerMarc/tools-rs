//! Human-in-the-loop tool gating, driven by `#[tool(...)]` metadata.
//!
//! This example shows how a single tool declaration can carry policy
//! information that the runtime reads through a typed
//! `ToolCollection<Policy>`. Tools flagged `requires_approval = true`
//! prompt the operator on stdin before executing; safe tools run straight
//! through. The same `#[tool]` attributes could feed any other policy
//! schema in a different binary — the tools don't know about HITL.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example hitl
//! ```

use std::io::{self, BufRead, Write};

use serde::Deserialize;
use serde_json::json;
use tools_core::{validate_tool_attrs, ToolCollection};
use tools_rs::{tool, FunctionCall};

// ---------- policy metadata ----------

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(default)]
struct Policy {
    /// Tool must be confirmed by a human before it runs.
    requires_approval: bool,
    /// Risk classification — drives prompt formatting and could feed
    /// audit logs, rate limits, or escalation rules in a real system.
    #[serde(default)]
    risk: Risk,
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Risk {
    #[default]
    Low,
    Medium,
    High,
}

// ---------- tools ----------
//
// Safe reads carry no attributes — they get `Policy::default()`, which
// has `requires_approval = false` and `risk = Low`.

#[tool]
/// Read a file from disk and return its contents.
async fn read_file(path: String) -> String {
    format!("(pretend contents of {path})")
}

#[tool]
/// List the entries in a directory.
async fn list_dir(path: String) -> String {
    format!("(pretend listing of {path})")
}

// Mutating tools opt in to approval. Risk tier is declared inline; the
// macro turns these flat key/value pairs into JSON the collection will
// deserialize into `Policy` at startup.

#[tool(requires_approval = true, risk = "medium")]
/// Write contents to a file, overwriting it if it exists.
async fn write_file(path: String, contents: String) -> String {
    format!("wrote {} bytes to {path}", contents.len())
}

#[tool(requires_approval = true, risk = "high")]
/// Permanently delete a file.
async fn delete_file(path: String) -> String {
    format!("deleted {path}")
}

#[tool(requires_approval = true, risk = "high")]
/// Drop a database table.
async fn drop_table(name: String) -> String {
    format!("dropped table `{name}`")
}

// ---------- HITL gate ----------

/// Decision returned by the operator (or by an auto-policy).
enum Decision {
    Approve,
    Deny,
}

fn prompt_operator(call: &FunctionCall, policy: Policy) -> Decision {
    let risk_label = match policy.risk {
        Risk::Low => "LOW",
        Risk::Medium => "MEDIUM",
        Risk::High => "HIGH",
    };
    println!();
    println!("┌─ approval required ──────────────────────────────");
    println!("│ tool      : {}", call.name);
    println!("│ risk      : {risk_label}");
    println!("│ arguments : {}", call.arguments);
    print!("└─ approve? [y/N] ");
    io::stdout().flush().ok();

    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).is_err() {
        return Decision::Deny;
    }
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Decision::Approve,
        _ => Decision::Deny,
    }
}

// ---------- driver ----------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CI guard rail: every #[tool(...)] in this binary must conform to
    // `Policy`. Catches typos like `requires_aproval = true` at startup,
    // before any LLM call lands. In a real test suite this would live in
    // a `#[test]` instead of main.
    if let Err(errors) = validate_tool_attrs::<Policy>() {
        eprintln!("policy validation failed:");
        for e in errors {
            eprintln!("  - {}", e);
        }
        std::process::exit(1);
    }

    let tools = ToolCollection::<Policy>::collect_tools()?;

    println!("registered tools:");
    let mut names: Vec<&str> = tools.iter().map(|(n, _)| n).collect();
    names.sort_unstable();
    for name in names {
        let entry = tools.get(name).unwrap();
        let gate = if entry.meta.requires_approval {
            format!("requires approval ({:?})", entry.meta.risk)
        } else {
            "auto".to_string()
        };
        println!("  - {name}: {gate}");
    }

    // A pretend LLM produced this sequence of tool calls. In a real
    // application this would come from the model's tool_calls output.
    let plan: Vec<FunctionCall> = vec![
        FunctionCall::new("read_file".into(), json!({ "path": "/etc/hosts" })),
        FunctionCall::new("list_dir".into(), json!({ "path": "/tmp" })),
        FunctionCall::new(
            "write_file".into(),
            json!({ "path": "/tmp/note.txt", "contents": "hello" }),
        ),
        FunctionCall::new("delete_file".into(), json!({ "path": "/tmp/old.log" })),
        FunctionCall::new("drop_table".into(), json!({ "name": "users" })),
    ];

    for call in plan {
        let entry = match tools.get(&call.name) {
            Some(e) => e,
            None => {
                println!("\n[skip] unknown tool: {}", call.name);
                continue;
            }
        };

        if entry.meta.requires_approval {
            match prompt_operator(&call, entry.meta) {
                Decision::Deny => {
                    println!("[deny] {} → skipped by operator", call.name);
                    continue;
                }
                Decision::Approve => {
                    println!("[ok]   {} → approved", call.name);
                }
            }
        }

        let response = tools.call(call).await?;
        println!("       result: {}", response.result);
    }

    Ok(())
}
