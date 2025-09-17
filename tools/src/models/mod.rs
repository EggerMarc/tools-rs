//! Core data models for the tools-rs library
//!
//! This module contains the core data structures used by the tools-rs library.

use core::fmt;
use futures::future::BoxFuture;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Value, to_string_pretty};
use std::any::TypeId;
use std::borrow::Cow;

use crate::error::ToolError;

/// Metadata about a tool
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    /// Name of the tool
    pub name: String,
    /// Description of the tool
    pub description: String,
    /// Signature information as a string
    pub signature: String,
}

/// Trait for types that can be used as tools
pub trait Tool {
    /// Get the description of the tool
    fn description(&self) -> &'static str;

    /// Get the metadata for the tool
    fn signature(&self) -> ToolMetadata;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallId(uuid::Uuid);

impl CallId {
    pub fn new() -> CallId {
        CallId(uuid::Uuid::new_v4())
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
        Ok(CallId(uuid))
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

/// Represents a function call with a name and JSON arguments.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct FunctionCall {
    /// ID Of the function call
    pub id: Option<CallId>,
    /// The name of the function to call
    pub name: String,
    /// The JSON arguments to pass to the function
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

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct FunctionResponse {
    /// ID of the function call
    pub id: Option<CallId>,
    /// Name of the function
    pub name: String,
    /// JSON Response of the function
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

/// A type representing a tool function
pub type ToolFunc = dyn Fn(Value) -> BoxFuture<'static, Result<Value, ToolError>> + Send + Sync;

/// Type signature information for a tool function
#[derive(Debug)]
pub struct TypeSignature {
    /// Type ID for the input type
    pub input_id: TypeId,
    /// Type ID for the output type
    pub output_id: TypeId,
    /// Name of the input type
    pub input_name: Cow<'static, str>,
    /// Name of the output type
    pub output_name: Cow<'static, str>,
}

pub struct ToolRegistration {
    /// Symbolic tool name
    pub name: &'static str,
    /// Doc-string shown to the LLM
    pub doc: &'static str,
    /// Async wrapper  (JSON in → JSON out)
    pub f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
    /// Zero-arg closure returning the *parameter* JSON-Schema
    pub param_schema: fn() -> Value,
    /// Zero-arg closure returning the *return type* JSON-Schema
    pub return_schema: fn() -> Value,
}

impl ToolRegistration {
    pub const fn new(
        name: &'static str,
        doc: &'static str,
        f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
        param_schema: fn() -> Value,
        return_schema: fn() -> Value,
    ) -> Self {
        Self {
            name,
            doc,
            f,
            param_schema,
            return_schema,
        }
    }
}
