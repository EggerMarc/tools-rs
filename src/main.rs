use toors::ToolCollection;
use toors_derive::{tools};

#[derive(Default)]
/// This structs contains multiplication helpers
struct Multiply;

#[tools]
impl Multiply {
    /// Multiply two values
    fn mul_two(a: i32, b: i32) -> i32 {
        a * b
    }
    

    /// Multiply three values, wow
    fn mul_three(a: i32, b: i32, c: i32) -> i32 {
        a * b * c
    }
}

#[derive(Default)]
struct Scream;

#[tools]
impl Scream {
    fn call(input: &str) -> String {
        input.to_uppercase()
    }
}

#[derive(Default)]
/// Some tool
struct DBConn {
    /// Url of the database
    url: String,

    /// Port of the database
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

#[tools]
/// Connects to a DB
impl DBConn {
    /// Call db
    #[doc = "Some old styled documentation"]
    fn call(&self) -> &Self {
        self
    }
}

fn main() { 
    let result = Multiply::mul_two(2, 3);
    println!("Math result: {}", result);

    for (name, tool) in Multiply::tools().iter() {
        println!(
            "Tool name: {}\n\tSignature: {}\n\tDescription: {}",
            name, tool.signature, tool.description
        );
        println!("-------------------");
    }
}
