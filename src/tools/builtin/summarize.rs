use crate::registry::{Tool, ToolContext, ToolHandler, ToolRegistry};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry) -> anyhow::Result<()> {
    registry.register(Tool {
        name: "text_summarize".to_string(),
        description: "Deterministically summarize input text (no LLM required)".to_string(),
        parameters: serde_json::json!({
            "type":"object",
            "additionalProperties": false,
            "required": ["text"],
            "properties": {
                "text": {"type":"string"},
                "max_chars": {"type":"integer","minimum":16,"maximum":20000}
            }
        }),
        handler: Arc::new(TextSummarizeTool),
    })?;
    Ok(())
}

struct TextSummarizeTool;

#[async_trait]
impl ToolHandler for TextSummarizeTool {
    async fn call(&self, args: Value, _ctx: ToolContext) -> anyhow::Result<Value> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing text"))?;
        let max_chars = args
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(1200)
            .min(20_000) as usize;

        let normalized = text.replace("\r\n", "\n");
        let mut lines = normalized.lines().filter(|l| !l.trim().is_empty());

        let first = lines.next().unwrap_or("").trim();
        let second = lines.next().unwrap_or("").trim();
        let third = lines.next().unwrap_or("").trim();

        let mut summary = String::new();
        for (i, line) in [first, second, third].into_iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            if i > 0 {
                summary.push('\n');
            }
            summary.push_str(line);
        }

        if summary.len() > max_chars {
            summary.truncate(max_chars);
        }

        Ok(serde_json::json!({
            "summary": summary,
            "input_chars": normalized.chars().count(),
            "summary_chars": summary.chars().count()
        }))
    }
}
