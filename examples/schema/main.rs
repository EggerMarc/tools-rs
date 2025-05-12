use serde::{Deserialize, Serialize};
use serde_json::json;
use toors_core::{FunctionCall, collect_tools, function_declarations, tool};

#[derive(Serialize, Deserialize, Debug)]
struct Person {
    name: String,
    age: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    hobbies: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchRequest {
    query: String,
    max_results: Option<u32>,
    filters: SearchFilters,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchFilters {
    categories: Vec<String>,
    min_rating: Option<f32>,
    date_range: Option<(String, String)>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchResult {
    title: String,
    url: String,
    description: String,
    rating: f32,
}

#[tool]
/// Create a new person with the given properties
async fn create_person(person: Person) -> Person {
    // In a real implementation, this might save to a database
    println!("Created person: {:?}", person);
    person
}

#[tool]
/// Search for resources matching the criteria
async fn search(request: SearchRequest) -> Vec<SearchResult> {
    println!("Searching for: {:?}", request);

    // In a real implementation, this would perform a search
    // Here we just return sample data
    vec![
        SearchResult {
            title: format!("Result for '{}'", request.query),
            url: "https://example.com/result1".to_string(),
            description: "This is a sample search result".to_string(),
            rating: 4.5,
        },
        SearchResult {
            title: format!("Another result for '{}'", request.query),
            url: "https://example.com/result2".to_string(),
            description: "This is another sample result".to_string(),
            rating: 3.8,
        },
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect all tools registered with the #[tool] macro
    let tools = collect_tools();

    // Generate function declarations with full JSON Schema (when schema feature is enabled)
    let declarations = function_declarations();

    print!("{declarations}");

    println!("Function Declarations JSON (with schema):");
    println!("{}", serde_json::to_string_pretty(&declarations)?);

    // Example of what a request to an OpenAI API might look like
    let openai_request = json!({
        "model": "gpt-4-turbo",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant with access to function calling."
            },
            {
                "role": "user",
                "content": "Create a person named Alice who is 30 years old and likes reading and hiking."
            }
        ],
        "tool_choice": "auto",
        "tools": [
            {
                "type": "function",
                "function": declarations.as_array().unwrap()[0]
            },
            {
                "type": "function",
                "function": declarations.as_array().unwrap()[1]
            }
        ]
    });

    println!("\nExample OpenAI API Request:");
    println!("{}", serde_json::to_string_pretty(&openai_request)?);

    // Example of calling a tool
    println!("\nExecuting 'create_person' tool:");
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        hobbies: vec!["reading".to_string(), "hiking".to_string()],
    };

    let result = tools
        .call(FunctionCall {
            name: "create_person".into(),
            arguments: json!(person),
        })
        .await?;

    println!("Created person result: {}", result);

    // Example of calling search tool
    println!("\nExecuting 'search' tool:");
    let search_request = SearchRequest {
        query: "rust programming".to_string(),
        max_results: Some(5),
        filters: SearchFilters {
            categories: vec!["programming".to_string(), "technology".to_string()],
            min_rating: Some(4.0),
            date_range: Some(("2023-01-01".to_string(), "2023-12-31".to_string())),
        },
    };

    let result = tools
        .call(FunctionCall {
            name: "search".into(),
            arguments: json!(search_request),
        })
        .await?;

    println!("Search results: {}", result);

    Ok(())
}
