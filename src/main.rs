use toors_derive::tool;

pub struct Add;

#[tool]
/// Adds two numbers
/// Can handle negative values
/// Arbitrarily long descriptions
/// Woohoooo!
impl Add {
    pub fn call(a: i32, b: i32) -> i32 {
        a + b
    }
}

pub struct Order {
    client_id: u32
}

/// Places an order based off of an sku and quantity
#[tool]
impl Order {
    pub fn call(&self, sku: i32, quantity: usize) {
        unimplemented!("Not yet ready!")
    }
}

fn main() {
    println!("Signature: {}", Add::signature());
    println!("Description\n{}", Add::description());
    println!("\n");
    println!("Signature: {}", Order::signature());
    println!("Description:\n{}", Order::description());
}
