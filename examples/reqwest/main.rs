use gemini::GeminiClient;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tools_rs::{FunctionCall, collect_tools, tool};
mod gemini;

#[derive(Serialize, Deserialize, Debug)]
struct ToolError {
    message: String,
}

impl From<reqwest::Error> for ToolError {
    fn from(err: reqwest::Error) -> Self {
        ToolError {
            message: err.to_string(),
        }
    }
}

#[tool]
/// Gets the weather based on longitude and latitude
async fn get_weather(lat: String, lon: String) -> Result<Value, ToolError> {
    let res = reqwest::get(format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m",
        lat, lon
    ))
    .await?
    .text()
    .await?;

    let json = json!(res);
    // println!("Json: {:?}", json);
    Ok(json)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let col = collect_tools();
    let client = Client::new();

    let mut payload = json!({
        "contents": [
            {
                "role": "user",
                "parts": [
                    { "text": "What's the weather like in Paris, estimate the latitude and longitude?" }
                ]
            }
        ],
        "tools": [{
            "functionDeclarations": col.json()?
        }]
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        std::env::var("GEMINI_API_KEY")?
    );
    let raw_req = client.post(&url).json(&payload);
    println!("First Payload: {:#?}", payload);
    let mut res_text = raw_req.send().await?.text().await?;
    let mut json_res: Value = serde_json::from_str(&res_text)?;

    // println!("Initial Response: {}", res_text);

    // Look for a function call in any candidate and any part
    let mut function_call: Option<Value> = None;

    // Gemini complexities, where to look for a functionCall
    if let Some(candidates) = json_res["candidates"].as_array() {
        for candidate in candidates {
            if let Some(parts) = candidate
                .get("content")
                .and_then(|c| c.get("parts"))
                .and_then(Value::as_array)
            {
                for part in parts {
                    if let Some(fc) = part.get("functionCall") {
                        function_call = Some(fc.clone());
                        break;
                    }
                }
            }
            if function_call.is_some() {
                break;
            }
        }
    }

    // If function call exists, call the tool and continue the conversation
    if let Some(fc) = function_call {
        let name = fc["name"].as_str().unwrap_or("<unknown>");
        let args = fc["args"].clone();

        // println!("Calling function: {} with args: {}", name, args);

        let func_res = col
            .call(FunctionCall {
                name: name.to_string(),
                arguments: args,
            })
            .await?;

        // Push tool response
        payload["contents"].as_array_mut().unwrap().push(json!({
            "parts": [{
                "functionCall": {
                    "id": fc["name"].as_str(),
                    "name": fc["name"].as_str(),
                    "args": fc["args"]
                },
            },{
                "functionResponse": {
                    "id": name,
                    "name": name,
                    "response": func_res
                }
            }]
        }));

        // Call model again with tool response
        res_text = client
            .post(&url)
            .json(&payload)
            .send()
            .await?
            .text()
            .await?;
        json_res = serde_json::from_str(&res_text)?;
    }

    // println!("Final Response: {:#}", json_res);

    let mut gemini = GeminiClient::new("gemini-2.0-flash".to_string());
    let res = gemini
        .call("What's the capital of france?".to_string())
        .await?;
    //println!("Testing answer: {:#?}", res);
    Ok(())
}
