use super::Detector;
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use std::collections::HashMap;

pub struct HighFreqExpressions {
    min_chunks: usize,
    max_results: usize,
}

impl Default for HighFreqExpressions {
    fn default() -> Self {
        Self {
            min_chunks: 5,
            max_results: 15,
        }
    }
}

fn sentences(text: &str) -> Vec<String> {
    text.split(|c: char| c == '。' || c == '！' || c == '？' || c == '\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn word_ngrams(words: &[String], n: usize) -> Vec<String> {
    words.windows(n).map(|w| w.join("")).collect()
}

impl Detector for HighFreqExpressions {
    fn id(&self) -> &'static str {
        "high_freq_expressions"
    }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.len() < 2 {
            return Vec::new();
        }

        let mut chunk_sentences: Vec<(usize, String)> = Vec::new();
        for (ci, chunk) in chunks.iter().enumerate() {
            for s in sentences(&chunk.content) {
                chunk_sentences.push((ci, s));
            }
        }

        let mut ngram_chunks: HashMap<String, Vec<usize>> = HashMap::new();
        for (ci, s) in &chunk_sentences {
            let words: Vec<String> = s
                .chars()
                .filter(|c| c.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(c))
                .map(|c| c.to_string())
                .collect();
            for n in 4..=10 {
                if words.len() < n {
                    break;
                }
                let mut seen_in_this_chunk = false;
                for ng in word_ngrams(&words, n) {
                    if ng.len() < 4 {
                        continue;
                    }
                    let entry = ngram_chunks.entry(ng).or_default();
                    if !seen_in_this_chunk || entry.last() != Some(ci) {
                        entry.push(*ci);
                        seen_in_this_chunk = true;
                    }
                }
            }
        }

        let mut ranked: Vec<(String, usize)> = ngram_chunks
            .into_iter()
            .filter(|(_, chunks)| chunks.len() >= self.min_chunks)
            .map(|(phrase, chunks)| {
                let unique_chunks: Vec<usize> = {
                    let mut v = chunks.clone();
                    v.sort();
                    v.dedup();
                    v
                };
                (phrase, unique_chunks.len())
            })
            .collect();

        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked.truncate(self.max_results);

        if ranked.is_empty() {
            return Vec::new();
        }

        let top = ranked.first().unwrap();
        let mut evidence = Vec::new();
        for chunk in chunks {
            if chunk.content.contains(&top.0) {
                evidence.push(EvidenceItem {
                    chunk_id: chunk.chunk_id.clone(),
                    chapter_title: chunk.chapter_title.clone(),
                    document_path: chunk.document_path.clone(),
                    snippet: chunk.content.chars().take(100).collect(),
                    match_count: Some(top.1 as u32),
                });
                if evidence.len() >= 5 {
                    break;
                }
            }
        }

        let extra_count = ranked.len().saturating_sub(1);
        let desc = if extra_count > 0 {
            format!(
                "高频表达 \"{}\" 出现在 {} 个章节。另有 {} 个高频短语。",
                top.0, top.1, extra_count
            )
        } else {
            format!("高频表达 \"{}\" 出现在 {} 个章节。", top.0, top.1)
        };

        vec![RepeatedIssue {
            issue_type: "expression".to_string(),
            severity: "low".to_string(),
            title: "高频重复表达".to_string(),
            description: desc,
            evidence,
            suggested_action: "检查是否为刻意重复的修辞，可考虑替换部分表达。".to_string(),
        }]
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
    fn detects_frequent_phrase() {
        let phrase = "沉默地站在窗前";
        let chunks: Vec<_> = (1..=6)
            .map(|i| chunk(&i.to_string(), &format!("一些文字。{}更多内容。", phrase)))
            .collect();
        let detector = HighFreqExpressions::default();
        let issues = detector.detect(&chunks);
        assert!(
            issues.iter().any(|i| i.issue_type == "expression"),
            "should find repeated phrase"
        );
    }

    #[test]
    fn single_chunk_no_issue() {
        let detector = HighFreqExpressions::default();
        let chunks = vec![chunk("1", "沉默地站在窗前。")];
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn different_phrases_no_match() {
        let texts = vec![
            "春天来了，万物复苏。",
            "大海广阔，波涛汹涌。",
            "高山巍峨，云雾缭绕。",
            "星空璀璨，银河闪耀。",
            "森林茂密，鸟语花香。",
            "沙漠无垠，烈日当空。",
        ];
        let chunks: Vec<_> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| chunk(&i.to_string(), t))
            .collect();
        let detector = HighFreqExpressions::default();
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn empty_chunks_no_issues() {
        let detector = HighFreqExpressions::default();
        let issues = detector.detect(&[]);
        assert_eq!(issues.len(), 0);
    }
}
