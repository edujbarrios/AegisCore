use crate::skills::{skill_schema, SkillSpec};
use anyhow::Context as _;
use jsonschema::JSONSchema;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SkillStoreItem {
    pub path: PathBuf,
    pub spec: SkillSpec,
}

#[derive(Debug, Clone)]
pub struct SkillStore {
    pub dir: PathBuf,
}

impl SkillStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn list(&self) -> anyhow::Result<Vec<PathBuf>> {
        if !self.dir.exists() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for entry in
            std::fs::read_dir(&self.dir).with_context(|| format!("read dir: {:?}", self.dir))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(ext, "json" | "toml") {
                out.push(path);
            }
        }
        out.sort();
        Ok(out)
    }

    pub fn load_all(&self) -> anyhow::Result<Vec<SkillStoreItem>> {
        let compiled = JSONSchema::compile(skill_schema()).context("compile skill schema")?;
        let mut items = vec![];
        for path in self.list()? {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name == ".gitkeep" {
                    continue;
                }
            }
            let spec = Self::load_one_with_schema(&compiled, &path)?;
            items.push(SkillStoreItem { path, spec });
        }
        Ok(items)
    }

    pub fn load_one(&self, name: &str) -> anyhow::Result<Option<SkillStoreItem>> {
        let compiled = JSONSchema::compile(skill_schema()).context("compile skill schema")?;
        for candidate in [
            self.dir.join(format!("{name}.toml")),
            self.dir.join(format!("{name}.json")),
        ] {
            if candidate.exists() {
                let spec = Self::load_one_with_schema(&compiled, &candidate)?;
                return Ok(Some(SkillStoreItem {
                    path: candidate,
                    spec,
                }));
            }
        }
        Ok(None)
    }

    pub fn save(&self, spec: &SkillSpec, format: SkillFormat) -> anyhow::Result<PathBuf> {
        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("create dir: {:?}", self.dir))?;
        let path = match format {
            SkillFormat::Toml => self.dir.join(format!("{}.toml", spec.name)),
            SkillFormat::Json => self.dir.join(format!("{}.json", spec.name)),
        };

        let compiled = JSONSchema::compile(skill_schema()).context("compile skill schema")?;
        let value = serde_json::to_value(spec).context("encode spec json")?;
        if let Err(errors) = compiled.validate(&value) {
            let msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join("; ");
            anyhow::bail!("skill spec invalid: {msg}");
        }

        let content = match format {
            SkillFormat::Toml => toml::to_string_pretty(spec).context("encode spec toml")?,
            SkillFormat::Json => serde_json::to_string_pretty(spec).context("encode spec json")?,
        };
        std::fs::write(&path, content).with_context(|| format!("write: {path:?}"))?;
        Ok(path)
    }

    pub fn delete(&self, name: &str) -> anyhow::Result<bool> {
        for candidate in [
            self.dir.join(format!("{name}.toml")),
            self.dir.join(format!("{name}.json")),
        ] {
            if candidate.exists() {
                std::fs::remove_file(&candidate)
                    .with_context(|| format!("remove: {candidate:?}"))?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn load_one_with_schema(compiled: &JSONSchema, path: &Path) -> anyhow::Result<SkillSpec> {
        let raw = std::fs::read_to_string(path).with_context(|| format!("read skill: {path:?}"))?;
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let spec: SkillSpec = match ext {
            "toml" => toml::from_str(&raw).with_context(|| format!("parse toml: {path:?}"))?,
            "json" => {
                serde_json::from_str(&raw).with_context(|| format!("parse json: {path:?}"))?
            }
            _ => anyhow::bail!("unsupported skill format: {path:?}"),
        };

        let value = serde_json::to_value(&spec).context("encode spec json")?;
        if let Err(errors) = compiled.validate(&value) {
            let msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join("; ");
            anyhow::bail!("skill spec invalid in {path:?}: {msg}");
        }
        Ok(spec)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SkillFormat {
    Toml,
    Json,
}
