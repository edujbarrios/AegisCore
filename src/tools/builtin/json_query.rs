use crate::registry::{Tool, ToolContext, ToolHandler, ToolRegistry};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry) -> anyhow::Result<()> {
    registry.register(Tool {
        name: "json_query".to_string(),
        description: "Query JSON using a JSON Pointer".to_string(),
        parameters: serde_json::json!({
            "type":"object",
            "additionalProperties": false,
            "required": ["json","pointer"],
            "properties": {
                "json": {},
                "pointer": {"type":"string"}
            }
        }),
        handler: Arc::new(JsonQueryTool),
    })?;
    Ok(())
}

struct JsonQueryTool;

#[async_trait]
impl ToolHandler for JsonQueryTool {
    async fn call(&self, args: Value, _ctx: ToolContext) -> anyhow::Result<Value> {
        let json = args
            .get("json")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing json"))?;
        let ptr = args
            .get("pointer")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing pointer"))?;

        let found = json.pointer(ptr).cloned();
        Ok(serde_json::json!({
            "pointer": ptr,
            "found": found
        }))
    }
}
