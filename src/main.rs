use toors::{Tool, ToolCollection};
use toors_derive::{tools, Tool};

#[derive(Default, Tool)]
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

#[derive(Default, Tool)]
struct Scream;

#[tools]
impl Scream {
    fn call(input: &str) -> String {
        input.to_uppercase()
    }
}

#[derive(Default, Tool)]
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
    let mut collection = ToolCollection::new();
    collection.add(Multiply::default());
    collection.add(Scream::default());
    collection.add(DBConn::new());



    for (name, tool) in DBConn::tools().iter() {
        println!(
            "tool name: {}\n\tSignature: {}\n\tDescription: {}",
            name, tool.signature, tool.description
        );
        println!("-------------------");
    }
    
    for (tool) in collection.list_tools().iter() {
        println!("Tool Signature:\n{}\n", {&tool.signature}); 
        println!("Tool Description:\n{}\n", {&tool.description});
        println!("--------------------");
    }
    println!(
        "Tool struct: DBConn\n\tDescription: {}\n\tArgs: {}",
        DBConn::description(),
        DBConn::signature()
    )
}
