use super::AgentProvider;
use crate::runtime::openai_types::ChatMessage;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Clone, Default)]
pub struct CodexProvider;

#[async_trait]
impl AgentProvider for CodexProvider {
    async fn execute(
        &self,
        _messages: Vec<ChatMessage>,
        _tools: Option<Vec<Value>>,
    ) -> anyhow::Result<ChatMessage> {
        anyhow::bail!("CodexProvider is not configured in this build")
    }
}
