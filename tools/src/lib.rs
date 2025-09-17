#![deny(unsafe_code)]

#[cfg(feature = "schema")]
extern crate schemars;

pub mod error;
pub mod models;
pub mod schema;

use std::{borrow::Cow, collections::HashMap, sync::Arc};

use futures::{FutureExt, future::BoxFuture};
use models::FunctionResponse;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{self, Value};

pub use error::{DeserializationError, ToolError};
pub use models::{FunctionCall, Tool, ToolFunc, ToolMetadata, ToolRegistration, TypeSignature};
pub use schema::{FunctionDecl, schema_to_json_schema};

use crate::models::CallId;

#[cfg(feature = "schema")]
pub trait MaybeJsonSchema: schemars::JsonSchema {}
#[cfg(feature = "schema")]
impl<T: schemars::JsonSchema> MaybeJsonSchema for T {}

#[cfg(not(feature = "schema"))]
pub trait MaybeJsonSchema {}
#[cfg(not(feature = "schema"))]
impl<T> MaybeJsonSchema for T {}

#[cfg(feature = "schema")]
fn schema_value<T: schemars::JsonSchema>() -> Result<Value, ToolError> {
    let schema = schema_to_json_schema::<T>();
    Ok(serde_json::to_value(schema)?)
}

#[cfg(not(feature = "schema"))]
fn schema_value<T>() -> Result<Value, ToolError> {
    Ok(Value::Null)
}

#[derive(Default)]
pub struct ToolCollection {
    funcs: HashMap<&'static str, Arc<ToolFunc>>,
    descriptions: HashMap<&'static str, &'static str>,
    signatures: HashMap<&'static str, TypeSignature>,
    declarations: HashMap<&'static str, FunctionDecl<'static>>,
}

impl ToolCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<I, O, F, Fut>(
        &mut self,
        name: &'static str,
        desc: &'static str,
        func: F,
    ) -> Result<&mut Self, ToolError>
    where
        I: 'static + DeserializeOwned + Serialize + Send + MaybeJsonSchema,
        O: 'static + Serialize + Send + MaybeJsonSchema,
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = O> + Send + 'static,
    {
        if self.funcs.contains_key(name) {
            return Err(ToolError::AlreadyRegistered { name });
        }

        self.descriptions.insert(name, desc);
        self.signatures.insert(
            name,
            TypeSignature {
                input_id: std::any::TypeId::of::<I>(),
                output_id: std::any::TypeId::of::<O>(),
                input_name: std::any::type_name::<I>().into(),
                output_name: std::any::type_name::<O>().into(),
            },
        );

        self.declarations.insert(
            name,
            FunctionDecl {
                name,
                description: desc,
                parameters: schema_value::<I>()?,
                returns: schema_value::<O>()?,
            },
        );

        let func_arc: Arc<F> = Arc::new(func);
        self.funcs.insert(
            name,
            Arc::new(
                move |raw: Value| -> BoxFuture<'static, Result<Value, ToolError>> {
                    let func = func_arc.clone();
                    async move {
                        let input: I =
                            serde_json::from_value(raw).map_err(DeserializationError::from)?;
                        let output: O = (func)(input).await;
                        serde_json::to_value(output).map_err(|e| ToolError::Runtime(e.to_string()))
                    }
                    .boxed()
                },
            ),
        );

        Ok(self)
    }

    pub async fn call(&self, call: FunctionCall) -> Result<FunctionResponse, ToolError> {
        let FunctionCall {
            id,
            name,
            arguments,
        } = call;
        let async_func = self
            .funcs
            .get(name.as_str())
            .ok_or(ToolError::FunctionNotFound {
                name: Cow::Owned(name.clone()),
            })?;
        let result = async_func(arguments).await?;

        Ok(FunctionResponse { id, name, result })
    }

    pub fn unregister(&mut self, name: &str) -> Result<(), ToolError> {
        if self.funcs.remove(name).is_none() {
            return Err(ToolError::FunctionNotFound {
                name: Cow::Owned(name.to_string()),
            });
        }
        self.descriptions.remove(name);
        self.signatures.remove(name);
        self.declarations.remove(name);
        Ok(())
    }

    pub fn descriptions(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.descriptions.iter().map(|(k, v)| (*k, *v))
    }

    pub fn signatures(&self) -> impl Iterator<Item = (&'static str, &TypeSignature)> + '_ {
        self.signatures.iter().map(|(k, v)| (*k, v))
    }

    pub fn collect_tools() -> Self {
        let mut hub = Self::new();

        for reg in inventory::iter::<ToolRegistration> {
            hub.descriptions.insert(reg.name, reg.doc);
            hub.funcs.insert(reg.name, Arc::new(reg.f));

            hub.declarations.insert(
                reg.name,
                FunctionDecl {
                    name: reg.name,
                    description: reg.doc,
                    parameters: (reg.param_schema)(),
                    returns: (reg.return_schema)(),
                },
            );
        }

        hub
    }

    pub fn json(&self) -> Result<Value, ToolError> {
        let list: Vec<&FunctionDecl> = self.declarations.values().collect();
        Ok(serde_json::to_value(list)?)
    }
}

inventory::collect!(ToolRegistration);

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "schema")]
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::{self, json};

    fn add<T: std::ops::Add<Output = T> + Copy>(a: T, b: T) -> T {
        a + b
    }
    fn concat<T: std::fmt::Display>(a: T, b: T) -> String {
        format!("{}{}", a, b)
    }
    fn noop() {}
    async fn async_foo() {}

    #[cfg_attr(feature = "schema", derive(JsonSchema))]
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct SomeArgs {
        a: i32,
        b: i32,
    }
    fn using_args(_a: SomeArgs) {}

    fn fc(name: &str, args: serde_json::Value) -> FunctionCall {
        FunctionCall::new(name.to_string(), args)
    }

    #[tokio::test]
    async fn test_collection() {
        let mut collection = ToolCollection::default();

        collection
            .register("add", "Adds two values", |t: (i32, i32)| async move {
                add(t.0, t.1)
            })
            .unwrap();
        collection
            .register(
                "concat",
                "Concatenates two strings",
                |t: (String, String)| async move { concat(t.0, t.1) },
            )
            .unwrap();
        collection
            .register("noop", "Does nothing", |_t: ()| async move { noop() })
            .unwrap();
        collection
            .register(
                "complex_args",
                "Uses complex args",
                |t: SomeArgs| async move { using_args(t) },
            )
            .unwrap();

        assert_eq!(
            collection
                .call(fc("add", json!([1, 2])))
                .await
                .unwrap()
                .result,
            json!(3)
        );
        assert_eq!(
            collection
                .call(fc("concat", json!(["hello", "world"])))
                .await
                .unwrap()
                .result,
            json!("helloworld")
        );
        assert_eq!(
            collection
                .call(fc("noop", json!(null)))
                .await
                .unwrap()
                .result,
            json!(null)
        );
        assert_eq!(
            collection
                .call(fc("complex_args", json!({ "a": 1, "b": 2 })))
                .await
                .unwrap()
                .result,
            json!(null)
        );
    }

    #[tokio::test]
    async fn test_boolean_function() {
        let mut col = ToolCollection::default();
        col.register(
            "is_even",
            "Checks even",
            |t: (i32,)| async move { t.0 % 2 == 0 },
        )
        .unwrap();

        assert_eq!(
            col.call(fc("is_even", json!([4]))).await.unwrap().result,
            json!(true)
        );
        assert_eq!(
            col.call(fc("is_even", json!([3]))).await.unwrap().result,
            json!(false)
        );
    }

    #[cfg_attr(feature = "schema", derive(JsonSchema))]
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
        )
        .unwrap();

        assert_eq!(
            col.call(fc("create_point", json!([10, 20])))
                .await
                .unwrap()
                .result,
            json!({ "x": 10, "y": 20 })
        );
    }

    #[tokio::test]
    async fn test_single_argument_tuple() {
        let mut col = ToolCollection::default();
        col.register("square", "Squares", |t: (i32,)| async move { t.0 * t.0 })
            .unwrap();

        assert_eq!(
            col.call(fc("square", json!([5]))).await.unwrap().result,
            json!(25)
        );
    }

    #[tokio::test]
    async fn test_invalid_function_name() {
        let mut col = ToolCollection::default();
        col.register("dummy", "does nothing", |_: ()| async {})
            .unwrap();

        let err = col.call(fc("ghost", json!([]))).await.unwrap_err();
        assert!(matches!(err, ToolError::FunctionNotFound { .. }));
    }

    #[tokio::test]
    async fn test_vector_argument() {
        let mut col = ToolCollection::default();
        col.register("sum", "Sum vec", |t: (Vec<i32>,)| async move {
            t.0.iter().sum::<i32>()
        })
        .unwrap();

        assert_eq!(
            col.call(fc("sum", json!([[1, 2, 3, 4]])))
                .await
                .unwrap()
                .result,
            json!(10)
        );
    }

    #[cfg_attr(feature = "schema", derive(JsonSchema))]
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
        })
        .unwrap();

        assert_eq!(
            col.call(fc(
                "config_info",
                json!({ "host": "localhost", "port": 8080 })
            ))
            .await
            .unwrap()
            .result,
            json!("localhost:8080")
        );
    }

    #[tokio::test]
    async fn test_deserialization_error() {
        let mut col = ToolCollection::default();
        col.register("subtract", "Sub two numbers", |t: (i32, i32)| async move {
            t.0 - t.1
        })
        .unwrap();

        let err = col
            .call(fc("subtract", json!(["a", "b"]))) // bad types → error
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::Deserialize(_)));
    }

    #[tokio::test]
    async fn test_string_function() {
        let mut col = ToolCollection::default();
        col.register("greet", "Greets", |t: (String,)| async move {
            format!("Hello, {}!", t.0)
        })
        .unwrap();

        assert_eq!(
            col.call(fc("greet", json!(["Alice"])))
                .await
                .unwrap()
                .result,
            json!("Hello, Alice!")
        );
    }

    #[tokio::test]
    async fn test_async_function() {
        let mut col = ToolCollection::default();
        col.register(
            "async_foo",
            "noop",
            |_: ()| async move { async_foo().await },
        )
        .unwrap();

        assert_eq!(
            col.call(fc("async_foo", json!(null))).await.unwrap().result,
            json!(null)
        );
    }
}

#[cfg(test)]
mod stateful_tests {
    use super::*;
    use serde_json::{self, json};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    fn fc(name: &str, args: serde_json::Value) -> FunctionCall {
        FunctionCall {
            id: Some(CallId::new()),
            name: name.to_string(),
            arguments: args,
        }
    }

    #[tokio::test]
    async fn test_stateful_counter_serial() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut col = ToolCollection::default();

        {
            let counter_inc = counter.clone(); // keep outer handle
            col.register("inc", "increments", move |_: ()| {
                let counter_inner = counter_inc.clone(); // handle for this call
                async move { counter_inner.fetch_add(1, Ordering::SeqCst) + 1 }
            })
            .unwrap();
        }

        {
            let counter_get = counter.clone();
            col.register("get", "gets value", move |_: ()| {
                let counter_inner = counter_get.clone();
                async move { counter_inner.load(Ordering::SeqCst) }
            })
            .unwrap();
        }

        assert_eq!(
            col.call(fc("inc", json!(null))).await.unwrap().result,
            json!(1)
        );
        assert_eq!(
            col.call(fc("inc", json!(null))).await.unwrap().result,
            json!(2)
        );
        assert_eq!(
            col.call(fc("inc", json!(null))).await.unwrap().result,
            json!(3)
        );
        assert_eq!(
            col.call(fc("get", json!(null))).await.unwrap().result,
            json!(3)
        );
    }

    #[tokio::test]
    async fn test_stateful_counter_concurrent() {
        use std::sync::Arc;

        let counter = Arc::new(AtomicUsize::new(0));

        let mut col_mut = ToolCollection::default();
        {
            let counter_inc = counter.clone();
            col_mut
                .register("inc", "increments", move |_: ()| {
                    let counter_inner = counter_inc.clone();
                    async move { counter_inner.fetch_add(1, Ordering::SeqCst) + 1 }
                })
                .unwrap();
        }

        let col = Arc::new(col_mut);

        let handles = (0..10)
            .map(|_| {
                let col_clone = col.clone();
                tokio::spawn(async move { col_clone.call(fc("inc", json!(null))).await })
            })
            .collect::<Vec<_>>();

        for h in handles {
            assert!(h.await.unwrap().is_ok());
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_stateful_vector_and_unregister() {
        let data: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(vec![]));
        let mut col = ToolCollection::default();

        {
            let data_push = data.clone();
            col.register("push", "pushes", move |t: (i32,)| {
                let data_inner = data_push.clone();
                async move {
                    let mut g = data_inner.lock().unwrap();
                    g.push(t.0);
                    g.len()
                }
            })
            .unwrap();
        }

        {
            let data_len = data.clone();
            col.register("len", "length", move |_: ()| {
                let data_inner = data_len.clone();
                async move { data_inner.lock().unwrap().len() }
            })
            .unwrap();
        }

        assert_eq!(
            col.call(fc("push", json!([1]))).await.unwrap().result,
            json!(1)
        );
        assert_eq!(
            col.call(fc("push", json!([2]))).await.unwrap().result,
            json!(2)
        );
        assert_eq!(
            col.call(fc("push", json!([3]))).await.unwrap().result,
            json!(3)
        );
        assert_eq!(
            col.call(fc("len", json!(null))).await.unwrap().result,
            json!(3)
        );

        col.unregister("push").unwrap();

        let err = col.call(fc("push", json!([4]))).await.unwrap_err();
        assert!(matches!(err, ToolError::FunctionNotFound { .. }));

        assert_eq!(data.lock().unwrap().as_slice(), &[1, 2, 3]);
    }
}
