//! Core data models for the toors library
//!
//! This module contains the core data structures used by the toors library.

use std::any::TypeId;
use std::borrow::Cow;
use serde::Deserialize;
use serde_json::Value;
use futures::future::BoxFuture;

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

/// Represents a function call with a name and JSON arguments.
#[derive(Debug, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call
    pub name: String,
    /// The JSON arguments to pass to the function
    pub arguments: Value,
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

/// Tool registration information
#[derive(Debug)]
pub struct ToolRegistration {
    /// Name of the tool
    pub name: &'static str,
    /// Documentation for the tool
    pub doc: &'static str,
    /// The function implementing the tool
    pub f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
}

impl ToolRegistration {
    /// Creates a new tool registration
    pub const fn new(
        name: &'static str,
        doc: &'static str,
        f: fn(Value) -> BoxFuture<'static, Result<Value, ToolError>>,
    ) -> Self {
        Self { name, doc, f }
    }
}