use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

/// Hard cap on user text sent per AI call — never send full novels.
/// 8000 字 ≈ 4000–5000 tokens, enough for focused excerpt analysis.
const MAX_INPUT_CHARS: usize = 8_000;

/// Count Unicode characters (not bytes) in a string.
fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Find the byte index of the Nth character boundary.
/// Returns s.len() if `n` exceeds the total char count.
fn char_boundary(s: &str, n: usize) -> usize {
    let mut bytes = 0;
    for (i, c) in s.char_indices() {
        if i >= n {
            return bytes;
        }
        bytes = i + c.len_utf8();
    }
    s.len()
}

pub struct OpenAiProvider {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    client: reqwest::blocking::Client,
}

impl OpenAiProvider {
    pub fn new(api_url: String, api_key: String, model: String) -> Self {
        Self {
            api_url,
            api_key,
            model,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    /// Maximum user text allowed in a single call.
    pub fn max_input_chars(&self) -> usize {
        MAX_INPUT_CHARS
    }

    /// Truncate text to the safe char limit, preserving whole sentences.
    /// Uses Unicode character count, not byte count.
    /// Returns (truncated, was_truncated) — never panics.
    pub fn truncate_input(text: &str, limit: usize) -> (&str, bool) {
        if char_count(text) <= limit {
            return (text, false);
        }
        // Find the byte position of the limit-th character
        let byte_limit = char_boundary(text, limit);
        // Back up to the last sentence boundary within the limit
        let boundary = text[..byte_limit]
            .rfind(|c| c == '。' || c == '！' || c == '？' || c == '\n')
            .unwrap_or(byte_limit);
        (&text[..boundary], true)
    }

    pub fn chat_completion(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let (safe_input, truncated) = Self::truncate_input(user_prompt, MAX_INPUT_CHARS);
        if truncated {
            eprintln!(
                "[novellossless] AI input truncated from {} chars to {} chars",
                char_count(user_prompt),
                char_count(safe_input),
            );
        }
        let url = format!("{}/v1/chat/completions", self.api_url.trim_end_matches('/'));
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: safe_input.to_string(),
                },
            ],
            temperature: 0.3,
            max_tokens: 2048,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .with_context(|| format!("AI API request failed to {}", url))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            anyhow::bail!("AI API returned {}: {}", status, body);
        }

        let body: ChatResponse = response.json().context("failed to parse AI API response")?;

        body.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .context("AI API returned no choices")
    }

    pub fn test_connection(&self) -> Result<String> {
        self.chat_completion("You are a helpful assistant.", "Respond with exactly: ok")
    }
}
