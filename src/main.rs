use serde_json::json;
use tools::FunctionCall;
use tools_macros::tool;
use tools_rs::collect_tools;

#[tool]
/// Adds two numbers (pair).
async fn add(pair: (i32, i32)) -> i32 {
    pair.0 + pair.1
}

#[tool]
/// Greets a person (name).
async fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub = collect_tools();

    let sum = hub
        .call(FunctionCall::new("add".into(), json!({ "pair": [3, 4] })))
        .await?
        .result;
    println!("add → {sum}"); // 7

    let hi = hub
        .call(FunctionCall::new(
            "greet".into(),
            json!({ "name": "Alice" }),
        ))
        .await?
        .result;
    println!("greet → {hi}"); // "Hello, Alice!"

    Ok(())
}
