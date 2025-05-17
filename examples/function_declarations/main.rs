//! demo_fixed.rs – corrected minimal demo for **Toors**

use serde::{Deserialize, Serialize};
use serde_json::json;
use toors_core::{FunctionCall, collect_tools, function_declarations, tool};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

// ─────────────────────────────────────────────────────────────
// Domain types
// ─────────────────────────────────────────────────────────────

#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
struct WeatherInfo {
    location: String,
    temperature: f32,
    conditions: String,
    humidity: u8,
}

// ─────────────────────────────────────────────────────────────
// Tools
// ─────────────────────────────────────────────────────────────

#[tool]
/// Return today’s date in ISO-8601 (`YYYY-MM-DD`) format.
async fn today() -> String {
    chrono::Utc::now().date_naive().to_string()
}

#[tool]
/// Calculate the factorial of an integer `n` (`0! = 1`).
async fn factorial(n: u64) -> u64 {
    (1..=n).product()
}

#[tool]
/// Fake weather report for a given location.
async fn weather(location: String) -> WeatherInfo {
    WeatherInfo {
        location,
        temperature: 22.5,
        conditions: "Sunny".into(),
        humidity: 45,
    }
}

// ─────────────────────────────────────────────────────────────
// Driver
// ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. compile-time inventory → runtime registry
    let tools = collect_tools();

    // 2. declarations for the LLM
    let decls = function_declarations();
    println!("=== Function Declarations ===");
    println!("{}", serde_json::to_string_pretty(&decls)?);

    // 3. sketch of a chat request (OpenAI style: `tools` array)
    let chat_request = json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "system",
              "content": "You are a helpful assistant with tool access." },
            { "role": "user",
              "content": "What's today's date and what's the factorial of 5?" }
        ],
        "tool_choice": "auto",
        "tools": decls
    });
    println!("\n=== Example LLM Request ===");
    println!("{}", serde_json::to_string_pretty(&chat_request)?);

    // 4. call `today`
    let date = tools
        .call(FunctionCall {
            name: "today".into(),
            arguments: json!({}), // empty object → no parameters
        })
        .await?;
    println!("\nToday  : {date}");

    // 5. call `factorial`
    let fact = tools
        .call(FunctionCall {
            name: "factorial".into(),
            arguments: json!({ "n": 5 }), // key == parameter name
        })
        .await?;
    println!("5!     : {fact}");

    // 6. call `weather`
    let meteo = tools
        .call(FunctionCall {
            name: "weather".into(),
            arguments: json!({ "location": "London" }),
        })
        .await?;
    println!("Weather: {meteo}");

    Ok(())
}
