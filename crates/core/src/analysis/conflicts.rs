use super::extractor::{ChunkInfo, Extraction, Extractor, IssueCandidate};
use regex::Regex;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub struct EyeColorConflictExtractor;

impl Extractor for EyeColorConflictExtractor {
    fn name(&self) -> &'static str {
        "eye_color_conflict"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let Ok(pattern) = Regex::new(
            r"([\p{Han}]{2,4}).{0,12}(黑色|灰蓝色|蓝色|褐色|金色|红色|琥珀色).{0,8}(?:眼睛|眼眸|眸子)",
        ) else {
            return Vec::new();
        };

        let mut facts = HashMap::<String, BTreeMap<String, Vec<&ChunkInfo>>>::new();

        for chunk in chunks {
            for captures in pattern.captures_iter(&chunk.content) {
                let Some(person) = captures.get(1).map(|m| normalize_name(m.as_str())) else {
                    continue;
                };
                let Some(color) = captures.get(2).map(|m| m.as_str().to_string()) else {
                    continue;
                };
                facts
                    .entry(person)
                    .or_default()
                    .entry(color)
                    .or_default()
                    .push(chunk);
            }
        }

        let mut results = Vec::new();
        for (person, colors) in facts {
            if colors.len() < 2 {
                continue;
            }
            let evidence: Vec<_> = colors
                .iter()
                .filter_map(|(color, chunks)| {
                    chunks.first().map(|chunk| {
                        json!({
                            "color": color,
                            "chunk_id": chunk.chunk_id,
                            "title": chunk.title,
                            "document_path": chunk.document_path,
                            "snippet": chunk.content.chars().take(100).collect::<String>(),
                        })
                    })
                })
                .collect();

            if let Ok(evidence_json) = serde_json::to_string(&evidence) {
                results.push(Extraction::Issue(IssueCandidate {
                    issue_type: "character_attribute_conflict".to_string(),
                    severity: "high".to_string(),
                    title: format!("{person} 的眼睛颜色可能前后不一致"),
                    description: format!("{person} 出现了多个眼睛颜色候选，请依据正文确认。"),
                    evidence_json,
                    suggested_actions_json: serde_json::to_string(&json!([
                        "保持旧设定",
                        "接受新设定",
                        "标记为伪装",
                        "标记为角色认知",
                        "标记误报"
                    ]))
                    .unwrap_or_default(),
                }));
            }
        }

        results
    }
}

#[derive(Default)]
pub struct RepeatExpressionExtractor;

impl Extractor for RepeatExpressionExtractor {
    fn name(&self) -> &'static str {
        "repeat_expression"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let watched_terms = ["雨夜", "沉默", "钟声", "秘密", "黑暗"];
        let mut results = Vec::new();

        for term in watched_terms {
            let hits: Vec<_> = chunks
                .iter()
                .filter(|chunk| chunk.content.contains(term))
                .collect();
            if hits.len() < 3 {
                continue;
            }
            let evidence: Vec<_> = hits
                .iter()
                .take(5)
                .map(|chunk| {
                    json!({
                        "chunk_id": chunk.chunk_id,
                        "title": chunk.title,
                        "document_path": chunk.document_path,
                        "snippet": make_snippet(&chunk.content, term),
                    })
                })
                .collect();

            if let Ok(evidence_json) = serde_json::to_string(&evidence) {
                results.push(Extraction::Issue(IssueCandidate {
                    issue_type: "repeat_expression".to_string(),
                    severity: "low".to_string(),
                    title: format!("“{term}”反复出现"),
                    description: format!(
                        "“{term}”在多个正文片段中重复出现，可在修订时确认是否有意保留。"
                    ),
                    evidence_json,
                    suggested_actions_json: serde_json::to_string(&json!([
                        "稍后处理",
                        "标记为有意为之",
                        "创建修订任务",
                        "标记误报"
                    ]))
                    .unwrap_or_default(),
                }));
            }
        }

        results
    }
}

fn normalize_name(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | '：' | '；')
    })
    .to_string()
}

fn make_snippet(content: &str, query: &str) -> String {
    content
        .find(query)
        .map(|byte_start| {
            let char_start = content[..byte_start].chars().count();
            let chars: Vec<char> = content.chars().collect();
            let prefix = char_start.saturating_sub(18);
            let suffix = (char_start + query.chars().count() + 18).min(chars.len());
            chars[prefix..suffix].iter().collect()
        })
        .unwrap_or_else(|| content.chars().take(60).collect())
}
