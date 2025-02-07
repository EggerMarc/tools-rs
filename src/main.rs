use toors::Tool;
use toors_derive::Tool;

/// This tool is an example.
#[derive(Tool)]
struct MyTool<T: Clone> {
    /// Argument which does some string manipulation.
    arg: String,

    /// It also works with generics!
    generic: T,
}

fn main() {
    // Create an instance of MyTool.
    let tool = MyTool {
        arg: "Hello".to_string(),
        generic: 1,
    };

    // Instance methods.
    println!("==============");
    println!("Description:\n{}", tool.description());
    println!("Signature metadata:\n{}", tool.signature());
}
