// === C10 Agent Runtime: LLM Provider ===
//
// Real LLM provider implementations for OpenAI-compatible APIs and Ollama.
// Uses reqwest blocking client since the agent loop runs on blocking threads.
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md Section 1.2

use crate::config::LlmSection;
use crate::types::{LlmRequest, LlmResponse};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Common error type for LLM provider operations.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("provider not configured: {0}")]
    NotConfigured(String),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// A provider that can complete chat prompts.
pub trait LlmProvider: std::fmt::Debug {
    fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;
}

// ── OpenAI-Compatible Provider ────────────────────────────────────────────

/// OpenAI / OpenAI-compatible provider (any API that mirrors /v1/chat/completions).
#[derive(Debug)]
pub struct OpenAiProvider {
    client: Client,
    api_url: String,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_url: String, api_key: String, model: String) -> Self {
        Self { client: Client::new(), api_url, api_key, model }
    }
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: String,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    total_tokens: u32,
}

impl LlmProvider for OpenAiProvider {
    fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let messages: Vec<OpenAiMessage> = {
            let mut msgs = Vec::with_capacity(request.messages.len() + 1);
            msgs.push(OpenAiMessage {
                role: "system".into(),
                content: request.system_prompt.clone(),
            });
            for msg in &request.messages {
                msgs.push(OpenAiMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                });
            }
            msgs
        };

        let body = OpenAiChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens,
        };

        let resp: OpenAiChatResponse = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?
            .error_for_status()
            .map_err(|e| LlmError::Api(e.to_string()))?
            .json()?;

        let choice = resp.choices.into_iter().next().ok_or_else(|| {
            LlmError::Api("empty choices in OpenAI response".into())
        })?;

        Ok(LlmResponse {
            content: choice.message.content,
            tokens_used: resp.usage.map(|u| u.total_tokens).unwrap_or(0),
            finish_reason: choice.finish_reason,
        })
    }
}

// ── Ollama Provider ───────────────────────────────────────────────────────

/// Ollama local provider.
#[derive(Debug)]
pub struct OllamaProvider {
    client: Client,
    api_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(api_url: String, model: String) -> Self {
        Self { client: Client::new(), api_url, model }
    }
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    message: OllamaMessage,
    total_duration: Option<u64>,
}

impl LlmProvider for OllamaProvider {
    fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut messages: Vec<OllamaMessage> = Vec::with_capacity(request.messages.len() + 1);
        messages.push(OllamaMessage {
            role: "system".into(),
            content: request.system_prompt.clone(),
        });
        for msg in &request.messages {
            messages.push(OllamaMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        let body = OllamaChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        let resp: OllamaResponseMessage = self
            .client
            .post(&self.api_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?
            .error_for_status()
            .map_err(|e| LlmError::Api(e.to_string()))?
            .json()?;

        // Ollama doesn't return token counts, so estimate roughly
        let tokens_used = (request.system_prompt.len()
            + request.messages.iter().map(|m| m.content.len()).sum::<usize>()
            + resp.message.content.len())
            / 4;

        Ok(LlmResponse {
            content: resp.message.content,
            tokens_used: tokens_used as u32,
            finish_reason: "stop".into(),
        })
    }
}

// ── Stub Provider (for testing without a live API) ────────────────────────

/// Stub provider that returns empty responses. Used for development/testing
/// when no LLM provider is configured.
#[derive(Debug)]
pub struct StubProvider;

impl LlmProvider for StubProvider {
    fn complete(&self, _request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            content: String::new(),
            tokens_used: 10,
            finish_reason: "stub".to_string(),
        })
    }
}

// ── Factory ───────────────────────────────────────────────────────────────

/// Create an LLM provider from the config section.
/// Falls back to StubProvider if config is minimal or blank.
pub fn provider_from_config(section: &LlmSection) -> Box<dyn LlmProvider> {
    match section.provider.to_lowercase().as_str() {
        "openai" | "openai-compatible" => {
            let api_url = section
                .api_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".into());
            let api_key = section.api_key.clone().unwrap_or_default();
            if api_key.is_empty() {
                Box::new(StubProvider)
            } else {
                Box::new(OpenAiProvider::new(api_url, api_key, section.model.clone()))
            }
        }
        "ollama" => {
            let api_url = section
                .api_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434/api/chat".into());
            Box::new(OllamaProvider::new(api_url, section.model.clone()))
        }
        _ => Box::new(StubProvider),
    }
}
