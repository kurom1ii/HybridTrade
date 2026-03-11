use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use super::models::AgentRole;

#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    common: Vec<SkillDoc>,
    per_agent: HashMap<String, Vec<SkillDoc>>,
}

#[derive(Debug, Clone)]
struct SkillDoc {
    file_name: String,
    body: String,
}

impl SkillRegistry {
    pub fn load(base_dir: &Path) -> Result<Self> {
        if !base_dir.exists() {
            return Ok(Self::default());
        }

        let common = load_markdown_files(&base_dir.join("common"))?;
        let mut per_agent = HashMap::new();

        let agents_dir = base_dir.join("agents");
        if agents_dir.exists() {
            for role in AgentRole::visible_agents() {
                let docs = load_agent_docs(&agents_dir, role.as_str())?;
                if !docs.is_empty() {
                    per_agent.insert(role.as_str().to_string(), docs);
                }
            }
        }

        Ok(Self { common, per_agent })
    }

    pub fn common_filenames(&self) -> Vec<String> {
        self.common
            .iter()
            .map(|item| item.file_name.clone())
            .collect()
    }

    pub fn common_markdown(&self) -> String {
        render_docs(&self.common)
    }

    pub fn agent_filenames(&self, role: AgentRole) -> Vec<String> {
        self.per_agent
            .get(role.as_str())
            .map(|items| items.iter().map(|item| item.file_name.clone()).collect())
            .unwrap_or_default()
    }

    pub fn agent_markdown(&self, role: AgentRole) -> String {
        self.per_agent
            .get(role.as_str())
            .map(|items| render_docs(items))
            .unwrap_or_default()
    }
}

fn load_agent_docs(agents_dir: &Path, role: &str) -> Result<Vec<SkillDoc>> {
    let mut docs = Vec::new();

    let role_keys = match role {
        "kuromi" => vec!["kuromi", "coordinator"],
        other => vec![other],
    };

    for key in role_keys {
        let single_file = agents_dir.join(format!("{key}.md"));
        if single_file.exists() {
            docs.push(load_markdown_file(&single_file)?);
        }

        let role_dir = agents_dir.join(key);
        if role_dir.exists() {
            docs.extend(load_markdown_files(&role_dir)?);
        }
    }

    Ok(docs)
}

fn load_markdown_files(dir: &Path) -> Result<Vec<SkillDoc>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("không thể đọc thư mục skills {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
        .collect::<Vec<PathBuf>>();

    entries.sort();

    entries
        .iter()
        .map(|path| load_markdown_file(path))
        .collect()
}

fn load_markdown_file(path: &Path) -> Result<SkillDoc> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("không thể đọc file skill {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("untitled-skill.md")
        .to_string();

    Ok(SkillDoc { file_name, body })
}

fn render_docs(docs: &[SkillDoc]) -> String {
    docs.iter()
        .map(|doc| doc.body.trim())
        .filter(|body| !body.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
