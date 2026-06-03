use crate::config::AegisConfig;
use crate::runtime::openai_types::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage};
use anyhow::Context as _;
use async_trait::async_trait;
use serde_json::Value;
use tracing::debug;

#[async_trait]
pub trait AgentProvider: Send + Sync {
    async fn execute(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<Value>>,
    ) -> anyhow::Result<ChatMessage>;
}

#[derive(Clone)]
pub struct OpenAiProvider {
    cfg: AegisConfig,
    client: reqwest::Client,
    api_key: String,
}

impl OpenAiProvider {
    pub fn new(cfg: AegisConfig) -> anyhow::Result<Self> {
        let api_key = std::env::var(&cfg.llm.api_key_env)
            .with_context(|| format!("missing env var {}", cfg.llm.api_key_env))?;
        let client = reqwest::Client::builder()
            .build()
            .context("build http client")?;
        Ok(Self {
            cfg,
            client,
            api_key,
        })
    }

    fn chat_completions_url(&self) -> anyhow::Result<reqwest::Url> {
        let mut base = self.cfg.llm.base_url.trim_end_matches('/').to_string();
        if !base.ends_with("/v1") {
            base.push_str("/v1");
        }
        let url = format!("{base}/chat/completions");
        reqwest::Url::parse(&url).context("parse base_url")
    }
}

#[async_trait]
impl AgentProvider for OpenAiProvider {
    async fn execute(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<Value>>,
    ) -> anyhow::Result<ChatMessage> {
        let url = self.chat_completions_url()?;
        let req = ChatCompletionRequest {
            model: self.cfg.llm.model.clone(),
            messages,
            tools,
            tool_choice: Some(serde_json::json!("auto")),
        };

        debug!(url = %url, model = %req.model, "sending provider request");
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .context("http send")?
            .error_for_status()
            .context("http status")?;

        let body: ChatCompletionResponse = resp.json().await.context("decode response")?;
        let choice = body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("provider returned no choices"))?;
        Ok(choice.message)
    }
}
