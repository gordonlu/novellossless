use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct KnowledgePackIndex {
    entries_by_dynasty: HashMap<String, Vec<String>>,
}

impl KnowledgePackIndex {
    pub fn add_dynasty_terms(&mut self, dynasty: &str, terms: &[&str]) {
        let entry = self
            .entries_by_dynasty
            .entry(dynasty.to_string())
            .or_default();
        for t in terms {
            if !entry.contains(&t.to_string()) {
                entry.push(t.to_string());
            }
        }
    }

    pub fn terms_for_dynasty(&self, dynasty: &str) -> &[String] {
        self.entries_by_dynasty
            .get(dynasty)
            .map(|v| v.as_slice())
            .unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries_by_dynasty.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgePackEntry {
    pub pack_name: String,
    pub pack_type: String,
    pub entries: Vec<KnowledgeItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeItem {
    pub term: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub dynasty: String,
    #[serde(default)]
    pub rank: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeToml {
    #[serde(default)]
    entry: Vec<KnowledgeItem>,
}

pub struct KnowledgePackLoader;

impl KnowledgePackLoader {
    pub fn load_all(profiles_root: &Path, profile_id: &str) -> Result<Vec<KnowledgePackEntry>> {
        let knowledge_dir = profiles_root.join(profile_id).join("knowledge");
        Self::load_all_from_dir(&knowledge_dir)
    }

    pub fn load_all_from_dir(knowledge_dir: &Path) -> Result<Vec<KnowledgePackEntry>> {
        if !knowledge_dir.exists() {
            return Ok(Vec::new());
        }

        let mut packs = Vec::new();
        let read_dir = std::fs::read_dir(knowledge_dir)
            .with_context(|| format!("failed to read {}", knowledge_dir.display()))?;

        for entry in read_dir {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if ext != "toml" {
                continue;
            }
            let content = std::fs::read_to_string(&path)?;
            let toml_data: KnowledgeToml = toml::from_str(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?;

            let pack_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let pack_type = toml_data
                .entry
                .first()
                .map(|e| e.category.clone())
                .unwrap_or_default();

            packs.push(KnowledgePackEntry {
                pack_name,
                pack_type,
                entries: toml_data.entry,
            });
        }

        Ok(packs)
    }

    pub fn build_index(packs: &[KnowledgePackEntry]) -> KnowledgePackIndex {
        let mut index = KnowledgePackIndex::default();
        for pack in packs {
            for item in &pack.entries {
                if !item.dynasty.is_empty() {
                    index.add_dynasty_terms(&item.dynasty, &[&item.term]);
                }
            }
        }
        index
    }
}
