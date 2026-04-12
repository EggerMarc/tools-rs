//! Shared context injection via `ToolCollection::builder().with_context(...)`.
//!
//! Tools that declare `ctx: Arc<T>` as their first parameter receive the
//! shared context automatically at call time. The caller only passes the
//! "real" arguments — context injection is invisible at the call site.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example context
//! ```

use std::sync::Arc;

use serde_json::json;
use tools_core::{NoMeta, ToolCollection};
use tools_rs::{FunctionCall, tool};

// ---------- shared context ----------

struct Context {
    mailer_url: String,
    db_name: String,
}

impl Context {
    fn build() -> Arc<Self> {
        Arc::new(Context {
            mailer_url: "smtp://mail.example.com".into(),
            db_name: "app_production".into(),
        })
    }
}

// ---------- tools that use context ----------

#[tool]
/// Sends an email via the configured mailer.
async fn send_email(ctx: Context, address: String, subject: String, body: String) -> String {
    format!(
        "[{}] sent to {address}: \"{subject}\" — {}",
        ctx.mailer_url, body
    )
}

#[tool]
/// Queries the database for a user by name.
async fn find_user(ctx: Context, name: String) -> String {
    format!(
        "[{}] SELECT * FROM users WHERE name = '{name}'",
        ctx.db_name
    )
}

// ---------- tools that don't need context ----------

#[tool]
/// Returns the current server time.
async fn server_time() -> String {
    "2026-04-12T10:30:00Z (pretend)".into()
}

// ---------- driver ----------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Context::build();

    let tools = ToolCollection::<NoMeta>::builder()
        .with_context(ctx)
        .collect()?;

    println!("registered tools:");
    let mut names: Vec<&str> = tools.iter().map(|(n, _)| n).collect();
    names.sort_unstable();
    for name in &names {
        let needs_ctx = if tools.get(name).unwrap().decl.parameters["properties"]
            .get("ctx")
            .is_none()
            && tools.get(name).unwrap().decl.parameters["properties"]
                .as_object()
                .map_or(true, |p| !p.is_empty())
        {
            ""
        } else {
            " (no args)"
        };
        println!("  - {name}{needs_ctx}");
    }

    // Simulate an LLM calling tools — caller never mentions ctx.
    let calls: Vec<FunctionCall> = vec![
        FunctionCall::new(
            "send_email".into(),
            json!({
                "address": "m@example.com",
                "subject": "Greetings",
                "body": "Nice to meet you"
            }),
        ),
        FunctionCall::new("find_user".into(), json!({ "name": "Alice" })),
        FunctionCall::new("server_time".into(), json!({})),
    ];

    for call in calls {
        println!("\n> calling `{}`", call.name);
        let resp = tools.call(call).await?;
        println!("  result: {}", resp.result);
    }

    Ok(())
}
