use crate::skills::SkillSpec;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct SkillRegistryEntry {
    pub spec: SkillSpec,
    pub source_path: Option<std::path::PathBuf>,
}

#[derive(Default, Clone)]
pub struct SkillRegistry {
    skills: BTreeMap<String, SkillRegistryEntry>,
}

impl SkillRegistry {
    pub fn register(&mut self, entry: SkillRegistryEntry) -> anyhow::Result<()> {
        let name = entry.spec.name.clone();
        if self.skills.contains_key(&name) {
            anyhow::bail!("skill already registered: {name}");
        }
        self.skills.insert(name, entry);
        Ok(())
    }

    pub fn unregister(&mut self, name: &str) -> Option<SkillRegistryEntry> {
        self.skills.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&SkillRegistryEntry> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&SkillRegistryEntry> {
        self.skills.values().collect()
    }
}
