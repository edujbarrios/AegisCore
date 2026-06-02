pub mod core;

use crate::registry::{SkillRegistry, ToolRegistry};

pub trait AegisModule: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn register_tools(&self, registry: &mut ToolRegistry) -> anyhow::Result<()>;
    fn register_skills(&self, registry: &mut SkillRegistry) -> anyhow::Result<()>;
}
