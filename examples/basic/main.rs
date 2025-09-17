use serde_json::json;
use tools_rs::{FunctionCall, collect_tools, tool};

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
    println!("Tools-rs Basic Example\n===================");

    let hub = collect_tools();

    let sum = hub
        .call(FunctionCall::new("add".into(), json!({ "pair": [3, 4] })))
        .await?
        .result;
    println!("add(3, 4) → {sum}");

    let greeting = hub
        .call(FunctionCall::new(
            "greet".into(),
            json!({ "name": "World" }),
        ))
        .await?
        .result;
    println!("greet(\"World\") → {greeting}");

    let fib = hub
        .call(FunctionCall::new("fibonacci".into(), json!({ "n": 10 })))
        .await?
        .result;
    println!("fibonacci(10) → {fib}");

    println!("\nAvailable tools:");
    for (name, description) in hub.descriptions() {
        println!("  - {name}: {description}");
    }

    Ok(())
}
