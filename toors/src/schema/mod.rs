//! Schema definitions for tool declarations
//!
//! This module contains types used to describe tool declarations in a format
//! that can be serialized to JSON and provided to LLMs.

use serde::Serialize;
#[cfg(feature = "schema")]
use serde_json;

/// Minimal, always-available description of a function
#[derive(Debug, Clone, Serialize)]
pub struct FunctionDecl<'a> {
    /// The name of the function
    pub name: &'a str,
    /// A description of what the function does
    pub description: &'a str,
    /// Parameter types for the function
    pub parameters: Vec<TypeName<'a>>,
    /// Return type of the function
    pub returns: TypeName<'a>,
}

/// Stringified Rust type; upgraded to JSON-Schema when the `schema` feature is enabled.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TypeName<'a> {
    /// Basic Rust type name representation
    Rust { rust: &'a str },

    /// JSON Schema representation (available with the `schema` feature)
    #[cfg(feature = "schema")]
    JsonSchema { schema: serde_json::Value },
}

/// Converts a type to its schema representation
#[cfg(not(feature = "schema"))]
pub fn type_to_decl<T: 'static>() -> TypeName<'static> {
    // Default implementation returns the Rust type name
    TypeName::Rust {
        rust: std::any::type_name::<T>(),
    }
}

#[cfg(feature = "schema")]
/// Helper function to check if a type is supported for schema generation
pub fn schema_for_safe<T: 'static>() -> Option<serde_json::Value> {
    // Only handle a few common types directly to avoid issues with JsonSchema
    // In a real implementation, you would add more type handling or use proper JsonSchema

    // This approach avoids requiring T: JsonSchema

    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<String>() {
        let schema = serde_json::json!({
            "type": "string"
        });
        Some(schema)
    } else if TypeId::of::<T>() == TypeId::of::<i32>() {
        let schema = serde_json::json!({
            "type": "integer",
            "format": "int32"
        });
        Some(schema)
    } else if TypeId::of::<T>() == TypeId::of::<f64>() {
        let schema = serde_json::json!({
            "type": "number",
            "format": "double"
        });
        Some(schema)
    } else if TypeId::of::<T>() == TypeId::of::<bool>() {
        let schema = serde_json::json!({
            "type": "boolean"
        });
        Some(schema)
    } else if TypeId::of::<T>() == TypeId::of::<()>() {
        let schema = serde_json::json!({
            "type": "null"
        });
        Some(schema)
    } else if std::any::type_name::<T>().starts_with("(")
        && std::any::type_name::<T>().ends_with(")")
    {
        // Basic tuple handling
        let schema = serde_json::json!({
            "type": "array",
            "description": std::any::type_name::<T>()
        });
        Some(schema)
    } else if std::any::type_name::<T>().starts_with("alloc::vec::Vec<") {
        // Basic vector handling
        let schema = serde_json::json!({
            "type": "array",
            "description": std::any::type_name::<T>()
        });
        Some(schema)
    } else if std::any::type_name::<T>().starts_with("alloc::string::String") {
        // Handle String explicitly
        let schema = serde_json::json!({
            "type": "string"
        });
        Some(schema)
    } else {
        // For other types, just return a generic object schema with the type name
        let schema = serde_json::json!({
            "type": "object",
            "description": std::any::type_name::<T>()
        });
        Some(schema)
    }
}

#[cfg(feature = "schema")]
impl<'a> TypeName<'a> {
    /// Create a new TypeName with schema from a type T
    pub fn from_type<T: 'static>() -> Self {
        if let Some(schema) = schema_for_safe::<T>() {
            TypeName::JsonSchema { schema }
        } else {
            TypeName::Rust {
                rust: std::any::type_name::<T>(),
            }
        }
    }
}

#[cfg(feature = "schema")]
/// Override for type_to_decl when schema feature is enabled
pub fn type_to_decl<T: 'static>() -> TypeName<'static> {
    TypeName::from_type::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_to_decl_primitive() {
        let decl = type_to_decl::<i32>();
        if let TypeName::Rust { rust } = decl {
            assert_eq!(rust, "i32");
        }
        #[cfg(feature = "schema")]
        {
            if let TypeName::JsonSchema { schema } = type_to_decl::<i32>() {
                assert_eq!(schema["type"], "integer");
            }
        }
    }

    #[test]
    fn test_type_to_decl_tuple() {
        let decl = type_to_decl::<(String, i32)>();
        if let TypeName::Rust { rust } = decl {
            assert_eq!(rust, "(alloc::string::String, i32)");
        }
    }
}
