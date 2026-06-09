use crate::skills::{skill_schema, SkillSpec};
use anyhow::Context as _;
use jsonschema::JSONSchema;
use serde::Deserialize;
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
            if matches!(ext, "json" | "toml" | "md") {
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
            self.dir.join(format!("{name}.md")),
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
            self.dir.join(format!("{name}.md")),
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
            "md" => parse_markdown_skill(&raw, path)?,
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

#[derive(Debug, Deserialize)]
struct SkillMarkdownFrontmatter {
    name: String,
    version: String,
    description: String,
    author: String,
    license: String,
    #[serde(alias = "allowed-tools")]
    allowed_tools: Vec<String>,
    #[serde(alias = "system-prompt")]
    system_prompt: Option<String>,
}

fn parse_markdown_skill(raw: &str, path: &Path) -> anyhow::Result<SkillSpec> {
    let (fm, lines) = parse_frontmatter(raw, path)?;

    let body = lines.collect::<Vec<_>>().join("\n");
    let body_prompt = body.trim();
    let system_prompt = if !body_prompt.is_empty() {
        body_prompt.to_string()
    } else if let Some(sp) = fm
        .system_prompt
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        sp.to_string()
    } else {
        anyhow::bail!("markdown skill missing system prompt body: {path:?}");
    };

    Ok(SkillSpec {
        name: fm.name,
        version: fm.version,
        description: fm.description,
        author: fm.author,
        license: fm.license,
        system_prompt,
        allowed_tools: fm.allowed_tools,
    })
}

fn parse_frontmatter<'a>(
    raw: &'a str,
    path: &Path,
) -> anyhow::Result<(SkillMarkdownFrontmatter, std::str::Lines<'a>)> {
    let mut lines = raw.lines();
    let first = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty markdown skill: {path:?}"))?;
    let first = first.trim_start_matches('\u{feff}');
    let delimiter = first.trim();
    if delimiter != "+++" && delimiter != "---" {
        anyhow::bail!(
            "markdown skill must start with frontmatter delimiter (+++ for TOML or --- for YAML): {path:?}"
        );
    }

    let mut frontmatter = String::new();
    let mut found_end = false;
    for line in lines.by_ref() {
        if line.trim() == delimiter {
            found_end = true;
            break;
        }
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }
    if !found_end {
        anyhow::bail!(
            "markdown skill missing closing frontmatter delimiter ({delimiter}): {path:?}"
        );
    }

    let fm: SkillMarkdownFrontmatter = if delimiter == "+++" {
        toml::from_str(&frontmatter).with_context(|| format!("parse TOML frontmatter: {path:?}"))?
    } else {
        serde_yaml::from_str(&frontmatter)
            .with_context(|| format!("parse YAML frontmatter: {path:?}"))?
    };
    Ok((fm, lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_skill_frontmatter_and_body() {
        let raw = r#"+++
name = "pdf_summarizer"
version = "0.1.0"
description = "Summarize a PDF into a short brief."
author = "Your Name"
license = "Apache-2.0"
allowed_tools = ["read_file", "text_summarize"]
+++
You are a helpful summarizer."#;
        let spec = parse_markdown_skill(raw, Path::new("skills/pdf_summarizer.md")).unwrap();
        assert_eq!(spec.name, "pdf_summarizer");
        assert_eq!(spec.allowed_tools, vec!["read_file", "text_summarize"]);
        assert_eq!(spec.system_prompt, "You are a helpful summarizer.");
    }

    #[test]
    fn markdown_skill_can_use_system_prompt_from_frontmatter_when_body_empty() {
        let raw = r#"+++
name = "hello"
version = "0.1.0"
description = "Say hello."
author = "Your Name"
license = "Apache-2.0"
allowed_tools = ["text_summarize"]
system_prompt = "Hello from frontmatter."
+++
"#;
        let spec = parse_markdown_skill(raw, Path::new("skills/hello.md")).unwrap();
        assert_eq!(spec.system_prompt, "Hello from frontmatter.");
    }

    #[test]
    fn rejects_markdown_without_frontmatter_delimiters() {
        let raw = "name = \"x\"";
        let err = parse_markdown_skill(raw, Path::new("skills/x.md")).unwrap_err();
        assert!(err
            .to_string()
            .contains("must start with frontmatter delimiter"));
    }

    #[test]
    fn parses_yaml_frontmatter_and_body() {
        let raw = r#"---
name: discussion
version: 0.1.0
description: Have an interactive discussion about a topic.
author: AegisCore
license: Apache-2.0
allowed-tools:
  - read_file
---
You are a discussion agent."#;
        let spec = parse_markdown_skill(raw, Path::new("skills/discussion.md")).unwrap();
        assert_eq!(spec.name, "discussion");
        assert_eq!(spec.allowed_tools, vec!["read_file"]);
        assert_eq!(spec.system_prompt, "You are a discussion agent.");
    }
}
