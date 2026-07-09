# Repeated Description Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 5 detectors for repeated descriptions (paragraph similarity, high-frequency expressions, repeated actions, dialogue patterns, psych description density) as a standalone crate.

**Architecture:** New `crates/repeated` crate with `Detector` trait → 5 detector implementations → `RepeatedDescriptionEngine` → called from `analyze_project()` in core → issues stored in existing `continuity_issues` table → surfaced by existing `list_issues` Tauri command.

**Tech Stack:** Rust, serde + serde_json only external deps.

## Global Constraints

- No external ML/embedding dependencies (no ndarray, candle, tokenizers)
- `ChunkInfo` defined within `crates/repeated` — lightweight view struct, no dependency on core or storage types
- All detectors receive `&[ChunkInfo]`, return `Vec<RepeatedIssue>`
- issue_type values prefixed `repeated_` to avoid uniqueness conflicts with existing extractors
- Integration goes into `analyze_project()` at line ~928 in `crates/core/src/lib.rs` (after extractor issues are upserted, before rules integration)

---

### Task 1: Crate skeleton + types

**Files:**
- Create: `crates/repeated/Cargo.toml`
- Create: `crates/repeated/src/lib.rs`
- Create: `crates/repeated/src/types.rs`
- Modify: `Cargo.toml` (workspace root — add `"crates/repeated"` to members)

**Interfaces:**
- Consumes: nothing
- Produces: `RepeatedIssue`, `EvidenceItem`, `ChunkInfo` types

- [ ] **Step 1: Create crate directory and Cargo.toml**

```bash
mkdir -p crates/repeated/src
```

`crates/repeated/Cargo.toml`:
```toml
[package]
name = "novellossless-repeated"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
```

- [ ] **Step 2: Write `crates/repeated/src/types.rs`**

```rust
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub chunk_id: String,
    pub document_id: String,
    pub document_path: String,
    pub chapter_title: String,
    pub chunk_index: i64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepeatedIssue {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence: Vec<EvidenceItem>,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceItem {
    pub chunk_id: String,
    pub chapter_title: String,
    pub document_path: String,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_count: Option<u32>,
}
```

- [ ] **Step 3: Write `crates/repeated/src/lib.rs`**

```rust
pub mod types;
mod detectors;

pub use detectors::Detector;
pub use types::{ChunkInfo, EvidenceItem, RepeatedIssue};

pub struct RepeatedDescriptionEngine {
    detectors: Vec<Box<dyn Detector>>,
}

impl RepeatedDescriptionEngine {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    pub fn with(mut self, detector: Box<dyn Detector>) -> Self {
        self.detectors.push(detector);
        self
    }

    pub fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.is_empty() {
            return Vec::new();
        }
        let mut results = Vec::new();
        for detector in &self.detectors {
            results.extend(detector.detect(chunks));
        }
        results
    }

    pub fn default() -> Self {
        Self {
            detectors: vec![
                Box::new(detectors::SimilarParagraphs::default()),
                Box::new(detectors::HighFreqExpressions::default()),
                Box::new(detectors::RepeatedActions::default()),
                Box::new(detectors::DialoguePatterns::default()),
                Box::new(detectors::PsychDensity::default()),
            ],
        }
    }
}

impl Default for RepeatedDescriptionEngine {
    fn default() -> Self {
        Self::default()
    }
}
```

- [ ] **Step 4: Add `crates/repeated` to workspace members in root `Cargo.toml`**

Edit `/home/gordon/code/novellossless/Cargo.toml` — add `"crates/repeated"` after `"crates/parser"`:
```toml
members = [
    "apps/cli",
    "apps/desktop/src-tauri",
    "crates/core",
    "crates/parser",
    "crates/repeated",
    "crates/profiles",
    "crates/impact",
    "crates/rules",
    "crates/storage",
    "crates/tasks",
    "crates/timeline",
    "crates/ai",
]
```

- [ ] **Step 5: Create `crates/repeated/src/detectors/mod.rs`**

```rust
mod similar_paragraphs;
mod high_freq_expressions;
mod repeated_actions;
mod dialogue_patterns;
mod psych_density;

pub use similar_paragraphs::SimilarParagraphs;
pub use high_freq_expressions::HighFreqExpressions;
pub use repeated_actions::RepeatedActions;
pub use dialogue_patterns::DialoguePatterns;
pub use psych_density::PsychDensity;

use crate::types::{ChunkInfo, RepeatedIssue};

pub trait Detector {
    fn id(&self) -> &'static str;
    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue>;
}
```

- [ ] **Step 6: Verify crate compiles**

Run: `cargo check -p novellossless-repeated`
Expected: success (no tests yet, just compiles)

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(repeated): create crate skeleton with types and detector trait"
```

---

### Task 2: SimilarParagraphs detector

**Files:**
- Create: `crates/repeated/src/detectors/similar_paragraphs.rs`
- Modify: `crates/repeated/src/detectors/mod.rs` (already listed in Task 1)

**Interfaces:**
- Consumes: `ChunkInfo`, paragraph-level Jaccard similarity helper
- Produces: `SimilarParagraphs` implementing `Detector`

- [ ] **Step 1: Write the test block**

Append to `similar_paragraphs.rs`:
```rust
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
        assert_eq!(issues[0].issue_type, "paragraph");
        assert!(issues[0].evidence.len() >= 2, "should have at least 2 evidence items");
    }

    #[test]
    fn no_false_positive_different_content() {
        let chunks = vec![
            make_chunk("1", "林澈推开木门，雨声扑面而来。屋内烛火摇曳。"),
            make_chunk("2", "沈微站在城楼上，远眺群山。风将她的披风吹起。"),
        ];
        let detector = SimilarParagraphs::default();
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0, "completely different content should not match");
    }

    #[test]
    fn ignores_short_paragraph() {
        let chunks = vec![
            make_chunk("1", "你好。"),
            make_chunk("2", "你好。"),
        ];
        let detector = SimilarParagraphs::default();
        let issues = detector.detect(&chunks);
        assert_eq!(issues.len(), 0, "paragraphs under 20 chars should be ignored");
    }

    #[test]
    fn empty_chunks_no_issues() {
        let detector = SimilarParagraphs::default();
        let issues = detector.detect(&[]);
        assert_eq!(issues.len(), 0);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p novellossless-repeated -- similar_paragraphs 2>&1`
Expected: compile errors (no SimilarParagraphs defined)

- [ ] **Step 3: Write implementation**

Helper: paragraph splitting on `[。！？\n]`. Tokenization: split on whitespace + common Chinese delimiters (non-CJK-chars as boundaries for Chinese text). Jaccard: intersection size / union size of token sets. Threshold ≥ 0.60.

```rust
use std::collections::HashSet;
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use super::Detector;

pub struct SimilarParagraphs {
    threshold: f64,
    min_paragraph_len: usize,
}

impl Default for SimilarParagraphs {
    fn default() -> Self {
        Self { threshold: 0.60, min_paragraph_len: 20 }
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
        if c.is_alphanumeric() || c.is_ascii_whitespace() || c == '\u{4e00}'..='\u{9fff}' {
            // Keep CJK chars and alphanumeric
            if c.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&c) {
                current.push(c);
            } else if !current.is_empty() {
                set.insert(current.clone());
                current.clear();
            }
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
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

impl Detector for SimilarParagraphs {
    fn id(&self) -> &'static str { "similar_paragraphs" }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.len() < 2 {
            return Vec::new();
        }

        // Extract all paragraphs with their chunk reference
        struct ParaRef {
            chunk_idx: usize,
            para_idx: usize,
            text: String,
        }

        let mut paras: Vec<ParaRef> = Vec::new();
        for (ci, chunk) in chunks.iter().enumerate() {
            for (pi, para) in split_paragraphs(&chunk.content).iter().enumerate() {
                if para.len() >= self.min_paragraph_len {
                    paras.push(ParaRef {
                        chunk_idx: ci,
                        para_idx: pi,
                        text: para.clone(),
                    });
                }
            }
        }

        let mut matched = vec![false; paras.len()];
        let mut issue_groups: Vec<(usize, String, Vec<EvidenceItem>)> = Vec::new();

        for i in 0..paras.len() {
            if matched[i] { continue; }
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
                if matched[j] { continue; }
                let tj = tokenize(&paras[j].text);
                let sim = jaccard_similarity(&ti, &tj);
                if sim >= self.threshold {
                    matched[j] = true;
                    group.push(EvidenceItem {
                        chunk_id: chunks[paras[j].chunk_idx].chunk_id.clone(),
                        chapter_title: chunks[paras[j].chunk_idx].chapter_title.clone(),
                        document_path: chunks[paras[j].chunk_idx].document_path.clone(),
                        snippet: paras[j].text.chars().take(120).collect(),
                        match_count: None,
                    });
                    if group.len() >= 5 { break; }
                }
            }
            if group.len() >= 2 {
                matched[i] = true;
                issue_groups.push((paras[i].chunk_idx, paras[i].text.chars().take(60).collect(), group));
            }
        }

        let mut results = Vec::new();
        for (_, sample, evidence) in issue_groups {
            results.push(RepeatedIssue {
                issue_type: "paragraph".to_string(),
                severity: "low".to_string(),
                title: "重复场景描写".to_string(),
                description: format!("以下段落出现在 {} 个不同章节，内容高度相似", evidence.len()),
                evidence,
                suggested_action: "检查是否为刻意重复的意象，可考虑删减或合并。".to_string(),
            });
        }
        results
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p novellossless-repeated -- similar_paragraphs`
Expected: 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(repeated): add SimilarParagraphs detector"
```

---

### Task 3: HighFreqExpressions detector

**Files:**
- Create: `crates/repeated/src/detectors/high_freq_expressions.rs`
- Modify: `crates/repeated/src/detectors/mod.rs` (nothing to change — already listed)

**Interfaces:**
- Consumes: `ChunkInfo`
- Produces: `HighFreqExpressions` implementing `Detector`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn chunk(id: &str, content: &str) -> ChunkInfo {
        ChunkInfo {
            chunk_id: format!("c{}", id), document_id: "d1".to_string(),
            document_path: "t.txt".to_string(),
            chapter_title: format!("Ch{}", id), chunk_index: id.parse().unwrap_or(0),
            content: content.to_string(),
        }
    }

    #[test]
    fn detects_frequent_phrase() {
        let phrase = "沉默地站在窗前";
        let chunks: Vec<_> = (1..=6).map(|i| chunk(&i.to_string(), &format!("一些文字。{}更多内容。", phrase))).collect();
        let detector = HighFreqExpressions::default();
        let issues = detector.detect(&chunks);
        assert!(issues.iter().any(|i| i.issue_type == "expression"), "should find repeated phrase");
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
        let chunks: Vec<_> = (1..=6).map(|i| chunk(&i.to_string(), &format!("这是第{}章的不同内容。", i))).collect();
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
```

- [ ] **Step 2: Run to verify fail**

Run: `cargo test -p novellossless-repeated -- high_freq_expressions`
Expected: compile error

- [ ] **Step 3: Write implementation**

```rust
use std::collections::HashMap;
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use super::Detector;

pub struct HighFreqExpressions {
    min_chunks: usize,
    max_results: usize,
}

impl Default for HighFreqExpressions {
    fn default() -> Self { Self { min_chunks: 5, max_results: 15 } }
}

fn sentences(text: &str) -> Vec<String> {
    text.split(|c: char| c == '。' || c == '！' || c == '？' || c == '\n')
        .map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string()).collect()
}

fn word_ngrams(words: &[String], n: usize) -> Vec<String> {
    words.windows(n).map(|w| w.join("")).collect()
}

impl Detector for HighFreqExpressions {
    fn id(&self) -> &'static str { "high_freq_expressions" }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.len() < 2 { return Vec::new(); }

        // Collect all sentences across chunks
        let mut chunk_sentences: Vec<(usize, String)> = Vec::new();
        for (ci, chunk) in chunks.iter().enumerate() {
            for s in sentences(&chunk.content) {
                chunk_sentences.push((ci, s));
            }
        }

        // Build n-gram -> set of chunk indices (dedup within chunk)
        let mut ngram_chunks: HashMap<String, Vec<usize>> = HashMap::new();
        for (ci, s) in &chunk_sentences {
            let words: Vec<String> = s.chars()
                .filter(|c| c.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(c))
                .map(|c| c.to_string())
                .collect();
            for n in 4..=10 {
                if words.len() < n { break; }
                let mut seen_in_this_chunk = false;
                for ng in word_ngrams(&words, n) {
                    if ng.len() < 4 { continue; }
                    let entry = ngram_chunks.entry(ng).or_default();
                    if !seen_in_this_chunk || entry.last() != Some(ci) {
                        entry.push(*ci);
                        seen_in_this_chunk = true;
                    }
                }
            }
        }

        let mut ranked: Vec<(String, usize)> = ngram_chunks.into_iter()
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

        if ranked.is_empty() { return Vec::new(); }

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
                if evidence.len() >= 5 { break; }
            }
        }

        let extra_count = ranked.len().saturating_sub(1);
        let desc = if extra_count > 0 {
            format!("高频表达 "{}" 出现在 {} 个章节。另有 {} 个高频短语。", top.0, top.1, extra_count)
        } else {
            format!("高频表达 "{}" 出现在 {} 个章节。", top.0, top.1)
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
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p novellossless-repeated -- high_freq_expressions`
Expected: 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(repeated): add HighFreqExpressions detector"
```

---

### Task 4: RepeatedActions detector

**Files:**
- Create: `crates/repeated/src/detectors/repeated_actions.rs`

**Interfaces:**
- Consumes: `ChunkInfo`
- Produces: `RepeatedActions` implementing `Detector`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn chunk(id: &str, content: &str) -> ChunkInfo { /* same as Task 2 */ ChunkInfo { chunk_id: format!("c{}", id), document_id: "d1".to_string(), document_path: "t.txt".to_string(), chapter_title: format!("Ch{}", id), chunk_index: id.parse().unwrap_or(0), content: content.to_string() } }

    #[test]
    fn detects_repeated_action() {
        let chunks: Vec<_> = (1..=5).map(|i| chunk(&i.to_string(), &format!("他拿起剑，{}。", if i == 3 { "转身离开" } else { "走向门口" }))).collect();
        let detector = RepeatedActions::default();
        let issues = detector.detect(&chunks);
        assert!(issues.iter().any(|i| i.issue_type == "action"), "should find repeated action");
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
        let chunks: Vec<_> = (1..=5).map(|i| chunk(&i.to_string(), &format!("他{}。", ["拿起剑","推开门","走向窗口","坐在椅上","躺了下来"][i as usize - 1]))).collect();
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
```

- [ ] **Step 2: Run to verify fail**

Run: `cargo test -p novellossless-repeated -- repeated_actions`
Expected: compile error

- [ ] **Step 3: Write implementation**

```rust
use std::collections::HashMap;
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use super::Detector;

pub struct RepeatedActions {
    min_chunks: usize,
}

impl Default for RepeatedActions {
    fn default() -> Self { Self { min_chunks: 4 } }
}

// Known Chinese action verbs
const ACTION_VERBS: &[&str] = &["拿起", "推开", "放下", "抱住", "握住", "拉着",
    "拍了拍", "点了点头", "摇了摇头", "站起身", "转过身", "低下头", "抬起头",
    "握紧", "松开", "扔下", "接过", "取出", "收起", "拔出", "插入"];

impl Detector for RepeatedActions {
    fn id(&self) -> &'static str { "repeated_actions" }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.len() < 2 { return Vec::new(); }

        // For each chunk, find all action patterns
        struct ActionMatch {
            verb: String,
            snippet: String,
        }

        let mut chunk_actions: Vec<Vec<ActionMatch>> = chunks.iter().map(|_| Vec::new()).collect();

        for (ci, chunk) in chunks.iter().enumerate() {
            for verb in ACTION_VERBS {
                let mut search_from = 0;
                while let Some(pos) = chunk.content[search_from..].find(verb) {
                    let abs_pos = search_from + pos;
                    let start = abs_pos.saturating_sub(10);
                    let end = (abs_pos + verb.len() + 15).min(chunk.content.len());
                    let snippet = &chunk.content[start..end];
                    chunk_actions[ci].push(ActionMatch {
                        verb: verb.to_string(),
                        snippet: snippet.to_string(),
                    });
                    search_from = abs_pos + verb.len();
                }
            }
        }

        // Group by verb across chunks
        let mut verb_chunks: HashMap<String, Vec<EvidenceItem>> = HashMap::new();
        for (ci, actions) in chunk_actions.iter().enumerate() {
            for action in actions {
                let entry = verb_chunks.entry(action.verb.clone()).or_default();
                // Only add this chunk once per verb
                if !entry.iter().any(|e| e.chunk_id == chunks[ci].chunk_id) {
                    entry.push(EvidenceItem {
                        chunk_id: chunks[ci].chunk_id.clone(),
                        chapter_title: chunks[ci].chapter_title.clone(),
                        document_path: chunks[ci].document_path.clone(),
                        snippet: action.snippet.clone(),
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
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p novellossless-repeated -- repeated_actions`
Expected: 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(repeated): add RepeatedActions detector"
```

---

### Task 5: DialoguePatterns detector

**Files:**
- Create: `crates/repeated/src/detectors/dialogue_patterns.rs`

**Interfaces:**
- Consumes: `ChunkInfo`
- Produces: `DialoguePatterns` implementing `Detector`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn chunk(id: &str, content: &str) -> ChunkInfo { /* same */ ChunkInfo { chunk_id: format!("c{}", id), document_id: "d1".to_string(), document_path: "t.txt".to_string(), chapter_title: format!("Ch{}", id), chunk_index: id.parse().unwrap_or(0), content: content.to_string() } }

    #[test]
    fn detects_dominant_pattern() {
        let texts: Vec<String> = (1..=7).map(|i| format!("林澈说：\"这是第{}次。\"", i)).collect();
        let chunks: Vec<_> = texts.into_iter().enumerate().map(|(i, t)| chunk(&(i+1).to_string(), &t)).collect();
        let detector = DialoguePatterns::default();
        let issues = detector.detect(&chunks);
        assert!(issues.iter().any(|i| i.issue_type == "dialogue"), "should find dominant dialogue pattern");
    }

    #[test]
    fn varied_patterns_no_issue() {
        let chunks: Vec<_> = (1..=7).map(|i| chunk(&i.to_string(), &format!("{}", [
            "林澈说：\"你好。\"",
            "沈微问：\"你来了？\"",
            "他压低声音：\"小心。\"",
            "她叹道：\"好吧。\"",
            "林澈开口：\"这件事。\"",
            "沈微继续：\"后来呢？\"",
            "他笑道：\"没问题。\""
        ][i as usize - 1]))).collect();
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
```

- [ ] **Step 2: Run to verify fail**

Run: `cargo test -p novellossless-repeated -- dialogue_patterns`
Expected: compile error

- [ ] **Step 3: Write implementation**

```rust
use std::collections::{HashMap, HashSet};
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use super::Detector;

const DIALOGUE_GUIDES: &[&str] = &[
    "说", "问", "道", "答道", "问道", "开口", "压低声音", "笑道", "叹道",
    "说道", "解释道", "补充道", "喊道", "叫道", "继续道",
];

pub struct DialoguePatterns {
    min_chunks: usize,
}

impl Default for DialoguePatterns {
    fn default() -> Self { Self { min_chunks: 6 } }
}

fn extract_dialogue_patterns(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    // Find all positions where dialogue guide word appears before ：or :
    for guide in DIALOGUE_GUIDES {
        let mut search_from = 0;
        while let Some(guide_pos) = content[search_from..].find(guide) {
            let abs_pos = search_from + guide_pos;
            let after = &content[abs_pos + guide.len()..];
            // Check if followed by ：or :
            if after.starts_with('：') || after.starts_with(':') {
                // Extract the speaker (1-8 chars before the guide)
                let before_start = if abs_pos >= 8 { abs_pos - 8 } else { 0 };
                let before = &content[before_start..abs_pos];
                // Look for the last CJK/non-punctuation block
                let speaker: String = before.chars()
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
    fn id(&self) -> &'static str { "dialogue_patterns" }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.len() < 2 { return Vec::new(); }

        // Collect unique patterns per chunk
        let mut pattern_chunks: HashMap<String, Vec<(usize, String)>> = HashMap::new();

        for (ci, chunk) in chunks.iter().enumerate() {
            let mut seen_in_chunk = HashSet::new();
            for pattern in extract_dialogue_patterns(&chunk.content) {
                if seen_in_chunk.insert(pattern.clone()) {
                    pattern_chunks.entry(pattern).or_default().push((ci, chunk.chunk_id.clone()));
                }
            }
        }

        let mut results = Vec::new();
        for (pattern, occurrences) in &pattern_chunks {
            let unique_chunks: HashSet<usize> = occurrences.iter().map(|(ci, _)| *ci).collect();
            if unique_chunks.len() >= self.min_chunks {
                let evidence: Vec<EvidenceItem> = occurrences.iter().take(5).map(|(ci, cid)| {
                    EvidenceItem {
                        chunk_id: cid.clone(),
                        chapter_title: chunks[*ci].chapter_title.clone(),
                        document_path: chunks[*ci].document_path.clone(),
                        snippet: pattern.clone(),
                        match_count: Some(unique_chunks.len() as u32),
                    }
                }).collect();

                results.push(RepeatedIssue {
                    issue_type: "dialogue".to_string(),
                    severity: "low".to_string(),
                    title: "重复对白引导模式".to_string(),
                    description: format!("模式 \"{}\" 在 {} 个不同章节中出现", pattern, unique_chunks.len()),
                    evidence,
                    suggested_action: "尝试使用不同的对白引导词，如'某某道'、'某某问'混合使用。".to_string(),
                });
            }
        }

        results
    }
}
```

No additional crate deps needed — only uses std collections.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p novellossless-repeated -- dialogue_patterns`
Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(repeated): add DialoguePatterns detector"
```

---

### Task 6: PsychDensity detector

**Files:**
- Create: `crates/repeated/src/detectors/psych_density.rs`

**Interfaces:**
- Consumes: `ChunkInfo`
- Produces: `PsychDensity` implementing `Detector`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn chunk(id: &str, content: &str) -> ChunkInfo { ChunkInfo { chunk_id: format!("c{}", id), document_id: "d1".to_string(), document_path: "t.txt".to_string(), chapter_title: format!("Ch{}", id), chunk_index: id.parse().unwrap_or(0), content: content.to_string() } }

    #[test]
    fn detects_high_density() {
        // ~5% psych keywords (5 keywords in ~100 chars)
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
```

- [ ] **Step 2: Run to verify fail**

Run: `cargo test -p novellossless-repeated -- psych_density`
Expected: compile error

- [ ] **Step 3: Write implementation**

```rust
use crate::types::{ChunkInfo, EvidenceItem, RepeatedIssue};
use super::Detector;

pub struct PsychDensity {
    threshold_ratio: f64,
    min_chunk_len: usize,
}

impl Default for PsychDensity {
    fn default() -> Self { Self { threshold_ratio: 0.04, min_chunk_len: 100 } }
}

const PSYCH_KEYWORDS: &[&str] = &[
    "想", "觉得", "感到", "知道", "明白", "以为", "仿佛", "似乎", "也许",
    "大概", "应该", "可能", "突然", "忽然", "不知", "记得", "忘记",
    "怕", "担心", "希望", "期待", "疑惑", "怀疑", "猜测", "推测",
    "意识到", "体会到", "感觉到", "领悟到",
];

fn is_in_dialogue(text: &str, pos: usize) -> bool {
    // Check if position is within 「」 or ""
    let before = &text[..pos];
    let openings = before.matches('「').count() + before.matches('"').count();
    let closings = before.matches('」').count() + before.matches('"').count();
    openings > closings
}

impl Detector for PsychDensity {
    fn id(&self) -> &'static str { "psych_density" }

    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue> {
        if chunks.is_empty() { return Vec::new(); }

        let mut issues = Vec::new();
        for chunk in chunks {
            if chunk.content.len() < self.min_chunk_len { continue; }

            let total_chars = chunk.content.chars().count();
            if total_chars == 0 { continue; }

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
                        ratio * 100.0, psych_count, total_chars, self.threshold_ratio * 100.0
                    ),
                    evidence: vec![EvidenceItem {
                        chunk_id: chunk.chunk_id.clone(),
                        chapter_title: chunk.chapter_title.clone(),
                        document_path: chunk.document_path.clone(),
                        snippet: chunk.content.chars().take(150).collect(),
                        match_count: Some(psych_count as u32),
                    }],
                    suggested_action: "考虑减少内心描写比例，适当增加对白或动作来推进叙事。".to_string(),
                });
            }
        }

        issues
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p novellossless-repeated -- psych_density`
Expected: 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(repeated): add PsychDensity detector"
```

---

### Task 7: Integration into core's analyze_project

**Files:**
- Modify: `crates/core/Cargo.toml` — add `novellossless-repeated` dep
- Modify: `crates/core/src/lib.rs` — add import + call in `analyze_project()`

**Interfaces:**
- Consumes: `RepeatedDescriptionEngine::default()`, `ChunkInfo` from core's existing `chunks` vec
- Produces: issues in `continuity_issues` with `issue_type` prefix `repeated_`

- [ ] **Step 1: Add dep to core/Cargo.toml**

Add after `novellossless-ai` line:
```toml
novellossless-repeated = { path = "../repeated" }
```

- [ ] **Step 2: Add import in core lib.rs**

Add near top imports:
```rust
use novellossless_repeated::{ChunkInfo as RepeatedChunkInfo, RepeatedDescriptionEngine};
```

- [ ] **Step 3: Insert detection call in analyze_project**

After line 927 (`self.storage.upsert_continuity_issues(project_id, &issues)?;`), before the rules integration comment:
```rust
        // Repeated description detection
        if !chunks.is_empty() {
            let repeated_chunks: Vec<RepeatedChunkInfo> = chunks
                .iter()
                .map(|c| RepeatedChunkInfo {
                    chunk_id: c.chunk_id.clone(),
                    document_id: c.document_id.clone(),
                    document_path: c.document_path.clone(),
                    chapter_title: c.title.clone(),
                    chunk_index: c.chunk_index,
                    content: c.content.clone(),
                })
                .collect();
            let engine = RepeatedDescriptionEngine::default();
            let repeated_issues = engine.detect(&repeated_chunks);
            for ri in repeated_issues {
                issues.push(NewContinuityIssue {
                    issue_type: format!("repeated_{}", ri.issue_type),
                    severity: ri.severity,
                    title: ri.title,
                    description: ri.description,
                    evidence_json: serde_json::to_string(&ri.evidence).unwrap_or_default(),
                    suggested_actions_json: serde_json::to_string(&[&ri.suggested_action])
                        .unwrap_or_default(),
                });
            }
        }
```

- [ ] **Step 4: Ensure `NewContinuityIssue` is in scope**

Check that `NewContinuityIssue` is imported or accessible. It is already used at line 910 in the extractor loop, so it must be in scope. Verify.

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p novellossless-core`
Expected: success

- [ ] **Step 6: Verify all existing tests still pass**

Run: `cargo test -p novellossless-repeated` then `cargo test -p novellossless-core`
Expected: all pass

- [ ] **Step 7: Add integration test in core**

In `crates/core/src/lib.rs`, in the `#[cfg(test)] mod tests` section, add:
```rust
    #[test]
    fn repeated_description_detection_integration() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db_path = dir.path().join("test.db");
        let core = NovelCore::open(&db_path)?;

        let project_id = core.import_project("test", dir.path())?;

        // Write two files with repeated content
        let file1 = dir.path().join("ch1.txt");
        let file2 = dir.path().join("ch2.txt");
        std::fs::write(&file1, "第一章\n\n林澈走进房间。他拿起剑，又放下。窗外雨夜。")?;
        std::fs::write(&file2, "第二章\n\n林澈走进房间。他拿起剑，又放下。窗外雨夜。")?;

        core.scan_project(&project_id)?;

        let issues = core.list_issues(&project_id, 50)?;
        let repeated_count = issues.iter().filter(|i| i.issue_type.starts_with("repeated_")).count();
        assert!(repeated_count > 0, "should detect at least one repeated description issue, got {}", repeated_count);
        Ok(())
    }
```

Add `use novellossless_repeated;` at the top of the test module if needed.

- [ ] **Step 8: Run integration test**

Run: `cargo test -p novellossless-core -- repeated_description_detection_integration`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(core): integrate repeated description detection into scan pipeline"
```

---

### Task 8: Final verification

- [ ] **Step 1: Full workspace build**

Run: `cargo test --workspace`
Expected: 81+ tests pass (existing + new)

- [ ] **Step 2: Format**

Run: `cargo fmt`

- [ ] **Step 3: Final commit if formatting changed**

```bash
git add -A
git commit -m "style: rustfmt"
```

- [ ] **Step 4: Print summary**

```bash
echo "=== Summary ===" && cargo test --workspace 2>&1 | grep "^test result" && echo "---" && git log --oneline -5
```
