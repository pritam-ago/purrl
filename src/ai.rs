use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct AiAction {
    pub intent: String,              // "search" or "open"
    pub query: Option<String>,       // text extracted from user input
    pub file_type: Option<String>,   // "pdf", "image", "text", etc.
    pub time_filter: Option<String>, // "yesterday", "last week", etc.
    pub folder_hint: Option<String>, // "downloads", "documents", etc.
}

pub fn interpret_command(input: &str) -> anyhow::Result<AiAction> {
    let client = Client::new();

    // We ask the LLM to output JSON ONLY.
    let prompt = format!(
        r#"You are an AI that converts natural language into command actions.
User input: "{input}"

Extract:
- intent (one of: search, open)
- query (keywords to search for)
- file_type (optional)
- time_filter (optional)
- folder_hint (optional)

Respond in JSON ONLY. Example:
{{
  "intent": "search",
  "query": "hostel fees",
  "file_type": "pdf",
  "time_filter": "yesterday",
  "folder_hint": "downloads"
}}"#
    );

    let body = json!({
        "model": "llama3:8b",
        "prompt": prompt,
        "stream": false
    });

    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&body)
        .send()?
        .json::<serde_json::Value>()?;

    let text_output = response["response"].as_str().unwrap_or("{}").trim();

    let parsed: AiAction = serde_json::from_str(text_output)?;

    Ok(parsed)
}
