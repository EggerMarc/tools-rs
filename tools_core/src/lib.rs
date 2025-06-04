#![deny(unsafe_code)]

use std::{borrow::Cow, collections::HashMap, sync::Arc};

use futures::{future::BoxFuture, FutureExt};
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{self, Value};

// Re-export once_cell for use in generated code
pub use once_cell;

// ============================================================================
// TOOL SCHEMA TRAIT AND IMPLEMENTATIONS
// ============================================================================

/// Trait for types that can generate a JSON Schema representation of themselves.
pub trait ToolSchema {
    fn schema() -> Value;
}

// Macro for implementing ToolSchema for primitive types with caching
macro_rules! prim {
    ($ty:ty, $name:expr) => {
        impl ToolSchema for $ty {
            fn schema() -> Value {
                static SCHEMA: Lazy<Value> = Lazy::new(|| serde_json::json!({ "type": $name }));
                SCHEMA.clone()
            }
        }
    };
}

prim!(bool, "boolean");
prim!(i8, "integer");
prim!(i16, "integer");
prim!(i32, "integer");
prim!(i64, "integer");
prim!(i128, "integer");
prim!(isize, "integer");
prim!(u8, "integer");
prim!(u16, "integer");
prim!(u32, "integer");
prim!(u64, "integer");
prim!(u128, "integer");
prim!(usize, "integer");
prim!(f32, "number");
prim!(f64, "number");

impl ToolSchema for &'_ str {
    fn schema() -> Value {
        static SCHEMA: Lazy<Value> = Lazy::new(|| serde_json::json!({ "type": "string" }));
        SCHEMA.clone()
    }
}

impl ToolSchema for str {
    fn schema() -> Value {
        static SCHEMA: Lazy<Value> = Lazy::new(|| serde_json::json!({ "type": "string" }));
        SCHEMA.clone()
    }
}

impl ToolSchema for String {
    fn schema() -> Value {
        static SCHEMA: Lazy<Value> = Lazy::new(|| serde_json::json!({ "type": "string" }));
        SCHEMA.clone()
    }
}

impl ToolSchema for () {
    fn schema() -> Value {
        static SCHEMA: Lazy<Value> = Lazy::new(|| serde_json::json!({ "type": "null" }));
        SCHEMA.clone()
    }
}

impl<T: ToolSchema> ToolSchema for Option<T> {
    fn schema() -> Value {
        // Note: For generic types, we can't use static caching since each T creates a different type
        // The derived implementations will handle caching for concrete types
        serde_json::json!({
            "anyOf": [
                T::schema(),
                { "type": "null" }
            ]
        })
    }
}

impl<T: ToolSchema> ToolSchema for Vec<T> {
    fn schema() -> Value {
        // Note: For generic types, we can't use static caching since each T creates a different type
        // The derived implementations will handle caching for concrete types
        serde_json::json!({
            "type": "array",
            "items": T::schema()
        })
    }
}

impl<T: ToolSchema> ToolSchema for HashMap<String, T> {
    fn schema() -> Value {
        // Note: For generic types, we can't use static caching since each T creates a different type
        // The derived implementations will handle caching for concrete types
        serde_json::json!({
            "type": "object",
            "additionalProperties": T::schema()
        })
    }
}

// Tuple implementations
macro_rules! impl_tuples {
    ($($len:expr => ($($n:tt $name:ident)+))+) => {
        $(
            impl<$($name: ToolSchema),+> ToolSchema for ($($name,)+) {
                fn schema() -> Value {
                    // Note: For generic tuples, we can't use static caching since each combination
                    // of types creates a different tuple type. The derived implementations will
                    // handle caching for concrete tuple types.
                    serde_json::json!({
                        "type": "array",
                        "prefixItems": [$($name::schema()),+],
                        "minItems": $len,
                        "maxItems": $len
                    })
                }
            }
        )+
    }
}

impl_tuples! {
    1 => (0 T0)
    2 => (0 T0 1 T1)
    3 => (0 T0 1 T1 2 T2)
    4 => (0 T0 1 T1 2 T2 3 T3)
    5 => (0 T0 1 T1 2 T2 3 T3 4 T4)
    6 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5)
    7 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6)
    8 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7)
    9 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8)
    10 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9)
    11 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10)
    12 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11)
    13 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12)
    14 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13)
    15 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14)
    16 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15)
    17 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16)
    18 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17)
    19 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18)
    20 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19)
    21 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20)
    22 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21)
    23 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21 22 T22)
    24 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21 22 T22 23 T23)
    25 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21 22 T22 23 T23 24 T24)
    26 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21 22 T22 23 T23 24 T24 25 T25)
    27 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21 22 T22 23 T23 24 T24 25 T25 26 T26)
    28 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21 22 T22 23 T23 24 T24 25 T25 26 T26 27 T27)
    29 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21 22 T22 23 T23 24 T24 25 T25 26 T26 27 T27 28 T28)
    30 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19 20 T20 21 T21 22 T22 23 T23 24 T24 25 T25 26 T26 27 T27 28 T28 29 T29)
}

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that can occur during tool operations
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool function '{name}' not found")]
    FunctionNotFound { name: Cow<'static, str> },

    #[error("Tool function '{name}' is already registered")]
    AlreadyRegistered { name: &'static str },

    #[error("Deserialization error: {0}")]
    Deserialize(#[from] DeserializationError),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Runtime error: {0}")]
    Runtime(String),
}

/// Specific deserialization errors
#[derive(Debug, thiserror::Error)]
#[error("Failed to deserialize JSON: {source}")]
pub struct DeserializationError {
    #[source]
    pub source: serde_json::Error,
}

impl From<serde_json::Error> for DeserializationError {
    fn from(err: serde_json::Error) -> Self {
        DeserializationError { source: err }
    }
}

// ============================================================================
// CORE MODELS
// ============================================================================

/// Represents a function call with name and arguments
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

/// Function signature for tools
pub type ToolFunc = dyn Fn(Value) -> BoxFuture<'static, Result<Value, ToolError>> + Send + Sync;

/// Metadata about a tool function
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: &'static str,
    pub description: &'static str,
}

/// Runtime type signature information
#[derive(Debug, Clone)]
pub struct TypeSignature {
    pub input_type: &'static str,
    pub output_type: &'static str,
}

/// Tool registration for inventory collection
pub struct ToolRegistration {
    pub name: &'static str,
    pub doc: &'static str,
    pub f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
    pub param_schema: fn() -> Value,
}

impl ToolRegistration {
    pub const fn new(
        name: &'static str,
        doc: &'static str,
        f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
        param_schema: fn() -> Value,
    ) -> Self {
        Self {
            name,
            doc,
            f,
            param_schema,
        }
    }
}

/// Represents a tool that can be called
#[derive(Debug, Clone)]
pub struct Tool {
    pub metadata: ToolMetadata,
    pub signature: TypeSignature,
}

// ============================================================================
// SCHEMA GENERATION
// ============================================================================

/// Function declaration for LLM consumption
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct FunctionDecl<'a> {
    #[serde(borrow)]
    pub name: &'a str,
    #[serde(borrow)]
    pub description: &'a str,
    pub parameters: Value,
}

impl<'a> FunctionDecl<'a> {
    pub fn new(name: &'a str, description: &'a str, parameters: Value) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}

// ============================================================================
// TOOL COLLECTION
// ============================================================================

fn schema_value<T: ToolSchema>() -> Result<Value, ToolError> {
    Ok(T::schema())
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
        I: 'static + DeserializeOwned + Serialize + Send + ToolSchema,
        O: 'static + Serialize + Send + ToolSchema,
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = O> + Send + 'static,
    {
        if self.funcs.contains_key(name) {
            return Err(ToolError::AlreadyRegistered { name });
        }

        self.descriptions.insert(name, desc);

        self.declarations
            .insert(name, FunctionDecl::new(name, desc, schema_value::<I>()?));

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

    pub async fn call(&self, call: FunctionCall) -> Result<Value, ToolError> {
        let FunctionCall { name, arguments } = call;
        let async_func = self
            .funcs
            .get(name.as_str())
            .ok_or(ToolError::FunctionNotFound {
                name: Cow::Owned(name),
            })?;
        async_func(arguments).await
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

    pub fn collect_tools() -> Self {
        let mut hub = Self::new();

        for reg in inventory::iter::<ToolRegistration> {
            hub.descriptions.insert(reg.name, reg.doc);
            hub.funcs.insert(reg.name, Arc::new(reg.f));

            hub.declarations.insert(
                reg.name,
                FunctionDecl::new(reg.name, reg.doc, (reg.param_schema)()),
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

// ============================================================================
// TESTS
// ============================================================================

// Schema tests commented out due to circular dependency with derive macro
// #[cfg(test)]
// mod schema_tests {
//     use super::*;
//     use serde_json::json;

//     #[test]
//     fn test_primitive_schemas() {
//         assert_eq!(bool::schema(), json!({"type": "boolean"}));
//         assert_eq!(i32::schema(), json!({"type": "integer"}));
//         assert_eq!(f64::schema(), json!({"type": "number"}));
//         assert_eq!(String::schema(), json!({"type": "string"}));
//         assert_eq!(<()>::schema(), json!({"type": "null"}));
//     }

//     #[test]
//     fn test_option_schema() {
//         assert_eq!(
//             <Option<i32>>::schema(),
//             json!({
//                 "anyOf": [
//                     {"type": "integer"},
//                     {"type": "null"}
//                 ]
//             })
//         );
//     }

//     #[test]
//     fn test_vec_schema() {
//         assert_eq!(
//             <Vec<String>>::schema(),
//             json!({"type": "array", "items": {"type": "string"}})
//         );
//     }

//     #[test]
//     fn test_tuple_schemas() {
//         assert_eq!(
//             <(i32,)>::schema(),
//             json!({
//                 "type": "array",
//                 "prefixItems": [{"type": "integer"}],
//                 "minItems": 1,
//                 "maxItems": 1
//             })
//         );

//         assert_eq!(
//             <(i32, String)>::schema(),
//             json!({
//                 "type": "array",
//                 "prefixItems": [{"type": "integer"}, {"type": "string"}],
//                 "minItems": 2,
//                 "maxItems": 2
//             })
//         );
//     }

//     #[test]
//     fn test_hashmap_schema() {
//         assert_eq!(
//             <HashMap<String, i32>>::schema(),
//             json!({
//                 "type": "object",
//                 "additionalProperties": {"type": "integer"}
//             })
//         );
//     }

//     #[derive(serde::Serialize, serde::Deserialize, ToolSchema)]
//     struct UserId(u64);

//     #[derive(serde::Serialize, serde::Deserialize, ToolSchema)]
//     struct Email(String);

//     #[derive(serde::Serialize, serde::Deserialize, ToolSchema)]
//     struct Temperature(f64);

//     #[derive(serde::Serialize, serde::Deserialize, ToolSchema)]
//     struct Count(usize);

//     #[test]
//     fn test_newtype_schemas() {
//         assert_eq!(
//             UserId::schema(),
//             json!({
//                 "type": "array",
//                 "prefixItems": [{"type": "integer"}],
//                 "minItems": 1,
//                 "maxItems": 1
//             })
//         );

//         assert_eq!(
//             Email::schema(),
//             json!({
//                 "type": "array",
//                 "prefixItems": [{"type": "string"}],
//                 "minItems": 1,
//                 "maxItems": 1
//             })
//         );

//         assert_eq!(
//             Temperature::schema(),
//             json!({
//                 "type": "array",
//                 "prefixItems": [{"type": "number"}],
//                 "minItems": 1,
//                 "maxItems": 1
//             })
//         );

//         assert_eq!(
//             Count::schema(),
//             json!({
//                 "type": "array",
//                 "prefixItems": [{"type": "integer"}],
//                 "minItems": 1,
//                 "maxItems": 1
//             })
//         );
//     }

//     #[derive(serde::Serialize, serde::Deserialize, ToolSchema)]
//     struct UserProfile {
//         id: UserId,
//         email: Email,
//         name: String,
//         age: Option<u32>,
//     }

//     #[test]
//     fn test_newtype_in_struct() {
//         let expected = json!({
//             "type": "object",
//             "properties": {
//                 "id": {"type": "array", "prefixItems": [{"type": "integer"}], "minItems": 1, "maxItems": 1},
//                 "email": {"type": "array", "prefixItems": [{"type": "string"}], "minItems": 1, "maxItems": 1},
//                 "name": {"type": "string"},
//                 "age": {"anyOf": [{"type": "integer"}, {"type": "null"}]}
//             },
//             "required": ["id", "email", "name"]
//         });

//         assert_eq!(UserProfile::schema(), expected);
//     }
// }

#[cfg(test)]
mod tool_tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::{self, json};

    fn add<T: std::ops::Add<Output = T> + Copy>(a: T, b: T) -> T {
        a + b
    }
    fn concat<T: std::fmt::Display>(a: T, b: T) -> String {
        format!("{}{}", a, b)
    }
    fn noop() {}
    // async fn async_foo() {}

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct SomeArgs {
        a: i32,
        b: i32,
    }
    // fn using_args(_a: SomeArgs) {}

    fn fc(name: &str, args: serde_json::Value) -> FunctionCall {
        FunctionCall {
            name: name.to_string(),
            arguments: args,
        }
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
        // Complex args test commented out due to ToolSchema derive requirement
        // collection
        //     .register(
        //         "complex_args",
        //         "Uses complex args",
        //         |t: SomeArgs| async move { using_args(t) },
        //     )
        //     .unwrap();

        assert_eq!(
            collection.call(fc("add", json!([1, 2]))).await.unwrap(),
            json!(3)
        );
        assert_eq!(
            collection
                .call(fc("concat", json!(["hello", "world"])))
                .await
                .unwrap(),
            json!("helloworld")
        );
        assert_eq!(
            collection.call(fc("noop", json!(null))).await.unwrap(),
            json!(null)
        );
        // Complex args test commented out due to ToolSchema derive requirement
        // assert_eq!(
        //     collection
        //         .call(fc("complex_args", json!({ "a": 1, "b": 2 })))
        //         .await
        //         .unwrap(),
        //     json!(null)
        // );
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
            col.call(fc("is_even", json!([4]))).await.unwrap(),
            json!(true)
        );
        assert_eq!(
            col.call(fc("is_even", json!([3]))).await.unwrap(),
            json!(false)
        );
    }

    // Complex return test commented out due to ToolSchema derive requirement
    // #[derive(Serialize, Deserialize, Debug, PartialEq, ToolSchema)]
    // struct Point {
    //     x: i32,
    //     y: i32,
    // }

    // #[tokio::test]
    // async fn test_complex_return() {
    //     let mut col = ToolCollection::default();
    //     col.register(
    //         "create_point",
    //         "Creates a point",
    //         |t: (i32, i32)| async move { Point { x: t.0, y: t.1 } },
    //     )
    //     .unwrap();

    //     assert_eq!(
    //         col.call(fc("create_point", json!([10, 20]))).await.unwrap(),
    //         json!({ "x": 10, "y": 20 })
    //     );
    // }

    #[tokio::test]
    async fn test_invalid_function_name() {
        let mut col = ToolCollection::default();
        col.register("dummy", "does nothing", |_: ()| async {})
            .unwrap();

        let err = col.call(fc("ghost", json!([]))).await.unwrap_err();
        assert!(matches!(err, ToolError::FunctionNotFound { .. }));
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
}

// Performance tests for schema caching (primitive types only)
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_schema_caching_primitives() {
        // Test that primitive type schemas are cached
        let schema1 = String::schema();
        let schema2 = String::schema();

        // Verify they're identical (same JSON content)
        assert_eq!(schema1, schema2);

        // Test multiple primitive types
        let int_schema1 = i32::schema();
        let int_schema2 = i32::schema();
        assert_eq!(int_schema1, int_schema2);

        let bool_schema1 = bool::schema();
        let bool_schema2 = bool::schema();
        assert_eq!(bool_schema1, bool_schema2);
    }

    #[test]
    fn test_schema_performance_primitive() {
        // Warm up the cache
        let _ = String::schema();

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = String::schema();
        }
        let cached_duration = start.elapsed();

        // Cached calls should be very fast (< 10ms for 1000 calls)
        assert!(
            cached_duration.as_millis() < 10,
            "Cached schema calls took too long: {:?}",
            cached_duration
        );
    }

    #[test]
    fn test_schema_performance_multiple_primitives() {
        // Test multiple primitive types for performance
        let _ = f64::schema(); // Warm up

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = f64::schema();
            let _ = u64::schema();
            let _ = bool::schema();
        }
        let cached_duration = start.elapsed();

        // Multiple primitive cached schemas should be very fast
        assert!(
            cached_duration.as_millis() < 20,
            "Cached primitive schema calls took too long: {:?}",
            cached_duration
        );
    }

    #[test]
    fn test_primitive_schema_content_correctness() {
        // Verify primitive schemas have expected structure
        let string_schema = String::schema();
        assert_eq!(string_schema["type"], "string");

        let int_schema = i32::schema();
        assert_eq!(int_schema["type"], "integer");

        let bool_schema = bool::schema();
        assert_eq!(bool_schema["type"], "boolean");

        let null_schema = <()>::schema();
        assert_eq!(null_schema["type"], "null");
    }

    #[test]
    fn test_concurrent_schema_access() {
        use std::thread;

        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    // Each thread gets primitive schemas multiple times
                    for _ in 0..100 {
                        let _ = String::schema();
                        let _ = i32::schema();
                        let _ = bool::schema();
                        let _ = f64::schema();
                    }
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify schema is still correct after concurrent access
        let schema = String::schema();
        assert_eq!(schema["type"], "string");
    }

    #[test]
    fn test_unit_type_caching() {
        // Test unit type caching
        let unit_type_schema1 = <()>::schema();
        let unit_type_schema2 = <()>::schema();
        assert_eq!(unit_type_schema1, unit_type_schema2);
        assert_eq!(unit_type_schema1["type"], "null");
    }

    #[test]
    fn benchmark_primitive_schema_generation() {
        const ITERATIONS: usize = 10_000;

        // Benchmark string type
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = String::schema();
        }
        let string_duration = start.elapsed();

        // Benchmark integer type
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = i32::schema();
        }
        let int_duration = start.elapsed();

        // Benchmark boolean type
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = bool::schema();
        }
        let bool_duration = start.elapsed();

        println!("Primitive schema generation performance (cached):");
        println!("  String ({} calls): {:?}", ITERATIONS, string_duration);
        println!("  Integer ({} calls): {:?}", ITERATIONS, int_duration);
        println!("  Boolean ({} calls): {:?}", ITERATIONS, bool_duration);

        // All should be very fast due to caching
        assert!(string_duration.as_millis() < 100);
        assert!(int_duration.as_millis() < 100);
        assert!(bool_duration.as_millis() < 100);
    }
}
