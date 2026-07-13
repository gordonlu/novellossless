use super::extractor::{ChunkInfo, Extraction, Extractor, NarrativeNodeCandidate};
use crate::profile::PeopleConfig;
use regex::Regex;
use std::collections::BTreeMap;

pub struct PersonExtractor {
    pub people_config: PeopleConfig,
}

impl PersonExtractor {
    pub fn new(people_config: PeopleConfig) -> Self {
        Self { people_config }
    }
}

impl Extractor for PersonExtractor {
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
                    if !self.is_valid_person_name(&name) {
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

        if self.people_config.enable_alias_detection {
            self.merge_aliases(&mut seen, chunks);
        }

        seen.into_iter()
            .filter(|(_, acc)| acc.count >= 1)
            .map(|(name, mut acc)| {
                acc.aliases.sort();
                acc.aliases.dedup();
                Extraction::Candidate(NarrativeNodeCandidate {
                    node_type: "person".to_string(),
                    name,
                    aliases: acc.aliases,
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

impl PersonExtractor {
    fn is_valid_person_name(&self, name: &str) -> bool {
        let stopwords = [
            "自己", "什么", "这里", "那里", "哪里", "这个", "那个", "他们", "我们", "你们", "没有",
            "不是", "已经", "突然", "之后", "以前", "此时", "这时", "那时", "一边",
        ];
        let len = name.chars().count();
        if len < self.people_config.min_name_length as usize
            || len > self.people_config.max_name_length as usize
        {
            return false;
        }
        if stopwords.contains(&name) {
            return false;
        }
        // Reject structural particles that indicate non-name phrases
        if name.starts_with('的')
            || name.starts_with('了')
            || name.starts_with('在')
            || name.starts_with('一')
            || name.starts_with('不')
            || name.starts_with('这')
            || name.starts_with('那')
            || name.starts_with('什')
            || name.starts_with('怎')
        {
            return false;
        }
        // Reject names ending with common sentence-final particles
        if name.ends_with('吗')
            || name.ends_with('么')
            || name.ends_with('的')
            || name.ends_with('了')
            || name.ends_with('着')
            || name.ends_with('过')
        {
            return false;
        }
        // Reject names containing possessive or structural particles mid-word
        if name.contains('的')
            || name.contains('了')
            || name.contains('在')
            || name.contains("一个")
            || name.contains("这个")
            || name.contains("那个")
            || name.contains("什么")
        {
            return false;
        }
        // Reject location/direction suffixes unlikely in person names
        if name.ends_with("里")
            || name.ends_with("中")
            || name.ends_with("上")
            || name.ends_with("下")
            || name.ends_with("前")
            || name.ends_with("后")
        {
            return false;
        }
        true
    }

    fn merge_aliases(
        &self,
        seen: &mut BTreeMap<String, CandidateAccumulator>,
        chunks: &[ChunkInfo],
    ) {
        let known_names: Vec<String> = seen.keys().cloned().collect();
        let alias_pairs: Vec<(String, String)> = known_names
            .iter()
            .flat_map(|name| {
                let mut pairs = Vec::new();
                if name.chars().count() == 2 {
                    let first_char: String = name.chars().take(1).collect();
                    pairs.push((first_char.clone() + "兄", name.clone()));
                    pairs.push((first_char + "公子", name.clone()));
                }
                pairs
            })
            .collect();

        for chunk in chunks {
            for (alias, full_name) in &alias_pairs {
                if chunk.content.contains(alias.as_str()) {
                    if let Some(entry) = seen.get_mut(full_name) {
                        entry.aliases.push(alias.clone());
                        entry.count += 1;
                    }
                }
            }
        }
    }
}

fn person_patterns() -> Result<Vec<Regex>, regex::Error> {
    vec![
        Regex::new(r"([\p{Han}]{2,4})(?:说|问|道|喊|低声|笑道|看着|走进|转身)"),
        Regex::new(r"(?:向|对|跟)([\p{Han}]{2,4})(?:说|问|道)"),
        Regex::new(r"([\p{Han}]{1,2}(?:兄|姐|弟|妹|叔|伯|婶|嫂|娘|爷|公|子|生|师|徒|君))"),
        Regex::new(r#""([\p{Han}]{2,4})[，,]"#),
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
