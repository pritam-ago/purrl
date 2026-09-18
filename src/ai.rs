use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// The model that turns a sentence into an AiAction. Named in the README as a
/// required pull.
const MODEL: &str = "llama3:8b";

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
        "model": MODEL,
        "prompt": prompt,
        "stream": false
    });

    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&body)
        .send()?
        .json::<serde_json::Value>()?;

    if let Some(err) = ollama_error(&response) {
        anyhow::bail!("Ollama: {err}");
    }

    let text_output = response["response"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("unexpected response from Ollama: {response}"))?
        .trim();

    let parsed: AiAction = serde_json::from_str(text_output).map_err(|e| {
        anyhow::anyhow!("{MODEL} did not return the expected JSON ({e}): {text_output}")
    })?;

    Ok(parsed)
}

/// Ollama reports a missing model, and most other failures, as an `error` field
/// in an otherwise ordinary JSON body. Without this check the error is silently
/// replaced by a fallback and surfaces as a deserialization complaint about a
/// field the user has never heard of.
pub fn ollama_error(response: &serde_json::Value) -> Option<String> {
    response
        .get("error")
        .and_then(|e| e.as_str())
        .map(|e| e.to_string())
}
