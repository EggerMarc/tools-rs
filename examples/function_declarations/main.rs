//! # Mini-Toolbox Demo
//!
//! This single-file crate shows **how to expose pure Rust helpers as
//! “tools” that a Large-Language-Model (LLM) can call at run-time**.
//!
//! It is deliberately tiny—just three utilities—so the full round-trip
//! fits on one screen:
//!
//! 1. **`today()`**&nbsp;→ `String` &nbsp;— returns *today* as
//!    `YYYY-MM-DD`.  
//! 2. **`factorial(n)`**&nbsp;→ `u64` &nbsp;— computes `n!`.  
//! 3. **`weather(loc)`**&nbsp;→ `WeatherInfo` &nbsp;— dummy forecast.
//!
//! Under the hood the [`tools_rs::tool`] macro
//!
//! * **Registers** each `async fn` in a link-time inventory so it can be
//!   discovered by name (`collect_tools()`).
//! * Optionally **generates JSON-Schema** for the parameters / return
//!   types (gated behind the `schema` feature) so an LLM can validate
//!   payloads before the call reaches Rust.
//!
//! The `main()` function walks through the canonical flow:
//!
//! ```text
//! compile-time registry  ──▶  runtime lookup
//!                     ▲                │
//!                     └─ declarations ◀┘
//! ```
//!
//! *Dump the declarations* → *build a chat-completion request* → *call
//! each tool directly* so you can compare the “LLM path” against the
//! “plain Rust path” side-by-side.

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use tools_rs::{FunctionCall, collect_tools, function_declarations, tool, ToolSchema};

// ────────────────────────────────────────────────────────────────────────────
// Data Transfer Objects
// ────────────────────────────────────────────────────────────────────────────

/// Result structure returned by [`weather`].
///
/// When the **`schema`** Cargo feature is enabled we derive
/// [`tool_schema::ToolSchema`] so the LLM receives an *exact* contract.
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct WeatherInfo {
    /// City or place name passed to the tool.
    location: String,
    /// Stubbed temperature in °C.
    temperature: f32,
    /// Human-readable conditions (e.g. “Sunny”).
    conditions: String,
    /// Relative humidity in percent.
    humidity: u8,
}

// ────────────────────────────────────────────────────────────────────────────
// Tool functions (the LLM-callable API surface)
// ────────────────────────────────────────────────────────────────────────────

#[tool]
/// Return *today’s* date in ISO-8601 calendar form (`YYYY-MM-DD`).
async fn today() -> String {
    chrono::Utc::now().date_naive().to_string()
}
#[tool]
/// Compute the factorial of **`n`** (`0! = 1`, `1! = 1`, …).
async fn factorial(n: u64) -> u64 {
    (1..=n).product()
}
#[tool]
/// Return a *fake* weather report for **`location`**.
async fn weather(location: String) -> WeatherInfo {
    WeatherInfo {
        location,
        temperature: 22.5,
        conditions: "Sunny".into(),
        humidity: 45,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Driver program
// ────────────────────────────────────────────────────────────────────────────

/// Boot a Tokio runtime and exercise the full *tool lifecycle*.
///
/// 1. **Discover** tools with [`collect_tools`].  
/// 2. **Generate** JSON declarations for OpenAI.  
/// 3. **Build** a chat-completion request showcasing those tools.  
/// 4. **Invoke** each tool directly to prove the registry works.
///
/// Any error (schema generation, malformed call, etc.) bubbles up through
/// `?` into the boxed trait object so the `main` signature stays minimal.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // — 1. Discovery —
    let tools = collect_tools();

    // — 2. Declarations —
    let decls: JsonValue = function_declarations()?;
    println!("=== Function Declarations ===");
    println!("{}", serde_json::to_string_pretty(&decls)?);

    // — 3. Example request payload as the LLM would see it —
    let chat_request = json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "system",
              "content": "You are a helpful assistant with tool access." },
            { "role": "user",
              "content": "What's today's date and what's the factorial of 5?" }
        ],
        "tool_choice": "auto",
        "tools": decls          // already in the correct shape
    });
    println!("\n=== Example LLM Request ===");
    println!("{}", serde_json::to_string_pretty(&chat_request)?);

    // — 4. Runtime calls (same code path the LLM would hit) —
    let date = tools
        .call(FunctionCall {
            name: "today".into(),
            arguments: json!({}),
        })
        .await?;
    println!("\nToday  : {date}");

    let fact = tools
        .call(FunctionCall {
            name: "factorial".into(),
            arguments: json!({ "n": 5 }),
        })
        .await?;
    println!("5!     : {fact}");

    let meteo = tools
        .call(FunctionCall {
            name: "weather".into(),
            arguments: json!({ "location": "London" }),
        })
        .await?;
    println!("Weather: {meteo}");

    Ok(())
}
