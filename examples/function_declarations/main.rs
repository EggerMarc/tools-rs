use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use tools_rs::{FunctionCall, collect_tools, function_declarations, tool};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
struct WeatherInfo {
    location: String,
    temperature: f32,
    conditions: String,
    humidity: u8,
}

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tools = collect_tools();
    let decls: JsonValue = function_declarations()?;

    println!("=== Function Declarations ===");
    println!("{}", serde_json::to_string_pretty(&decls)?);

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

    let date = tools
        .call(FunctionCall::new("today".into(), json!({})))
        .await?
        .result;
    println!("\nToday  : {date}");

    let fact = tools
        .call(FunctionCall::new("factorial".into(), json!({ "n": 5 })))
        .await?
        .result;
    println!("5!     : {fact}");

    let meteo = tools
        .call(FunctionCall::new(
            "weather".into(),
            json!({ "location": "London" }),
        ))
        .await?
        .result;
    println!("Weather: {meteo}");

    Ok(())
}
