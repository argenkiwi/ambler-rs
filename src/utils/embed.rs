//! Embeddings from an Ollama-style host (`OLLAMA_HOST`, default `localhost:11434`).

use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct Response {
    embedding: Vec<f32>,
}

fn base_url() -> String {
    let host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "localhost:11434".into());
    if host.contains("://") {
        host
    } else {
        format!("http://{host}")
    }
}

/// Embeds `text`, or `None` if the host is unreachable or answers badly.
#[must_use]
pub fn embed(text: &str) -> Option<Vec<f32>> {
    let model = std::env::var("AZK_EMBED_MODEL").unwrap_or_else(|_| "nomic-embed-text".into());
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .timeout_connect(Some(Duration::from_secs(2)))
        .build()
        .into();
    let response: Response = agent
        .post(format!("{}/api/embeddings", base_url()))
        .send_json(serde_json::json!({ "model": model, "prompt": text }))
        .ok()?
        .body_mut()
        .read_json()
        .ok()?;
    (!response.embedding.is_empty()).then_some(response.embedding)
}
