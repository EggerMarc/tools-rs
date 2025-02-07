use toors_derive::{Tool};
use toors::{ToolCollection};
use std::io;

#[derive(Tool)]
struct DB {
    /// Database URL
    url: String
    
    /// Database port
    port: String
}

#[tools]
impl DB {
    /// Connects to a new db
    fn from(url: String, port: String, token: UUID) -> Self {
        DB {
            url,
            port
        }
    }
    
    /// Fetch orders given 
    async fn fetch_orders(id: &u32) -> Opion<&[Order]> {
       Some(sql!("select * from orders where id = {}", id))
    }

    /// Fetches user id, searching through name
    fn fetch_user_id(name: &str) -> Option<u32> {
        Some(sql!("select id from users where name = {}", name))
    }
}

#[tokio:main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = LLMFramework(model="your model");
    let mut collection = ToolCollection::new();
    collection.add(DB);

    let mut prompt = format!("you are a customer success analyst. You can access a database using the following tools, {:?}", collection);
    loop {
        println!("Enter your message:");
        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input).expect("Failed to read line");
        prompt = format!("{}\nUser: {}", prompt, user_input.trim());
        let result = llm.build(prompt).invoke();
        println!("LLM Response: {}", result);
    }
}
