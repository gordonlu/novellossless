use super::extractor::{ChunkInfo, Extraction, Extractor, ForeshadowCandidate};
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Default)]
pub struct ForeshadowExtractor;

impl Extractor for ForeshadowExtractor {
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
        let mut seen: BTreeMap<String, ForeshadowAccumulator> = BTreeMap::new();

        for chunk in chunks {
            for sentence in split_sentences(&chunk.content) {
                if !markers.iter().any(|m| sentence.contains(m)) {
                    continue;
                }
                let title = sentence.chars().take(28).collect::<String>();

                seen.entry(title.clone())
                    .or_insert_with(|| ForeshadowAccumulator {
                        first_chunk: chunk.clone(),
                        latest_chunk: chunk.clone(),
                        mention_count: 0,
                        related_names: Vec::new(),
                    });

                if let Some(acc) = seen.get_mut(&title) {
                    acc.latest_chunk = chunk.clone();
                    acc.mention_count += 1;

                    for cap in name_pattern().captures_iter(&sentence) {
                        let n = cap.get(1).unwrap().as_str().to_string();
                        if n.chars().count() >= 2 {
                            acc.related_names.push(n);
                        }
                    }
                }
            }
        }

        seen.into_iter()
            .map(|(title, acc)| {
                let gap = acc.latest_chunk.chunk_index - acc.first_chunk.chunk_index;
                let risk = calculate_risk(gap, acc.mention_count);
                let mut related: Vec<String> = acc.related_names;
                related.sort();
                related.dedup();

                Extraction::Foreshadow(ForeshadowCandidate {
                    title,
                    foreshadow_type: "explicit_clue".to_string(),
                    first_chunk_id: acc.first_chunk.chunk_id.clone(),
                    latest_chunk_id: acc.latest_chunk.chunk_id.clone(),
                    risk_level: risk.to_string(),
                    evidence: acc.first_chunk.content.chars().take(120).collect(),
                    related_nodes: related,
                })
            })
            .collect()
    }
}

struct ForeshadowAccumulator {
    first_chunk: ChunkInfo,
    latest_chunk: ChunkInfo,
    mention_count: i64,
    related_names: Vec<String>,
}

fn calculate_risk(chapter_gap: i64, mention_count: i64) -> &'static str {
    let score = chapter_gap
        .saturating_abs()
        .saturating_mul(2)
        .saturating_mul(mention_count);
    if score >= 20 {
        "high"
    } else if score >= 10 {
        "medium"
    } else {
        "low"
    }
}

fn name_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"([\p{Han}]{2,4})").unwrap())
}

fn split_sentences(content: &str) -> Vec<String> {
    content
        .split(['。', '！', '？', '\n'])
        .map(str::trim)
        .filter(|s| s.chars().count() >= 8)
        .map(ToString::to_string)
        .collect()
}
