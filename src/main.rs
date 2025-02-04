use toors::ToolCollection;
use toors_derive::tool;

#[derive(Default)]
struct Multiply;

#[tool]
/// Multiply two values 
impl Multiply {
    fn call(a: i32, b: i32) -> i32 {
        a + b
    }
}

#[derive(Default)]
struct Scream;

#[tool]
/// AAAAAA
/// I'm so scared
impl Scream {
    fn call(input: &str) -> String {
        input.to_uppercase()
    }
}

struct DBConn {
    url: String,
    port: u32,
}

impl DBConn {
    fn new() -> Self {
        Self {
            url: "https://localhost".to_string(),
            port: 3000,
        }
    }
}

#[tool]
/// Connects to a DB
impl DBConn {
    fn call(&self) -> &Self {
        self
    }
}

fn main() {
    let mut collection = ToolCollection::new();
    collection.add(Multiply::default());
    collection.add(Scream::default());
    collection.add(DBConn::new());

    // LLM context: Show available tools
    for tool in collection.list_tools() {
        println!("Tool Signature: {}", tool.signature);
        println!("Description: {}", tool.description);
        println!("-------------------");
    }

    // When LLM selects a tool
    if let Some(math) = collection.get_tool::<Multiply>() {
        let result = Multiply::call(2, 3);
        println!("Math result: {}", result);
    }
}
