use crate::config::AegisConfig;
use crate::modules::core::CoreModule;
use crate::modules::AegisModule;
use crate::registry::{SkillRegistry, SkillRegistryEntry, ToolContext, ToolRegistry};
use crate::runtime::AgentRuntime;
use crate::skills::{SkillFormat, SkillSpec, SkillStore};
use anyhow::Context as _;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "aegiscore",
    version,
    about = "AegisCore: personal agent runtime"
)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init,
    Create {
        prompt: String,
    },
    List,
    Inspect {
        name: String,
    },
    Run {
        name: String,
        #[arg(long)]
        input: Option<PathBuf>,
    },
    Delete {
        name: String,
    },
    Tools,
    Modules,
    Serve,
}

pub async fn entrypoint() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cfg = AegisConfig::load(cli.config.as_deref()).context("load config")?;

    match cli.command {
        Command::Init => cmd_init().await,
        Command::Create { prompt } => cmd_create(&prompt).await,
        Command::List => cmd_list().await,
        Command::Inspect { name } => cmd_inspect(&name).await,
        Command::Run { name, input } => cmd_run(&cfg, &name, input.as_deref()).await,
        Command::Delete { name } => cmd_delete(&name).await,
        Command::Tools => cmd_tools(&cfg).await,
        Command::Modules => cmd_modules().await,
        Command::Serve => cmd_serve(cfg).await,
    }
}

async fn cmd_init() -> anyhow::Result<()> {
    for dir in ["docs", "examples", "skills", "modules"] {
        tokio::fs::create_dir_all(dir)
            .await
            .with_context(|| format!("create dir: {dir}"))?;
    }

    if !Path::new(".env.example").exists() {
        tokio::fs::write(".env.example", "OPENAI_API_KEY=\n")
            .await
            .context("write .env.example")?;
    }

    if !Path::new("aegiscore.toml").exists() {
        tokio::fs::write("aegiscore.toml", include_str!("../../aegiscore.toml"))
            .await
            .context("write aegiscore.toml")?;
    }

    info!("initialized aegiscore workspace");
    Ok(())
}

async fn cmd_create(prompt: &str) -> anyhow::Result<()> {
    let name = infer_skill_name(prompt);
    let store = SkillStore::new("skills");
    let spec = SkillSpec {
        name: name.clone(),
        version: "0.1.0".to_string(),
        description: prompt.to_string(),
        author: "Eduardo J. Barrios".to_string(),
        license: "Apache-2.0".to_string(),
        system_prompt: prompt.to_string(),
        allowed_tools: vec!["read_file".to_string(), "text_summarize".to_string()],
    };

    let path = store.save(&spec, SkillFormat::Toml).context("save skill")?;
    info!(skill = %name, path = ?path, "created skill");
    Ok(())
}

async fn cmd_list() -> anyhow::Result<()> {
    let store = SkillStore::new("skills");
    for path in store.list().context("list skills")? {
        if let Some(file) = path.file_name().and_then(|s| s.to_str()) {
            if file == ".gitkeep" {
                continue;
            }
        }
        println!("{}", path.display());
    }
    Ok(())
}

async fn cmd_inspect(name: &str) -> anyhow::Result<()> {
    let store = SkillStore::new("skills");
    let item = store
        .load_one(name)
        .context("load skill")?
        .ok_or_else(|| anyhow::anyhow!("skill not found: {name}"))?;
    println!("{}", serde_json::to_string_pretty(&item.spec)?);
    Ok(())
}

async fn cmd_delete(name: &str) -> anyhow::Result<()> {
    let store = SkillStore::new("skills");
    let deleted = store.delete(name).context("delete skill")?;
    if !deleted {
        anyhow::bail!("skill not found: {name}");
    }
    info!(skill = %name, "deleted skill");
    Ok(())
}

async fn cmd_tools(cfg: &AegisConfig) -> anyhow::Result<()> {
    let mut tools = ToolRegistry::default();
    CoreModule.register_tools(&mut tools)?;

    let ctx = tool_ctx_from_config(cfg)?;
    let tool_names = tools
        .list()
        .into_iter()
        .map(|t| t.name.as_str())
        .collect::<Vec<_>>();
    info!(allow_dangerous_tools = ctx.allow_dangerous_tools, tools = ?tool_names, "available tools");
    for t in tools.list() {
        println!("{} - {}", t.name, t.description);
    }
    Ok(())
}

async fn cmd_modules() -> anyhow::Result<()> {
    println!(
        "{} - {} ({})",
        CoreModule.name(),
        CoreModule.description(),
        CoreModule.version()
    );
    Ok(())
}

async fn cmd_run(cfg: &AegisConfig, name: &str, input: Option<&Path>) -> anyhow::Result<()> {
    let mut tools = ToolRegistry::default();
    CoreModule.register_tools(&mut tools)?;

    let mut skills = SkillRegistry::default();
    let store = SkillStore::new("skills");
    for item in store.load_all().context("load skills")? {
        skills.register(SkillRegistryEntry {
            spec: item.spec,
            source_path: Some(item.path),
        })?;
    }

    let input_json = match input {
        None => serde_json::json!({}),
        Some(path) => {
            let raw = tokio::fs::read_to_string(path)
                .await
                .with_context(|| format!("read input: {path:?}"))?;
            serde_json::from_str(&raw).with_context(|| format!("parse json: {path:?}"))?
        }
    };

    let ctx = tool_ctx_from_config(cfg)?;
    let runtime = AgentRuntime::new(cfg.clone(), Arc::new(tools), Arc::new(skills), ctx);
    let out = runtime.run_skill(name, input_json).await?;
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

async fn cmd_serve(cfg: AegisConfig) -> anyhow::Result<()> {
    crate::server::serve(cfg).await
}

fn tool_ctx_from_config(cfg: &AegisConfig) -> anyhow::Result<ToolContext> {
    let mut ctx = ToolContext::default_for_repo()?;
    ctx.fs_root = canonicalize_or_current(&cfg.runtime.fs_root)?;
    ctx.allow_dangerous_tools = cfg.runtime.allow_dangerous_tools;
    ctx.max_read_bytes = cfg.runtime.max_read_bytes;
    ctx.max_write_bytes = cfg.runtime.max_write_bytes;
    ctx.http_max_bytes = cfg.runtime.http_max_bytes;
    ctx.http_timeout_ms = cfg.runtime.http_timeout_ms;
    ctx.shell_timeout_ms = cfg.runtime.shell_timeout_ms;
    ctx.tool_concurrency = Arc::new(Semaphore::new(8));
    Ok(ctx)
}

fn canonicalize_or_current(path: &Path) -> anyhow::Result<PathBuf> {
    let p = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(std::fs::canonicalize(&p).unwrap_or(p))
}

fn infer_skill_name(prompt: &str) -> String {
    let lc = prompt.to_ascii_lowercase();
    if lc.contains("pdf") && lc.contains("summar") {
        return "pdf_summarizer".to_string();
    }

    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in prompt.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "new_skill".to_string()
    } else {
        out
    }
}
