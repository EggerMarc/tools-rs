use serde_json::{Number, Value};
use toors::Tool;
use toors_derive::{tools, Tool};

struct Args {
    a: i32,
    b: i32
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    // Create an instance of MyTool.
    let tool = MyTool {
        arg: "Hello".to_string(),
        generic: 1,
    };

    let args = Args {
        a: 1,
        b: 2
    };

    add(**args);

    // Instance methods.
    println!("==============");
    println!("Description:\n{}", tool.description());
    println!("Signature metadata:\n{}", tool.signature());
    println!("\ntools:\n{}", tool.tools().get("some_func").unwrap()); // TODO work on this
} 
