use reqwest::blocking::Client;
use serde_json::json;

/// Generate embedding for some text
pub fn embed_text(text: &str) -> anyhow::Result<Vec<f32>> {
    let client = Client::new();

    let body = json!({
        "model": "nomic-embed-text",
        "input": text
    });

    let response = client
        .post("http://localhost:11434/api/embeddings")
        .json(&body)
        .send()?
        .json::<serde_json::Value>()?;

    let embedding_vec = response["embedding"]
        .as_array()
        .expect("embedding missing")
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();

    Ok(embedding_vec)
}
