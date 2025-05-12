use serde_json::json;
use toors_core::{collect_tools, function_declarations, tool, FunctionCall};

#[tool]
/// Return the current date in ISO-8601 format.
async fn today(_: ()) -> String {
    use chrono::Utc;
    Utc::now().date_naive().to_string()
}

#[tool]
/// Calculate the factorial of a number.
async fn factorial(n: u64) -> u64 {
    let mut result = 1;
    for i in 2..=n {
        result *= i;
    }
    result
}

#[tool]
/// Get weather information for a location.
async fn weather(location: String) -> WeatherInfo {
    // In a real implementation, this would call a weather API
    WeatherInfo {
        location,
        temperature: 22.5,
        conditions: "Sunny".to_string(),
        humidity: 45,
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct WeatherInfo {
    location: String,
    temperature: f32,
    conditions: String,
    humidity: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect all tools registered with the #[tool] macro
    let tools = collect_tools();
    
    // Generate function declarations for an LLM
    let declarations = function_declarations();
    
    println!("Function Declarations JSON:");
    println!("{}", serde_json::to_string_pretty(&declarations)?);
    
    // Example of what a request to an LLM might look like
    let llm_request = json!({
        "model": "gpt-4-turbo",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant with access to function calling."
            },
            {
                "role": "user",
                "content": "What's today's date and what's the factorial of 5?"
            }
        ],
        "functionDeclarations": declarations
    });
    
    println!("\nExample LLM Request:");
    println!("{}", serde_json::to_string_pretty(&llm_request)?);
    
    // Example of calling a tool
    println!("\nExecuting 'today' tool:");
    let result = tools
        .call(FunctionCall {
            name: "today".into(),
            arguments: json!(()),
        })
        .await?;
    println!("Today's date: {}", result);
    
    println!("\nExecuting 'factorial' tool:");
    let result = tools
        .call(FunctionCall {
            name: "factorial".into(),
            arguments: json!(5),
        })
        .await?;
    println!("Factorial of 5: {}", result);
    
    Ok(())
}