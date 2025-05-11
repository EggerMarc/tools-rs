use serde_json::json;
use toors_core::{collect_tools, tool, FunctionCall};

#[tool]
/// Adds two numbers.
async fn add(pair: (i32, i32)) -> i32 {
    pair.0 + pair.1
}

#[tool]
/// Greets a person.
async fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub = collect_tools();

    let sum = hub
        .call(FunctionCall {
            name: "add".into(),
            arguments: json!([3, 4]),
        })
        .await?;
    println!("add → {sum}");

    let hi = hub
        .call(FunctionCall {
            name: "greet".into(),
            arguments: json!("Alice"),
        })
        .await?;
    println!("greet → {hi}");

    Ok(())
}
