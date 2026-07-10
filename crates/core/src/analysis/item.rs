use super::extractor::{ChunkInfo, Extraction, Extractor, NarrativeNodeCandidate};
use regex::Regex;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct ItemExtractor;

impl Extractor for ItemExtractor {
    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let mut seen = BTreeMap::new();
        let item_nouns = [
            "钥匙", "信", "戒指", "刀", "剑", "书", "照片", "芯片", "卷轴", "玉佩", "伞", "令牌",
            "地图", "药瓶", "手札", "玉简",
        ];
        let noun_pattern = format!(r"(?:\p{{Han}}{{0,4}}?)({})", item_nouns.join("|"));
        let Ok(patterns) = vec![
            Regex::new(&noun_pattern),
            Regex::new(r"(?:拿起|藏起|交给|寻找|丢失|夺走|握住)([\p{Han}]{1,6})"),
        ]
        .into_iter()
        .collect::<Result<Vec<_>, _>>() else {
            return Vec::new();
        };

        for chunk in chunks {
            for pattern in &patterns {
                for captures in pattern.captures_iter(&chunk.content) {
                    let Some(raw) = captures.get(1).map(|m| m.as_str()) else {
                        continue;
                    };
                    let name = strip_quantity_prefix(raw.trim());
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
        }

        seen.into_iter()
            .map(|(name, (count, first, latest))| {
                Extraction::Candidate(NarrativeNodeCandidate {
                    node_type: "item".to_string(),
                    name,
                    aliases: Vec::new(),
                    occurrence_count: count,
                    first_chunk_id: first,
                    latest_chunk_id: latest,
                    confidence: (50 + count.saturating_mul(10)).min(90),
                })
            })
            .collect()
    }
}

fn strip_quantity_prefix(value: &str) -> String {
    let prefixes = [
        "那枚", "这枚", "一枚", "那把", "这把", "一把", "那封", "这封", "一封",
    ];
    for prefix in prefixes {
        if let Some(stripped) = value.strip_prefix(prefix) {
            return stripped.to_string();
        }
    }
    value.to_string()
}
