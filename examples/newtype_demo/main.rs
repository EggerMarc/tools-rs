//! # Newtype Demo: Making Function Parameters More Descriptive
//!
//! This example demonstrates how to use Rust's newtype pattern to create
//! more descriptive and type-safe function parameters in tools-rs.
//!
//! Newtypes provide:
//! - **Type safety**: Prevent mixing up similar primitive types
//! - **Self-documenting APIs**: Parameter names become part of the type system
//! - **Clear intent**: No ambiguity about what each parameter represents

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use tools_rs::{FunctionCall, collect_tools, function_declarations, tool, ToolSchema};

// ────────────────────────────────────────────────────────────────────────────
// Newtype Definitions: Making primitives meaningful
// ────────────────────────────────────────────────────────────────────────────

/// Customer identifier - prevents mixing with other ID types
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct CustomerId(u64);

/// Hotel room number - could be alphanumeric
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct RoomNumber(String);

/// Number of nights for a booking
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct NightCount(u32);

/// Monetary amount in USD cents (to avoid floating point issues)
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct UsdCents(u64);

/// Geographic latitude in decimal degrees
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct Latitude(f64);

/// Geographic longitude in decimal degrees
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct Longitude(f64);

/// Email address as a validated string
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct EmailAddress(String);

/// Account identifier for financial operations
#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct AccountId(String);

// ────────────────────────────────────────────────────────────────────────────
// Complex Types: Combining newtypes for rich APIs
// ────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct BookingRequest {
    customer_id: CustomerId,
    room_number: RoomNumber,
    nights: NightCount,
}

#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct PaymentInfo {
    from_account: AccountId,
    to_account: AccountId,
    amount: UsdCents,
}

#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct LocationCoordinates {
    latitude: Latitude,
    longitude: Longitude,
}

#[derive(Serialize, Deserialize, Debug, ToolSchema)]
struct BookingConfirmation {
    booking_id: String,
    total_cost: UsdCents,
    customer_email: EmailAddress,
}

// ────────────────────────────────────────────────────────────────────────────
// Tool Functions: Clear, type-safe APIs
// ────────────────────────────────────────────────────────────────────────────

#[tool]
/// Create a hotel booking with type-safe parameters.
/// 
/// This function demonstrates how newtypes make the API self-documenting:
/// - `customer_id`: Clearly a customer identifier, not a room or booking ID
/// - `room_number`: Obviously a room identifier, not a customer ID  
/// - `nights`: Unambiguously the number of nights, not rooms or people
async fn create_booking(request: BookingRequest) -> BookingConfirmation {
    let rate_per_night = UsdCents(12500); // $125.00 per night
    let total_cost = UsdCents(request.nights.0 as u64 * rate_per_night.0);
    
    BookingConfirmation {
        booking_id: format!("BK-{}-{}", request.customer_id.0, request.room_number.0),
        total_cost,
        customer_email: EmailAddress(format!("customer{}@example.com", request.customer_id.0)),
    }
}

#[tool]
/// Process a payment between accounts with clear parameter types.
///
/// The newtype pattern prevents common bugs like:
/// - Swapping from_account and to_account
/// - Accidentally passing a customer ID as an account ID
/// - Mixing up amount and fee parameters
async fn process_payment(payment: PaymentInfo) -> String {
    format!(
        "Transferred ${:.2} from account {} to account {}",
        payment.amount.0 as f64 / 100.0,
        payment.from_account.0,
        payment.to_account.0
    )
}

#[tool]
/// Find nearby hotels using precise geographic coordinates.
///
/// Newtypes prevent the classic lat/lng swap bug and make it clear
/// which parameter is which. No more guessing "is X latitude or longitude?"
async fn find_nearby_hotels(location: LocationCoordinates) -> Vec<String> {
    vec![
        format!("Hotel A (0.5 km from {}, {})", location.latitude.0, location.longitude.0),
        format!("Hotel B (1.2 km from {}, {})", location.latitude.0, location.longitude.0),
        format!("Hotel C (2.1 km from {}, {})", location.latitude.0, location.longitude.0),
    ]
}

// ────────────────────────────────────────────────────────────────────────────
// Demonstration: Compare with unsafe primitives
// ────────────────────────────────────────────────────────────────────────────

#[tool]
/// Example of unclear parameters (what NOT to do).
/// 
/// This function has ambiguous parameters that could easily be mixed up:
/// - What do the u64 values represent?
/// - Is the f64 amount in dollars, cents, or another currency?
/// - Which coordinate is latitude vs longitude?
async fn unclear_booking(customer: u64, room: String, nights: u32, lat: f64, lng: f64) -> String {
    format!("Booked room {} for customer {} for {} nights near {}, {}", 
            room, customer, nights, lat, lng)
}

// ────────────────────────────────────────────────────────────────────────────
// Main: Demonstrate the difference
// ────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Newtype Demo: Descriptive Function Parameters ===\n");
    
    // Show the function declarations
    let tools = collect_tools();
    let declarations: JsonValue = function_declarations()?;
    
    println!("📋 Function Declarations with Newtype Parameters:");
    println!("{}\n", serde_json::to_string_pretty(&declarations)?);
    
    // ───────── Demonstrate type-safe bookings ─────────
    println!("🏨 Creating a type-safe booking...");
    let booking_request = BookingRequest {
        customer_id: CustomerId(12345),
        room_number: RoomNumber("A101".to_string()),
        nights: NightCount(3),
    };
    
    let booking_result = tools.call(FunctionCall {
        name: "create_booking".to_string(),
        arguments: json!({ "request": booking_request }),
    }).await?;
    
    println!("✅ Booking result: {}\n", booking_result);
    
    // ───────── Demonstrate type-safe payments ─────────
    println!("💳 Processing a type-safe payment...");
    let payment_info = PaymentInfo {
        from_account: AccountId("ACC-789".to_string()),
        to_account: AccountId("ACC-456".to_string()),
        amount: UsdCents(37500), // $375.00
    };
    
    let payment_result = tools.call(FunctionCall {
        name: "process_payment".to_string(),
        arguments: json!({ "payment": payment_info }),
    }).await?;
    
    println!("✅ Payment result: {}\n", payment_result);
    
    // ───────── Demonstrate type-safe coordinates ─────────
    println!("📍 Finding hotels with precise coordinates...");
    let location = LocationCoordinates {
        latitude: Latitude(40.7128),   // New York City latitude
        longitude: Longitude(-74.0060), // New York City longitude
    };
    
    let hotels_result = tools.call(FunctionCall {
        name: "find_nearby_hotels".to_string(),
        arguments: json!({ "location": location }),
    }).await?;
    
    println!("✅ Nearby hotels: {}\n", hotels_result);
    
    // ───────── Show the difference with unclear parameters ─────────
    println!("⚠️  Compare with unclear parameters (error-prone):");
    let unclear_result = tools.call(FunctionCall {
        name: "unclear_booking".to_string(),
        arguments: json!({
            "customer": 12345,
            "room": "A101", 
            "nights": 3,
            "lat": 40.7128,
            "lng": -74.0060
        }),
    }).await?;
    
    println!("❓ Unclear result: {}\n", unclear_result);
    
    println!("🎯 Key Benefits of Newtypes:");
    println!("   • Prevent parameter mix-ups at compile time");
    println!("   • Make APIs self-documenting");
    println!("   • Provide semantic meaning to primitive types");
    println!("   • Enable better tooling and IDE support");
    println!("   • Generate clear JSON schemas for LLMs");
    
    Ok(())
}