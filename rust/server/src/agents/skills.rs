use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    commands: HashMap<String, SkillDoc>,
}

#[derive(Debug, Clone)]
struct SkillDoc {
    body: String,
}

impl SkillRegistry {
    pub fn load(base_dir: &Path) -> Result<Self> {
        if !base_dir.exists() {
            return Ok(Self::default());
        }

        Ok(Self {
            commands: load_command_files(&base_dir.join("commands"))?,
        })
    }

    pub fn available_commands(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn command_markdown(&self, name: &str) -> Option<String> {
        self.commands
            .get(name)
            .map(|doc| doc.body.trim().to_string())
            .filter(|body| !body.is_empty())
    }

    pub fn resolve_command_name(&self, name: &str) -> Option<String> {
        self.commands
            .keys()
            .find(|candidate| candidate.eq_ignore_ascii_case(name.trim()))
            .cloned()
    }

    pub fn mentioned_commands(&self, message: &str) -> Vec<String> {
        let lowered = message.to_ascii_lowercase();
        let mut names = self.available_commands();
        names.retain(|name| {
            command_aliases(name)
                .iter()
                .any(|alias| lowered.contains(alias))
        });
        names
    }
}

fn load_command_files(dir: &Path) -> Result<HashMap<String, SkillDoc>> {
    if !dir.exists() {
        return Ok(HashMap::new());
    }

    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("không thể đọc thư mục commands {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
        .collect::<Vec<PathBuf>>();

    entries.sort();

    let mut commands = HashMap::new();
    for path in entries {
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_string();
        let doc = load_markdown_file(&path)?;
        commands.insert(name, doc);
    }

    Ok(commands)
}

fn load_markdown_file(path: &Path) -> Result<SkillDoc> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("không thể đọc file skill {}", path.display()))?;

    Ok(SkillDoc { body })
}

fn command_aliases(name: &str) -> Vec<String> {
    let lower = name.trim().to_ascii_lowercase();
    let mut aliases = vec![lower.clone()];

    let spaced = lower.replace(['-', '_'], " ");
    if !aliases.iter().any(|value| value == &spaced) {
        aliases.push(spaced);
    }

    let underscored = lower.replace(['-', ' '], "_");
    if !aliases.iter().any(|value| value == &underscored) {
        aliases.push(underscored);
    }

    let dashed = lower.replace(['_', ' '], "-");
    if !aliases.iter().any(|value| value == &dashed) {
        aliases.push(dashed);
    }

    aliases
}
