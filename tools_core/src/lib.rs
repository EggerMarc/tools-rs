#![deny(unsafe_code)]

pub mod builder;
pub use builder::ToolsBuilder;

use core::fmt;
use std::{
    any::{Any, TypeId},
    borrow::Cow,
    collections::HashMap,
    sync::Arc,
};

use futures::{FutureExt, future::BoxFuture};
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use serde_json::{self, Value, to_string_pretty};

// Re-export once_cell
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

    #[error("tool `{tool}` has attributes that do not match the metadata schema: {error}")]
    BadMeta {
        tool: &'static str,
        error: String,
    },

    #[error("validation failed for {} tool(s): {summary}", .errors.len())]
    MetaValidation {
        errors: Vec<MetaValidationError>,
        summary: String,
    },

    #[error("tool `{tool}` requires context but none was provided")]
    MissingCtx { tool: &'static str },

    #[error("tool `{tool}` expects context type `{expected}` but collection has `{got}`")]
    CtxTypeMismatch {
        tool: &'static str,
        expected: String,
        got: String,
    },
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
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct FunctionCall {
    pub id: Option<CallId>,
    pub name: String,
    pub arguments: Value,
}

impl FunctionCall {
    pub fn new(name: String, arguments: Value) -> FunctionCall {
        FunctionCall {
            id: Some(CallId::new()),
            name,
            arguments,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallId(String);

impl CallId {
    pub fn new() -> CallId {
        CallId(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for CallId {
    fn default() -> Self {
        CallId::new()
    }
}

impl<'de> Deserialize<'de> for CallId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let uuid = uuid::Uuid::parse_str(&s).map_err(serde::de::Error::custom)?;
        Ok(CallId(uuid.to_string()))
    }
}

impl Serialize for CallId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl fmt::Display for CallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CallId> for String {
    fn from(id: CallId) -> Self {
        id.0.to_string()
    }
}

impl From<String> for CallId {
    fn from(id: String) -> Self {
        CallId(id)
    }
}

/// Represents a function response with name and arguments
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct FunctionResponse {
    pub id: Option<CallId>,
    pub name: String,
    pub result: Value,
}

impl fmt::Display for FunctionResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id_str = self
            .id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "<none>".to_string());

        let pretty_result =
            to_string_pretty(&self.result).unwrap_or_else(|_| "<invalid json>".to_string());

        write!(
            f,
            "FunctionResponse {{\n  id: {},\n  name: \"{}\",\n  result: {}\n}}",
            id_str,
            self.name,
            pretty_result.replace("\n", "\n  ") // indent JSON
        )
    }
}

/// Function signature for tools
pub type ToolFunc = dyn Fn(Value, Option<Arc<dyn Any + Send + Sync>>)
    -> BoxFuture<'static, Result<Value, ToolError>>
    + Send
    + Sync;

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

/// Default metadata type for [`ToolCollection`]. Empty struct that
/// deserializes from any JSON object, ignoring all fields. Use this when
/// you don't care about per-tool attributes.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoMeta {}

/// Helper trait that lets [`ToolCollection::register`] accept either a real
/// `M` or `()` when `M = NoMeta`. Passing `()` to a typed collection fails
/// at compile time — there's no silent default.
pub trait MetaArg<M> {
    fn into_meta(self) -> M;
}

impl<M> MetaArg<M> for M {
    fn into_meta(self) -> M {
        self
    }
}

impl MetaArg<NoMeta> for () {
    fn into_meta(self) -> NoMeta {
        NoMeta {}
    }
}

/// Tool registration for inventory collection. Constructed via struct
/// literal in macro-generated code; field additions are minor-version
/// breaking changes.
pub struct ToolRegistration {
    pub name: &'static str,
    pub doc: &'static str,
    pub f: fn(
        Value,
        Option<Arc<dyn Any + Send + Sync>>,
    ) -> BoxFuture<'static, Result<Value, ToolError>>,
    pub param_schema: fn() -> Value,
    /// JSON object literal of the attributes declared in `#[tool(...)]`.
    /// `"{}"` when no attributes were given. Deserialized into the
    /// collection's `M` at [`ToolCollection::collect_tools`] time.
    pub meta_json: &'static str,
    /// `true` when the tool's first parameter is named `ctx`.
    pub needs_ctx: bool,
    /// Returns the [`TypeId`] of the expected context type `T` (the inner
    /// type of `Arc<T>`). `None` when `needs_ctx` is `false`.
    pub ctx_type_id: Option<fn() -> TypeId>,
    /// Human-readable name of the expected context type, for error
    /// messages. Empty string when `needs_ctx` is `false`.
    pub ctx_type_name: &'static str,
}

/// Per-tool attribute validation error. Reported by
/// [`validate_tool_attrs`] and [`validate_tool_attrs_for`].
#[derive(Debug, Clone)]
pub struct MetaValidationError {
    pub tool: Cow<'static, str>,
    pub error: String,
}

impl fmt::Display for MetaValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tool `{}`: {}", self.tool, self.error)
    }
}

impl std::error::Error for MetaValidationError {}

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

/// One entry in a [`ToolCollection`]: callable function, schema, and the
/// metadata typed against the collection's `M` parameter.
pub struct ToolEntry<M> {
    pub func: Arc<ToolFunc>,
    pub decl: FunctionDecl<'static>,
    pub meta: M,
}

impl<M: Clone> Clone for ToolEntry<M> {
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            decl: self.decl.clone(),
            meta: self.meta.clone(),
        }
    }
}

/// Collection of registered tools, parameterized by a metadata type `M`.
///
/// `M` defaults to [`NoMeta`] — an empty struct that swallows any
/// `#[tool(...)]` attributes a tool declared. Opt into typed metadata by
/// setting `M` explicitly:
///
/// ```ignore
/// let tools = ToolCollection::<MyPolicy>::collect_tools()?;
/// if tools.meta("delete_file").unwrap().requires_approval { ... }
/// ```
pub struct ToolCollection<M = NoMeta> {
    entries: HashMap<&'static str, ToolEntry<M>>,
    ctx: Option<Arc<dyn Any + Send + Sync>>,
}

impl<M> Default for ToolCollection<M> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            ctx: None,
        }
    }
}

impl<M: Clone> Clone for ToolCollection<M> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            ctx: self.ctx.clone(),
        }
    }
}

impl<M> ToolCollection<M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a [`CollectionBuilder`] for constructing a collection with
    /// shared context and/or custom configuration.
    pub fn builder() -> CollectionBuilder<M> {
        CollectionBuilder {
            ctx: None,
            ctx_type_id: None,
            ctx_type_name: "",
            _meta: std::marker::PhantomData,
        }
    }

    /// Register a tool from a pre-built JSON schema and a raw async closure.
    ///
    /// Unlike [`register`][Self::register], this bypasses `ToolSchema`
    /// derivation — the caller supplies the JSON schema directly. The
    /// closure receives only the JSON arguments (no context). This is the
    /// foundation for FFI adapters that register tools from scripting
    /// languages.
    ///
    /// Pass `()` as `meta` for `ToolCollection<NoMeta>`; pass an `M` for
    /// typed collections.
    pub fn register_raw<A: MetaArg<M>>(
        &mut self,
        name: &'static str,
        description: &'static str,
        parameters: Value,
        func: impl Fn(Value) -> BoxFuture<'static, Result<Value, ToolError>> + Send + Sync + 'static,
        meta: A,
    ) -> Result<&mut Self, ToolError> {
        if self.entries.contains_key(name) {
            return Err(ToolError::AlreadyRegistered { name });
        }

        let boxed: Arc<ToolFunc> = Arc::new(
            move |raw: Value, _ctx: Option<Arc<dyn Any + Send + Sync>>| func(raw),
        );

        self.entries.insert(
            name,
            ToolEntry {
                func: boxed,
                decl: FunctionDecl::new(name, description, parameters),
                meta: meta.into_meta(),
            },
        );

        Ok(self)
    }

    /// Register a tool programmatically. Pass `()` as `meta` for
    /// `ToolCollection<NoMeta>`; pass an `M` for typed collections.
    /// Passing `()` to a typed collection is a compile error.
    pub fn register<A, I, O, F, Fut>(
        &mut self,
        name: &'static str,
        desc: &'static str,
        func: F,
        meta: A,
    ) -> Result<&mut Self, ToolError>
    where
        A: MetaArg<M>,
        I: 'static + DeserializeOwned + Serialize + Send + ToolSchema,
        O: 'static + Serialize + Send + ToolSchema,
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = O> + Send + 'static,
    {
        if self.entries.contains_key(name) {
            return Err(ToolError::AlreadyRegistered { name });
        }

        let func_arc: Arc<F> = Arc::new(func);
        let boxed: Arc<ToolFunc> = Arc::new(
            move |raw: Value,
                  _ctx: Option<Arc<dyn Any + Send + Sync>>|
                  -> BoxFuture<'static, Result<Value, ToolError>> {
                let func = func_arc.clone();
                async move {
                    let input: I =
                        serde_json::from_value(raw).map_err(DeserializationError::from)?;
                    let output: O = (func)(input).await;
                    serde_json::to_value(output).map_err(|e| ToolError::Runtime(e.to_string()))
                }
                .boxed()
            },
        );

        self.entries.insert(
            name,
            ToolEntry {
                func: boxed,
                decl: FunctionDecl::new(name, desc, schema_value::<I>()?),
                meta: meta.into_meta(),
            },
        );

        Ok(self)
    }

    pub async fn call(&self, call: FunctionCall) -> Result<FunctionResponse, ToolError> {
        let FunctionCall {
            id,
            name,
            arguments,
        } = call;
        let entry = self
            .entries
            .get(name.as_str())
            .ok_or(ToolError::FunctionNotFound {
                name: Cow::Owned(name.clone()),
            })?;

        let result = (entry.func)(arguments, self.ctx.clone()).await?;
        Ok(FunctionResponse { id, name, result })
    }

    pub fn unregister(&mut self, name: &str) -> Result<(), ToolError> {
        if self.entries.remove(name).is_none() {
            return Err(ToolError::FunctionNotFound {
                name: Cow::Owned(name.to_string()),
            });
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&ToolEntry<M>> {
        self.entries.get(name)
    }

    pub fn meta(&self, name: &str) -> Option<&M> {
        self.entries.get(name).map(|e| &e.meta)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &ToolEntry<M>)> + '_ {
        self.entries.iter().map(|(k, v)| (*k, v))
    }

    pub fn descriptions(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.entries.iter().map(|(k, v)| (*k, v.decl.description))
    }

    pub fn json(&self) -> Result<Value, ToolError> {
        let list: Vec<&FunctionDecl> = self.entries.values().map(|e| &e.decl).collect();
        Ok(serde_json::to_value(list)?)
    }
}

impl<M: DeserializeOwned> ToolCollection<M> {
    /// Collect every tool registered via `#[tool]`. Fails fast on the first
    /// tool whose `meta_json` blob does not deserialize into `M`.
    ///
    /// For accumulated, CI-friendly validation use [`validate_tool_attrs`].
    pub fn collect_tools() -> Result<Self, ToolError> {
        collect_inventory_inner(None, None, "")
    }
}

/// Validate every registered tool's `#[tool(...)]` attributes against `M`,
/// accumulating all failures. Use in CI tests to catch attribute typos
/// before they hit `collect_tools` at runtime.
pub fn validate_tool_attrs<M: DeserializeOwned>() -> Result<(), Vec<MetaValidationError>> {
    let mut errors = Vec::new();
    for reg in inventory::iter::<ToolRegistration> {
        if let Err(e) = serde_json::from_str::<M>(reg.meta_json) {
            errors.push(MetaValidationError {
                tool: Cow::Borrowed(reg.name),
                error: e.to_string(),
            });
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Like [`validate_tool_attrs`] but only checks the named subset. Useful
/// for workspaces with multiple metadata schemas, each gating a different
/// group of tools. Returns an error for any name that does not match a
/// registered tool — typos in the test list are as bad as typos in the
/// attributes.
pub fn validate_tool_attrs_for<M: DeserializeOwned>(
    names: &[&str],
) -> Result<(), Vec<MetaValidationError>> {
    use std::collections::HashSet;
    let wanted: HashSet<&str> = names.iter().copied().collect();
    let mut found: HashSet<&str> = HashSet::new();
    let mut errors = Vec::new();

    for reg in inventory::iter::<ToolRegistration> {
        if !wanted.contains(reg.name) {
            continue;
        }
        found.insert(reg.name);
        if let Err(e) = serde_json::from_str::<M>(reg.meta_json) {
            errors.push(MetaValidationError {
                tool: Cow::Borrowed(reg.name),
                error: e.to_string(),
            });
        }
    }

    for missing in wanted.difference(&found) {
        errors.push(MetaValidationError {
            tool: Cow::Owned((*missing).to_string()),
            error: "no tool with this name is registered".to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

inventory::collect!(ToolRegistration);

// ============================================================================
// SHARED INVENTORY HELPER
// ============================================================================

/// Shared logic for collecting tools from the global `inventory`. Used by
/// both [`CollectionBuilder::collect`] and [`ToolsBuilder::collect`].
pub(crate) fn collect_inventory_inner<M: DeserializeOwned>(
    ctx: Option<Arc<dyn Any + Send + Sync>>,
    ctx_type_id: Option<TypeId>,
    ctx_type_name: &str,
) -> Result<ToolCollection<M>, ToolError> {
    let mut entries = HashMap::new();

    for reg in inventory::iter::<ToolRegistration> {
        if reg.needs_ctx {
            let Some(provided_id) = ctx_type_id else {
                return Err(ToolError::MissingCtx { tool: reg.name });
            };
            let expected_id = (reg.ctx_type_id.unwrap())();
            if expected_id != provided_id {
                return Err(ToolError::CtxTypeMismatch {
                    tool: reg.name,
                    expected: reg.ctx_type_name.to_string(),
                    got: ctx_type_name.to_string(),
                });
            }
        }

        let meta: M = serde_json::from_str(reg.meta_json).map_err(|e| ToolError::BadMeta {
            tool: reg.name,
            error: e.to_string(),
        })?;

        entries.insert(
            reg.name,
            ToolEntry {
                func: Arc::new(reg.f),
                decl: FunctionDecl::new(reg.name, reg.doc, (reg.param_schema)()),
                meta,
            },
        );
    }

    Ok(ToolCollection { entries, ctx })
}

// ============================================================================
// COLLECTION BUILDER
// ============================================================================

/// Builder for [`ToolCollection`] with support for shared context
/// injection. Construct via [`ToolCollection::builder()`].
///
/// ```ignore
/// let ctx = Arc::new(MyState::new());
/// let tools = ToolCollection::<Policy>::builder()
///     .with_context(ctx)
///     .collect()?;
/// ```
pub struct CollectionBuilder<M = NoMeta> {
    ctx: Option<Arc<dyn Any + Send + Sync>>,
    ctx_type_id: Option<TypeId>,
    ctx_type_name: &'static str,
    _meta: std::marker::PhantomData<M>,
}

impl<M> CollectionBuilder<M> {
    /// Attach a shared context that will be injected into every tool whose
    /// first parameter is named `ctx`. The context type `T` must match what
    /// the tools expect — a mismatch is caught at [`collect()`][Self::collect]
    /// time with a clear error.
    pub fn with_context<T: Send + Sync + 'static>(mut self, ctx: Arc<T>) -> Self {
        self.ctx_type_id = Some(TypeId::of::<T>());
        self.ctx_type_name = std::any::type_name::<T>();
        self.ctx = Some(ctx);
        self
    }
}

impl<M: DeserializeOwned> CollectionBuilder<M> {
    /// Build the collection from the global tool inventory. Validates:
    ///
    /// - Every tool's `meta_json` deserializes into `M`.
    /// - Every `needs_ctx` tool's expected `TypeId` matches the builder's.
    /// - No `needs_ctx` tool exists when no context was provided.
    pub fn collect(self) -> Result<ToolCollection<M>, ToolError> {
        collect_inventory_inner(self.ctx, self.ctx_type_id, self.ctx_type_name)
    }
}

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
        FunctionCall::new(name.to_string(), args)
    }

    #[tokio::test]
    async fn test_collection() {
        let mut collection: ToolCollection = ToolCollection::default();

        collection
            .register(
                "add",
                "Adds two values",
                |t: (i32, i32)| async move { add(t.0, t.1) },
                (),
            )
            .unwrap();
        collection
            .register(
                "concat",
                "Concatenates two strings",
                |t: (String, String)| async move { concat(t.0, t.1) },
                (),
            )
            .unwrap();
        collection
            .register(
                "noop",
                "Does nothing",
                |_t: ()| async move { noop() },
                (),
            )
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
        let mut col: ToolCollection = ToolCollection::default();
        col.register(
            "is_even",
            "Checks even",
            |t: (i32,)| async move { t.0 % 2 == 0 },
            (),
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

    // Complex return test commented out due to ToolSchema derive requirement
    // #[derive(Serialize, Deserialize, Debug, PartialEq, ToolSchema)]
    // struct Point {
    //     x: i32,
    //     y: i32,
    // }

    // #[tokio::test]
    // async fn test_complex_return() {
    //     let mut col: ToolCollection = ToolCollection::default();
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
        let mut col: ToolCollection = ToolCollection::default();
        col.register("dummy", "does nothing", |_: ()| async {}, ())
            .unwrap();

        let err = col.call(fc("ghost", json!([]))).await.unwrap_err();
        assert!(matches!(err, ToolError::FunctionNotFound { .. }));
    }

    #[tokio::test]
    async fn test_deserialization_error() {
        let mut col: ToolCollection = ToolCollection::default();
        col.register(
            "subtract",
            "Sub two numbers",
            |t: (i32, i32)| async move { t.0 - t.1 },
            (),
        )
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
