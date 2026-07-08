use std::collections::HashMap;

pub struct KnowledgePackLoader;

impl KnowledgePackLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KnowledgePackLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct KnowledgePackIndex {
    dynasty_terms: HashMap<String, Vec<String>>,
}

impl KnowledgePackIndex {
    pub fn new() -> Self {
        Self {
            dynasty_terms: HashMap::new(),
        }
    }

    pub fn add_dynasty_terms(&mut self, dynasty: &str, terms: &[&str]) {
        let entry = self.dynasty_terms.entry(dynasty.to_string()).or_default();
        for t in terms {
            if !entry.contains(&t.to_string()) {
                entry.push(t.to_string());
            }
        }
    }

    pub fn terms_for_dynasty(&self, dynasty: &str) -> Vec<String> {
        self.dynasty_terms.get(dynasty).cloned().unwrap_or_default()
    }
}
