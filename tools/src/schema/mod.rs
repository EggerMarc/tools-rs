use serde::Serialize;
use serde_json::Value;

/// `FunctionDecl` – metadata emitted by the runtime for each registered tool.
/// Generates OpenAI function calling format directly.
#[derive(Debug, Clone, Serialize)]
pub struct FunctionDecl<'a> {
    #[serde(rename = "type")]
    pub function_type: &'static str,
    pub function: FunctionDetails<'a>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDetails<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub parameters: Value,
}

impl<'a> FunctionDecl<'a> {
    pub fn new(name: &'a str, description: &'a str, parameters: Value) -> Self {
        Self {
            function_type: "function",
            function: FunctionDetails {
                name,
                description,
                parameters,
            },
        }
    }
}
