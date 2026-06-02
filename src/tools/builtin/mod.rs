mod fs;
mod http;
mod json_query;
mod shell;
mod summarize;

use crate::registry::ToolRegistry;

pub fn register_all(registry: &mut ToolRegistry) -> anyhow::Result<()> {
    fs::register(registry)?;
    http::register(registry)?;
    json_query::register(registry)?;
    summarize::register(registry)?;
    shell::register(registry)?;
    Ok(())
}
