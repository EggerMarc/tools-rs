//! Quick-start: one tool, tuple argument, runtime registry listing.

use serde_json::json;
use tools_rs::{FunctionCall, collect_tools, function_declarations, tool};

#[tool]
/// Adds two numbers. The `pair` argument is a tuple.
async fn add(pair: (i32, i32)) -> i32 {
    pair.0 + pair.1
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub = collect_tools();

    let _ = function_declarations()?;

    println!(
        "add(3,4) = {}",
        hub.call(FunctionCall {
            name: "add".into(),
            arguments: json!({ "pair": [3, 4] }),
        })
        .await?
    );

    println!("tools:");
    for name in hub.descriptions().map(|(name, _)| name) {
        println!("  - {name}");
    }

    Ok(())
}
