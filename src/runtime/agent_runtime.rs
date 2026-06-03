use crate::config::AegisConfig;
use crate::providers::{AgentProvider, LocalProvider, OpenAiProvider};
use crate::registry::{SkillRegistry, ToolContext, ToolRegistry};
use crate::runtime::openai_types::ChatMessage;
use anyhow::Context as _;
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};

pub struct AgentRuntime {
    cfg: AegisConfig,
    tools: Arc<ToolRegistry>,
    skills: Arc<SkillRegistry>,
    tool_ctx: ToolContext,
}

impl AgentRuntime {
    pub fn new(
        cfg: AegisConfig,
        tools: Arc<ToolRegistry>,
        skills: Arc<SkillRegistry>,
        tool_ctx: ToolContext,
    ) -> Self {
        Self {
            cfg,
            tools,
            skills,
            tool_ctx,
        }
    }

    pub async fn run_skill(&self, name: &str, input: Value) -> anyhow::Result<Value> {
        let entry = self
            .skills
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("unknown skill: {name}"))?;

        let provider: Arc<dyn AgentProvider> = match OpenAiProvider::new(self.cfg.clone()) {
            Ok(p) => Arc::new(p),
            Err(err) => {
                warn!(%err, "falling back to LocalProvider (no LLM credentials configured)");
                Arc::new(LocalProvider)
            }
        };
        let allowed_tools = entry.spec.allowed_tools.clone();

        let mut messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(entry.spec.system_prompt.clone()),
                tool_call_id: None,
                tool_calls: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(input.to_string()),
                tool_call_id: None,
                tool_calls: None,
            },
        ];

        let openai_tools = self.tools.export_openai_tools(&allowed_tools);
        let max_rounds = self.cfg.runtime.max_tool_rounds;

        for round in 0..=max_rounds {
            let resp = provider
                .execute(messages.clone(), Some(openai_tools.clone()))
                .await
                .with_context(|| format!("provider execute (round {round})"))?;

            let mut msg = resp;
            let tool_calls = msg.tool_calls.take().unwrap_or_default();
            if tool_calls.is_empty() {
                let content = msg.content.take().unwrap_or_default();
                return Ok(serde_json::json!({
                    "skill": name,
                    "output": content
                }));
            }

            if round == max_rounds {
                warn!(skill = %name, "max tool rounds reached");
                return Ok(serde_json::json!({
                    "skill": name,
                    "error": "max tool rounds reached",
                }));
            }

            messages.push(msg);

            for call in tool_calls {
                let tool_name = call.function.name;
                if !allowed_tools.iter().any(|t| t == &tool_name) {
                    anyhow::bail!("skill {name} attempted disallowed tool: {tool_name}");
                }

                let args: Value = serde_json::from_str(&call.function.arguments)
                    .context("parse tool arguments")?;
                info!(skill = %name, tool = %tool_name, "executing tool");
                let _permit = self.tool_ctx.tool_concurrency.acquire().await?;
                let out = self
                    .tools
                    .execute(&tool_name, args, self.tool_ctx.clone())
                    .await
                    .with_context(|| format!("tool failed: {tool_name}"))?;

                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: Some(out.to_string()),
                    tool_call_id: Some(call.id),
                    tool_calls: None,
                });
            }
        }

        anyhow::bail!("unreachable")
    }
}
