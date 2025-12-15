use reqwest::blocking::Client;
use serde_json::json;

/// Generate an embedding for a piece of text using Ollama
pub fn embed_text(text: &str) -> anyhow::Result<Vec<f32>> {
    let client = Client::new();

    // IMPORTANT: Ollama embeddings expect `prompt`, not `input`
    let body = json!({
        "model": "nomic-embed-text",
        "prompt": text
    });

    let resp: serde_json::Value = client
        .post("http://localhost:11434/api/embeddings")
        .json(&body)
        .send()?
        .json()?;

    // Case 1: { "embedding": [...] }
    if let Some(arr) = resp.get("embedding").and_then(|v| v.as_array()) {
        let vec: Vec<f32> = arr
            .iter()
            .map(|x| x.as_f64().unwrap_or(0.0) as f32)
            .collect();

        if vec.is_empty() {
            anyhow::bail!("Received empty embedding from Ollama");
        }

        return Ok(vec);
    }

    // Case 2: { "data": [ { "embedding": [...] } ] }
    if let Some(arr) = resp
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|o| o.get("embedding"))
        .and_then(|v| v.as_array())
    {
        let vec: Vec<f32> = arr
            .iter()
            .map(|x| x.as_f64().unwrap_or(0.0) as f32)
            .collect();

        if vec.is_empty() {
            anyhow::bail!("Received empty embedding from Ollama");
        }

        return Ok(vec);
    }

    // ❌ If neither format matched
    anyhow::bail!("Invalid embedding response from Ollama: {}", resp);
}
