use crate::registry::{Tool, ToolContext, ToolHandler, ToolRegistry};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

pub fn register(registry: &mut ToolRegistry) -> anyhow::Result<()> {
    registry.register(Tool {
        name: "read_file".to_string(),
        description: "Read a file from the sandboxed filesystem".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["path"],
            "properties": {
                "path": {"type":"string"},
                "max_bytes": {"type":"integer","minimum":1}
            }
        }),
        handler: Arc::new(ReadFileTool),
    })?;

    registry.register(Tool {
        name: "write_file".to_string(),
        description: "Write a file to the sandboxed filesystem".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["path","content"],
            "properties": {
                "path": {"type":"string"},
                "content": {"type":"string"},
                "create_dirs": {"type":"boolean"},
                "overwrite": {"type":"boolean"}
            }
        }),
        handler: Arc::new(WriteFileTool),
    })?;

    registry.register(Tool {
        name: "list_files".to_string(),
        description: "List files under a directory in the sandbox".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "path": {"type":"string"},
                "recursive": {"type":"boolean"},
                "max_entries": {"type":"integer","minimum":1,"maximum":5000}
            }
        }),
        handler: Arc::new(ListFilesTool),
    })?;

    Ok(())
}

struct ReadFileTool;

#[async_trait]
impl ToolHandler for ReadFileTool {
    async fn call(&self, args: Value, ctx: ToolContext) -> anyhow::Result<Value> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing path"))?;
        let max_bytes = args
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(ctx.max_read_bytes)
            .min(ctx.max_read_bytes);

        let resolved = resolve_sandbox_path(&ctx.fs_root, path)?;
        debug!(path = %path, resolved = ?resolved, "read_file");
        let data = tokio::fs::read(&resolved).await?;
        let truncated = (data.len() as u64) > max_bytes;
        let bytes = if truncated {
            data.into_iter()
                .take(max_bytes as usize)
                .collect::<Vec<_>>()
        } else {
            data
        };
        let content = String::from_utf8_lossy(&bytes).to_string();
        Ok(serde_json::json!({
            "path": path,
            "resolved": resolved.display().to_string(),
            "truncated": truncated,
            "content": content
        }))
    }
}

struct WriteFileTool;

#[async_trait]
impl ToolHandler for WriteFileTool {
    async fn call(&self, args: Value, ctx: ToolContext) -> anyhow::Result<Value> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing path"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing content"))?;
        let create_dirs = args
            .get("create_dirs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let overwrite = args
            .get("overwrite")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if (content.len() as u64) > ctx.max_write_bytes {
            anyhow::bail!("content exceeds max_write_bytes");
        }

        let resolved = resolve_sandbox_path(&ctx.fs_root, path)?;
        if resolved.exists() && !overwrite {
            anyhow::bail!("file exists (set overwrite=true): {path}");
        }
        if create_dirs {
            if let Some(parent) = resolved.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        tokio::fs::write(&resolved, content).await?;
        Ok(serde_json::json!({
            "path": path,
            "resolved": resolved.display().to_string(),
            "bytes": content.len()
        }))
    }
}

struct ListFilesTool;

#[async_trait]
impl ToolHandler for ListFilesTool {
    async fn call(&self, args: Value, ctx: ToolContext) -> anyhow::Result<Value> {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_entries = args
            .get("max_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000)
            .min(5000) as usize;

        let resolved = resolve_sandbox_path(&ctx.fs_root, path)?;
        let entries = list_entries(&resolved, recursive, max_entries).await?;
        Ok(serde_json::json!({
            "path": path,
            "resolved": resolved.display().to_string(),
            "entries": entries
        }))
    }
}

async fn list_entries(
    dir: &Path,
    recursive: bool,
    max_entries: usize,
) -> anyhow::Result<Vec<Value>> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let mut rd = tokio::fs::read_dir(&cur).await?;
        while let Some(entry) = rd.next_entry().await? {
            if out.len() >= max_entries {
                return Ok(out);
            }
            let meta = entry.metadata().await?;
            let p = entry.path();
            out.push(serde_json::json!({
                "path": p.display().to_string(),
                "is_dir": meta.is_dir(),
                "size_bytes": if meta.is_file() { Some(meta.len()) } else { None::<u64> }
            }));
            if recursive && meta.is_dir() {
                stack.push(p);
            }
        }
    }
    Ok(out)
}

fn resolve_sandbox_path(root: &Path, input: &str) -> anyhow::Result<PathBuf> {
    if input.trim().is_empty() {
        anyhow::bail!("empty path");
    }
    let rel = Path::new(input);
    if rel.is_absolute() {
        anyhow::bail!("absolute paths are not allowed");
    }
    for comp in rel.components() {
        if matches!(comp, std::path::Component::ParentDir) {
            anyhow::bail!("parent directory components ('..') are not allowed");
        }
    }

    let joined = root.join(rel);
    let canon_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let canon_joined = std::fs::canonicalize(&joined).unwrap_or(joined);
    if !canon_joined.starts_with(&canon_root) {
        anyhow::bail!("path escapes sandbox root");
    }
    Ok(canon_joined)
}
