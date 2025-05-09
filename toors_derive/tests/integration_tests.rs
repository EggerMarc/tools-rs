extern crate toors_derive;
use toors_derive::Tool;
use toors::Tool;

#[derive(Tool, Default)]
/// Test structure for tool functionality
struct Test {
    /// Some documentation for field a
    a: i32,
    /// Documentation for field b
    b: String,
}

#[test]
fn access_tools() {
    let test = Test::default();
    let metadata = test.signature();
    
    // Verify the metadata contains the right information
    assert_eq!(metadata.name, "Test");
    assert!(metadata.description.contains("Test structure for tool functionality"));
    assert!(metadata.signature.contains("a: i32"));
    assert!(metadata.signature.contains("b: String"));
}