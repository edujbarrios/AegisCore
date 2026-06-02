use super::AgentProvider;
use crate::runtime::openai_types::ChatMessage;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Clone, Default)]
pub struct LocalProvider;

#[async_trait]
impl AgentProvider for LocalProvider {
    async fn execute(
        &self,
        messages: Vec<ChatMessage>,
        _tools: Option<Vec<Value>>,
    ) -> anyhow::Result<ChatMessage> {
        let last = messages.into_iter().rev().find(|m| m.role == "user");
        Ok(ChatMessage {
            role: "assistant".to_string(),
            content: Some(format!(
                "LocalProvider (no LLM configured). Received: {}",
                last.and_then(|m| m.content).unwrap_or_default()
            )),
            tool_call_id: None,
            tool_calls: None,
        })
    }
}
