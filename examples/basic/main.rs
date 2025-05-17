//! toors_basic.rs – minimal demo that matches the new Toors
//! wrapper-struct convention (each parameter lives in a JSON object whose
//! keys equal the Rust argument names).

use serde_json::json;
use toors_core::{FunctionCall, collect_tools, tool};

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

#[tool]
/// Calculates the Fibonacci number at the given position.
async fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 | 2 => 1,
        n => {
            let (mut a, mut b) = (0, 1);
            for _ in 2..=n {
                let tmp = a + b;
                a = b;
                b = tmp;
            }
            b
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Toors Basic Example\n===================");

    let hub = collect_tools();

    // ------------------------------------------------------------------
    // 1. add(3, 4)   — JSON key must match the parameter name `pair`
    // ------------------------------------------------------------------
    let sum = hub
        .call(FunctionCall {
            name: "add".into(),
            arguments: json!({ "pair": [3, 4] }),
        })
        .await?;
    println!("add(3, 4) → {sum}");

    // ------------------------------------------------------------------
    // 2. greet("World")  — key is `name`
    // ------------------------------------------------------------------
    let greeting = hub
        .call(FunctionCall {
            name: "greet".into(),
            arguments: json!({ "name": "World" }),
        })
        .await?;
    println!("greet(\"World\") → {greeting}");

    // ------------------------------------------------------------------
    // 3. fibonacci(10)   — key is `n`
    // ------------------------------------------------------------------
    let fib = hub
        .call(FunctionCall {
            name: "fibonacci".into(),
            arguments: json!({ "n": 10 }),
        })
        .await?;
    println!("fibonacci(10) → {fib}");

    // ------------------------------------------------------------------
    // Available tools
    // ------------------------------------------------------------------
    println!("\nAvailable tools:");
    for (name, description) in hub.descriptions() {
        println!("  - {name}: {description}");
    }

    Ok(())
}
