use super::extractor::{ChunkInfo, Extraction, Extractor, ForeshadowCandidate};
use std::collections::BTreeSet;

#[derive(Default)]
pub struct ForeshadowExtractor;

impl Extractor for ForeshadowExtractor {
    fn name(&self) -> &'static str {
        "foreshadow"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let markers = [
            "秘密",
            "线索",
            "预感",
            "总觉得",
            "似乎",
            "好像",
            "日后",
            "终有一日",
            "钥匙",
            "信物",
            "谜",
        ];
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();

        for chunk in chunks {
            for sentence in split_sentences(&chunk.content) {
                if !markers.iter().any(|m| sentence.contains(m)) {
                    continue;
                }
                let title = sentence.chars().take(28).collect::<String>();
                let key = format!("{}:{}", chunk.chunk_id, title);
                if !seen.insert(key) {
                    continue;
                }
                items.push(Extraction::Foreshadow(ForeshadowCandidate {
                    title,
                    foreshadow_type: "explicit_clue".to_string(),
                    first_chunk_id: chunk.chunk_id.clone(),
                    latest_chunk_id: chunk.chunk_id.clone(),
                    risk_level: "medium".to_string(),
                    evidence: sentence.chars().take(120).collect(),
                    related_nodes: Vec::new(),
                }));
            }
        }

        items
    }
}

fn split_sentences(content: &str) -> Vec<String> {
    content
        .split(['。', '！', '？', '\n'])
        .map(str::trim)
        .filter(|s| s.chars().count() >= 8)
        .map(ToString::to_string)
        .collect()
}
