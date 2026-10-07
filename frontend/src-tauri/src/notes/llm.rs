//! Minimal OpenAI-compatible chat client used by the notes features (OpenRouter today, Publik later).

use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

pub const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// OpenRouter only: ask for low reasoning effort. GLM 5.3 Flash rejects disabling reasoning outright.
    pub low_reasoning: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LlmResult {
    pub content: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    /// Provider-reported cost in USD when available (OpenRouter returns it with usage.include).
    pub cost_usd: Option<f64>,
}

#[derive(Debug)]
pub enum LlmError {
    /// 402 from the provider: out of credit. Carries the provider message.
    PaymentRequired(String),
    Http(u16, String),
    Network(String),
    Malformed(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::PaymentRequired(m) => write!(f, "out of credit: {m}"),
            LlmError::Http(code, m) => write!(f, "provider returned {code}: {m}"),
            LlmError::Network(m) => write!(f, "network error: {m}"),
            LlmError::Malformed(m) => write!(f, "unexpected response: {m}"),
        }
    }
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    cost: Option<f64>,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Response {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

pub async fn chat(
    cfg: &LlmConfig,
    system: &str,
    user: &str,
    json_mode: bool,
    temperature: f32,
) -> Result<LlmResult, LlmError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(150))
        .build()
        .map_err(|e| LlmError::Network(e.to_string()))?;
    let mut body = json!({
        "model": cfg.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ],
        "temperature": temperature,
        "usage": {"include": true}
    });
    if json_mode {
        body["response_format"] = json!({"type": "json_object"});
    }
    if cfg.low_reasoning {
        body["reasoning"] = json!({"effort": "low"});
    }
    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));

    // One retry for timeouts, dropped connections, rate limits and transient upstream errors.
    // Provider latency occasionally spikes for minutes; a fresh request usually lands on a faster backend.
    let mut attempt = 0;
    let (status, text) = loop {
        attempt += 1;
        let result = async {
            let resp = client
                .post(&url)
                .bearer_auth(&cfg.api_key)
                .header("X-Title", "Cornflake")
                .json(&body)
                .send()
                .await?;
            let status = resp.status().as_u16();
            Ok::<_, reqwest::Error>((status, resp.text().await?))
        }
        .await;
        match result {
            Ok((status, _)) if attempt == 1 && matches!(status, 429 | 502 | 503 | 504) => {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(_) if attempt == 1 => {}
            Ok(ok) => break ok,
            Err(e) => return Err(LlmError::Network(e.to_string())),
        }
    };
    if status == 402 {
        return Err(LlmError::PaymentRequired(provider_message(&text)));
    }
    if !(200..300).contains(&status) {
        return Err(LlmError::Http(status, provider_message(&text)));
    }
    let parsed: Response =
        serde_json::from_str(&text).map_err(|e| LlmError::Malformed(format!("{e}")))?;
    let content = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| LlmError::Malformed("no message content".into()))?;
    let usage = parsed.usage.unwrap_or(Usage { prompt_tokens: 0, completion_tokens: 0, cost: None });
    Ok(LlmResult {
        content,
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        cost_usd: usage.cost,
    })
}

fn provider_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| body.chars().take(300).collect())
}
