#![deny(unsafe_code)]

use serde_json::Value;
use std::collections::HashMap;

/// Describe yourself as a JSON-Schema v2020-12 document.
pub trait ToolSchema {
    fn schema() -> Value;
}

pub use tool_schema_derive::ToolSchema;

macro_rules! prim {
    ($t:ty, $json_type:expr) => {
        impl ToolSchema for $t {
            fn schema() -> Value {
                serde_json::json!({ "type": $json_type })
            }
        }
    };
}

// Boolean type
prim!(bool, "boolean");

// Integer types
prim!(i8, "integer");
prim!(i16, "integer");
prim!(i32, "integer");
prim!(i64, "integer");
prim!(isize, "integer");
prim!(u8, "integer");
prim!(u16, "integer");
prim!(u32, "integer");
prim!(u64, "integer");
prim!(usize, "integer");
prim!(i128, "integer");
prim!(u128, "integer");

// Floating point types
prim!(f32, "number");
prim!(f64, "number");

// Character type
prim!(char, "string");

// String types
prim!(String, "string");

// &str with any lifetime
impl<'a> ToolSchema for &'a str {
    fn schema() -> Value {
        serde_json::json!({ "type": "string" })
    }
}

// DST string slice
impl ToolSchema for str {
    fn schema() -> Value {
        serde_json::json!({ "type": "string" })
    }
}

// Unit type
impl ToolSchema for () {
    fn schema() -> Value {
        serde_json::json!({ "type": "null" })
    }
}

// Option<T>
impl<T: ToolSchema> ToolSchema for Option<T> {
    fn schema() -> Value {
        serde_json::json!({
            "anyOf": [
                T::schema(),
                { "type": "null" }
            ]
        })
    }
}

// Vec<T>
impl<T: ToolSchema> ToolSchema for Vec<T> {
    fn schema() -> Value {
        serde_json::json!({
            "type": "array",
            "items": T::schema()
        })
    }
}

// HashMap<String, T>
impl<T: ToolSchema> ToolSchema for HashMap<String, T> {
    fn schema() -> Value {
        serde_json::json!({
            "type": "object",
            "additionalProperties": T::schema()
        })
    }
}

// Tuple implementations up to 25 elements
macro_rules! impl_tuples {
    ($($T:ident),+; $len:expr) => {
        impl<$($T: ToolSchema),+> ToolSchema for ($($T,)+) {
            fn schema() -> Value {
                serde_json::json!({
                    "type": "array",
                    "prefixItems": [$($T::schema()),+],
                    "minItems": $len,
                    "maxItems": $len
                })
            }
        }
    };
}

impl_tuples!(A; 1);
impl_tuples!(A, B; 2);
impl_tuples!(A, B, C; 3);
impl_tuples!(A, B, C, D; 4);
impl_tuples!(A, B, C, D, E; 5);
impl_tuples!(A, B, C, D, E, F; 6);
impl_tuples!(A, B, C, D, E, F, G; 7);
impl_tuples!(A, B, C, D, E, F, G, H; 8);
impl_tuples!(A, B, C, D, E, F, G, H, I; 9);
impl_tuples!(A, B, C, D, E, F, G, H, I, J; 10);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K; 11);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L; 12);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M; 13);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N; 14);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O; 15);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P; 16);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q; 17);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R; 18);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S; 19);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T; 20);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U; 21);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V; 22);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W; 23);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X; 24);
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y; 25);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_schemas() {
        assert_eq!(bool::schema(), serde_json::json!({ "type": "boolean" }));
        assert_eq!(i32::schema(), serde_json::json!({ "type": "integer" }));
        assert_eq!(f64::schema(), serde_json::json!({ "type": "number" }));
        assert_eq!(String::schema(), serde_json::json!({ "type": "string" }));
        assert_eq!(<&str>::schema(), serde_json::json!({ "type": "string" }));
        assert_eq!(<()>::schema(), serde_json::json!({ "type": "null" }));
    }

    #[test]
    fn test_option_schema() {
        let expected = serde_json::json!({
            "anyOf": [
                { "type": "string" },
                { "type": "null" }
            ]
        });
        assert_eq!(<Option<String>>::schema(), expected);
    }

    #[test]
    fn test_vec_schema() {
        let expected = serde_json::json!({
            "type": "array",
            "items": { "type": "integer" }
        });
        assert_eq!(<Vec<i32>>::schema(), expected);
    }

    #[test]
    fn test_tuple_schemas() {
        let single_tuple = serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }],
            "minItems": 1,
            "maxItems": 1
        });
        assert_eq!(<(String,)>::schema(), single_tuple);

        let pair_tuple = serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }, { "type": "boolean" }],
            "minItems": 2,
            "maxItems": 2
        });
        assert_eq!(<(i32, bool)>::schema(), pair_tuple);
    }

    #[test]
    fn test_extended_tuple_schemas() {
        // Test 4-element tuple
        let quad_tuple = serde_json::json!({
            "type": "array",
            "prefixItems": [
                { "type": "string" },
                { "type": "integer" },
                { "type": "boolean" },
                { "type": "number" }
            ],
            "minItems": 4,
            "maxItems": 4
        });
        assert_eq!(<(String, i32, bool, f64)>::schema(), quad_tuple);

        // Test 8-element tuple
        let octet_tuple = serde_json::json!({
            "type": "array",
            "prefixItems": [
                { "type": "string" },
                { "type": "integer" },
                { "type": "boolean" },
                { "type": "number" },
                { "type": "string" },
                { "type": "integer" },
                { "type": "boolean" },
                { "type": "number" }
            ],
            "minItems": 8,
            "maxItems": 8
        });
        assert_eq!(
            <(String, i32, bool, f64, String, i32, bool, f64)>::schema(),
            octet_tuple
        );

        // Test 16-element tuple (maximum supported)
        let max_tuple = serde_json::json!({
            "type": "array",
            "prefixItems": [
                { "type": "string" }, { "type": "integer" }, { "type": "boolean" }, { "type": "number" },
                { "type": "string" }, { "type": "integer" }, { "type": "boolean" }, { "type": "number" },
                { "type": "string" }, { "type": "integer" }, { "type": "boolean" }, { "type": "number" },
                { "type": "string" }, { "type": "integer" }, { "type": "boolean" }, { "type": "number" }
            ],
            "minItems": 16,
            "maxItems": 16
        });
        assert_eq!(
            <(
                String,
                i32,
                bool,
                f64,
                String,
                i32,
                bool,
                f64,
                String,
                i32,
                bool,
                f64,
                String,
                i32,
                bool,
                f64
            )>::schema(),
            max_tuple
        );
    }

    #[test]
    fn test_missing_primitives() {
        assert_eq!(i128::schema(), serde_json::json!({ "type": "integer" }));
        assert_eq!(u128::schema(), serde_json::json!({ "type": "integer" }));
        assert_eq!(char::schema(), serde_json::json!({ "type": "string" }));
        assert_eq!(<str>::schema(), serde_json::json!({ "type": "string" }));
    }

    #[test]
    fn test_hashmap_schema() {
        let expected = serde_json::json!({
            "type": "object",
            "additionalProperties": { "type": "integer" }
        });
        assert_eq!(<HashMap<String, i32>>::schema(), expected);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Newtype Pattern Support
// ────────────────────────────────────────────────────────────────────────────
//
// The newtype pattern is a powerful Rust idiom for creating type-safe wrappers
// around primitive types. This crate fully supports newtypes through the
// ToolSchema derive macro, enabling more descriptive and safer APIs.
//
// ## What is the Newtype Pattern?
//
// A newtype is a tuple struct with a single field:
//
// ```rust
// #[derive(ToolSchema)]
// struct UserId(u64);
//
// #[derive(ToolSchema)]
// struct EmailAddress(String);
// ```
//
// ## Benefits for Tool APIs
//
// 1. **Type Safety**: Prevents mixing up similar primitive types
//    ```rust
//    fn transfer(from: AccountId, to: AccountId, amount: UsdCents) // Clear!
//    fn transfer(from: u64, to: u64, amount: u64)                // Confusing!
//    ```
//
// 2. **Self-Documenting**: Parameter types convey semantic meaning
//    ```rust
//    struct Location {
//        lat: Latitude,   // Unambiguous
//        lng: Longitude,  // No confusion about order
//    }
//    ```
//
// 3. **Compile-Time Validation**: Catch errors at build time
//    ```rust
//    let user_id = UserId(123);
//    let account_id = AccountId(456);
//    transfer(user_id, account_id, amount); // Compile error!
//    ```
//
// ## JSON Schema Generation
//
// Newtypes generate array schemas with single items:
//
// ```json
// {
//   "type": "array",
//   "prefixItems": [{"type": "integer"}],
//   "minItems": 1,
//   "maxItems": 1
// }
// ```
//
// This preserves type safety while remaining JSON-serializable and
// distinguishable from primitive types in the schema.
//
// ## Best Practices
//
// **Good**: Use newtypes for domain-specific concepts
// ```rust
// #[derive(ToolSchema)]
// struct CustomerId(u64);
//
// #[derive(ToolSchema)]
// struct ProductPrice(u64); // Price in cents
// ```
//
// **Bad**: Don't overuse for simple primitives
// ```rust
// struct Count(i32);    // Probably unnecessary
// struct Name(String);  // String is already descriptive
// ```
//
// See the examples/newtype_demo for a complete working example.

#[cfg(test)]
mod newtype_tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tool_schema_derive::ToolSchema;

    // Example newtypes for better parameter descriptions
    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct UserId(u64);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct Email(String);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct Temperature(f32);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct Count(i32);

    #[test]
    fn test_newtype_schemas() {
        // Newtypes should generate array schemas with single items
        // This makes them distinguishable from primitive types while
        // maintaining type safety

        let user_id_schema = UserId::schema();
        let expected_user_id = serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }],
            "minItems": 1,
            "maxItems": 1
        });
        assert_eq!(user_id_schema, expected_user_id);

        let email_schema = Email::schema();
        let expected_email = serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }],
            "minItems": 1,
            "maxItems": 1
        });
        assert_eq!(email_schema, expected_email);

        let temp_schema = Temperature::schema();
        let expected_temp = serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "number" }],
            "minItems": 1,
            "maxItems": 1
        });
        assert_eq!(temp_schema, expected_temp);
    }

    // Example of using newtypes in a more complex structure
    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct UserProfile {
        id: UserId,
        email: Email,
        name: String,
        age: Option<Count>,
    }

    #[test]
    fn test_newtype_in_struct() {
        let profile_schema = UserProfile::schema();

        // Verify the struct contains our newtype fields
        assert!(profile_schema["properties"]["id"].is_object());
        assert!(profile_schema["properties"]["email"].is_object());
        assert_eq!(profile_schema["properties"]["name"]["type"], "string");

        // The newtype fields should have array schemas
        assert_eq!(profile_schema["properties"]["id"]["type"], "array");
        assert_eq!(profile_schema["properties"]["email"]["type"], "array");
    }

    // Example: Function with unclear parameters (before newtypes)
    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct TransferRequest {
        from_account: u64, // Unclear: is this an ID, account number, or something else?
        to_account: u64,   // Same issue here
        amount: f64,       // In what currency? What precision?
        fee: f64,          // Same currency as amount?
    }

    // Example: Function with descriptive parameters (after newtypes)
    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct AccountId(u64);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct CurrencyAmount(f64);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct TransactionFee(f64);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct ImprovedTransferRequest {
        from_account: AccountId, // Clear: this is an account identifier
        to_account: AccountId,   // Clear: this is also an account identifier
        amount: CurrencyAmount,  // Clear: this is a monetary amount
        fee: TransactionFee,     // Clear: this is specifically a transaction fee
    }

    #[test]
    fn test_descriptive_vs_unclear_parameters() {
        let unclear_schema = TransferRequest::schema();
        let clear_schema = ImprovedTransferRequest::schema();

        // Both should be valid schemas
        assert_eq!(unclear_schema["type"], "object");
        assert_eq!(clear_schema["type"], "object");

        // The unclear version uses primitive types directly
        assert_eq!(
            unclear_schema["properties"]["from_account"]["type"],
            "integer"
        );
        assert_eq!(unclear_schema["properties"]["amount"]["type"], "number");

        // The clear version uses newtype wrappers (array schemas)
        assert_eq!(clear_schema["properties"]["from_account"]["type"], "array");
        assert_eq!(clear_schema["properties"]["amount"]["type"], "array");

        // Newtypes provide type safety at compile time while maintaining
        // clear semantic meaning in the API
    }

    // Example: Geographic coordinates with and without newtypes
    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct Latitude(f64);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct Longitude(f64);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct UnclearLocation {
        x: f64, // Is this latitude or longitude? What's the order?
        y: f64, // Same confusion here
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct ClearLocation {
        lat: Latitude,  // Unambiguous: this is latitude
        lng: Longitude, // Unambiguous: this is longitude
    }

    #[test]
    fn test_geographic_newtype_clarity() {
        let unclear_loc = UnclearLocation::schema();
        let clear_loc = ClearLocation::schema();

        // Both generate valid schemas
        assert!(unclear_loc["properties"]["x"]["type"] == "number");
        assert!(unclear_loc["properties"]["y"]["type"] == "number");

        assert!(clear_loc["properties"]["lat"]["type"] == "array");
        assert!(clear_loc["properties"]["lng"]["type"] == "array");

        // The newtype version prevents bugs like accidentally swapping lat/lng
        // and makes the API self-documenting
    }
}

// Additional examples showing how newtypes improve API design
#[cfg(test)]
mod newtype_api_examples {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tool_schema_derive::ToolSchema;

    // Domain-specific newtypes for a booking system
    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct BookingId(String);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct CustomerId(u64);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct RoomNumber(String);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct NightCount(u32);

    #[derive(Debug, Clone, Serialize, Deserialize, ToolSchema)]
    struct BookingRequest {
        customer: CustomerId, // Type-safe: can't accidentally pass a booking ID
        room: RoomNumber,     // Clear: this is a room identifier, not a customer ID
        nights: NightCount,   // Semantic: number of nights, not rooms or customers
    }

    #[test]
    fn test_booking_system_newtypes() {
        let schema = BookingRequest::schema();

        // Verify the schema structure
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["customer"].is_object());
        assert!(schema["properties"]["room"].is_object());
        assert!(schema["properties"]["nights"].is_object());

        // All newtype fields should generate array schemas
        assert_eq!(schema["properties"]["customer"]["type"], "array");
        assert_eq!(schema["properties"]["room"]["type"], "array");
        assert_eq!(schema["properties"]["nights"]["type"], "array");
    }
}
