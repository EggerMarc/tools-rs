use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tools_rs::ToolCollection;

pub struct GeminiClient {
    url: String,
    client: reqwest::Client,
    pub history: GeminiHistory,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiHistory {
    contents: Vec<GeminiContent>,
}
impl GeminiHistory {
    fn new() -> Self {
        Self { contents: vec![] }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GeminiContent {
    parts: Vec<GeminiParts>,
    role: String,
}

impl GeminiContent {
    fn from_string(role: String, text: String) -> Self {
        GeminiContent {
            parts: vec![GeminiParts::Text(GeminiText { text })],
            role: role.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum GeminiParts {
    FunctionCall(GeminiFunctionCall),
    FuctionResponse(GeminiFunctionResponse),
    Text(GeminiText),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeminiFunctionResponse {
    id: String,
    name: String,
    response: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeminiText {
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeminiFunctionCall {
    id: String,
    name: String,
    args: Value,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiCandidate {
    content: GeminiContent,
}

impl GeminiClient {
    pub fn new(model_id: String) -> Self {
        Self {
            url: format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model_id,
                std::env::var("GEMINI_API_KEY").expect("Couldn't find GEMINI_API_KEY in env"),
            ),
            client: reqwest::Client::new(),
            history: GeminiHistory::new(),
        }
    }

    pub async fn call(
        &mut self,
        prompt: String,
        tools: Option<ToolCollection>,
    ) -> reqwest::Result<GeminiResponse> {
        self.history
            .contents
            .push(GeminiContent::from_string("user".to_string(), prompt));

        let payload = json!({
            "contents": self.history.contents,
            "tools": [{
                "functionDeclarations": tools.map(|t| t.json().unwrap())
            }]
        });

        println!("Payload: {:#?}", payload);
        let req = self.client.post(self.url.clone()).json(&payload);
        let res = req.send().await?;
        let out = res.json::<GeminiResponse>().await?;
        Ok(out)
    }
}
