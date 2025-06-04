use serde::{Deserialize, Serialize};
use tools_rs::ToolSchema;
use tools_rs::{FunctionCall, collect_tools, tool};

#[derive(Serialize, Deserialize, ToolSchema)]
struct TestInput {
    value: i32,
}

#[derive(Serialize, Deserialize, ToolSchema)]
struct TestOutput {
    doubled: i32,
}

#[tool]
/// Double the input value
async fn double_value(input: TestInput) -> TestOutput {
    TestOutput {
        doubled: input.value * 2,
    }
}

#[tokio::test]
async fn test_schema_generation_without_features() {
    // Collect tools should work
    let tools = collect_tools();

    // Should be able to generate JSON schema
    let json_result = tools.json();
    assert!(json_result.is_ok());

    let json = json_result.unwrap();
    assert!(json.is_array());

    // Should contain our tool
    let tools_array = json.as_array().unwrap();
    assert!(!tools_array.is_empty());

    // Find our double_value tool
    let double_tool = tools_array
        .iter()
        .find(|tool| tool["name"] == "double_value")
        .expect("double_value tool should be registered");

    // Verify it has proper schema
    assert_eq!(double_tool["name"], "double_value");
    assert_eq!(double_tool["description"], "Double the input value");
    assert!(double_tool["parameters"].is_object());

    // Verify the tool actually works
    let call = FunctionCall {
        name: "double_value".to_string(),
        arguments: serde_json::json!({
            "input": { "value": 21 }
        }),
    };

    let result = tools.call(call).await;
    assert!(result.is_ok());

    let output: TestOutput = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(output.doubled, 42);
}

#[test]
fn test_schema_traits_available() {
    // Verify that ToolSchema trait is available and working
    let _input_schema = TestInput::schema();
    let _output_schema = TestOutput::schema();

    // Basic types should also have schemas
    let _string_schema = String::schema();
    let _int_schema = i32::schema();
    let _bool_schema = bool::schema();
}
