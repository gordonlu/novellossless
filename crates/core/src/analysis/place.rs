use super::extractor::{ChunkInfo, Extraction, Extractor, NarrativeNodeCandidate};
use regex::Regex;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct PlaceExtractor;

impl Extractor for PlaceExtractor {
    fn name(&self) -> &'static str {
        "place"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let mut seen = BTreeMap::new();
        let suffix_patterns = [
            "城", "镇", "村", "街", "巷", "楼", "塔", "宫", "殿", "府", "山", "谷", "阁", "院",
            "桥", "寺", "观", "港", "站", "基地", "星球", "舰船",
        ];
        let pattern_str = format!(r"([\p{{Han}}]{{1,6}}(?:{}))", suffix_patterns.join("|"));
        let Ok(pattern) = Regex::new(&pattern_str) else {
            return Vec::new();
        };

        for chunk in chunks {
            for captures in pattern.captures_iter(&chunk.content) {
                let Some(raw) = captures.get(1).map(|m| m.as_str()) else {
                    continue;
                };
                let name = raw.trim().to_string();
                if name.chars().count() < 2 {
                    continue;
                }
                seen.entry(name.clone())
                    .or_insert_with(|| (0, chunk.chunk_id.clone(), chunk.chunk_id.clone()));
                if let Some((count, _, latest)) = seen.get_mut(&name) {
                    *count += 1;
                    *latest = chunk.chunk_id.clone();
                }
            }
        }

        seen.into_iter()
            .map(|(name, (count, first, latest))| {
                Extraction::Candidate(NarrativeNodeCandidate {
                    node_type: "place".to_string(),
                    name,
                    aliases: Vec::new(),
                    summary: String::new(),
                    occurrence_count: count,
                    first_chunk_id: first,
                    latest_chunk_id: latest,
                    confidence: (50 + count.saturating_mul(10)).min(90),
                })
            })
            .collect()
    }
}
