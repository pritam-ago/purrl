use anyhow::Result;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub idx: usize,          // 1-based index
    pub path: String,
    pub file_name: String,
    pub ext: String,
    pub folder: String,
    pub score: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct DeciderResponse {
    choice: Option<usize>,   // 1..N OR null
    confidence: f32,         // 0.0 .. 1.0
}

pub fn decide_best(query: &str, candidates: &[Candidate]) -> Result<Option<usize>> {
    if candidates.is_empty() {
        return Ok(None);
    }

    // Build a strict prompt: AI must choose an index or null.
    let mut lines = Vec::new();
    lines.push(format!("User query: \"{}\"\n", query));
    lines.push("Choose the SINGLE best matching file from the candidates below.".to_string());
    lines.push("If you're not sure, choose null.".to_string());
    lines.push("You MUST choose only from the given indices. Do NOT invent paths.".to_string());
    lines.push("Reply ONLY in valid JSON with keys: choice, confidence.".to_string());
    lines.push("Example: {\"choice\": 2, \"confidence\": 0.82} or {\"choice\": null, \"confidence\": 0.3}\n".to_string());

    lines.push("Candidates:".to_string());
    for c in candidates {
        lines.push(format!(
            "{}. {} (ext={}, folder={}, score={:.4})",
            c.idx, c.file_name, c.ext, c.folder, c.score
        ));
    }

    let prompt = lines.join("\n");

    // --- Ollama call ---
    // Use /api/generate with a small local model (you can change model name).
    // If you already have a specific local text model, set it here.
    let body = json!({
        "model": "llama3.2:3b",        // change if needed
        "prompt": prompt,
        "stream": false,
        "temperature": 0.1
    });

    let client = Client::new();
    let resp: serde_json::Value = client
        .post("http://localhost:11434/api/generate")
        .json(&body)
        .send()?
        .json()?;

    // Ollama generate response usually has { "response": "..." }
    let raw = resp
        .get("response")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    // Extract JSON safely: find first '{' .. last '}' (helps when model adds extra text)
    let json_str = extract_json_object(&raw).unwrap_or_else(|| raw.clone());

    let parsed: DeciderResponse = serde_json::from_str(&json_str)?;

    // Guardrails
    if parsed.confidence < 0.7 {
        return Ok(None);
    }

    if let Some(choice) = parsed.choice {
        if choice >= 1 && choice <= candidates.len() {
            return Ok(Some(choice));
        }
    }

    Ok(None)
}

fn extract_json_object(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(s[start..=end].to_string())
}
