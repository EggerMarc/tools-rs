//! Demonstration of how the **tools-rs** `#[tool]` macro turns ordinary
//! `async fn`s into *call-by-name* “tools” that an LLM (or your own code)
//! can invoke through a type-safe registry.
//!
//! Steps performed in `main`:
//! 1.  **Collect** every `#[tool]` function that was compiled into the binary (link-time inventory → runtime registry).
//! 2.  **Generate** JSON-Schema declarations so the LLM endpoint knows each function’s name, parameters, return type and natural-language description.
//! 3.  **Build** an example chat-completion request that exposes those tools to the model.
//! 4.  **Invoke** the same tools directly from Rust to prove they work at runtime too.

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::error::Error;

use tools_rs::{FunctionCall, collect_tools, function_declarations, tool};

use tool_schema::ToolSchema;

// ────────────────────────────────────────────────────────────────────────────
// Domain models
// ────────────────────────────────────────────────────────────────────────────

/// A simple person with an optional list of hobbies.
///
/// When the `schema` Cargo feature is enabled this struct will also emit
/// JSON-Schema via `tool_schema` so the LLM (or any other consumer) can
/// validate requests at the **payload** layer.
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct Person {
    /// The person’s full name.
    name: String,

    /// Age in years.
    age: u32,

    /// Optional hobbies.
    ///
    /// `serde(default)` deserialises this as an empty vector when the
    /// field is missing; `skip_serializing_if = "Vec::is_empty"` omits
    /// it from the outbound JSON when it *is* empty, keeping payloads
    /// compact.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hobbies: Vec<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// Search API models
// ────────────────────────────────────────────────────────────────────────────

/// High-level search request issued by the user / LLM.
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct SearchRequest {
    /// Free-text query.
    query: String,

    /// Soft cap on number of results.
    max_results: Option<u32>,

    /// Structured filters applied server-side.
    filters: SearchFilters,
}

/// Optional filter block that can be attached to a [`SearchRequest`].
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct SearchFilters {
    /// List of categories to constrain the search to.
    categories: Vec<String>,

    /// Minimum rating threshold.
    min_rating: Option<f32>,

    /// Inclusive date span.
    date_range: Option<DateRange>,
}

/// Inclusive date range for search filtering.
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct DateRange {
    start: String,
    end: String,
}

/// Single search hit returned to the caller.
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct SearchResult {
    title: String,
    url: String,
    description: String,
    rating: f32,
}

// ────────────────────────────────────────────────────────────────────────────
// Tools (these are the functions an LLM can call)
// ────────────────────────────────────────────────────────────────────────────

/// Create a new [`Person`] and echo it back.
///
/// The `#[tool]` macro:
/// * Registers the function in an *inventory* so it can be discovered by
///   name at runtime.
/// * Derives JSON-Schema for the input/return types.
/// * Ensures the function is `async` (wrapping it if necessary) so it can
///   run on any executor.
#[tool]
/// Create a new [`Person`] and echo it back.
async fn create_person(person: Person) -> Person {
    println!("Created person: {person:?}");
    person
}

/// Run a content search and return mock results.
///
/// In a real system this would call an index, database, or third-party
/// API.  Here we stub it out with deterministic dummy data so the focus
/// stays on the *tool plumbing*.
#[tool]
/// Run a content search and return mock results.
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

// ────────────────────────────────────────────────────────────────────────────
// Program entry-point
// ────────────────────────────────────────────────────────────────────────────

/// Spin up a Tokio runtime and walk through the full **tool lifecycle**.
///
/// *Discovery → Declaration → Example Chat Payload → Direct Invocation*
/// shows both the *static* (schema) and *dynamic* (runtime) sides in one
/// go.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Compile-time inventory → runtime registry
    let tools = collect_tools();

    // 2. Emit JSON-Schema function declarations
    let declarations: JsonValue = function_declarations()?; // may fail → bubble up

    println!("=== Function Declarations ===");
    println!("{}", serde_json::to_string_pretty(&declarations)?);

    // 3. Skeleton of a Chat-completion request --------------------------------
    let decl_array = declarations
        .as_array()
        .ok_or("function_declarations() did not return a JSON array")?;

    // The declarations already come in the correct OpenAI format
    let tools_field: Vec<JsonValue> = decl_array
        .iter()
        .cloned()
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

    // 4. Direct invocation of `create_person` ---------------------------------
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
        .await?; // ToolError → Box<dyn Error>

    println!("\nCreated person (runtime): {created}");

    // 5. Direct invocation of `search` ----------------------------------------
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
        .await?;

    println!("\nSearch results (runtime): {results}");

    Ok(())
}
