use serde_json::{Value, json};
use tools_rs::{FunctionCall, collect_tools, tool};

#[tool]
/// Gets weather data for given coordinates
async fn get_weather(lat: f64, lon: f64) -> Result<Value, String> {
    reqwest::get(&format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m",
        lat, lon
    ))
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| e.to_string())
}

#[tool]
/// Counts instance in string
async fn count_instance(s: String, sub: String) -> i32 {
    s.matches(&sub).count() as i32
}

async fn gemini_chat(
    prompt: &str,
    tools: &tools_rs::ToolCollection,
    api_key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );
    let mut history = vec![json!({"role": "user", "parts": [{"text": prompt}]})];
    let tools_decl = tools.json()?;

    loop {
        let res: Value = client
            .post(&url)
            .json(&json!({
                "contents": &history,
                "tools": {"functionDeclarations": tools_decl}
            }))
            .send()
            .await?
            .json()
            .await?;

        let content = &res["candidates"][0]["content"];
        history.push(json!({"role": "model", "parts": content["parts"]}));

        let part = &content["parts"][0];

        if let Some(fc) = part.get("functionCall") {
            let result = tools
                .call(FunctionCall {
                    name: fc["name"].as_str().unwrap().to_string(),
                    arguments: fc["args"].clone(),
                })
                .await?;
            history.push(json!({
                "role": "model",
                "parts": [{"functionResponse": {"name": fc["name"], "response": {"value": result}}}]
            }));
        } else if let Some(text) = part["text"].as_str() {
            return Ok(text.to_string());
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = gemini_chat(
        "How many letter rs are in the word ratatouille",
        &collect_tools(),
        &std::env::var("GEMINI_API_KEY")?,
    )
    .await?;

    println!("{}", response);
    Ok(())
}
