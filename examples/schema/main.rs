//! main.rs – End-to-end demo for **Toors**
//!
//! *With JSON-Schema*  
//! ```bash
//! cargo run --example schema --features schema
//! ```
//!
//! *Without JSON-Schema* (smaller binary; schemas become `null`)  
//! ```bash
//! cargo run --example schema
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::error::Error;
use toors_core::{FunctionCall, collect_tools, function_declarations, tool};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

// ─────────────────────────────────────────────────────────────────────────────
// Domain types
// ─────────────────────────────────────────────────────────────────────────────

#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
struct Person {
    name: String,
    age: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hobbies: Vec<String>,
}

#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
struct SearchRequest {
    query: String,
    max_results: Option<u32>,
    filters: SearchFilters,
}

#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
struct SearchFilters {
    categories: Vec<String>,
    min_rating: Option<f32>,
    date_range: Option<DateRange>,
}

#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
struct DateRange {
    start: String,
    end: String,
}

#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug)]
struct SearchResult {
    title: String,
    url: String,
    description: String,
    rating: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tools exposed to the LLM
// ─────────────────────────────────────────────────────────────────────────────

#[tool]
/// Create a new person and return it.
async fn create_person(person: Person) -> Person {
    println!("Created person: {person:?}");
    person
}

#[tool]
/// Search for resources matching the criteria.
async fn search(request: SearchRequest) -> Vec<SearchResult> {
    println!("Searching for: {request:?}");
    vec![
        SearchResult {
            title: format!("Result for '{}'", request.query),
            url: "https://example.com/result1".into(),
            description: "Sample search result".into(),
            rating: 4.5,
        },
        SearchResult {
            title: format!("Another result for '{}'", request.query),
            url: "https://example.com/result2".into(),
            description: "Another sample search result".into(),
            rating: 3.8,
        },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Driver
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Compile-time inventory → runtime registry
    let tools = collect_tools();

    // 2. JSON function declarations (may fail → bubble up)
    let declarations: JsonValue = function_declarations()?; // ◀ was plain Value

    println!("=== Function Declarations ===");
    println!("{}", serde_json::to_string_pretty(&declarations)?);

    // 3. Skeleton of a Chat-completion request
    let decl_array = declarations
        .as_array()
        .ok_or("function_declarations() did not return a JSON array")?; // ◀ no unwrap

    let tools_field: Vec<JsonValue> = decl_array
        .iter()
        .map(|f| json!({ "type": "function", "function": f }))
        .collect();

    let chat_request = json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "system",
              "content": "You are a helpful assistant with tool access." },
            { "role": "user",
              "content": "Create a person named Alice who is 30 years old and likes reading and hiking." }
        ],
        "tool_choice": "auto",
        "tools": tools_field
    });

    println!("\n=== Example Chat Request ===");
    println!("{}", serde_json::to_string_pretty(&chat_request)?);

    // 4. Direct invocation of `create_person`
    let alice = Person {
        name: "Alice".into(),
        age: 30,
        hobbies: vec!["reading".into(), "hiking".into()],
    };

    let created = tools
        .call(FunctionCall {
            name: "create_person".into(),
            arguments: json!({ "person": alice }),
        })
        .await?; // ◀ ToolError → Box<dyn Error>

    println!("\nCreated person (runtime): {created}");

    // 5. Direct invocation of `search`
    let req = SearchRequest {
        query: "rust programming".into(),
        max_results: Some(5),
        filters: SearchFilters {
            categories: vec!["programming".into(), "technology".into()],
            min_rating: Some(4.0),
            date_range: Some(DateRange {
                start: "2024-01-01".into(),
                end: "2024-12-31".into(),
            }),
        },
    };

    let results = tools
        .call(FunctionCall {
            name: "search".into(),
            arguments: json!({ "request": req }),
        })
        .await?; // ◀ same

    println!("\nSearch results (runtime): {results}");

    Ok(())
}
