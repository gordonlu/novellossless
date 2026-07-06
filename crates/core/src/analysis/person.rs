use super::extractor::{ChunkInfo, Extraction, Extractor, NarrativeNodeCandidate};
use regex::Regex;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct PersonExtractor;

impl Extractor for PersonExtractor {
    fn name(&self) -> &'static str {
        "person"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let mut seen = BTreeMap::<String, CandidateAccumulator>::new();
        let Ok(patterns) = person_patterns() else {
            return Vec::new();
        };

        for chunk in chunks {
            for pattern in &patterns {
                for captures in pattern.captures_iter(&chunk.content) {
                    let Some(raw) = captures.get(1).map(|m| m.as_str()) else {
                        continue;
                    };
                    let name = normalize_name(raw);
                    if !is_valid_person_name(&name) {
                        continue;
                    }
                    seen.entry(name.clone())
                        .or_insert_with(|| CandidateAccumulator {
                            count: 0,
                            first_chunk_id: chunk.chunk_id.clone(),
                            latest_chunk_id: chunk.chunk_id.clone(),
                            aliases: Vec::new(),
                        });
                    if let Some(entry) = seen.get_mut(&name) {
                        entry.count += 1;
                        entry.latest_chunk_id = chunk.chunk_id.clone();
                    }
                }
            }
        }

        seen.into_iter()
            .filter(|(_, acc)| acc.count >= 1)
            .map(|(name, acc)| {
                Extraction::Candidate(NarrativeNodeCandidate {
                    node_type: "person".to_string(),
                    name,
                    aliases: Vec::new(),
                    summary: String::new(),
                    occurrence_count: acc.count,
                    first_chunk_id: acc.first_chunk_id,
                    latest_chunk_id: acc.latest_chunk_id,
                    confidence: (50 + acc.count.saturating_mul(10)).min(90),
                })
            })
            .collect()
    }
}

struct CandidateAccumulator {
    count: i64,
    first_chunk_id: String,
    latest_chunk_id: String,
    aliases: Vec<String>,
}

fn person_patterns() -> Result<Vec<Regex>, regex::Error> {
    vec![
        Regex::new(r"([\p{Han}]{2,4})(?:说|问|道|喊|低声|笑道|看着|走进|转身)"),
        Regex::new(r"(?:向|对|跟)([\p{Han}]{2,4})(?:说|问|道)"),
    ]
    .into_iter()
    .collect()
}

fn normalize_name(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '，' | '。' | '、' | '：' | '；' | '“' | '”' | '"' | '\'' | '《' | '》'
            )
    })
    .to_string()
}

fn is_valid_person_name(name: &str) -> bool {
    let stopwords = [
        "自己", "什么", "这里", "那里", "哪里", "这个", "那个", "他们", "我们", "你们", "没有",
        "不是", "已经", "突然",
    ];
    name.chars().count() >= 2
        && !stopwords.contains(&name)
        && !name.ends_with("里")
        && !name.ends_with("中")
}
