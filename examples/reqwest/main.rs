use serde_json::{Value, json};
use tools_rs::{FunctionCall, collect_tools, tool};

#[tool]
/// Gets the current temperature for given coordinates
async fn get_weather(lat: f64, lon: f64) -> Result<f64, String> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,wind_speed_10m&hourly=temperature_2m,relative_humidity_2m,wind_speed_10m",
        lat, lon
    );

    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;

    let json: Value = response.json().await.map_err(|e| e.to_string())?;

    json.get("current")
        .and_then(|current| current.get("temperature_2m"))
        .and_then(|temp| temp.as_f64())
        .ok_or("Missing temperature_2m in response".to_string())
}

#[tool]
/// Counts instance in string
async fn count_instance(s: String, sub: String) -> i32 {
    s.matches(&sub).count() as i32
}

#[tool]
/// Send email
async fn send_email(to: String, content: String) {
    println!("Email sent to {},\n{}\n", to, content)
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
        let response = client
            .post(&url)
            .json(&json!({
                "contents": &history,
                "tools": {"functionDeclarations": tools_decl}
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let json: Value = response.json().await?;
            println!(
                "Error: {:#?}, on the following history: {:#?}",
                json, history
            );
            return Err(format!("Gemini API error: {}", json).into());
        }

        let res: Value = response.json().await?;

        let content = &res["candidates"][0]["content"];
        history.push(json!({"role": "model", "parts": content["parts"]}));

        let parts = content["parts"].as_array().unwrap();
        let mut function_responses: Vec<Value> = vec![];
        for part in parts {
            if let Some(fc) = part.get("functionCall") {
                let result = tools
                    .call(FunctionCall {
                        name: fc["name"].as_str().unwrap().to_string(),
                        arguments: fc["args"].clone(),
                    })
                    .await?;
                function_responses.push(json!({
                    "functionResponse": {"name": fc["name"], "response": {"value": result}}
                }));
            } else if let Some(text) = part["text"].as_str() {
                return Ok(text.to_string());
            }
        }
        history.push(json!({
            "role": "function",
            "parts": function_responses
        }));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = gemini_chat(
        "How many letter rs are in the word ratatouille. Also, what's the weather like today in Paris? Could you also send an email to bob (bob@gmail.com) saying how much I liked yesterday's invite?",
        &collect_tools(),
        &std::env::var("GEMINI_API_KEY")?,
    )
    .await?;

    println!("{}", response);
    Ok(())
}
