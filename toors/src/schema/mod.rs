use serde::Serialize;
use serde_json::Value;

#[cfg(feature = "schema")]
use schemars::{JsonSchema, schema_for};

/// Return the JSON-Schema of `T` as a plain `serde_json::Value`.
///
/// When the `schema` feature is **disabled**, this always returns `null`.
#[inline]
#[cfg(feature = "schema")]
pub fn schema_to_json_schema<T>() -> Value
where
    T: JsonSchema + ?Sized,
{
    let root = schema_for!(T); // schemars ⇒ RootSchema
    serde_json::to_value(root.schema).expect("serialising RootSchema never fails")
}

#[inline]
#[cfg(not(feature = "schema"))]
pub fn schema_to_json_schema<T>() -> Value {
    Value::Null
}

/// `FunctionDecl` – metadata emitted by the runtime for each registered tool.
#[derive(Debug, Clone, Serialize)]
pub struct FunctionDecl<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub parameters: Value,
    pub returns: Value,
}
