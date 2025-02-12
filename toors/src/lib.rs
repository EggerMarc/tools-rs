use erased_serde::Serialize as ErasedSerialize;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Default)]
pub struct ToolCollection {
    funcs: HashMap<&'static str, Arc<ToolFunc>>,
    async_funcs: HashMap<&'static str, Arc<AsyncToolFunc>>,
    descriptions: HashMap<&'static str, &'static str>,
    signatures: HashMap<&'static str, (TypeId, TypeId)>, // (Input TypeId, Output TypeId)
}

type ToolFunc =
    dyn Fn(Box<dyn Any + Send + Sync>) -> Box<dyn ErasedSerialize + Send + Sync> + Send + Sync;
type AsyncToolFunc = dyn Fn(
        Box<dyn Any + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = Box<dyn ErasedSerialize + Send + Sync>> + Send>>
    + Send
    + Sync;

impl ToolCollection {
    pub fn new() -> Self {
        Self {
            funcs: HashMap::new(),
            async_funcs: HashMap::new(),
            descriptions: HashMap::new(),
            signatures: HashMap::new(),
        }
    }

    pub fn register<I, O, F>(
        &mut self,
        name: &'static str,
        description: &'static str,
        func: F,
    ) -> &Self
    where
        I: 'static + Serialize + DeserializeOwned + Send + Sync,
        O: 'static + Serialize + Send + Sync,
        F: Fn(I) -> O + Send + Sync + 'static,
    {
        self.descriptions.insert(name, description);
        self.signatures
            .insert(name, (TypeId::of::<I>(), TypeId::of::<O>()));

        self.funcs.insert(
            name,
            Arc::new(move |input: Box<dyn Any + Send + Sync>| {
                let args = input.downcast_ref::<Args>().expect("Invalid argument type");
                let input_cloned = args.0.clone();

                let typed_input: I = if input_cloned.is_empty() {
                    serde_json::from_value(Value::Null)
                } else if input_cloned.len() == 1 {
                    serde_json::from_value(Value::Array(input_cloned.clone())).or_else(|_| {
                        serde_json::from_value(
                            input_cloned.into_iter().next().expect("Expected a value"),
                        )
                    })
                } else {
                    serde_json::from_value(Value::Array(input_cloned))
                }
                .expect("Failed to deserialize input into the expected type");

                let output = func(typed_input);
                Box::new(output) as Box<dyn ErasedSerialize + Send + Sync>
            }),
        );
        self
    }
    pub fn register_async<I, O, F, Fut>(
        &mut self,
        name: &'static str,
        description: &'static str,
        func: F,
    ) -> &Self
    where
        I: 'static + Serialize + DeserializeOwned + Send + Sync,
        O: 'static + Serialize + Send + Sync,
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = O> + Send + 'static,
    {
        self.descriptions.insert(name, description);
        self.signatures
            .insert(name, (TypeId::of::<I>(), TypeId::of::<O>()));

        self.async_funcs.insert(
            name,
            Arc::new(move |input: Box<dyn Any + Send + Sync>| {
                let args = input.downcast_ref::<Args>().expect("Invalid argument type");
                let input_cloned = args.0.clone();

                let typed_input: I = if input_cloned.is_empty() {
                    serde_json::from_value(Value::Null)
                } else if input_cloned.len() == 1 {
                    serde_json::from_value(Value::Array(input_cloned.clone())).or_else(|_| {
                        serde_json::from_value(
                            input_cloned.into_iter().next().expect("Expected a value"),
                        )
                    })
                } else {
                    serde_json::from_value(Value::Array(input_cloned))
                }
                .expect("Failed to deserialize input into the expected type");

                let future = func(typed_input);
                let boxed_future: Pin<
                    Box<dyn Future<Output = Box<dyn ErasedSerialize + Send + Sync>> + Send>,
                > = Box::pin(async move {
                    let output = future.await;
                    Box::new(output) as Box<dyn ErasedSerialize + Send + Sync>
                });
                boxed_future
            }),
        );
        self
    }
    pub async fn call(
        &self,
        name: &str,
        input_str: &str,
    ) -> Result<Box<dyn ErasedSerialize + Send + Sync>, String> {
        if let Some(func) = self.funcs.get(name) {
            // ... (Synchronous call - no changes)
            let (_, args) =
                parse(input_str).map_err(|e| format!("Failed to parse input: {}", e))?;
            let result = func(Box::new(args));
            Ok(result)
        } else if let Some(async_func) = self.async_funcs.get(name) {
            let (_, args) =
                parse(input_str).map_err(|e| format!("Failed to parse input: {}", e))?;
            let future = async_func(Box::new(args));
            let result = future.await; // Await the future here
            Ok(result)
        } else {
            Err(format!("Function '{}' not found", name))
        }
    }
    pub async fn call_to_json(&self, name: &str, input_str: &str) -> Result<Value, String> {
        let result = self.call(name, input_str).await?;

        serde_json::to_value(&*result).map_err(|e| format!("Serialization error: {}", e))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Args(Vec<Value>);

fn parse(cmd: &str) -> Result<(&str, Args), Box<dyn std::error::Error>> {
    let re = Regex::new(r"^(.+?)\((.*)\)$")?;
    let captures = re.captures(cmd).ok_or("Invalid command format")?;

    let name = captures.get(1).unwrap().as_str();
    let args_str = captures.get(2).unwrap().as_str();
    let args_json = format!("[{}]", args_str);
    let args: Args = serde_json::from_str(&args_json)?;

    Ok((name, args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn add<T: std::ops::Add<Output = T> + Copy>(a: T, b: T) -> T {
        a + b
    }

    fn concat<T: std::fmt::Display>(a: T, b: T) -> String {
        format!("{}{}", a, b)
    }

    fn noop() {}

    #[derive(PartialEq, Serialize, Deserialize)]
    struct SomeArgs {
        a: i32,
        b: i32,
    }
    fn using_args(_args: SomeArgs) {}

    // Test the registration and calling of functions via ToolCollection.
    #[tokio::test]
    async fn test_collection() {
        let mut collection = ToolCollection::default();
        collection.register("add", "Adds two values", |args: (i32, i32)| {
            add(args.0, args.1)
        });
        collection.register(
            "concat",
            "Concatenates two strings",
            |args: (String, String)| concat(args.0, args.1),
        );
        collection.register("noop", "Does nothing", |_args: ()| noop());
        collection.register("complex_args", "Uses complex args", |args: SomeArgs| {
            using_args(args)
        });

        let add_result = collection
            .call_to_json("add", "add(1,2)")
            .await
            .expect("Failed to call add");
        let concat_result = collection
            .call_to_json("concat", "concat(\"hello\",\"world\")")
            .await
            .expect("Failed to call concat");
        let noop_result = collection
            .call_to_json("noop", "noop()").await
            .expect("Failed to call noop");
        let complex_args_result = collection
            .call_to_json("complex_args", "complex_args({\"a\":1,\"b\":2})").await
            .expect("Failed to call complex_args");

        assert_eq!(add_result, json!(3));
        assert_eq!(concat_result, json!("helloworld"));
        assert_eq!(noop_result, json!(null));
        assert_eq!(complex_args_result, json!(null)); // using_args returns () => null
    }

    // Test the parser with various correct inputs.
    #[test]
    fn test_parser_success() {
        let test_cases = vec![
            (
                "function(1,2)",
                ("function", Args(vec![json!(1), json!(2)])),
            ),
            (
                "function(\"hello\", \"world\")",
                ("function", Args(vec![json!("hello"), json!("world")])),
            ),
            (
                "function([1,2,3], {\"a\":1})",
                ("function", Args(vec![json!([1, 2, 3]), json!({"a":1})])),
            ),
            ("function()", ("function", Args(vec![]))),
            ("add(1,2)", ("add", Args(vec![json!(1), json!(2)]))),
            (
                "concat(\"hello\", \"world\")",
                ("concat", Args(vec![json!("hello"), json!("world")])),
            ),
        ];

        for (cmd, expected) in test_cases {
            let (name, args): (&str, Args) =
                parse(cmd).unwrap_or_else(|_| panic!("Failed to unwrap on {:?}", cmd));
            assert_eq!(name, expected.0);
            assert_eq!(args, expected.1);
        }
    }

    // Test parsing failures with malformed commands.
    #[test]
    fn test_parser_failure() {
        let invalid_inputs = vec!["missing_parenthesis", "func(1,2", "func1,2)", "func(,)"];
        for input in invalid_inputs {
            assert!(parse(input).is_err(), "Parser should fail on: {}", input);
        }
    }

    // Test calling a function that returns a boolean.
    #[tokio::test]
    async fn test_boolean_function() {
        let mut collection = ToolCollection::default();
        collection.register("is_even", "Checks if a number is even", |args: (i32,)| {
            args.0 % 2 == 0
        });
        let result_even = collection
            .call_to_json("is_even", "is_even(4)").await
            .expect("Failed to call is_even with even number");
        let result_odd = collection
            .call_to_json("is_even", "is_even(3)").await
            .expect("Failed to call is_even with odd number");
        assert_eq!(result_even, json!(true));
        assert_eq!(result_odd, json!(false));
    }

    // Test calling a function that returns a complex (struct) value.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[tokio::test]
    async fn test_complex_return() {
        let mut collection = ToolCollection::default();
        collection.register("create_point", "Creates a point", |args: (i32, i32)| {
            Point {
                x: args.0,
                y: args.1,
            }
        });
        let result = collection
            .call_to_json("create_point", "create_point(10,20)").await
            .expect("Failed to call create_point");
        assert_eq!(result, json!({"x": 10, "y": 20}));
    }

    // Test a function that takes a tuple with a single element.
    #[tokio::test]
    async fn test_single_argument_tuple() {
        let mut collection = ToolCollection::default();
        collection.register("square", "Squares a number", |args: (i32,)| {
            let x = args.0;
            x * x
        });
        let result = collection
            .call_to_json("square", "square(5)").await
            .expect("Failed to call square");
        assert_eq!(result, json!(25));
    }

    // Test calling a function with a non-existent name.
    #[tokio::test]
    async fn test_invalid_function_name() {
        let mut collection = ToolCollection::default();
        collection.register("test", "Dummy function", |_args: ()| {});
        let result = collection.call_to_json("non_existent", "non_existent()").await;
        assert!(
            result.is_err(),
            "Calling a non-existent function should return an error"
        );
    }

    // Test a function that takes a vector as an argument.
    #[tokio::test]
    async fn test_vector_argument() {
        let mut collection = ToolCollection::default();
        collection.register("sum", "Sums an array of numbers", |args: (Vec<i32>,)| {
            args.0.iter().sum::<i32>()
        });
        let result = collection
            .call_to_json("sum", "sum([1,2,3,4])").await
            .expect("Failed to call sum");
        assert_eq!(result, json!(10));
    }

    // Test a function that accepts a struct as input.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Config {
        host: String,
        port: u16,
    }

    #[tokio::test]
    async fn test_struct_argument() {
        let mut collection = ToolCollection::default();
        collection.register("config_info", "Processes a config", |config: Config| {
            format!("{}:{}", config.host, config.port)
        });
        let result = collection
            .call_to_json(
                "config_info",
                "config_info({\"host\": \"localhost\", \"port\": 8080})",
            ).await
            .expect("Failed to call config_info");
        assert_eq!(result, json!("localhost:8080"));
    }

    // Test error handling when deserialization of the input fails.
    #[tokio::test]
    async fn test_deserialization_error() {
        let mut collection = ToolCollection::default();
        collection.register("subtract", "Subtracts two numbers", |args: (i32, i32)| {
            args.0 - args.1
        });
        // Provide strings instead of numbers to trigger a deserialization error.
        let result = collection.call_to_json("subtract", "subtract(\"a\", \"b\")").await;
        assert!(
            result.is_err(),
            "Deserialization should fail for invalid input types"
        );
    }

    // Test a function that takes a string and returns a greeting.
    #[tokio::test]
    async fn test_string_function() {
        let mut collection = ToolCollection::default();
        collection.register("greet", "Greets someone", |args: (String,)| {
            format!("Hello, {}!", args.0)
        });
        let result = collection
            .call_to_json("greet", "greet(\"Alice\")").await
            .expect("Failed to call greet");
        assert_eq!(result, json!("Hello, Alice!"));
    }
    
    async fn async_foo() {}

    #[tokio::test]
    async fn test_async_function() {
        let mut collection = ToolCollection::default();
        collection.register_async("async_foo", "This function does nil", |_args: ()| async {
            async_foo().await
        });

        let result = collection.call_to_json("async_foo", "async_foo()").await.expect("Failed to call async_foo");
        assert_eq!(result, json!(()));
    }
}



