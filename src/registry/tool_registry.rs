use async_trait::async_trait;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct ToolContext {
    pub fs_root: PathBuf,
    pub allow_dangerous_tools: bool,
    pub max_read_bytes: u64,
    pub max_write_bytes: u64,
    pub http_max_bytes: u64,
    pub http_timeout_ms: u64,
    pub shell_timeout_ms: u64,
    pub tool_concurrency: Arc<Semaphore>,
}

impl ToolContext {
    pub fn default_for_repo() -> anyhow::Result<Self> {
        let fs_root = std::env::current_dir()?;
        Ok(Self {
            fs_root,
            allow_dangerous_tools: false,
            max_read_bytes: 1024 * 1024,
            max_write_bytes: 1024 * 1024,
            http_max_bytes: 2 * 1024 * 1024,
            http_timeout_ms: 15_000,
            shell_timeout_ms: 15_000,
            tool_concurrency: Arc::new(Semaphore::new(8)),
        })
    }
}

#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub handler: Arc<dyn ToolHandler>,
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn call(&self, args: Value, ctx: ToolContext) -> anyhow::Result<Value>;
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Tool>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: Tool) -> anyhow::Result<()> {
        if self.tools.contains_key(&tool.name) {
            anyhow::bail!("tool already registered: {}", tool.name);
        }
        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    pub fn unregister(&mut self, name: &str) -> Option<Tool> {
        self.tools.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    pub fn list(&self) -> Vec<&Tool> {
        self.tools.values().collect()
    }

    pub async fn execute(
        &self,
        name: &str,
        args: Value,
        ctx: ToolContext,
    ) -> anyhow::Result<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("unknown tool: {name}"))?;
        tool.handler.call(args, ctx).await
    }

    pub fn export_openai_tools(&self, allowlist: &[String]) -> Vec<Value> {
        let allow: std::collections::BTreeSet<&str> =
            allowlist.iter().map(|s| s.as_str()).collect();
        self.tools
            .values()
            .filter(|t| allow.contains(t.name.as_str()))
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect()
    }
}
