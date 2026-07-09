use super::Detector;
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};

pub struct PsychDensity {
    threshold_ratio: f64,
    min_chunk_len: usize,
}

impl Default for PsychDensity {
    fn default() -> Self {
        Self {
            threshold_ratio: 0.04,
            min_chunk_len: 100,
        }
    }
}

const PSYCH_KEYWORDS: &[&str] = &[
    "想",
    "觉得",
    "感到",
    "知道",
    "明白",
    "以为",
    "仿佛",
    "似乎",
    "也许",
    "大概",
    "应该",
    "可能",
    "突然",
    "忽然",
    "不知",
    "记得",
    "忘记",
    "怕",
    "担心",
    "希望",
    "期待",
    "疑惑",
    "怀疑",
    "猜测",
    "推测",
    "意识到",
    "体会到",
    "感觉到",
    "领悟到",
];

fn is_in_dialogue(text: &str, pos: usize) -> bool {
    let before = &text[..pos];
    let openings = before.matches('「').count() + before.matches('"').count();
    let closings = before.matches('」').count() + before.matches('"').count();
    openings > closings
}

impl Detector for PsychDensity {
    fn id(&self) -> &'static str {
        "psych_density"
    }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.is_empty() {
            return Vec::new();
        }

        let mut issues = Vec::new();
        for chunk in chunks {
            if chunk.content.len() < self.min_chunk_len {
                continue;
            }

            let total_chars = chunk.content.chars().count();
            if total_chars == 0 {
                continue;
            }

            let mut psych_count = 0;
            for kw in PSYCH_KEYWORDS {
                let mut search_from = 0;
                while let Some(pos) = chunk.content[search_from..].find(kw) {
                    let abs_pos = search_from + pos;
                    if !is_in_dialogue(&chunk.content, abs_pos) {
                        psych_count += 1;
                    }
                    search_from = abs_pos + kw.len();
                }
            }

            let ratio = psych_count as f64 / total_chars as f64;
            if ratio >= self.threshold_ratio {
                issues.push(RepeatedIssue {
                    issue_type: "psych_density".to_string(),
                    severity: "info".to_string(),
                    title: "过密心理描写".to_string(),
                    description: format!(
                        "本章心理描写关键词密度 {:.1}%（{} 个关键词 / {} 字），超过阈值 {}%。",
                        ratio * 100.0,
                        psych_count,
                        total_chars,
                        self.threshold_ratio * 100.0
                    ),
                    evidence: vec![EvidenceItem {
                        chunk_id: chunk.chunk_id.clone(),
                        chapter_title: chunk.chapter_title.clone(),
                        document_path: chunk.document_path.clone(),
                        snippet: chunk.content.chars().take(150).collect(),
                        match_count: Some(psych_count as u32),
                    }],
                    suggested_action: "考虑减少内心描写比例，适当增加对白或动作来推进叙事。"
                        .to_string(),
                });
            }
        }
        issues
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
    fn detects_high_density() {
        let text = "他想，她大概知道这件事。他觉得心里突然明白了什么。他以为这就是真相。也许她早就知道了。";
        let chunks = vec![chunk("1", text)];
        let detector = PsychDensity::default();
        let issues = detector.detect(&chunks);
        assert!(!issues.is_empty(), "should flag high psych density");
    }

    #[test]
    fn low_density_no_issue() {
        let text = "林澈走进房间。沈微坐在窗边。窗外下着雨。他倒了一杯茶。";
        let chunks = vec![chunk("1", text)];
        let detector = PsychDensity::default();
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn ignores_short_chunks() {
        let chunks = vec![chunk("1", "他")];
        let detector = PsychDensity::default();
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn empty_chunks_no_issues() {
        let detector = PsychDensity::default();
        let issues = detector.detect(&[]);
        assert_eq!(issues.len(), 0);
    }
}
