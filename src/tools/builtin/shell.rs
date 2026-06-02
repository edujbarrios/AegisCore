use crate::registry::{Tool, ToolContext, ToolHandler, ToolRegistry};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

pub fn register(registry: &mut ToolRegistry) -> anyhow::Result<()> {
    registry.register(Tool {
        name: "shell_command".to_string(),
        description: "Run a shell command (disabled by default; requires allow_dangerous_tools)"
            .to_string(),
        parameters: serde_json::json!({
            "type":"object",
            "additionalProperties": false,
            "required": ["command"],
            "properties": {
                "command": {"type":"string"},
                "args": {"type":"array","items":{"type":"string"},"maxItems":64},
                "timeout_ms": {"type":"integer","minimum":1,"maximum":600000}
            }
        }),
        handler: Arc::new(ShellCommandTool),
    })?;
    Ok(())
}

struct ShellCommandTool;

#[async_trait]
impl ToolHandler for ShellCommandTool {
    async fn call(&self, args: Value, ctx: ToolContext) -> anyhow::Result<Value> {
        if !ctx.allow_dangerous_tools {
            anyhow::bail!(
                "shell_command is disabled by default (set runtime.allow_dangerous_tools=true)"
            );
        }

        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing command"))?;
        if command.contains('/') || command.contains('\\') {
            anyhow::bail!("command must be a bare executable name (no path separators)");
        }

        let argv = args
            .get("args")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>();

        let timeout_ms = args
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(ctx.shell_timeout_ms)
            .min(600_000);

        let mut cmd = Command::new(command);
        cmd.args(argv);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output =
            tokio::time::timeout(Duration::from_millis(timeout_ms), cmd.output()).await??;
        Ok(serde_json::json!({
            "status": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }))
    }
}
