use serde_json::json;
use toors_core::{collect_tools, FunctionCall, tool};

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
/// Calculates the fibonacci number at the given position.
async fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 | 2 => 1,
        n => {
            let mut a = 0u64;
            let mut b = 1u64;
            
            for _ in 2..=n {
                let temp = a + b;
                a = b;
                b = temp;
            }
            
            b
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Toors Basic Example");
    println!("===================");
    
    let hub = collect_tools();
    
    // Example 1: Add two numbers
    let sum = hub
        .call(FunctionCall {
            name: "add".into(),
            arguments: json!([3, 4]),
        })
        .await?;
    println!("add(3, 4) → {sum}");
    
    // Example 2: Greet a person
    let greeting = hub
        .call(FunctionCall {
            name: "greet".into(),
            arguments: json!("World"),
        })
        .await?;
    println!("greet(\"World\") → {greeting}");
    
    // Example 3: Calculate fibonacci
    let fib = hub
        .call(FunctionCall {
            name: "fibonacci".into(),
            arguments: json!(10),
        })
        .await?;
    println!("fibonacci(10) → {fib}");
    
    // List all available tools
    println!("\nAvailable tools:");
    for (name, description) in hub.descriptions() {
        println!("  - {name}: {description}");
    }
    
    Ok(())
}