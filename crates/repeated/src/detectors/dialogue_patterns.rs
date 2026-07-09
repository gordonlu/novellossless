use super::Detector;
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use std::collections::{HashMap, HashSet};

const DIALOGUE_GUIDES: &[&str] = &[
    "说",
    "问",
    "道",
    "答道",
    "问道",
    "开口",
    "压低声音",
    "笑道",
    "叹道",
    "说道",
    "解释道",
    "补充道",
    "喊道",
    "叫道",
    "继续道",
];

pub struct DialoguePatterns {
    min_chunks: usize,
}

impl Default for DialoguePatterns {
    fn default() -> Self {
        Self { min_chunks: 6 }
    }
}

fn extract_dialogue_patterns(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    for guide in DIALOGUE_GUIDES {
        let mut search_from = 0;
        while let Some(guide_pos) = content[search_from..].find(guide) {
            let abs_pos = search_from + guide_pos;
            let after = &content[abs_pos + guide.len()..];
            if after.starts_with('：') || after.starts_with(':') {
                let before_start = if abs_pos >= 8 { abs_pos - 8 } else { 0 };
                let before = &content[before_start..abs_pos];
                let speaker: String = before
                    .chars()
                    .filter(|c| c.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(c))
                    .collect();
                if !speaker.is_empty() {
                    patterns.push(format!("某某{}", guide));
                }
            }
            search_from = abs_pos + guide.len();
        }
    }
    patterns
}

impl Detector for DialoguePatterns {
    fn id(&self) -> &'static str {
        "dialogue_patterns"
    }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.len() < 2 {
            return Vec::new();
        }

        let mut pattern_chunks: HashMap<String, Vec<(usize, String)>> = HashMap::new();

        for (ci, chunk) in chunks.iter().enumerate() {
            let mut seen_in_chunk = HashSet::new();
            for pattern in extract_dialogue_patterns(&chunk.content) {
                if seen_in_chunk.insert(pattern.clone()) {
                    pattern_chunks
                        .entry(pattern)
                        .or_default()
                        .push((ci, chunk.chunk_id.clone()));
                }
            }
        }

        let mut results = Vec::new();
        for (pattern, occurrences) in &pattern_chunks {
            let unique_chunks: HashSet<usize> = occurrences.iter().map(|(ci, _)| *ci).collect();
            if unique_chunks.len() >= self.min_chunks {
                let evidence: Vec<EvidenceItem> = occurrences
                    .iter()
                    .take(5)
                    .map(|(ci, cid)| EvidenceItem {
                        chunk_id: cid.clone(),
                        chapter_title: chunks[*ci].chapter_title.clone(),
                        document_path: chunks[*ci].document_path.clone(),
                        snippet: pattern.clone(),
                        match_count: Some(unique_chunks.len() as u32),
                    })
                    .collect();
                results.push(RepeatedIssue {
                    issue_type: "dialogue".to_string(),
                    severity: "low".to_string(),
                    title: "重复对白引导模式".to_string(),
                    description: format!(
                        "模式 \"{}\" 在 {} 个不同章节中出现",
                        pattern,
                        unique_chunks.len()
                    ),
                    evidence,
                    suggested_action: "尝试使用不同的对白引导词，如'某某道'、'某某问'混合使用。"
                        .to_string(),
                });
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn chunk(id: &str, content: &str) -> ChunkInfo {
        ChunkInfo {
            chunk_id: format!("c{}", id),
            document_id: "d1".to_string(),
            document_path: "t.txt".to_string(),
            chapter_title: format!("Ch{}", id),
            chunk_index: id.parse().unwrap_or(0),
            content: content.to_string(),
        }
    }

    #[test]
    fn detects_dominant_pattern() {
        let texts: Vec<String> = (1..=7)
            .map(|i| format!("林澈说：\"这是第{}次。\"", i))
            .collect();
        let chunks: Vec<_> = texts
            .into_iter()
            .enumerate()
            .map(|(i, t)| chunk(&(i + 1).to_string(), &t))
            .collect();
        let detector = DialoguePatterns::default();
        let issues = detector.detect(&chunks);
        assert!(
            issues.iter().any(|i| i.issue_type == "dialogue"),
            "should find dominant dialogue pattern"
        );
    }

    #[test]
    fn varied_patterns_no_issue() {
        let chunks: Vec<_> = (1..=7)
            .map(|i| {
                chunk(
                    &i.to_string(),
                    &format!(
                        "{}",
                        [
                            "林澈说：\"你好。\"",
                            "沈微问：\"你来了？\"",
                            "他压低声音：\"小心。\"",
                            "她叹道：\"好吧。\"",
                            "林澈开口：\"这件事。\"",
                            "沈微继续：\"后来呢？\"",
                            "他笑道：\"没问题。\""
                        ][i as usize - 1]
                    ),
                )
            })
            .collect();
        let detector = DialoguePatterns::default();
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn empty_chunks_no_issues() {
        let detector = DialoguePatterns::default();
        let issues = detector.detect(&[]);
        assert_eq!(issues.len(), 0);
    }
}
