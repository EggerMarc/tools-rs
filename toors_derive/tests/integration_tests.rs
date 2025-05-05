extern crate toors_derive;
use toors_derive::{leg_add_field, add_field, ToolProvider};

#[derive(ToolProvider, Default)]
struct Test {
    /// Some documentation
    a: i32,
}

#[leg_add_field]
#[derive(Default)]
struct AStruct;

#[test]
fn access_tools() {
    let test = Test::default();
    let astruct = AStruct {
        a: "Some value".to_string(),
    };
}
