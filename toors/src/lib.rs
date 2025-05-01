//! Async‑only ToolCollection — a minimal runtime for LLM function‑calling
//! ----------------------------------------------------------------------
//! This refactoring removes all synchronous paths and enforces that every
//! registered tool is **async‑first**.  You register an async function and
//! call it by text (e.g. `sum(1,2,3)`), receiving the result as `serde_json::Value`.
//!
//! #### Example
//!
//! ```rust,no_run
//! # use tool_collection_async::*;
//! # #[tokio::main] async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut tools = ToolCollection::new();
//!
//! tools.register(
//!     "sum",
//!     "Returns the sum of an array of numbers",
//!     |values: Vec<i32>| async move { values.iter().sum::<i32>() },
//! );
//!
//! let out = tools.call_to_json("sum", "sum(1,2,3)").await?;
//! assert_eq!(out, serde_json::json!(6));
//! # Ok(()) }
//! ```
//! ----------------------------------------------------------------------

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

/// Stores async tools keyed by name and handles dispatch/serialization.
#[derive(Default)]
pub struct ToolCollection {
    funcs: HashMap<&'static str, Arc<AsyncToolFunc>>, // name → async closure
    descriptions: HashMap<&'static str, &'static str>, // name → human description
    signatures: HashMap<&'static str, (TypeId, TypeId)>, // name → (Input, Output) type IDs
}

/// Dyn‑erased async function signature used internally.
type AsyncToolFunc = dyn Fn(
        Box<dyn Any + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = Box<dyn ErasedSerialize + Send + Sync>> + Send>>
    + Send
    + Sync;

impl ToolCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an **async** Rust function so it can be called from text.
    ///
    /// * `name` – identifier used in the textual command (e.g. `"sum"`).
    /// * `description` – natural‑language explanation (useful for LLMs).
    /// * `func` – async function/closure from `I` → `O` where both I and O are Serde‑serializable.
    pub fn register<I, O, F, Fut>(
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
        // Persist metadata for later export (OpenAI/Gemini function‑calling arrays, etc.).
        self.descriptions.insert(name, description);
        self.signatures
            .insert(name, (TypeId::of::<I>(), TypeId::of::<O>()));

        // Wrap the user function into a dyn‑erased async closure that accepts `Any`.
        self.funcs.insert(
            name,
            Arc::new(move |input: Box<dyn Any + Send + Sync>| {
                let args = input.downcast_ref::<Args>().expect("Invalid argument type");
                let input_cloned = args.0.clone();

                // Try to coerce the JSON vector into the expected `I` type.
                let typed_input: I = if input_cloned.is_empty() {
                    serde_json::from_value(Value::Null)
                } else if input_cloned.len() == 1 {
                    // Single value may be `[x]` or raw `x`.
                    serde_json::from_value(Value::Array(input_cloned.clone())).or_else(|_| {
                        serde_json::from_value(
                            input_cloned.into_iter().next().expect("Expected a value"),
                        )
                    })
                } else {
                    serde_json::from_value(Value::Array(input_cloned))
                }
                .expect("Failed to deserialize input into the expected type");

                // Run the user function and box its future / result.
                let fut = func(typed_input);
                Box::pin(async move {
                    let output = fut.await;
                    Box::new(output) as Box<dyn ErasedSerialize + Send + Sync>
                })
            }),
        );
        self
    }

    /// Executes a registered tool by textual command and returns its raw `Serialize`‑able output.
    pub async fn call(
        &self,
        name: &str,
        input_str: &str,
    ) -> Result<Box<dyn ErasedSerialize + Send + Sync>, String> {
        let async_func = self
            .funcs
            .get(name)
            .ok_or_else(|| format!("Function '{name}' not found"))?;

        let (_, args) = parse(input_str).map_err(|e| format!("Failed to parse input: {e}"))?;
        let result = async_func(Box::new(args)).await;
        Ok(result)
    }

    /// Convenience wrapper that serializes the result to `serde_json::Value`.
    pub async fn call_to_json(&self, name: &str, input_str: &str) -> Result<Value, String> {
        let result = self.call(name, input_str).await?;
        serde_json::to_value(&*result).map_err(|e| format!("Serialization error: {e}"))
    }

    // ------------------------------------------------------------------
    // Optional: public getters for metadata (useful when exporting schemas?)
    // ------------------------------------------------------------------

    /// Returns an iterator of (name, description).
    pub fn descriptions(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.descriptions.iter().map(|(k, v)| (*k, *v))
    }

    /// Returns an iterator of (name, (TypeId_in, TypeId_out)).
    pub fn signatures(&self) -> impl Iterator<Item = (&'static str, (TypeId, TypeId))> + '_ {
        self.signatures.iter().map(|(k, v)| (*k, *v))
    }
}

// ----------------------------------------------------------------------
// Parsing helpers
// ----------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Args(Vec<Value>);

/// Parses commands in the format `name(arg1,arg2,...)` → (name, Args).
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

    // ------------------------------------------------------------------
    // Helper items shared by several tests
    // ------------------------------------------------------------------
    fn add<T: std::ops::Add<Output = T> + Copy>(a: T, b: T) -> T {
        a + b
    }
    fn concat<T: std::fmt::Display>(a: T, b: T) -> String {
        format!("{}{}", a, b)
    }
    fn noop() {}
    async fn async_foo() {}

    #[derive(PartialEq, Serialize, Deserialize)]
    struct SomeArgs {
        a: i32,
        b: i32,
    }
    fn using_args(_args: SomeArgs) {}

    // ------------------------------------------------------------------
    // Collection smoke-test (multiple signatures)
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_collection() {
        let mut collection = ToolCollection::default();

        collection.register("add", "Adds two values", |t: (i32, i32)| async move {
            add(t.0, t.1)
        });
        collection.register(
            "concat",
            "Concatenates two strings",
            |t: (String, String)| async move { concat(t.0, t.1) },
        );
        collection.register("noop", "Does nothing", |_t: ()| async move { noop() });
        collection.register(
            "complex_args",
            "Uses complex args",
            |t: SomeArgs| async move { using_args(t) },
        );

        assert_eq!(
            collection.call_to_json("add", "add(1,2)").await.unwrap(),
            json!(3)
        );
        assert_eq!(
            collection
                .call_to_json("concat", "concat(\"hello\",\"world\")")
                .await
                .unwrap(),
            json!("helloworld")
        );
        assert_eq!(
            collection.call_to_json("noop", "noop()").await.unwrap(),
            json!(null)
        );
        assert_eq!(
            collection
                .call_to_json("complex_args", "complex_args({\"a\":1,\"b\":2})")
                .await
                .unwrap(),
            json!(null)
        );
    }

    // ------------------------------------------------------------------
    // Parser: success cases (now async just to satisfy “all async”)
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_parser_success() {
        let cases = vec![
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
                ("function", Args(vec![json!([1, 2, 3]), json!({ "a": 1 })])),
            ),
            ("function()", ("function", Args(vec![]))),
            ("add(1,2)", ("add", Args(vec![json!(1), json!(2)]))),
            (
                "concat(\"hello\", \"world\")",
                ("concat", Args(vec![json!("hello"), json!("world")])),
            ),
        ];

        for (cmd, expected) in cases {
            let (name, args) = parse(cmd).expect("parser should succeed");
            assert_eq!(name, expected.0);
            assert_eq!(args, expected.1);
        }
    }

    // ------------------------------------------------------------------
    // Parser: failure cases
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_parser_failure() {
        for bad in ["missing_parenthesis", "func(1,2", "func1,2)", "func(,)"] {
            assert!(parse(bad).is_err(), "Parser should fail on: {bad}");
        }
    }

    // ------------------------------------------------------------------
    // Boolean return value
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_boolean_function() {
        let mut col = ToolCollection::default();
        col.register(
            "is_even",
            "Checks even",
            |t: (i32,)| async move { t.0 % 2 == 0 },
        );

        assert_eq!(
            col.call_to_json("is_even", "is_even(4)").await.unwrap(),
            json!(true)
        );
        assert_eq!(
            col.call_to_json("is_even", "is_even(3)").await.unwrap(),
            json!(false)
        );
    }

    // ------------------------------------------------------------------
    // Complex struct return
    // ------------------------------------------------------------------
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[tokio::test]
    async fn test_complex_return() {
        let mut col = ToolCollection::default();
        col.register(
            "create_point",
            "Creates a point",
            |t: (i32, i32)| async move { Point { x: t.0, y: t.1 } },
        );

        assert_eq!(
            col.call_to_json("create_point", "create_point(10,20)")
                .await
                .unwrap(),
            json!({"x": 10, "y": 20})
        );
    }

    // ------------------------------------------------------------------
    // Single-argument tuple
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_single_argument_tuple() {
        let mut col = ToolCollection::default();
        col.register("square", "Squares", |t: (i32,)| async move { t.0 * t.0 });

        assert_eq!(
            col.call_to_json("square", "square(5)").await.unwrap(),
            json!(25)
        );
    }

    // ------------------------------------------------------------------
    // Invalid function name
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_invalid_function_name() {
        let mut col = ToolCollection::default();
        col.register("dummy", "does nothing", |_: ()| async {}); // <- removed (())
        assert!(col.call_to_json("ghost", "ghost()").await.is_err());
    }

    // ------------------------------------------------------------------
    // Vector argument
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_vector_argument() {
        let mut col = ToolCollection::default();
        col.register("sum", "Sum vec", |t: (Vec<i32>,)| async move {
            t.0.iter().sum::<i32>()
        });

        assert_eq!(
            col.call_to_json("sum", "sum([1,2,3,4])").await.unwrap(),
            json!(10)
        );
    }

    // ------------------------------------------------------------------
    // Struct argument
    // ------------------------------------------------------------------
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Config {
        host: String,
        port: u16,
    }

    #[tokio::test]
    async fn test_struct_argument() {
        let mut col = ToolCollection::default();
        col.register("config_info", "Formats config", |c: Config| async move {
            format!("{}:{}", c.host, c.port)
        });

        assert_eq!(
            col.call_to_json(
                "config_info",
                "config_info({\"host\":\"localhost\",\"port\":8080})"
            )
            .await
            .unwrap(),
            json!("localhost:8080")
        );
    }

    // ------------------------------------------------------------------
    // Deserialization error
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_deserialization_error() {
        let mut col = ToolCollection::default();
        col.register("subtract", "Sub two numbers", |t: (i32, i32)| async move {
            t.0 - t.1
        });

        // Run the call in its own task; if the task panics, `JoinHandle` is Err.
        let handle =
            tokio::spawn(
                async move { col.call_to_json("subtract", "subtract(\"a\",\"b\")").await },
            );

        let join_res = handle.await; // JoinError if the task panicked
        assert!(
            join_res.is_err() && join_res.unwrap_err().is_panic(),
            "expected panic due to bad input types"
        );
    }

    // ------------------------------------------------------------------
    // Simple greeting
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_string_function() {
        let mut col = ToolCollection::default();
        col.register("greet", "Greets", |t: (String,)| async move {
            format!("Hello, {}!", t.0)
        });

        assert_eq!(
            col.call_to_json("greet", "greet(\"Alice\")").await.unwrap(),
            json!("Hello, Alice!")
        );
    }

    // ------------------------------------------------------------------
    // Async forwarding
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn test_async_function() {
        let mut col = ToolCollection::default();
        col.register(
            "async_foo",
            "noop",
            |_: ()| async move { async_foo().await },
        ); // <- removed (())
        assert_eq!(
            col.call_to_json("async_foo", "async_foo()").await.unwrap(),
            json!(null)
        );
    }
}
