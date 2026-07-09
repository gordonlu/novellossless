use super::Detector;
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use std::collections::HashMap;

pub struct RepeatedActions {
    min_chunks: usize,
}

impl Default for RepeatedActions {
    fn default() -> Self {
        Self { min_chunks: 4 }
    }
}

const ACTION_VERBS: &[&str] = &[
    "拿起",
    "推开",
    "放下",
    "抱住",
    "握住",
    "拉着",
    "拍了拍",
    "点了点头",
    "摇了摇头",
    "站起身",
    "转过身",
    "低下头",
    "抬起头",
    "握紧",
    "松开",
    "扔下",
    "接过",
    "取出",
    "收起",
    "拔出",
    "插入",
];

impl Detector for RepeatedActions {
    fn id(&self) -> &'static str {
        "repeated_actions"
    }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.len() < 2 {
            return Vec::new();
        }

        let mut chunk_actions: Vec<Vec<(String, String)>> =
            chunks.iter().map(|_| Vec::new()).collect();

        for (ci, chunk) in chunks.iter().enumerate() {
            for verb in ACTION_VERBS {
                let mut search_from = 0;
                while let Some(pos) = chunk.content[search_from..].find(verb) {
                    let abs_pos = search_from + pos;
                    let start = abs_pos.saturating_sub(10);
                    let end = (abs_pos + verb.len() + 15).min(chunk.content.len());
                    let snippet = &chunk.content[start..end];
                    chunk_actions[ci].push((verb.to_string(), snippet.to_string()));
                    search_from = abs_pos + verb.len();
                }
            }
        }

        let mut verb_chunks: HashMap<String, Vec<EvidenceItem>> = HashMap::new();
        for (ci, actions) in chunk_actions.iter().enumerate() {
            for (verb, _) in actions {
                let entry = verb_chunks.entry(verb.clone()).or_default();
                if !entry.iter().any(|e| e.chunk_id == chunks[ci].chunk_id) {
                    let snippet = actions
                        .iter()
                        .find(|(v, _)| v == verb)
                        .map(|(_, s)| s.clone())
                        .unwrap_or_default();
                    entry.push(EvidenceItem {
                        chunk_id: chunks[ci].chunk_id.clone(),
                        chapter_title: chunks[ci].chapter_title.clone(),
                        document_path: chunks[ci].document_path.clone(),
                        snippet,
                        match_count: None,
                    });
                }
            }
        }

        let mut results = Vec::new();
        for (verb, evidence) in verb_chunks {
            if evidence.len() >= self.min_chunks {
                results.push(RepeatedIssue {
                    issue_type: "action".to_string(),
                    severity: "low".to_string(),
                    title: format!("重复动作：{}", verb),
                    description: format!("\"{}\" 出现在 {} 个不同章节", verb, evidence.len()),
                    evidence,
                    suggested_action: "检查是否为刻意重复，可考虑替换同义动作。".to_string(),
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
    fn detects_repeated_action() {
        let chunks: Vec<_> = (1..=5)
            .map(|i| {
                chunk(
                    &i.to_string(),
                    &format!(
                        "他拿起剑，{}。",
                        if i == 3 {
                            "转身离开"
                        } else {
                            "走向门口"
                        }
                    ),
                )
            })
            .collect();
        let detector = RepeatedActions::default();
        let issues = detector.detect(&chunks);
        assert!(
            issues.iter().any(|i| i.issue_type == "action"),
            "should find repeated action"
        );
    }

    #[test]
    fn single_chunk_no_issue() {
        let detector = RepeatedActions::default();
        let chunks = vec![chunk("1", "他拿起剑。")];
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn different_actions_no_match() {
        let chunks: Vec<_> = (1..=5)
            .map(|i| {
                chunk(
                    &i.to_string(),
                    &format!(
                        "他{}。",
                        ["拿起剑", "推开门", "走向窗口", "坐在椅上", "躺了下来"][i as usize - 1]
                    ),
                )
            })
            .collect();
        let detector = RepeatedActions::default();
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn empty_chunks_no_issues() {
        let detector = RepeatedActions::default();
        let issues = detector.detect(&[]);
        assert_eq!(issues.len(), 0);
    }
}
