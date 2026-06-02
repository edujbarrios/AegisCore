use crate::modules::AegisModule;
use crate::registry::{SkillRegistry, ToolRegistry};
use crate::tools::builtin;

#[derive(Clone, Copy)]
pub struct CoreModule;

impl AegisModule for CoreModule {
    fn name(&self) -> &'static str {
        "core"
    }

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &'static str {
        "Built-in registries, tools, and runtime wiring"
    }

    fn register_tools(&self, registry: &mut ToolRegistry) -> anyhow::Result<()> {
        builtin::register_all(registry)
    }

    fn register_skills(&self, _registry: &mut SkillRegistry) -> anyhow::Result<()> {
        Ok(())
    }
}
