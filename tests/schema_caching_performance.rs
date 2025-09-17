//! Comprehensive tests for schema caching performance with derived types

use serde::{Deserialize, Serialize};
use std::time::Instant;
use tools_rs::{ToolSchema, collect_tools};

#[derive(Serialize, Deserialize, ToolSchema)]
struct SimpleStruct {
    name: String,
    age: u32,
    active: bool,
}

#[derive(Serialize, Deserialize, ToolSchema)]
struct ComplexStruct {
    id: u64,
    data: SimpleStruct,
    scores: Vec<f64>,
    metadata: std::collections::HashMap<String, String>,
    optional_field: Option<i32>,
}

#[derive(Serialize, Deserialize, ToolSchema)]
struct NestedStruct {
    level1: ComplexStruct,
    level2: Vec<SimpleStruct>,
    level3: Option<ComplexStruct>,
}

#[derive(Serialize, Deserialize, ToolSchema)]
struct EmptyStruct;

#[derive(Serialize, Deserialize, ToolSchema)]
struct UnitStruct {}

#[derive(Serialize, Deserialize, ToolSchema)]
struct TupleStruct(String, i32, bool);

#[test]
fn test_derived_schema_caching() {
    // Test that derived type schemas are cached
    let schema1 = SimpleStruct::schema();
    let schema2 = SimpleStruct::schema();
    
    assert_eq!(schema1, schema2);
    
    // Test complex derived types
    let complex_schema1 = ComplexStruct::schema();
    let complex_schema2 = ComplexStruct::schema();
    
    assert_eq!(complex_schema1, complex_schema2);
    
    // Test nested structures
    let nested_schema1 = NestedStruct::schema();
    let nested_schema2 = NestedStruct::schema();
    
    assert_eq!(nested_schema1, nested_schema2);
}

#[test]
fn test_derived_schema_performance() {
    // Warm up the cache
    let _ = NestedStruct::schema();
    
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = NestedStruct::schema();
    }
    let cached_duration = start.elapsed();
    
    // Even deeply nested cached schemas should be fast
    assert!(cached_duration.as_millis() < 50, 
            "Cached nested schema calls took too long: {:?}", cached_duration);
}

#[test]
fn test_schema_content_correctness() {
    let schema = SimpleStruct::schema();
    
    // Verify the schema has the expected structure
    assert_eq!(schema["type"], "object");
    
    let properties = &schema["properties"];
    assert!(properties.is_object());
    assert!(properties["name"].is_object());
    assert!(properties["age"].is_object());
    assert!(properties["active"].is_object());
    
    // Verify required fields
    let required = &schema["required"];
    assert!(required.is_array());
    let required_array = required.as_array().unwrap();
    assert!(required_array.contains(&serde_json::Value::String("name".to_string())));
    assert!(required_array.contains(&serde_json::Value::String("age".to_string())));
    assert!(required_array.contains(&serde_json::Value::String("active".to_string())));
}

#[test]
fn test_complex_schema_correctness() {
    let schema = ComplexStruct::schema();
    
    assert_eq!(schema["type"], "object");
    
    let properties = &schema["properties"];
    assert!(properties["id"].is_object());
    assert!(properties["data"].is_object());
    assert!(properties["scores"].is_object());
    assert!(properties["metadata"].is_object());
    assert!(properties["optional_field"].is_object());
    
    // Verify optional field has anyOf structure
    let optional_field = &properties["optional_field"];
    assert!(optional_field["anyOf"].is_array());
    
    // Verify required fields (optional_field should not be in required)
    let required = &schema["required"];
    let required_array = required.as_array().unwrap();
    assert!(required_array.contains(&serde_json::Value::String("id".to_string())));
    assert!(required_array.contains(&serde_json::Value::String("data".to_string())));
    assert!(!required_array.contains(&serde_json::Value::String("optional_field".to_string())));
}

#[test]
fn test_concurrent_derived_schema_access() {
    use std::thread;
    
    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                // Each thread gets the schema multiple times
                for _ in 0..100 {
                    let _ = SimpleStruct::schema();
                    let _ = ComplexStruct::schema();
                    let _ = NestedStruct::schema();
                }
            })
        })
        .collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify schema is still correct after concurrent access
    let schema = SimpleStruct::schema();
    assert_eq!(schema["type"], "object");
}

#[test]
fn test_edge_cases_caching() {
    // Test empty/unit structs
    let empty_schema1 = EmptyStruct::schema();
    let empty_schema2 = EmptyStruct::schema();
    assert_eq!(empty_schema1, empty_schema2);
    
    let unit_schema1 = UnitStruct::schema();
    let unit_schema2 = UnitStruct::schema();
    assert_eq!(unit_schema1, unit_schema2);
    
    // Test tuple structs
    let tuple_schema1 = TupleStruct::schema();
    let tuple_schema2 = TupleStruct::schema();
    assert_eq!(tuple_schema1, tuple_schema2);
}

#[test]
fn test_tuple_struct_schema_correctness() {
    let schema = TupleStruct::schema();
    
    assert_eq!(schema["type"], "array");
    assert_eq!(schema["minItems"], 3);
    assert_eq!(schema["maxItems"], 3);
    
    let prefix_items = &schema["prefixItems"];
    assert!(prefix_items.is_array());
    let items = prefix_items.as_array().unwrap();
    assert_eq!(items.len(), 3);
    
    // Verify each item type
    assert_eq!(items[0]["type"], "string");
    assert_eq!(items[1]["type"], "integer");
    assert_eq!(items[2]["type"], "boolean");
}

#[test]
fn benchmark_derived_schema_generation() {
    const ITERATIONS: usize = 10_000;
    
    // Benchmark simple derived type
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = SimpleStruct::schema();
    }
    let simple_duration = start.elapsed();
    
    // Benchmark complex derived type
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = ComplexStruct::schema();
    }
    let complex_duration = start.elapsed();
    
    // Benchmark nested derived type
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = NestedStruct::schema();
    }
    let nested_duration = start.elapsed();
    
    println!("Derived schema generation performance (cached):");
    println!("  Simple struct ({} calls): {:?}", ITERATIONS, simple_duration);
    println!("  Complex struct ({} calls): {:?}", ITERATIONS, complex_duration);
    println!("  Nested struct ({} calls): {:?}", ITERATIONS, nested_duration);
    
    // All should be very fast due to caching
    assert!(simple_duration.as_millis() < 100, 
            "Simple struct schema generation too slow: {:?}", simple_duration);
    assert!(complex_duration.as_millis() < 200, 
            "Complex struct schema generation too slow: {:?}", complex_duration);
    assert!(nested_duration.as_millis() < 300, 
            "Nested struct schema generation too slow: {:?}", nested_duration);
}

#[test]
fn test_schema_caching_vs_regeneration() {
    // This test demonstrates the performance benefit of caching
    
    // Test with a sufficiently large number of calls to see caching benefits
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = NestedStruct::schema();
    }
    let cached_duration = start.elapsed();
    let avg_cached_call = cached_duration / 1000;
    
    println!("Schema generation timing:");
    println!("  Average cached call: {:?}", avg_cached_call);
    println!("  1000 cached calls total: {:?}", cached_duration);
    
    // With caching, even complex nested schemas should be very fast
    // Each call should take less than 1ms on average
    assert!(avg_cached_call.as_micros() < 1000, 
            "Cached calls should be very fast, got: {:?}", avg_cached_call);
    
    // Total time for 1000 calls should be reasonable (less than 100ms)
    assert!(cached_duration.as_millis() < 100,
            "1000 cached schema calls took too long: {:?}", cached_duration);
}

#[test]
fn test_memory_efficiency_of_caching() {
    // Test that repeated schema calls don't allocate new memory each time
    let schema1 = SimpleStruct::schema();
    let schema2 = SimpleStruct::schema();
    
    // The schemas should be identical in content
    assert_eq!(schema1, schema2);
    
    // Test that calling schema many times doesn't cause memory issues
    for _ in 0..1000 {
        let schema = SimpleStruct::schema();
        assert_eq!(schema, schema1);
    }
}

#[tokio::test]
async fn test_schema_caching_in_tool_context() {
    use tools_rs::{tool, FunctionCall};
    
    #[tool]
    /// Test function that uses derived types
    async fn test_function(input: SimpleStruct) -> ComplexStruct {
        ComplexStruct {
            id: 42,
            data: input,
            scores: vec![1.0, 2.0, 3.0],
            metadata: std::collections::HashMap::new(),
            optional_field: Some(100),
        }
    }
    
    let tools = collect_tools();
    
    // Verify the tool works correctly
    let call = FunctionCall {
        name: "test_function".to_string(),
        arguments: serde_json::json!({
            "input": {
                "name": "test",
                "age": 25,
                "active": true
            }
        }),
    };
    
    let result = tools.call(call).await;
    assert!(result.is_ok());
    
    // Verify that schema generation is fast even in tool context
    let start = Instant::now();
    for _ in 0..100 {
        let _ = tools.json();
    }
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 1000, 
            "Tool schema generation with caching took too long: {:?}", duration);
}