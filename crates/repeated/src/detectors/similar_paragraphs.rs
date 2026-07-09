use super::Detector;
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use std::collections::HashSet;

pub struct SimilarParagraphs {
    threshold: f64,
    min_paragraph_len: usize,
}

impl Default for SimilarParagraphs {
    fn default() -> Self {
        Self {
            threshold: 0.60,
            min_paragraph_len: 20,
        }
    }
}

fn split_paragraphs(text: &str) -> Vec<String> {
    text.split(|c: char| c == '。' || c == '！' || c == '？' || c == '\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn tokenize(text: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&c) {
            current.push(c);
        } else {
            if !current.is_empty() {
                set.insert(current.clone());
                current.clear();
            }
        }
    }
    if !current.is_empty() {
        set.insert(current);
    }
    set
}

fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

impl Detector for SimilarParagraphs {
    fn id(&self) -> &'static str {
        "similar_paragraphs"
    }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.len() < 2 {
            return Vec::new();
        }

        struct ParaRef {
            chunk_idx: usize,
            text: String,
        }

        let mut paras: Vec<ParaRef> = Vec::new();
        for (ci, chunk) in chunks.iter().enumerate() {
            for para in split_paragraphs(&chunk.content) {
                if para.len() >= self.min_paragraph_len {
                    paras.push(ParaRef {
                        chunk_idx: ci,
                        text: para,
                    });
                }
            }
        }

        let mut matched = vec![false; paras.len()];
        let mut issue_groups: Vec<(String, Vec<EvidenceItem>)> = Vec::new();

        for i in 0..paras.len() {
            if matched[i] {
                continue;
            }
            let ti = tokenize(&paras[i].text);
            let mut group: Vec<EvidenceItem> = Vec::new();
            group.push(EvidenceItem {
                chunk_id: chunks[paras[i].chunk_idx].chunk_id.clone(),
                chapter_title: chunks[paras[i].chunk_idx].chapter_title.clone(),
                document_path: chunks[paras[i].chunk_idx].document_path.clone(),
                snippet: paras[i].text.chars().take(120).collect(),
                match_count: None,
            });
            for j in (i + 1)..paras.len() {
                if matched[j] {
                    continue;
                }
                let tj = tokenize(&paras[j].text);
                if jaccard_similarity(&ti, &tj) >= self.threshold {
                    matched[j] = true;
                    group.push(EvidenceItem {
                        chunk_id: chunks[paras[j].chunk_idx].chunk_id.clone(),
                        chapter_title: chunks[paras[j].chunk_idx].chapter_title.clone(),
                        document_path: chunks[paras[j].chunk_idx].document_path.clone(),
                        snippet: paras[j].text.chars().take(120).collect(),
                        match_count: None,
                    });
                    if group.len() >= 5 {
                        break;
                    }
                }
            }
            if group.len() >= 2 {
                matched[i] = true;
                issue_groups.push((paras[i].text.chars().take(60).collect(), group));
            }
        }

        issue_groups
            .into_iter()
            .map(|(_sample, evidence)| RepeatedIssue {
                issue_type: "paragraph".to_string(),
                severity: "low".to_string(),
                title: "重复场景描写".to_string(),
                description: format!("以下段落出现在 {} 个不同章节，内容高度相似", evidence.len()),
                evidence,
                suggested_action: "检查是否为刻意重复的意象，可考虑删减或合并。".to_string(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChunkInfo;

    fn make_chunk(id: &str, content: &str) -> ChunkInfo {
        ChunkInfo {
            chunk_id: id.to_string(),
            document_id: "doc1".to_string(),
            document_path: "test.txt".to_string(),
            chapter_title: format!("Chapter {}", id),
            chunk_index: id.parse().unwrap_or(0),
            content: content.to_string(),
        }
    }

    #[test]
    fn detects_identical_paragraph() {
        let text = "雨夜，霓虹灯在雾气中晕开一片模糊的光。";
        let chunks = vec![
            make_chunk("1", &format!("前面的话。{}后面的话。", text)),
            make_chunk("2", &format!("不同的开头。{}不同的结尾。", text)),
        ];
        let detector = SimilarParagraphs::default();
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 1, "should find one repeated paragraph");
        assert!(
            issues[0].evidence.len() >= 2,
            "should have at least 2 evidence items"
        );
    }

    #[test]
    fn no_false_positive_different_content() {
        let chunks = vec![
            make_chunk("1", "林澈推开木门，雨声扑面而来。屋内烛火摇曳。"),
            make_chunk("2", "沈微站在城楼上，远眺群山。风将她的披风吹起。"),
        ];
        let detector = SimilarParagraphs::default();
        let issues = detector.detect(&chunks);
        assert_eq!(
            issues.len(),
            0,
            "completely different content should not match"
        );
    }

    #[test]
    fn ignores_short_paragraph() {
        let chunks = vec![make_chunk("1", "你好。"), make_chunk("2", "你好。")];
        let detector = SimilarParagraphs::default();
        let issues = detector.detect(&chunks);
        assert_eq!(
            issues.len(),
            0,
            "paragraphs under 20 chars should be ignored"
        );
    }

    #[test]
    fn empty_chunks_no_issues() {
        let detector = SimilarParagraphs::default();
        let issues = detector.detect(&[]);
        assert_eq!(issues.len(), 0);
    }
}
