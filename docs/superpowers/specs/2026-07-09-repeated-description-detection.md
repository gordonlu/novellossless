# Repeated Description Detection — Design Spec

## Problem

The codebase lacks any repeated-description analysis. The closest analogue is
`RepeatExpressionExtractor` which checks 5 hardcoded keywords against a simple
`content.contains(term)` — no n-gram, no similarity, no frequency tracking.
Romanced authors need detection of repeated scenes, high-frequency filler
phrases, repeated actions, repetitive dialogue patterns, and dense
psychological description.

## Approach

New standalone crate `crates/repeated`, minimum external deps.

## Architecture

```
crates/repeated/
├── Cargo.toml
├── src/
│   ├── lib.rs              # RepeatedDescriptionEngine (public API)
│   ├── detectors/
│   │   ├── mod.rs          # Detector trait + registry
│   │   ├── similar_paragraphs.rs
│   │   ├── high_freq_expressions.rs
│   │   ├── repeated_actions.rs
│   │   ├── dialogue_patterns.rs
│   │   └── psych_density.rs
│   └── types.rs            # RepeatedIssue + EvidenceItem
```

### Detector trait

```rust
pub trait Detector {
    fn id(&self) -> &'static str;
    fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue>;
}
```

`ChunkInfo` is defined in `crates/repeated/src/types.rs` (a lightweight view
struct, not importing core types):

```rust
pub struct ChunkInfo {
    pub chunk_id: String,
    pub document_id: String,
    pub document_path: String,
    pub chapter_title: String,
    pub chunk_index: i64,
    pub content: String,
}
```

### RepeatedIssue

```rust
pub struct RepeatedIssue {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence: Vec<EvidenceItem>,
    pub suggested_action: String,
}

pub struct EvidenceItem {
    pub chunk_id: String,
    pub chapter_title: String,
    pub document_path: String,
    pub snippet: String,
    pub match_count: Option<u32>,
}
```

### RepeatedDescriptionEngine

```rust
pub struct RepeatedDescriptionEngine {
    detectors: Vec<Box<dyn Detector>>,
}

impl RepeatedDescriptionEngine {
    pub fn new() -> Self;
    pub fn with(mut self, detector: Box<dyn Detector>) -> Self;
    pub fn detect(&self, chunks: &[ChunkInfo]) -> Vec<RepeatedIssue>;
    // Convenience: builds the default set of 5 detectors
    pub fn default() -> Self;
}
```

## Detector Specifications

### 1. SimilarParagraphs

- **Method**: Paragraph-level Jaccard similarity on tokenized word sets.
  Sentence splitting on `[。！？\n]`, word segmentation on whitespace + common
  Chinese delimiters (not a full segmenter — just split on non-CJK-chars).
- **Threshold**: Jaccard ≥ 0.60 across different chunks → issue.
- **Output**: Severe "low". Groups all matching paragraph pairs across chapters.
- **Edge cases**: Ignores paragraphs < 20 characters. Deduplicates within same
  chunk (no self-match). At most 5 evidence pairs per issue group.

### 2. HighFreqExpressions

- **Method**: Sentence-level n-gram frequency. Split chunks into sentences,
  extract n-grams (word-level, n=4..10). Count global frequency across all
  chunks.
- **Threshold**: Any n-gram appearing in ≥ 5 distinct chunks → issue.
  Additionally: single content words (Chinese 2+ char) appearing in ≥ 10
  distinct chunks → potential filler word.
- **Output**: Severity "info" for filler words, "low" for phrase repetition.
  Top repeat offenders in description. Top 15 max per scan.
- **Edge cases**: Filters out n-grams that are purely punctuation or number
  sequences. Cross-chapter dedup (same phrase counted once per chapter).

### 3. RepeatedActions

- **Method**: Pattern `[character] + [action verb + object]` — detect by
  regex on known Chinese action verbs (看、说、问、拿、走、放、推、拉、抱、
  拿、握、拍).
- **Threshold**: Same verb+object pattern in ≥ 4 distinct chunks → issue.
- **Output**: Severity "low". Lists all appearances with character name,
  action, and chapter.
- **Edge cases**: Ignores common filler verbs (是、有、在、能、会、要).

### 4. DialoguePatterns

- **Method**: Detect dialogue guide patterns — `"某某说"`, `"某某问"`,
  `"某某道"`, `"某某开口"`, `"某某压低声音"`, etc. Count per-chapter
  frequency. Cross-chapter variance analysis: if the distribution is extremely
  flat (same opening every time) or one pattern dominates >60% → issue.
- **Threshold**: Single dialogue pattern used in ≥ 6 distinct chunks → issue.
- **Output**: Severity "low". Lists pattern and examples.
- **Edge cases**: Recognizes both "某某说：" and "说：" and "说，" patterns.

### 5. PsychDescriptionDensity

- **Method**: Known Chinese psychological-description keywords
  (想、觉得、感到、知道、明白、以为、仿佛、似乎、也许、大概、应该、可能、
  突然、忽然、不知、记得、忘记、怕、担心、希望、期待). Count occurrences
  per 1000 chars in each chunk.
- **Threshold**: Any chunk where psych-keyword density > 4% of total character
  count → issue. Also detect chunks in the top 5% of density across the
  project.
- **Output**: Severity "info". Shows density percentage and the dense passage.
- **Edge cases**: Only counts "content" occurrences, not in dialogue markers
  (skips text in「」or ""). Ignores chunks < 100 chars.

## Integration with Core

In `analyze_project()`, after existing extractors:

```rust
use novellossless_repeated::RepeatedDescriptionEngine;

// Build chunks as ChunkInfo slices (from memory — cheap, already loaded)
let chunk_infos: Vec<ChunkInfo> = chunks.iter().map(|c| ChunkInfo { .. }).collect();
let engine = RepeatedDescriptionEngine::default();
let repeated_issues = engine.detect(&chunk_infos);
for ri in repeated_issues {
    issues.push(NewContinuityIssue {
        issue_type: format!("repeated_{}", ri.issue_type),
        severity: ri.severity,
        title: ri.title,
        description: ri.description,
        evidence_json: serde_json::to_string(&ri.evidence).unwrap_or_default(),
        suggested_actions_json: serde_json::to_string(&[&ri.suggested_action]).unwrap_or_default(),
    });
}
// Then existing storage.upsert_continuity_issues(project_id, &issues)
```

No new Tauri command needed — issues flow through `continuity_issues` and
appear in the existing `list_issues` / dashboard.

## Performance

- All detectors are O(n) or O(n log n) per chunk set.
- Text is already in memory (no I/O).
- No networking, no blocking.
- Expected: < 50ms for a 50-chapter novel on a modern CPU.

## Dependencies

- `repeated/Cargo.toml` will depend on: `serde`, `serde_json` (for output
  serialization).
- No external ML deps (no ndarray, no candle, no tokenizers).
- `Cargo.toml` of `novellossless-core` will add:
  `novellossless-repeated = { path = "../repeated" }`

## Testing Strategy

### Unit tests in `crates/repeated`

- **SimilarParagraphs**: Two chunks with one identical paragraph → issue.
  Two chunks with completely different content → no issue.
  Short paragraph ignored.
- **HighFreqExpressions**: 5+ chunks sharing a 4-gram → issue. Single chunk no
  match.
- **RepeatedActions**: Four chunks with "他拿起剑" → issue. One chunk no
  issue.
- **DialoguePatterns**: Six chunks using "某某说：" → issue.
- **PsychDensity**: Chunk with 5% psych keywords → issue. Chunk with 1% no
  issue.
- **Engine integration**: Empty chunks → empty result.

### Integration test in `crates/core`

- Add `verify_repeated_description_detection` test: create a project with
  known repeated content, run scan, verify issues appear with correct
  issue_type prefix `repeated_`.

## Out of Scope (P0)

- SimHash / MinHash (can add as a second similarity backend later)
- Emotion/repeated-mood detection
- Narrative-image network tracking
- AI-powered interpretation
- Frontend-specific display changes (the existing `list_issues` works)
