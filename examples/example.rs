#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tools_rs::{FunctionCall, collect_tools, tool};

#[tool]
/// Adds two numbers.
async fn add(pair: (i32, i32)) -> i32 {
    pair.0 + pair.1
}

#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
    format!("Weather for {}: 22 °C, Sunny", location)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tools = collect_tools();

    println!("Available tools:");
    for (name, description) in tools.descriptions() {
        println!("  - {}: {}", name, description);
    }

    // ------ add ----------------------------------------------------------
    let sum = tools
        .call(FunctionCall::new(
            "add".into(),
            json!({ "pair": [5, 7] }), //  👈  field name = pair
        ))
        .await?;
    println!("\nAdd result: {}", sum.result);

    // ------ calculate ----------------------------------------------------
    let calc = tools
        .call(FunctionCall::new(
            "calculate".into(),
            json!({
                "req": {                     // 👈  field name = req
                    "operation": "multiply",
                    "a": 3.5,
                    "b": 2.0
                }
            }),
        ))
        .await?;
    println!("Calculate result: {}", calc.result);

    // ------ get_weather --------------------------------------------------
    let weather = tools
        .call(FunctionCall::new(
            "get_weather".into(),
            json!({ "location": "London" }), // 👈  field name = location
        ))
        .await?;
    println!("Weather result: {}", weather.result);

    let decls = tools.json()?; // now `decls: serde_json::Value`
    println!(
        "\nFunction declarations:\n{}",
        serde_json::to_string_pretty(&decls)? // <— OK, `Value: Serialize`
    );

    Ok(())
}
