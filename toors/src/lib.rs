//! lib/tool_collection.rs
//! A minimal runtime for LLM “function‑calling”, fully JSON‑driven.

pub mod toors_errors;

use std::{any::TypeId, borrow::Cow, collections::HashMap, sync::Arc};

use futures::{future::BoxFuture, FutureExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use toors_errors::{DeserializationError, ToolError};

/* ───────────────────────────── PUBLIC TYPES ────────────────────────── */

/// Envelope produced by the LLM / router when it wants to call a tool.
#[derive(Debug, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    /// Exact JSON emitted by the model.
    pub arguments: Value,
}

/// Async function signature stored in the registry.
pub type ToolFunc = dyn Fn(Value) -> BoxFuture<'static, Result<Value, ToolError>> + Send + Sync;

/// Registry that owns all functions plus a bit of metadata.
#[derive(Default)]
pub struct ToolCollection {
    funcs: HashMap<&'static str, Arc<ToolFunc>>,
    descriptions: HashMap<&'static str, &'static str>,
    signatures: HashMap<&'static str, (TypeId, TypeId)>,
}

/* ───────────────────────────── IMPLEMENTATION ──────────────────────── */

impl ToolCollection {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /*──────────── fluent `register` – returns &mut Self ───────────────*/
    pub fn register<I, O, F, Fut>(
        &mut self,
        name: &'static str,
        desc: &'static str,
        func: F,
    ) -> &mut Self
    where
        I: 'static + DeserializeOwned + Send,
        O: 'static + Serialize + Send,
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = O> + Send + 'static,
    {
        /* 1. metadata */
        self.descriptions.insert(name, desc);
        self.signatures
            .insert(name, (TypeId::of::<I>(), TypeId::of::<O>()));

        /* 2. wrapper */
        let func_arc: Arc<F> = Arc::new(func);
        self.funcs.insert(
            name,
            Arc::new(
                move |raw: Value| -> BoxFuture<'static, Result<Value, ToolError>> {
                    let func_clone = func_arc.clone();
                    async move {
                        /* 2a. JSON → I */
                        let input: I = serde_json::from_value(raw).map_err(|e| {
                            ToolError::Deserialize(DeserializationError(Cow::Owned(e.to_string())))
                        })?;

                        /* 2b. run user code */
                        let output: O = (func_clone)(input).await;

                        /* 2c. I/O → JSON */
                        serde_json::to_value(output).map_err(|e| ToolError::Runtime(e.to_string()))
                    }
                    .boxed()
                },
            ),
        );

        self
    }

    /// Invoke a tool with the JSON envelope produced by the model.
    pub async fn call(&self, call: FunctionCall) -> Result<Value, ToolError> {
        let async_func = self.funcs.get(call.name.as_str()).ok_or_else(|| {
            let leaked: &'static str = Box::leak(call.name.into_boxed_str());
            ToolError::FunctionNotFound { name: leaked }
        })?;

        async_func(call.arguments).await
    }

    /* small helpers */
    pub fn descriptions(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.descriptions.iter().map(|(k, v)| (*k, *v))
    }

    pub fn signatures(&self) -> impl Iterator<Item = (&'static str, (TypeId, TypeId))> + '_ {
        self.signatures.iter().map(|(k, v)| (*k, *v))
    }

    /*──────────── auto‑populate every #[tool] function ────────────────*/
    pub fn collect_tools() -> Self {
        let mut hub = Self::new();
        for reg in inventory::iter::<ToolRegistration> {
            hub.descriptions.insert(reg.name, reg.doc);
            hub.funcs.insert(reg.name, Arc::new(reg.f));
        }
        hub
    }
}

/* ─────────────────────────── inventory glue ────────────────────────── */

/// One registration record, submitted by the `#[tool]` proc‑macro.
pub struct ToolRegistration {
    pub name: &'static str,
    pub doc: &'static str,
    pub f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
}

impl ToolRegistration {
    pub const fn new(
        name: &'static str,
        doc: &'static str,
        f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
    ) -> Self {
        Self { name, doc, f }
    }
}

inventory::collect!(ToolRegistration);
#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    // ------------------------------------------------------------
    // Helpers shared by several tests
    // ------------------------------------------------------------
    fn add<T: std::ops::Add<Output = T> + Copy>(a: T, b: T) -> T {
        a + b
    }
    fn concat<T: std::fmt::Display>(a: T, b: T) -> String {
        format!("{a}{b}")
    }
    fn noop() {}
    async fn async_foo() {}

    #[derive(Serialize, Deserialize, PartialEq)]
    struct SomeArgs {
        a: i32,
        b: i32,
    }
    fn using_args(_args: SomeArgs) {}

    // Convenience wrapper so the assertions stay short.
    async fn call_ok(
        col: &ToolCollection,
        name: &str,
        args: serde_json::Value,
    ) -> serde_json::Value {
        col.call(FunctionCall {
            name: name.into(),
            arguments: args,
        })
        .await
        .expect("tool should succeed")
    }

    // ------------------------------------------------------------
    // Smoke‑test: four different signatures
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_collection() {
        let mut col = ToolCollection::default();

        col.register("add", "Adds two values", |t: (i32, i32)| async move {
            add(t.0, t.1)
        });
        col.register(
            "concat",
            "Concatenates two strings",
            |t: (String, String)| async move { concat(t.0, t.1) },
        );
        col.register("noop", "Does nothing", |_t: ()| async move { noop() });
        col.register(
            "complex_args",
            "Uses complex args",
            |t: SomeArgs| async move { using_args(t) },
        );

        assert_eq!(call_ok(&col, "add", json!([1, 2])).await, json!(3));
        assert_eq!(
            call_ok(&col, "concat", json!(["hello", "world"])).await,
            json!("helloworld")
        );
        assert_eq!(call_ok(&col, "noop", json!(null)).await, json!(null));
        assert_eq!(
            call_ok(
                &col,
                "complex_args",
                json!({ "a": 1, "b": 2 }) // struct, not tuple
            )
            .await,
            json!(null)
        );
    }

    // ------------------------------------------------------------
    // Boolean return value
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_boolean_function() {
        let mut col = ToolCollection::default();
        col.register(
            "is_even",
            "Checks even",
            |t: (i32,)| async move { t.0 % 2 == 0 },
        );

        assert_eq!(call_ok(&col, "is_even", json!([4])).await, json!(true));
        assert_eq!(call_ok(&col, "is_even", json!([3])).await, json!(false));
    }

    // ------------------------------------------------------------
    // Complex struct return
    // ------------------------------------------------------------
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
            call_ok(&col, "create_point", json!([10, 20])).await,
            json!({ "x": 10, "y": 20 })
        );
    }

    // ------------------------------------------------------------
    // Single‑argument tuple
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_single_argument_tuple() {
        let mut col = ToolCollection::default();
        col.register("square", "Squares", |t: (i32,)| async move { t.0 * t.0 });

        assert_eq!(call_ok(&col, "square", json!([5])).await, json!(25));
    }

    // ------------------------------------------------------------
    // Invalid function name → FunctionNotFound
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_invalid_function_name() {
        let mut col = ToolCollection::default();
        col.register("dummy", "does nothing", |_: ()| async {});

        let err = col
            .call(FunctionCall {
                name: "ghost".into(),
                arguments: json!([]),
            })
            .await
            .unwrap_err();

        matches!(err, ToolError::FunctionNotFound { .. });
    }

    // ------------------------------------------------------------
    // Vector argument
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_vector_argument() {
        let mut col = ToolCollection::default();
        col.register("sum", "Sum vec", |t: (Vec<i32>,)| async move {
            t.0.iter().sum::<i32>()
        });

        assert_eq!(call_ok(&col, "sum", json!([[1, 2, 3, 4]])).await, json!(10));
    }

    // ------------------------------------------------------------
    // Struct argument → string result
    // ------------------------------------------------------------
    #[derive(Serialize, Deserialize)]
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
            call_ok(
                &col,
                "config_info",
                json!({ "host": "localhost", "port": 8080 })
            )
            .await,
            json!("localhost:8080")
        );
    }

    // ------------------------------------------------------------
    // Deserialisation error → ToolError::Deserialize
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_deserialization_error() {
        let mut col = ToolCollection::default();
        col.register("subtract", "Sub two numbers", |t: (i32, i32)| async move {
            t.0 - t.1
        });

        let err = col
            .call(FunctionCall {
                name: "subtract".into(),
                arguments: json!(["a", "b"]), // wrong types
            })
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::Deserialize(_)));
    }

    // ------------------------------------------------------------
    // Arity mismatch (one element instead of two) → Deserialize error
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_arity_mismatch() {
        let mut col = ToolCollection::default();
        col.register("add", "Adds two numbers", |t: (i32, i32)| async move {
            t.0 + t.1
        });

        let err = col
            .call(FunctionCall {
                name: "add".into(),
                arguments: json!([42]), // only one arg
            })
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::Deserialize(_)));
    }

    // ------------------------------------------------------------
    // Async function
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_async_function() {
        let mut col = ToolCollection::default();
        col.register(
            "async_foo",
            "noop",
            |_: ()| async move { async_foo().await },
        );

        assert_eq!(
            call_ok(&col, "async_foo", json!(null)).await,
            serde_json::Value::Null
        );
    }

    // ------------------------------------------------------------
    // Concurrent calls
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_concurrent_calls() {
        let mut col = ToolCollection::default();
        col.register("add", "Adds", |t: (i32, i32)| async move { t.0 + t.1 });
        col.register("concat", "Concats", |t: (String, String)| async move {
            format!("{}{}", t.0, t.1)
        });

        let add_fut = col.call(FunctionCall {
            name: "add".into(),
            arguments: json!([10, 20]),
        });
        let concat_fut = col.call(FunctionCall {
            name: "concat".into(),
            arguments: json!(["foo", "bar"]),
        });

        let (add_res, concat_res) = tokio::join!(add_fut, concat_fut);

        assert_eq!(add_res.unwrap(), json!(30));
        assert_eq!(concat_res.unwrap(), json!("foobar"));
    }

    // ------------------------------------------------------------
    // Metadata reflection
    // ------------------------------------------------------------
    #[test]
    fn test_metadata_reflection() {
        let mut col = ToolCollection::default();
        col.register("noop", "Does nothing", |_: ()| async {});
        col.register("square", "Squares", |t: (i32,)| async move { t.0 * t.0 });

        // Descriptions
        let map: std::collections::HashMap<_, _> = col.descriptions().collect();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("noop"), Some(&"Does nothing"));
        assert_eq!(map.get("square"), Some(&"Squares"));

        // Signatures
        let sigs: std::collections::HashMap<_, _> = col.signatures().collect();
        let (in_id, out_id) = sigs.get("square").copied().unwrap();
        assert_eq!(in_id, std::any::TypeId::of::<(i32,)>());
        assert_eq!(out_id, std::any::TypeId::of::<i32>());
    }

    // ------------------------------------------------------------
    // Unit argument
    // ------------------------------------------------------------
    #[tokio::test]
    async fn test_unit_argument() {
        let mut col = ToolCollection::default();
        col.register("unit", "accepts ()", |_: ()| async {});

        assert_eq!(
            call_ok(&col, "unit", json!(null)).await,
            serde_json::Value::Null
        );
    }

    // ------------------------------------------------------------
    // Stress test: 10 000 parallel calls
    // ------------------------------------------------------------
    use futures::future::join_all;
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread", worker_threads = 16)]
    async fn concurrency_test() {
        const N_TASKS: usize = 10_000;

        let mut col = ToolCollection::default();
        col.register("add", "Adds", |t: (i32, i32)| async move { t.0 + t.1 });
        let col = Arc::new(col);

        let mut tasks = Vec::with_capacity(N_TASKS);
        for i in 0..N_TASKS {
            let col = col.clone();
            tasks.push(tokio::spawn(async move {
                let a = (i as i32) % 1_000;
                let b = ((i * 7) as i32) % 1_000;

                let out = col
                    .call(FunctionCall {
                        name: "add".into(),
                        arguments: json!([a, b]),
                    })
                    .await
                    .unwrap();

                assert_eq!(out, json!(a + b));
            }));
        }

        join_all(tasks)
            .await
            .into_iter()
            .for_each(|r| r.expect("task panicked"));
    }
}
