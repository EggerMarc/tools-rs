use serde::{Deserialize, Serialize};
use serde_json::json;
use toors_core::{collect_tools, tool, FunctionCall};

#[tool]
/// Adds two numbers.
async fn add(pair: (i32, i32)) -> i32 {
    pair.0 + pair.1
}

#[derive(Serialize, Deserialize)]
struct CalculateRequest {
    operation: String,
    a: f64,
    b: f64,
}

#[tool]
/// Performs basic arithmetic operations.
async fn calculate(req: CalculateRequest) -> f64 {
    match req.operation.as_str() {
        "add" => req.a + req.b,
        "subtract" => req.a - req.b,
        "multiply" => req.a * req.b,
        "divide" => req.a / req.b,
        _ => panic!("Unsupported operation: {}", req.operation),
    }
}

#[tool]
/// Gets weather information for a location.
async fn get_weather(location: String) -> String {
    format!("Weather for {}: 22°C, Sunny", location)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect all registered tools
    let tools = collect_tools();
    
    // List all available tools
    println!("Available tools:");
    for (name, description) in tools.descriptions() {
        println!("  - {}: {}", name, description);
    }
    
    // Call the add function
    let result = tools
        .call(FunctionCall {
            name: "add".into(),
            arguments: json!((5, 7)),
        })
        .await?;
    println!("\nAdd result: {}", result);
    
    // Call the calculate function
    let result = tools
        .call(FunctionCall {
            name: "calculate".into(),
            arguments: json!({
                "operation": "multiply", 
                "a": 3.5, 
                "b": 2.0
            }),
        })
        .await?;
    println!("Calculate result: {}", result);
    
    // Call the weather function
    let result = tools
        .call(FunctionCall {
            name: "get_weather".into(),
            arguments: json!("London"),
        })
        .await?;
    println!("Weather result: {}", result);
    
    // Export function declarations
    let declarations = tools.json();
    println!("\nFunction declarations:");
    println!("{}", serde_json::to_string_pretty(&declarations)?);
    
    Ok(())
}