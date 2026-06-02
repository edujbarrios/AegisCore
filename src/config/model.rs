use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AegisConfig {
    pub llm: LlmConfig,
    pub runtime: RuntimeConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub api_key_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub max_tool_rounds: usize,
    pub allow_dangerous_tools: bool,
    pub fs_root: PathBuf,
    pub max_read_bytes: u64,
    pub max_write_bytes: u64,
    pub http_max_bytes: u64,
    pub http_timeout_ms: u64,
    pub shell_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for AegisConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-4.1-mini".to_string(),
                api_key_env: "OPENAI_API_KEY".to_string(),
            },
            runtime: RuntimeConfig {
                max_tool_rounds: 8,
                allow_dangerous_tools: false,
                fs_root: PathBuf::from("."),
                max_read_bytes: 1024 * 1024,
                max_write_bytes: 1024 * 1024,
                http_max_bytes: 2 * 1024 * 1024,
                http_timeout_ms: 15_000,
                shell_timeout_ms: 15_000,
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8787,
            },
        }
    }
}

impl AegisConfig {
    pub fn load(path: Option<&Path>) -> anyhow::Result<Self> {
        let path = path.unwrap_or_else(|| Path::new("aegiscore.toml"));
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("read config: {path:?}"))?;
        let cfg: AegisConfig =
            toml::from_str(&raw).with_context(|| format!("parse TOML: {path:?}"))?;
        Ok(cfg)
    }
}
