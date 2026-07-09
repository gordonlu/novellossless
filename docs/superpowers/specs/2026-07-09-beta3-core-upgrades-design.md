# Beta 3 — Core Upgrades: Rules, Timeline, Tasks, Impact, AI

> Phase: Beta 3 (core upgrades)  
> Date: 2026-07-09  
> Status: Draft  
> Target: v1.0 stable release capability

## 1. Overview

Five new crates under `crates/`, each extending the core analysis pipeline with `profiles/` profile integration:

| Crate | Path | PRD Section | Scope |
|-------|------|-------------|-------|
| `novellossless-rules` | `crates/rules/` | 8.15 | Setting rule system — manual + extracted + AI-aided rules with conflict detection |
| `novellossless-timeline` | `crates/timeline/` | 8.14 | Timeline enhancement — time expression extraction, sequence, conflict detection |
| `novellossless-tasks` | `crates/tasks/` | 8.21 | Revision task CRUD, auto-creation from conflicts/foreshadows |
| `novellossless-impact` | `crates/impact/` | 8.19 | Revision impact analysis — detects affected cards/foreshadows/rules after file changes |
| `novellossless-ai` | `crates/ai/` | 14 | deeplossless adapter providing extract_rules / analyze_timeline / analyze_impact |

New storage tables (added to `novellossless-storage::init()`):

- `world_rules`
- `timeline_events`
- `revision_tasks`

### 1.1 Dependency Graph

```
crates/storage (new tables + CRUD)
  ← crates/rules
  ← crates/timeline
  ← crates/tasks
  ← crates/impact (also depends on core, tasks)
  ← crates/core (analyze_project integration)
crates/ai (depends on deeplossless = "=0.7.4")
  ← crates/rules (extract_rules call)
  ← crates/timeline (analyze_timeline call)
  ← crates/impact (analyze_impact call)
```

## 2. `crates/rules` — Setting Rule System

### 2.1 Data Model

```rust
/// A worldbuilding rule that the novel text should not contradict.
#[derive(Debug, Clone)]
pub struct WorldRule {
    pub id: String,
    pub project_id: String,
    pub name: String,              // "魔法不能凭空制造生命"
    pub description: String,      // detailed explanation
    pub rule_type: String,          // "world" | "organization" | "ability" | "technology" | "social" | "belief" | "rumor"
    pub keywords: Vec<String>,     // ["魔法", "生命", "创造", "禁术"]
    pub positive: bool,            // true=must be followed, false=must NOT happen
    pub source_chunk_id: Option<String>,
    pub confidence: i32,           // 100=manual, 50=extracted, 70=AI
    pub status: String,            // "active" | "deprecated" | "candidate"
    pub created_at: String,
    pub updated_at: String,
}
```

### 2.2 Storage

`world_rules` table in `crates/storage`, methods:
- `upsert_rule(&self, rule: &WorldRule) -> Result<()>`
- `list_rules(&self, project_id: &str) -> Result<Vec<WorldRule>>`
- `get_rule(&self, id: &str) -> Result<Option<WorldRule>>`
- `delete_rule(&self, id: &str) -> Result<()>`

### 2.3 Rule Source: Manual

User creates/edits rules via Tauri command + Settings UI. Manual rules get `confidence: 100`.

### 2.4 Rule Source: Text Extraction (L1 Offline)

During `analyze_project`, scan chunks for rule-like patterns:
- "不能/无法/不可/禁止/不得 …" — prohibition
- "只有…才能/必须…才能" — prerequisite
- "只要…就/一旦…就" — conditional
- "从来/从未/永远不" — universal
- Keywords in rule's keyword list matching chunk content

Extraction yields a `WorldRule` with `confidence: 50` and `status: "candidate"`.

### 2.5 Rule Source: AI via `crates/ai` (L3 Enhanced)

If AI is enabled and available, call `AiProvider::extract_rules(chunks)` to identify implicit rules.

### 2.6 Conflict Detection

During `analyze_project`, for each active rule:
- Find chunks where rule keywords appear together with contradictory keywords
- If `positive=true` and chunk suggests the rule is broken → IssueCandidate("rule_conflict", "high")
- If `positive=false` and chunk suggests the rule is followed → IssueCandidate("rule_conflict", "medium")

Rules are associated with profile `checks.enabled` containing rule_conflict.

### 2.7 Integration

In `crates/core/src/lib.rs`, `analyze_project`:
- After existing analysis, if enabled profiles include rule checks:
  - Load rules for project
  - Run conflict detection
  - Call AI extraction if configured

### 2.8 Testing

- 1 test: manual rule creation and storage
- 1 test: conflict detection on known violation
- 1 test: no false positive on compliant text
- 1 test: extraction pattern matching

## 3. `crates/timeline` — Timeline Enhancement

### 3.1 Data Model

```rust
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub id: String,
    pub project_id: String,
    pub chunk_id: String,
    pub chunk_index: i64,
    pub document_path: String,
    pub title: String,
    pub order_index: i64,           // global sequence across all documents
    pub time_expression: String,    // "三天后", "天宝三载"
    pub estimated_order: Option<i64>, // resolved chronological order
    pub participants: Vec<String>,
    pub location: String,
    pub is_flashback: bool,
    pub confidence: i32,
}
```

### 3.2 Storage

`timeline_events` table, methods:
- `upsert_timeline_event(&self, event: &TimelineEvent) -> Result<()>`
- `list_timeline_events(&self, project_id: &str) -> Result<Vec<TimelineEvent>>`
- `delete_timeline_events(&self, project_id: &str) -> Result<()>` — call on full re-scan

### 3.3 Time Expression Extraction

Algorithm (pure Rust, no AI):
1. Relative time (offset-based):
   - Patterns: "(\d+)天[后|之[后|前]]", "(\d+)个?月[后|前]", "(\d+)年[后|前]", "次日/翌日/次日", "翌年", "数日/数月" → parse number, compute offset
   - Accumulate a running time cursor per document: `cursor += offset`
2. Absolute time (year names):
   - 年号+数字: "天宝(\d+)载", "贞观(\d+)年", "开元(\d+)年"
   - Set absolute year when found; also add to dynasty knowledge index
3. Relative time of day:
   - "清晨|早上|上午|中午|下午|傍晚|晚上|深夜|午夜" → set time-of-day flag
4. Flashback detection:
   - "回忆起|想起|那年|曾经|当时|那时" → mark `is_flashback=true`

### 3.4 Conflict Detection

- **Same-person two-locations**: Within same chronological range, character appears in two different locations → IssueCandidate
- **Order contradiction**: `parsed_order` suggests A before B but text presents B before A → IssueCandidate
- **Unexplained gap**: Large `parsed_order` jump between consecutive chunks → IssueCandidate

### 3.5 Integration

`crates/core` calls `timeline_extract(chunks)` → `timeline_check(events)` → `storage.upsert_timeline_events`.

### 3.6 Testing

- 1 test: relative time "三天后" → correct offset
- 1 test: absolute time "贞观三年" → year set
- 1 test: flashback detection
- 1 test: same-person two-locations conflict

## 4. `crates/tasks` — Revision Task System

### 4.1 Data Model

```rust
#[derive(Debug, Clone)]
pub struct RevisionTask {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub task_type: String,     // "conflict" | "foreshadow" | "revision_impact" | "manual" | "rule_conflict"
    pub priority: String,      // "high" | "medium" | "low"
    pub source_issue_id: Option<String>,
    pub source_foreshadow_id: Option<String>,
    pub related_chunk_ids: Vec<String>,
    pub status: String,        // "pending" | "in_progress" | "resolved" | "ignored" | "abandoned"
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
    pub notes: String,
}
```

### 4.2 Storage

`revision_tasks` table, CRUD methods.

### 4.3 Auto-Creation

In `analyze_project`, after issues and foreshadows are computed, create tasks:
- Each new high-severity issue → `RevisionTask { task_type: "conflict", priority: "high" }`
- Each foreshadow with risk >= "high" → `RevisionTask { task_type: "foreshadow", priority: "medium" }`
- Deduplicate by `(project_id, task_type, source_issue_id/source_foreshadow_id)`

### 4.4 Manual Creation

Tauri command `create_revision_task(projectId, title, taskType, priority, notes)`.

### 4.5 Testing

- 1 test: create task
- 1 test: update task status
- 1 test: auto-create from issue
- 1 test: deduplicate on re-scan

## 5. `crates/impact` — Revision Impact Analysis

### 5.1 Purpose

When `incremental_scan_file` detects a file change + diff, analyze what downstream content is affected.

### 5.2 Algorithm

1. Get the diff (added/removed/modified chunks)
2. For removed chunks: query storage for any narrative_nodes, foreshadow_items, continuity_issues, world_rules whose `first_chunk_id` or `source_chunk_id` references the removed chunk
3. For modified chunks: check if content difference changes any node name, keyword, or location reference
4. Impact result:

```rust
pub struct RevisionImpact {
    pub project_id: String,
    pub doc_id: String,
    pub affected_nodes: Vec<String>,       // node IDs
    pub affected_foreshadows: Vec<String>,
    pub affected_rules: Vec<String>,
    pub affected_chunks_after: Vec<String>, // chunks chronologically after the change
    pub summary: String,
}
```

### 5.3 Integration

`impact::analyze` called from `incremental_scan_file` after `diff_chunks` and before returning `ScanResult`.

Creates `RevisionTask` for any affected node/foreshadow/rule with `task_type: "revision_impact"`.

### 5.4 Testing

- 1 test: removed chunk referenced by narrative node → node listed
- 1 test: no impact when chunk content unchanged
- 1 test: impact creates task

## 6. `crates/ai` — Deeplossless Adapter

### 6.1 Interface

```rust
/// Result from AI rule extraction
pub struct ExtractedRule {
    pub name: String,
    pub description: String,
    pub rule_type: String,
    pub keywords: Vec<String>,
    pub positive: bool,
}

/// Result from AI timeline analysis
pub struct TimelineInsight {
    pub chunk_id: String,
    pub time_description: String,
    pub suggested_order: Option<i64>,
    pub is_flashback: bool,
}

/// Result from AI revision impact analysis
pub struct ImpactInsight {
    pub summary: String,
    pub affected_areas: Vec<String>,
}

pub trait AiProvider {
    fn extract_rules(&self, chunks: &[&str]) -> Result<Vec<ExtractedRule>>;
    fn analyze_timeline(&self, chunks: &[&str]) -> Result<Vec<TimelineInsight>>;
    fn analyze_impact(&self, old: &[&str], new: &[&str], diff_desc: &str) -> Result<ImpactInsight>;
}
```

### 6.2 DeeplosslessProvider

```rust
pub struct DeeplosslessProvider;

impl AiProvider for DeeplosslessProvider {
    fn extract_rules(...) -> ... { /* calls deeplossless */ }
    fn analyze_timeline(...) -> ... { /* calls deeplossless */ }
    fn analyze_impact(...) -> ... { /* calls deeplossless */ }
}

// No-op provider when AI not configured
pub struct NoopProvider;
impl AiProvider for NoopProvider { /* all return empty */ }
```

### 6.3 Configuration

`crates/storage` already has `app_settings`. Add:
- `ai_provider` = "deeplossless" | "none"
- `ai_deeplossless_endpoint` = "https://..."
- `ai_deeplossless_key` = "..."

`crates/core`'s `NovelCore` holds `Box<dyn AiProvider>` which is `NoopProvider` by default, upgraded when ai settings are present.

## 7. Integration Plan

### 7.1 Phase Order

1. **Storage** — Add 3 tables to existing `init()`
2. **crates/tasks** — Simplest, no external deps beyond storage
3. **crates/rules** — Storage + extraction + conflict detection
4. **crates/timeline** — Storage + time expression extraction + conflict
5. **crates/impact** — Depends on tasks + core diff infrastructure
6. **crates/ai** — Interface + DeeplosslessProvider stub; can be parallel with 2-5
7. **Core integration** — Wire all into `analyze_project` and `incremental_scan_file`

### 7.2 NovelCore Changes

```rust
// New fields
rules_engine: rules::RuleEngine,
timeline_engine: timeline::TimelineEngine,
ai_provider: Box<dyn ai::AiProvider>,

// Enhanced analyze_project
fn analyze_project(&self, ...) -> ... {
    // existing analysis
    self.rules_engine.check(&chunks, &self.storage)?;
    self.timeline_engine.extract(&chunks, &self.storage)?;
    self.task_engine.auto_create_from_issues(...)?;
}

// Enhanced incremental_scan_file
fn incremental_scan_file(&self, ...) -> ... {
    // ... existing diff logic
    self.impact_engine.analyze(&old_chunks, &new_chunks, ...)?;
}
```

### 7.3 Profile Integration

Each crate exports checks compatible with `profiles/` IssueEmitter integration:
- `rules` exports `rule_conflict` check ID
- `timeline` exports `time_anomaly` check ID

## 8. Spec Review

- [x] No placeholders / TBDs
- [x] Architecture matches feature descriptions
- [x] Scope focused: 5 crates, each with clear boundary
- [x] Ambiguity resolved: time extraction uses rules, not LLM; AI is optional and abstracted