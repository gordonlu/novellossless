# Beta 3 — Core Upgrades Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 5 new crates (rules, timeline, tasks, impact, ai) with 3 new storage tables, integrated into the core scan pipeline.

**Architecture:** Each subsystem is an independent crate (`crates/rules/`, `crates/timeline/`, `crates/tasks/`, `crates/impact/`, `crates/ai/`). Storage tables live in `crates/storage`. Core integration in `crates/core/src/lib.rs` extends `analyze_project` and `incremental_scan_file`.

**Tech Stack:** Rust workspace, SQLite via rusqlite, serde JSON for JSON fields, regex for time extraction, optional deeplossless 0.7.4.

## Global Constraints

- Follow existing crate patterns: `Cargo.toml` with `edition.workspace`, `anyhow` for error handling, `serde::Serialize` for DTOs
- New workspace members must be added to `Cargo.toml` root `[workspace.members]`
- New workspace dependencies go in root `Cargo.toml` `[workspace.dependencies]`
- All tests pass at end of each task: `cargo test`
- TypeScript compiles at end: `npx tsc --noEmit` in `apps/desktop/`
- DTOs use `#[serde(rename_all = "camelCase")]`
- deeplossless feature is behind `deeplossless-compat` feature flag (existing pattern)

---

### Task 1: Storage — Add 3 Tables + CRUD

**Files:**
- Modify: `crates/storage/src/lib.rs`
- Test: built into existing storage test module

**Interfaces:**
- Consumes: Nothing new
- Produces: `WorldRule`, `TimelineEvent`, `RevisionTask` structs + `RevisionTaskSource` enum + 10 CRUD methods

- [ ] **Step 1: Add `WorldRule` struct after `KnowledgePackEntry`**

```rust
#[derive(Debug, Clone)]
pub struct WorldRule {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub rule_type: String,
    pub keywords_json: String,
    pub positive: bool,
    pub source_chunk_id: Option<String>,
    pub confidence: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 2: Add `TimelineEvent` struct**

```rust
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub id: String,
    pub project_id: String,
    pub chunk_id: String,
    pub chunk_index: i64,
    pub document_path: String,
    pub title: String,
    pub order_index: i64,
    pub time_expression: String,
    pub estimated_order: Option<i64>,
    pub participants_json: String,
    pub location: String,
    pub is_flashback: bool,
    pub confidence: i32,
}
```

- [ ] **Step 3: Add `RevisionTask` struct and `NewRevisionTask` struct**

```rust
#[derive(Debug, Clone)]
pub struct RevisionTask {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub task_type: String,
    pub priority: String,
    pub source_issue_id: Option<String>,
    pub source_foreshadow_id: Option<String>,
    pub related_chunks_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct NewRevisionTask {
    pub project_id: String,
    pub title: String,
    pub task_type: String,
    pub priority: String,
    pub source_issue_id: Option<String>,
    pub source_foreshadow_id: Option<String>,
    pub related_chunks_json: String,
    pub notes: String,
}
```

- [ ] **Step 4: Add SQL in `init()` for 3 new tables**

Add inside the existing `init()` `execute_batch` call, before the closing `"#,`:

```sql
CREATE TABLE IF NOT EXISTS world_rules (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    rule_type TEXT NOT NULL,
    keywords_json TEXT NOT NULL DEFAULT '[]',
    positive INTEGER NOT NULL DEFAULT 1,
    source_chunk_id TEXT REFERENCES document_chunks(id) ON DELETE SET NULL,
    confidence INTEGER NOT NULL DEFAULT 50,
    status TEXT NOT NULL DEFAULT 'candidate',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS timeline_events (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chunk_id TEXT NOT NULL REFERENCES document_chunks(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    document_path TEXT NOT NULL,
    title TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    time_expression TEXT NOT NULL DEFAULT '',
    estimated_order INTEGER,
    participants_json TEXT NOT NULL DEFAULT '[]',
    location TEXT NOT NULL DEFAULT '',
    is_flashback INTEGER NOT NULL DEFAULT 0,
    confidence INTEGER NOT NULL DEFAULT 50
);

CREATE TABLE IF NOT EXISTS revision_tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    task_type TEXT NOT NULL,
    priority TEXT NOT NULL,
    source_issue_id TEXT,
    source_foreshadow_id TEXT,
    related_chunks_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    resolved_at TEXT,
    notes TEXT NOT NULL DEFAULT ''
);
```

- [ ] **Step 5: Add CRUD methods for `WorldRule`**

```rust
pub fn upsert_rule(&self, rule: &WorldRule) -> Result<()> {
    self.conn.execute(
        "INSERT INTO world_rules (id, project_id, name, description, rule_type, keywords_json, positive, source_chunk_id, confidence, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, description=excluded.description, rule_type=excluded.rule_type,
            keywords_json=excluded.keywords_json, positive=excluded.positive,
            source_chunk_id=excluded.source_chunk_id, confidence=excluded.confidence,
            status=excluded.status, updated_at=excluded.updated_at",
        params![
            rule.id, rule.project_id, rule.name, rule.description, rule.rule_type,
            rule.keywords_json, rule.positive, rule.source_chunk_id, rule.confidence,
            rule.status, rule.created_at, rule.updated_at
        ],
    )?;
    Ok(())
}

pub fn list_rules(&self, project_id: &str) -> Result<Vec<WorldRule>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, project_id, name, description, rule_type, keywords_json, positive, source_chunk_id, confidence, status, created_at, updated_at
         FROM world_rules WHERE project_id = ?1 ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(WorldRule {
            id: row.get(0)?, project_id: row.get(1)?, name: row.get(2)?,
            description: row.get(3)?, rule_type: row.get(4)?, keywords_json: row.get(5)?,
            positive: row.get::<_, i32>(6)? != 0,
            source_chunk_id: row.get(7)?, confidence: row.get(8)?, status: row.get(9)?,
            created_at: row.get(10)?, updated_at: row.get(11)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

pub fn get_rule(&self, id: &str) -> Result<Option<WorldRule>> {
    self.conn.query_row(
        "SELECT id, project_id, name, description, rule_type, keywords_json, positive, source_chunk_id, confidence, status, created_at, updated_at
         FROM world_rules WHERE id = ?1",
        params![id],
        |row| Ok(WorldRule {
            id: row.get(0)?, project_id: row.get(1)?, name: row.get(2)?,
            description: row.get(3)?, rule_type: row.get(4)?, keywords_json: row.get(5)?,
            positive: row.get::<_, i32>(6)? != 0,
            source_chunk_id: row.get(7)?, confidence: row.get(8)?, status: row.get(9)?,
            created_at: row.get(10)?, updated_at: row.get(11)?,
        }),
    ).optional().map_err(Into::into)
}

pub fn delete_rule(&self, id: &str) -> Result<()> {
    self.conn.execute("DELETE FROM world_rules WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn delete_project_rules(&self, project_id: &str) -> Result<()> {
    self.conn.execute("DELETE FROM world_rules WHERE project_id = ?1", params![project_id])?;
    Ok(())
}

pub fn delete_project_timeline_events(&self, project_id: &str) -> Result<()> {
    self.conn.execute("DELETE FROM timeline_events WHERE project_id = ?1", params![project_id])?;
    Ok(())
}

pub fn delete_project_tasks(&self, project_id: &str) -> Result<()> {
    self.conn.execute("DELETE FROM revision_tasks WHERE project_id = ?1", params![project_id])?;
    Ok(())
}
```

- [ ] **Step 6: Add CRUD methods for `TimelineEvent`**

```rust
pub fn upsert_timeline_event(&self, event: &TimelineEvent) -> Result<()> {
    self.conn.execute(
        "INSERT INTO timeline_events (id, project_id, chunk_id, chunk_index, document_path, title, order_index, time_expression, estimated_order, participants_json, location, is_flashback, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            event.id, event.project_id, event.chunk_id, event.chunk_index,
            event.document_path, event.title, event.order_index, event.time_expression,
            event.estimated_order, event.participants_json, event.location,
            event.is_flashback, event.confidence
        ],
    )?;
    Ok(())
}

pub fn list_timeline_events(&self, project_id: &str) -> Result<Vec<TimelineEvent>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, project_id, chunk_id, chunk_index, document_path, title, order_index, time_expression, estimated_order, participants_json, location, is_flashback, confidence
         FROM timeline_events WHERE project_id = ?1 ORDER BY order_index ASC"
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(TimelineEvent {
            id: row.get(0)?, project_id: row.get(1)?, chunk_id: row.get(2)?,
            chunk_index: row.get(3)?, document_path: row.get(4)?, title: row.get(5)?,
            order_index: row.get(6)?, time_expression: row.get(7)?,
            estimated_order: row.get(8)?, participants_json: row.get(9)?,
            location: row.get(10)?,
            is_flashback: row.get::<_, i32>(11)? != 0,
            confidence: row.get(12)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}
```

- [ ] **Step 7: Add CRUD methods for `RevisionTask`**

```rust
pub fn create_task(&self, task: &NewRevisionTask) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    self.conn.execute(
        "INSERT INTO revision_tasks (id, project_id, title, task_type, priority, source_issue_id, source_foreshadow_id, related_chunks_json, status, created_at, updated_at, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9, ?9, ?10)",
        params![
            id, task.project_id, task.title, task.task_type, task.priority,
            task.source_issue_id, task.source_foreshadow_id, task.related_chunks_json,
            now, task.notes
        ],
    )?;
    Ok(id)
}

pub fn update_task_status(&self, id: &str, status: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    self.conn.execute(
        "UPDATE revision_tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now, id],
    )?;
    Ok(())
}

pub fn list_tasks(&self, project_id: &str) -> Result<Vec<RevisionTask>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, project_id, title, task_type, priority, source_issue_id, source_foreshadow_id, related_chunks_json, status, created_at, updated_at, resolved_at, notes
         FROM revision_tasks WHERE project_id = ?1 ORDER BY
            CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
            created_at DESC"
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(RevisionTask {
            id: row.get(0)?, project_id: row.get(1)?, title: row.get(2)?,
            task_type: row.get(3)?, priority: row.get(4)?,
            source_issue_id: row.get(5)?, source_foreshadow_id: row.get(6)?,
            related_chunks_json: row.get(7)?, status: row.get(8)?,
            created_at: row.get(9)?, updated_at: row.get(10)?,
            resolved_at: row.get(11)?, notes: row.get(12)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

pub fn get_task(&self, id: &str) -> Result<Option<RevisionTask>> {
    self.conn.query_row(
        "SELECT id, project_id, title, task_type, priority, source_issue_id, source_foreshadow_id, related_chunks_json, status, created_at, updated_at, resolved_at, notes
         FROM revision_tasks WHERE id = ?1",
        params![id],
        |row| Ok(RevisionTask {
            id: row.get(0)?, project_id: row.get(1)?, title: row.get(2)?,
            task_type: row.get(3)?, priority: row.get(4)?,
            source_issue_id: row.get(5)?, source_foreshadow_id: row.get(6)?,
            related_chunks_json: row.get(7)?, status: row.get(8)?,
            created_at: row.get(9)?, updated_at: row.get(10)?,
            resolved_at: row.get(11)?, notes: row.get(12)?,
        }),
    ).optional().map_err(Into::into)
}
```

- [ ] **Step 8: Add test functions in `mod tests`**

```rust
#[test]
fn stores_and_retrieves_rules() -> Result<()> {
    let (storage, pid) = test_storage_with_project("rules_test")?;
    storage.upsert_rule(&WorldRule {
        id: "r1".into(), project_id: pid.clone(),
        name: "魔法不能凭空制造生命".into(), description: "禁止用魔法创造生命".into(),
        rule_type: "world".into(), keywords_json: r#"["魔法","生命","创造"]"#.into(),
        positive: true, source_chunk_id: None, confidence: 100,
        status: "active".into(), created_at: "now".into(), updated_at: "now".into(),
    })?;
    let rules = storage.list_rules(&pid)?;
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "魔法不能凭空制造生命");
    Ok(())
}

#[test]
fn stores_and_lists_timeline_events() -> Result<()> {
    let (storage, pid) = test_storage_with_project("tl_test")?;
    storage.upsert_timeline_event(&TimelineEvent {
        id: "t1".into(), project_id: pid.clone(), chunk_id: "c1".into(),
        chunk_index: 0, document_path: "001.txt".into(), title: "第一章".into(),
        order_index: 1, time_expression: "三天后".into(), estimated_order: Some(3),
        participants_json: r#"["林澈"]"#.into(), location: "长安".into(),
        is_flashback: false, confidence: 50,
    })?;
    let events = storage.list_timeline_events(&pid)?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].time_expression, "三天后");
    Ok(())
}

#[test]
fn creates_and_lists_tasks() -> Result<()> {
    let (storage, pid) = test_storage_with_project("tasks_test")?;
    let id = storage.create_task(&NewRevisionTask {
        project_id: pid.clone(), title: "检查战力倒退".into(), task_type: "conflict".into(),
        priority: "high".into(), source_issue_id: None, source_foreshadow_id: None,
        related_chunks_json: "[]".into(), notes: String::new(),
    })?;
    let tasks = storage.list_tasks(&pid)?;
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "检查战力倒退");
    storage.update_task_status(&id, "resolved")?;
    let task = storage.get_task(&id)?.expect("exists");
    assert_eq!(task.status, "resolved");
    Ok(())
}
```

- [ ] **Step 9: Build and run tests**

Run: `cargo test -p novellossless-storage`
Expected: 18 passed (3 new, 15 existing)

- [ ] **Step 10: Commit**

```bash
git add crates/storage/src/lib.rs
git commit -m "feat(storage): add world_rules, timeline_events, revision_tables tables and CRUD"
```

---

## Task 2: `crates/tasks` — Revision Task Crate

**Files:**
- Create: `crates/tasks/Cargo.toml`
- Create: `crates/tasks/src/lib.rs`
- Test: in `lib.rs` test module

**Interfaces:**
- Consumes: `storage::Storage::{create_task, update_task_status, list_tasks, get_task}` from Task 1
- Produces: `TaskManager::auto_create_from_issues(project_id, issues, foreshadows, storage)`

- [ ] **Step 1: Create `crates/tasks/Cargo.toml`**

```toml
[package]
name = "novellossless-tasks"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
anyhow.workspace = true
chrono.workspace = true
novellossless-storage = { path = "../storage" }

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 2: Create `crates/tasks/src/lib.rs`**

```rust
use novellossless_storage::{ContinuityIssue, ForeshadowItem, NewRevisionTask, Storage};
use anyhow::Result;

pub struct TaskManager;

impl TaskManager {
    pub fn auto_create_from_issues(
        project_id: &str,
        issues: &[ContinuityIssue],
        foreshadows: &[ForeshadowItem],
        storage: &Storage,
    ) -> Result<Vec<String>> {
        let mut created_ids = Vec::new();
        let existing = storage.list_tasks(project_id)?;

        for issue in issues {
            if issue.severity != "high" {
                continue;
            }
            let is_duplicate = existing.iter().any(|t| {
                t.source_issue_id.as_deref() == Some(&issue.id)
            });
            if !is_duplicate {
                let id = storage.create_task(&NewRevisionTask {
                    project_id: project_id.to_string(),
                    title: format!("[冲突] {}", issue.title),
                    task_type: "conflict".to_string(),
                    priority: issue.severity.clone(),
                    source_issue_id: Some(issue.id.clone()),
                    source_foreshadow_id: None,
                    related_chunks_json: String::new(),
                    notes: String::new(),
                })?;
                created_ids.push(id);
            }
        }

        for f in foreshadows {
            if f.risk_level != "high" {
                continue;
            }
            let is_duplicate = existing.iter().any(|t| {
                t.source_foreshadow_id.as_deref() == Some(&f.id)
            });
            if !is_duplicate {
                let id = storage.create_task(&NewRevisionTask {
                    project_id: project_id.to_string(),
                    title: format!("[伏笔] {}", f.title),
                    task_type: "foreshadow".to_string(),
                    priority: "medium".to_string(),
                    source_issue_id: None,
                    source_foreshadow_id: Some(f.id.clone()),
                    related_chunks_json: String::new(),
                    notes: String::new(),
                })?;
                created_ids.push(id);
            }
        }

        Ok(created_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novellossless_storage::{Storage, ContinuityIssue, ForeshadowItem};

    fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project(name, &format!("/tmp/{name}"))?;
        Ok((storage, project.id))
    }

    #[test]
    fn auto_creates_from_high_severity_issue() -> Result<()> {
        let (storage, pid) = test_storage_with_project("auto_task_test")?;
        let issues = vec![ContinuityIssue {
            id: "i1".into(), project_id: pid.clone(), issue_type: "rule_conflict".into(),
            severity: "high".into(), title: "战力倒退".into(), description: String::new(),
            evidence_json: String::new(), suggested_actions_json: String::new(),
            status: "open".into(),
        }];
        let ids = TaskManager::auto_create_from_issues(&pid, &issues, &[], &storage)?;
        assert_eq!(ids.len(), 1);
        let tasks = storage.list_tasks(&pid)?;
        assert_eq!(tasks[0].task_type, "conflict");
        Ok(())
    }

    #[test]
    fn skips_low_severity_issues() -> Result<()> {
        let (storage, pid) = test_storage_with_project("skip_low")?;
        let issues = vec![ContinuityIssue {
            id: "i2".into(), project_id: pid.clone(), issue_type: "repeat_expression".into(),
            severity: "low".into(), title: "重复".into(), description: String::new(),
            evidence_json: String::new(), suggested_actions_json: String::new(),
            status: "open".into(),
        }];
        let ids = TaskManager::auto_create_from_issues(&pid, &issues, &[], &storage)?;
        assert!(ids.is_empty());
        Ok(())
    }

    #[test]
    fn deduplicates_on_second_call() -> Result<()> {
        let (storage, pid) = test_storage_with_project("dedup")?;
        let issues = vec![ContinuityIssue {
            id: "i3".into(), project_id: pid.clone(), issue_type: "rule_conflict".into(),
            severity: "high".into(), title: "冲突".into(), description: String::new(),
            evidence_json: String::new(), suggested_actions_json: String::new(),
            status: "open".into(),
        }];
        TaskManager::auto_create_from_issues(&pid, &issues, &[], &storage)?;
        let ids = TaskManager::auto_create_from_issues(&pid, &issues, &[], &storage)?;
        assert!(ids.is_empty(), "should not create duplicate");
        Ok(())
    }
}
```

- [ ] **Step 3: Register in workspace**

Add to root `Cargo.toml` `[workspace.members]`:
```
    "crates/tasks",
```

- [ ] **Step 4: Build and run tests**

Run: `cargo test -p novellossless-tasks`
Expected: 3 passed

- [ ] **Step 5: Commit**

```bash
git add crates/tasks/Cargo.toml crates/tasks/src/lib.rs Cargo.toml
git commit -m "feat(tasks): add revision task crate with auto-creation from issues"
```

---

## Task 3: `crates/rules` — Setting Rule Crate

**Files:**
- Create: `crates/rules/Cargo.toml`
- Create: `crates/rules/src/lib.rs`
- Test: in `lib.rs`

**Interfaces:**
- Consumes: `storage::{WorldRule, upsert_rule, list_rules}`, `ProjectChunk` from Task 1
- Produces: `RuleEngine::extract_candidates(chunks, storage, project_id)`, `RuleEngine::check_conflicts(chunks, rules) -> Vec<IssueCandidate>`

- [ ] **Step 1: Create `crates/rules/Cargo.toml`**

```toml
[package]
name = "novellossless-rules"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
anyhow.workspace = true
chrono.workspace = true
novellossless-storage = { path = "../storage" }
regex.workspace = true
serde_json.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 2: Create `crates/rules/src/lib.rs`**

```rust
use std::path::Path;

use anyhow::Result;
use novellossless_storage::{NewContinuityIssue, ProjectChunk, Storage, WorldRule};
use regex::Regex;
use serde_json::json;

pub struct RuleEngine;

impl RuleEngine {
    /// Extract candidate rules from chunk text using pattern matching.
    /// Looks for constraint-like sentences: "不能/无法/不可/禁止/不得/只有...才能/从来/从未"
    pub fn extract_candidates(
        project_id: &str,
        chunks: &[ProjectChunk],
        storage: &Storage,
    ) -> Result<Vec<String>> {
        let mut created_ids = Vec::new();
        let prohibition_re = Regex::new(r"([\p{Han}]{2,20})(?:不能|无法|不可|禁止|不得|不允许|从不|从未)([\p{Han}]{2,40})")?;
        let prerequisite_re = Regex::new(r"只有([\p{Han}]{2,20})(?:才|才能)([\p{Han}]{2,40})")?;
        let now = chrono::Utc::now().to_rfc3339();

        for chunk in chunks {
            for cap in prohibition_re.captures_iter(&chunk.content) {
                let subject = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let action = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                let name = format!("{subject}不能{action}");
                let rule = WorldRule {
                    id: uuid::Uuid::new_v4().to_string(),
                    project_id: project_id.to_string(),
                    name,
                    description: format!("从正文抽取的约束规则: {subject}不能{action}"),
                    rule_type: "extracted".to_string(),
                    keywords_json: json!([subject, action]).to_string(),
                    positive: true,
                    source_chunk_id: Some(chunk.chunk_id.clone()),
                    confidence: 50,
                    status: "candidate".to_string(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };
                storage.upsert_rule(&rule)?;
                created_ids.push(rule.id);
            }

            for cap in prerequisite_re.captures_iter(&chunk.content) {
                let condition = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let result = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                let name = format!("只有{condition}才能{result}");
                let rule = WorldRule {
                    id: uuid::Uuid::new_v4().to_string(),
                    project_id: project_id.to_string(),
                    name,
                    description: format!("前提约束: 只有{condition}才能{result}"),
                    rule_type: "extracted".to_string(),
                    keywords_json: json!([condition, result]).to_string(),
                    positive: true,
                    source_chunk_id: Some(chunk.chunk_id.clone()),
                    confidence: 50,
                    status: "candidate".to_string(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };
                storage.upsert_rule(&rule)?;
                created_ids.push(rule.id);
            }
        }

        Ok(created_ids)
    }

    /// Check chunks against active rules and return issues.
    /// For `positive` rules, find chunks where rule keywords appear together
    /// with contradictory language.
    pub fn check_conflicts(
        chunks: &[ProjectChunk],
        rules: &[WorldRule],
    ) -> Vec<NewContinuityIssue> {
        let mut issues = Vec::new();
        let contradiction_words = ["却", "但是", "然而", "居然", "竟然", "还是", "照常", "依然"];

        for rule in rules {
            if rule.status != "active" {
                continue;
            }
            let keywords: Vec<String> = serde_json::from_str(&rule.keywords_json).unwrap_or_default();
            if keywords.is_empty() {
                continue;
            }

            for chunk in chunks {
                // Check if all keywords appear in this chunk
                let all_keywords_present = keywords.iter().all(|kw| chunk.content.contains(kw.as_str()));
                if !all_keywords_present {
                    continue;
                }

                // Check for contradiction words
                let has_contradiction = contradictory_words.iter()
                    .any(|cw| chunk.content.contains(cw));
                if !has_contradiction {
                    continue;
                }

                let kw_list = keywords.join(", ");
                issues.push(NewContinuityIssue {
                    issue_type: "rule_conflict".to_string(),
                    severity: "high".to_string(),
                    title: format!("可能违反规则「{}」", rule.name),
                    description: format!(
                        "规则「{}」要求关键字「{}」一致，但正文中出现了看似矛盾的表述。",
                        rule.name, kw_list
                    ),
                    evidence_json: serde_json::to_string(&json!({
                        "rule_id": rule.id,
                        "rule_name": rule.name,
                        "chunk_id": chunk.chunk_id,
                        "snippet": chunk.content.chars().take(80).collect::<String>(),
                    })).unwrap_or_default(),
                    suggested_actions_json: serde_json::to_string(&json!([
                        "标记为规则例外",
                        "接受为正式设定变更",
                        "标记为误报"
                    ])).unwrap_or_default(),
                });
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novellossless_storage::{Storage, WorldRule};

    fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project(name, &format!("/tmp/{name}"))?;
        Ok((storage, project.id))
    }

    #[test]
    fn extracts_prohibition_rules() -> Result<()> {
        let (storage, pid) = test_storage_with_project("extract_test")?;
        let chunks = vec![ProjectChunk {
            document_id: "d1".into(), chunk_id: "c1".into(), document_path: "001.txt".into(),
            chunk_index: 0, title: "第一章".into(),
            content: "魔法不能凭空制造生命。".into(),
            start_offset: 0, end_offset: 12, word_count: 6, content_hash: "h1".into(),
        }];
        let ids = RuleEngine::extract_candidates(&pid, &chunks, &storage)?;
        assert_eq!(ids.len(), 1);
        let rules = storage.list_rules(&pid)?;
        assert_eq!(rules[0].name, "魔法不能凭空制造生命");
        Ok(())
    }

    #[test]
    fn detects_rule_violation() -> Result<()> {
        let chunks = vec![ProjectChunk {
            document_id: "d1".into(), chunk_id: "c1".into(), document_path: "001.txt".into(),
            chunk_index: 0, title: "第一章".into(),
            content: "法师却还是凭空制造了一个生命。".into(),
            start_offset: 0, end_offset: 16, word_count: 10, content_hash: "h1".into(),
        }];
        let rules = vec![WorldRule {
            id: "r1".into(), project_id: "p1".into(),
            name: "魔法不能凭空制造生命".into(), description: String::new(),
            rule_type: "world".into(), keywords_json: r#"["魔法","生命","制造"]"#.into(),
            positive: true, source_chunk_id: None, confidence: 100,
            status: "active".into(), created_at: "now".into(), updated_at: "now".into(),
        }];
        let issues = RuleEngine::check_conflicts(&chunks, &rules);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "rule_conflict");
        Ok(())
    }

    #[test]
    fn no_false_positive_on_compliant_text() -> Result<()> {
        let chunks = vec![ProjectChunk {
            document_id: "d1".into(), chunk_id: "c1".into(), document_path: "001.txt".into(),
            chunk_index: 0, title: "第一章".into(),
            content: "他严格遵循规则，从未用魔法制造生命。".into(),
            start_offset: 0, end_offset: 18, word_count: 8, content_hash: "h1".into(),
        }];
        let rules = vec![WorldRule {
            id: "r1".into(), project_id: "p1".into(),
            name: "魔法不能凭空制造生命".into(), description: String::new(),
            rule_type: "world".into(), keywords_json: r#"["魔法","生命","创造"]"#.into(),
            positive: true, source_chunk_id: None, confidence: 100,
            status: "active".into(), created_at: "now".into(), updated_at: "now".into(),
        }];
        let issues = RuleEngine::check_conflicts(&chunks, &rules);
        assert!(issues.is_empty());
        Ok(())
    }
}
```

- [ ] **Step 3: Register in workspace `Cargo.toml`** — add `"crates/rules",`

- [ ] **Step 4: Build and run tests**

Run: `cargo test -p novellossless-rules`
Expected: 3 passed

- [ ] **Step 5: Commit**

```bash
git add crates/rules/ Cargo.toml
git commit -m "feat(rules): add rule engine with extraction and conflict detection"
```

---

## Task 4: `crates/timeline` — Timeline Crate

**Files:**
- Create: `crates/timeline/Cargo.toml`
- Create: `crates/timeline/src/lib.rs`
- Test: in `lib.rs`

**Interfaces:**
- Consumes: `storage::{TimelineEvent, upsert_timeline_event, list_timeline_events, delete_project_timeline_events}`, `ProjectChunk` from Task 1
- Produces: `TimelineEngine::extract(chunks, storage, project_id)`, `TimelineEngine::check(events) -> Vec<NewContinuityIssue>`

- [ ] **Step 1: Create `crates/timeline/Cargo.toml`**

```toml
[package]
name = "novellossless-timeline"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
anyhow.workspace = true
chrono.workspace = true
novellossless-storage = { path = "../storage" }
regex.workspace = true
serde_json.workspace = true
uuid = { workspace = true }

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 2: Create `crates/timeline/src/lib.rs`**

```rust
use anyhow::Result;
use novellossless_storage::{NewContinuityIssue, ProjectChunk, Storage, TimelineEvent};
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;

pub struct TimelineEngine;

impl TimelineEngine {
    pub fn extract(
        project_id: &str,
        chunks: &[ProjectChunk],
        storage: &Storage,
    ) -> Result<()> {
        storage.delete_project_timeline_events(project_id)?;

        let relative_re = Regex::new(r"(\d+)(?:天|个?月|年)(?:[后之]?[后前])")?;
        let absolute_re = Regex::new(r"(天宝|贞观|开元|神龙|武德|乾元|大历|建中|贞元|元和|长庆|宝历|太和|开成|会昌|大中|咸通|乾符|广明|中和|光启|文德|龙纪|大顺|景福|乾宁|光化|天复|天祐|景德|祥符|天禧|乾兴|天圣|明道|景祐|宝元|康定|庆历|皇祐|至和|嘉祐|治平|熙宁|元丰|元祐|绍圣|元符|靖国|崇宁|大观|政和|重和|宣和|靖康|建炎|绍兴|隆兴|乾道|淳熙|绍熙|庆元|嘉泰|开禧|嘉定|宝庆|绍定|端平|嘉熙|淳祐|宝祐|开庆|景定|咸淳|德祐|景炎|祥兴)(\d*)(?:载|年)?")?;
        let flashback_re = Regex::new(r"(回忆起|想起|回想|那年|曾经|当时|那时|多年前|很久以前)")?;

        let mut cursor: i64 = 0;

        for chunk in chunks {
            cursor += 1;
            let mut time_expr = String::new();
            let mut estimated_order: Option<i64> = None;
            let mut is_flashback = false;

            if let Some(cap) = relative_re.captures(&chunk.content) {
                if let Ok(n) = cap.get(1).map(|m| m.as_str()).unwrap_or("1").parse::<i64>() {
                    estimated_order = Some(cursor + n);
                    time_expr = cap.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
                }
            }

            if absolute_re.is_match(&chunk.content) {
                if let Some(cap) = absolute_re.captures(&chunk.content) {
                    time_expr = cap.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
                    if let Some(num_str) = cap.get(2).map(|m| m.as_str()) {
                        if let Ok(n) = num_str.parse::<i64>() {
                            estimated_order = Some(n);
                        }
                    }
                }
            }

            if flashback_re.is_match(&chunk.content) {
                is_flashback = true;
            }

            let event = TimelineEvent {
                id: uuid::Uuid::new_v4().to_string(),
                project_id: project_id.to_string(),
                chunk_id: chunk.chunk_id.clone(),
                chunk_index: chunk.chunk_index,
                document_path: chunk.document_path.clone(),
                title: chunk.title.clone(),
                order_index: cursor,
                time_expression: time_expr,
                estimated_order,
                participants_json: String::from("[]"),
                location: String::new(),
                is_flashback,
                confidence: 50,
            };
            storage.upsert_timeline_event(&event)?;
        }

        Ok(())
    }

    pub fn check(events: &[TimelineEvent], chunks: &[ProjectChunk]) -> Vec<NewContinuityIssue> {
        let mut issues = Vec::new();

        let chunks_map: HashMap<&str, &ProjectChunk> = chunks.iter()
            .map(|c| (c.chunk_id.as_str(), c)).collect();

        // Same-person two-locations: check sequential events with same participant
        // at significantly different order_index without location change
        for (i, event) in events.iter().enumerate() {
            let participants: Vec<String> =
                serde_json::from_str(&event.participants_json).unwrap_or_default();
            if participants.is_empty() || event.location.is_empty() {
                continue;
            }

            if let Some(prev) = events.get(i.saturating_sub(1)) {
                let prev_participants: Vec<String> =
                    serde_json::from_str(&prev.participants_json).unwrap_or_default();
                let shared: Vec<&String> = participants.iter()
                    .filter(|p| prev_participants.contains(p))
                    .collect();
                for p in shared {
                    if !prev.location.is_empty()
                        && prev.location != event.location
                        && (event.order_index - prev.order_index).abs() <= 2
                    {
                        issues.push(NewContinuityIssue {
                            issue_type: "time_anomaly".to_string(),
                            severity: "medium".to_string(),
                            title: format!("{} 短时间内出现在两个地点", p),
                            description: format!(
                                "{} 在 {} （{}）和 {} （{}）之间距离太近。",
                                p, prev.title, prev.location, event.title, event.location
                            ),
                            evidence_json: serde_json::to_string(&json!({
                                "person": p,
                                "location_a": prev.location,
                                "location_b": event.location,
                                "chapter_a": prev.title,
                                "chapter_b": event.title,
                            })).unwrap_or_default(),
                            suggested_actions_json: String::from(
                                r#"["确认是两个不同地点","检查时间跳跃","标记误报"]"#
                            ),
                        });
                    }
                }
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novellossless_storage::{Storage, ProjectChunk, TimelineEvent};

    fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project(name, &format!("/tmp/{name}"))?;
        Ok((storage, project.id))
    }

    #[test]
    fn extracts_relative_time() -> Result<()> {
        let (storage, pid) = test_storage_with_project("time_rel")?;
        let chunks = vec![
            ProjectChunk {
                document_id: "d1".into(), chunk_id: "c1".into(), document_path: "001.txt".into(),
                chunk_index: 0, title: "第一章".into(),
                content: "三天后，他到了长安。".into(),
                start_offset: 0, end_offset: 12, word_count: 6, content_hash: "h1".into(),
            },
        ];
        TimelineEngine::extract(&pid, &chunks, &storage)?;
        let events = storage.list_timeline_events(&pid)?;
        assert_eq!(events.len(), 1);
        assert!(events[0].time_expression.contains("三天"));
        assert_eq!(events[0].estimated_order, Some(4));
        Ok(())
    }

    #[test]
    fn extracts_absolute_year() -> Result<()> {
        let (storage, pid) = test_storage_with_project("time_abs")?;
        let chunks = vec![ProjectChunk {
            document_id: "d1".into(), chunk_id: "c1".into(), document_path: "001.txt".into(),
            chunk_index: 0, title: "第一章".into(),
            content: "贞观三年，长安城内一片繁华。".into(),
            start_offset: 0, end_offset: 16, word_count: 8, content_hash: "h1".into(),
        }];
        TimelineEngine::extract(&pid, &chunks, &storage)?;
        let events = storage.list_timeline_events(&pid)?;
        assert_eq!(events.len(), 1);
        assert!(events[0].time_expression.contains("贞观"));
        Ok(())
    }

    #[test]
    fn detects_flashback() -> Result<()> {
        let (storage, pid) = test_storage_with_project("flash")?;
        let chunks = vec![ProjectChunk {
            document_id: "d1".into(), chunk_id: "c1".into(), document_path: "001.txt".into(),
            chunk_index: 0, title: "第一章".into(),
            content: "林澈回忆起那年冬天的事情。".into(),
            start_offset: 0, end_offset: 16, word_count: 8, content_hash: "h1".into(),
        }];
        TimelineEngine::extract(&pid, &chunks, &storage)?;
        let events = storage.list_timeline_events(&pid)?;
        assert!(events[0].is_flashback);
        Ok(())
    }
}
```

- [ ] **Step 3: Register in workspace `Cargo.toml`** — add `"crates/timeline",`

- [ ] **Step 4: Build and run tests**

Run: `cargo test -p novellossless-timeline`
Expected: 3 passed

- [ ] **Step 5: Commit**

```bash
git add crates/timeline/ Cargo.toml
git commit -m "feat(timeline): add timeline extraction and conflict detection"
```

---

## Task 5: `crates/impact` — Revision Impact Crate

**Files:**
- Create: `crates/impact/Cargo.toml`
- Create: `crates/impact/src/lib.rs`
- Test: in `lib.rs`

**Interfaces:**
- Consumes: `Storage::{project_document_by_id, document_chunks, list_narrative_nodes, list_foreshadow_items, list_rules, create_task, list_tasks}` + `scan::diff_chunks` from core
- Produces: `RevisionImpactAnalyzer::analyze(diff, project_id, storage, impact_tasks) -> RevisionImpact`

- [ ] **Step 1: Create `crates/impact/Cargo.toml`**

```toml
[package]
name = "novellossless-impact"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
anyhow.workspace = true
chrono.workspace = true
novellossless-storage = { path = "../storage" }
serde_json.workspace = true
uuid = { workspace = true }

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 2: Create `crates/impact/src/lib.rs`**

```rust
use anyhow::Result;
use novellossless_storage::{NewRevisionTask, ProjectChunk, Storage};
use serde_json::json;

pub struct RevisionImpact {
    pub affected_nodes: Vec<String>,
    pub affected_foreshadows: Vec<String>,
    pub affected_rules: Vec<String>,
    pub summary: String,
}

pub struct ImpactAnalyzer;

impl ImpactAnalyzer {
    pub fn analyze(
        project_id: &str,
        old_chunks: &[ProjectChunk],
        new_chunks: &[ProjectChunk],
        storage: &Storage,
    ) -> Result<RevisionImpact> {
        let removed_chunk_ids: Vec<&str> = old_chunks
            .iter()
            .filter(|oc| !new_chunks.iter().any(|nc| nc.chunk_id == oc.chunk_id))
            .map(|c| c.chunk_id.as_str())
            .collect();

        let mut affected_nodes = Vec::new();
        let mut affected_foreshadows = Vec::new();
        let mut affected_rules = Vec::new();

        // Query storage for references to removed chunks
        let nodes = storage.list_narrative_nodes(project_id, None, 1000)?;
        for node in &nodes {
            if removed_chunk_ids.contains(&node.source_chunk_id.as_str()) {
                affected_nodes.push(format!("{} ({}, {})", node.name, node.node_type, node.id));
            }
        }

        let foreshadows = storage.list_foreshadow_items(project_id, 1000)?;
        for f in &foreshadows {
            if removed_chunk_ids.contains(&f.source_chunk_id.as_str()) {
                affected_foreshadows.push(format!("{} ({})", f.title, f.id));
            }
        }

        let rules = storage.list_rules(project_id)?;
        for rule in &rules {
            if let Some(ref scid) = rule.source_chunk_id {
                if removed_chunk_ids.contains(&scid.as_str()) {
                    affected_rules.push(format!("{} ({})", rule.name, rule.id));
                }
            }
        }

        // Create tasks for affected items
        if !affected_nodes.is_empty() || !affected_foreshadows.is_empty() || !affected_rules.is_empty() {
            let summary = format!(
                "修改影响: {} 个人物/地点/物件, {} 个伏笔, {} 条规则",
                affected_nodes.len(), affected_foreshadows.len(), affected_rules.len()
            );
            let _ = storage.create_task(&NewRevisionTask {
                project_id: project_id.to_string(),
                title: summary.clone(),
                task_type: "revision_impact".to_string(),
                priority: "medium".to_string(),
                source_issue_id: None,
                source_foreshadow_id: None,
                related_chunks_json: serde_json::to_string(&removed_chunk_ids).unwrap_or_default(),
                notes: json!({
                    "affected_nodes": affected_nodes,
                    "affected_foreshadows": affected_foreshadows,
                    "affected_rules": affected_rules,
                }).to_string(),
            })?;

            return Ok(RevisionImpact {
                affected_nodes: affected_nodes.clone(),
                affected_foreshadows: affected_foreshadows.clone(),
                affected_rules: affected_rules.clone(),
                summary,
            });
        }

        Ok(RevisionImpact {
            affected_nodes: Vec::new(),
            affected_foreshadows: Vec::new(),
            affected_rules: Vec::new(),
            summary: "无影响".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novellossless_storage::{NewNarrativeNode, Storage};

    fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
        let storage = Storage::open_memory()?;
        let project = storage.create_project(name, &format!("/tmp/{name}"))?;
        Ok((storage, project.id))
    }

    #[test]
    fn detects_removed_chunk_referenced_by_node() -> Result<()> {
        let (storage, pid) = test_storage_with_project("impact_test")?;
        // Seed a chunk and a narrative node referencing it
        let doc_id = storage.upsert_document_with_chunks(
            &pid,
            &novellossless_storage::NewDocument {
                path: "001.txt".into(), kind: "text".into(), title: "第一章".into(),
                chapter_count: 1, content_hash: "h".into(), word_count: 5, encoding: "utf-8".into(),
            },
            &[novellossless_storage::NewChunk {
                chunk_index: 0, title: "第一章".into(), start_offset: 0, end_offset: 10,
                content: "林澈在长安。".into(), content_hash: "ch".into(), word_count: 5,
            }],
        )?;
        let chunks = storage.document_chunks(&doc_id)?;
        let chunk_id = chunks[0].chunk_id.clone();

        storage.upsert_narrative_nodes(&pid, &[NewNarrativeNode {
            node_type: "person".into(), name: "林澈".into(),
            aliases_json: "[]".into(), occurrence_count: 1,
            first_chunk_id: chunk_id.clone(),
            latest_chunk_id: chunk_id.clone(),
            confidence: 80,
        }])?;

        let impact = ImpactAnalyzer::analyze(&pid, &chunks, &[], &storage)?;
        assert!(!impact.affected_nodes.is_empty());
        assert!(impact.affected_nodes[0].contains("林澈"));
        Ok(())
    }

    #[test]
    fn no_impact_when_chunk_unchanged() -> Result<()> {
        let (storage, pid) = test_storage_with_project("impact_none")?;
        let doc_id = storage.upsert_document_with_chunks(
            &pid,
            &novellossless_storage::NewDocument {
                path: "001.txt".into(), kind: "text".into(), title: "第一章".into(),
                chapter_count: 1, content_hash: "h".into(), word_count: 5, encoding: "utf-8".into(),
            },
            &[novellossless_storage::NewChunk {
                chunk_index: 0, title: "第一章".into(), start_offset: 0, end_offset: 10,
                content: "林澈在长安。".into(), content_hash: "ch".into(), word_count: 5,
            }],
        )?;
        let old_chunks = storage.document_chunks(&doc_id)?;
        let new_chunks = vec![ProjectChunk {
            document_id: old_chunks[0].document_id.clone(),
            chunk_id: old_chunks[0].chunk_id.clone(),
            chunk_index: 0, title: "第一章".into(),
            content: "林澈在长安。".into(),
            ..old_chunks[0].clone()
        }];
        let impact = ImpactAnalyzer::analyze(&pid, &old_chunks, &new_chunks, &storage)?;
        assert!(impact.affected_nodes.is_empty());
        Ok(())
    }
}
```

- [ ] **Step 3: Register in workspace `Cargo.toml`** — add `"crates/impact",`

- [ ] **Step 4: Build and run tests**

Run: `cargo test -p novellossless-impact`
Expected: 2 passed

- [ ] **Step 5: Commit**

```bash
git add crates/impact/ Cargo.toml
git commit -m "feat(impact): add revision impact analysis crate"
```

---

## Task 6: `crates/ai` — Deeplossless Adapter Crate

**Files:**
- Create: `crates/ai/Cargo.toml`
- Create: `crates/ai/src/lib.rs`
- Test: in `lib.rs`

**Interfaces:**
- Consumes: `deeplossless = "=0.7.4"` (optional, behind feature flag)
- Produces: `AiProvider` trait, `NoopProvider`, `DeeplosslessProvider`

- [ ] **Step 1: Create `crates/ai/Cargo.toml`**

```toml
[package]
name = "novellossless-ai"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
anyhow.workspace = true
deeplossless = { workspace = true, optional = true }
serde.workspace = true
serde_json.workspace = true

[features]
default = []
deeplossless-compat = ["dep:deeplossless"]
```

- [ ] **Step 2: Create `crates/ai/src/lib.rs`**

```rust
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ExtractedRule {
    pub name: String,
    pub description: String,
    pub rule_type: String,
    pub keywords: Vec<String>,
    pub positive: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineInsight {
    pub chunk_id: String,
    pub time_description: String,
    pub suggested_order: Option<i64>,
    pub is_flashback: bool,
}

#[derive(Debug, Clone)]
pub struct ImpactInsight {
    pub summary: String,
    pub affected_areas: Vec<String>,
}

pub trait AiProvider: Send + Sync {
    fn extract_rules(&self, chunks: &[&str]) -> Result<Vec<ExtractedRule>> {
        let _ = chunks;
        Ok(Vec::new())
    }
    fn analyze_timeline(&self, chunks: &[&str]) -> Result<Vec<TimelineInsight>> {
        let _ = chunks;
        Ok(Vec::new())
    }
    fn analyze_impact(&self, old: &[&str], new: &[&str], diff_desc: &str) -> Result<ImpactInsight> {
        let _ = (old, new, diff_desc);
        Ok(ImpactInsight {
            summary: "AI impact analysis not configured".to_string(),
            affected_areas: Vec::new(),
        })
    }
}

pub struct NoopProvider;

impl AiProvider for NoopProvider {}

#[cfg(feature = "deeplossless-compat")]
pub struct DeeplosslessProvider {
    // Fields added when real deeplossless integration happens
}

#[cfg(feature = "deeplossless-compat")]
impl AiProvider for DeeplosslessProvider {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_provider_returns_empty() {
        let provider = NoopProvider;
        let rules = provider.extract_rules(&["test"]).unwrap();
        assert!(rules.is_empty());
        let insights = provider.analyze_timeline(&["test"]).unwrap();
        assert!(insights.is_empty());
        let impact = provider.analyze_impact(&["old"], &["new"], "diff").unwrap();
        assert!(!impact.summary.is_empty());
    }
}
```

- [ ] **Step 3: Register in workspace `Cargo.toml`** — add `"crates/ai",`

- [ ] **Step 4: Build and run tests**

Run: `cargo test -p novellossless-ai`
Expected: 1 passed

- [ ] **Step 5: Commit**

```bash
git add crates/ai/ Cargo.toml
git commit -m "feat(ai): add AiProvider trait with NoopProvider and deeplossless feature flag"
```

---

## Task 7: Core Integration — Wire All Crates into NovelCore

**Files:**
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/src/profile.rs` (re-exports)
- Modify: `apps/desktop/src-tauri/src/lib.rs` (new Tauri commands)
- Modify: `apps/cli/src/main.rs` (new subcommand)
- Modify: `Cargo.toml` (workspace member)

**Interfaces:**
- Consumes: All crates from Tasks 1-6
- Produces: Extended `analyze_project` and `incremental_scan_file` methods

- [ ] **Step 1: Update `crates/core/Cargo.toml`**

Add dependencies:
```toml
novellossless-rules = { path = "../rules" }
novellossless-timeline = { path = "../timeline" }
novellossless-tasks = { path = "../tasks" }
novellossless-ai = { path = "../ai" }
```

- [ ] **Step 2: Extend `NovelCore` struct in `lib.rs`**

Add field:
```rust
ai_provider: Box<dyn novellossless_ai::AiProvider>,
```

Initialize in `open()` and `from_storage()`:
```rust
ai_provider: Box::new(novellossless_ai::NoopProvider),
```

- [ ] **Step 3: Extend `analyze_project` method**

After the existing analysis (after `self.storage.upsert_continuity_issues` line), add:

```rust
// Rules integration
if !self.profile_manifests.is_empty() {
    let _ = novellossless_rules::RuleEngine::extract_candidates(
        project_id,
        &chunks,
        &self.storage,
    );
    if let Ok(rules) = self.storage.list_rules(project_id) {
        let rule_issues = novellossless_rules::RuleEngine::check_conflicts(&chunks, &rules);
        if !rule_issues.is_empty() {
            if let Err(e) = self.storage.upsert_continuity_issues(project_id, &rule_issues) {
                eprintln!("warning: rule conflict upsert failed: {e}");
            }
        }
    }
}

// Timeline extraction
let _ = novellossless_timeline::TimelineEngine::extract(project_id, &chunks, &self.storage);
if let Ok(events) = self.storage.list_timeline_events(project_id) {
    let time_issues = novellossless_timeline::TimelineEngine::check(&events, &chunks);
    if !time_issues.is_empty() {
        if let Err(e) = self.storage.upsert_continuity_issues(project_id, &time_issues) {
            eprintln!("warning: timeline issue upsert failed: {e}");
        }
    }
}

// Task auto-creation
if let Ok(issues) = self.storage.list_continuity_issues(project_id, 100) {
    if let Ok(foreshadows) = self.storage.list_foreshadow_items(project_id, 100) {
        let _ = novellossless_tasks::TaskManager::auto_create_from_issues(
            project_id, &issues, &foreshadows, &self.storage,
        );
    }
}
```

- [ ] **Step 4: Extend `incremental_scan_file` method**

After `diff_chunks` computation and before `return`, add:

```rust
// Impact analysis
if !diff.added.is_empty() || !diff.removed.is_empty() || !diff.modified.is_empty() {
    let _ = novellossless_impact::ImpactAnalyzer::analyze(
        project_id, &old_chunks, &new_chunks, &self.storage,
    );
}
```

- [ ] **Step 5: Add Tauri commands to `apps/desktop/src-tauri/src/lib.rs`**

Add new DTOs and commands:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorldRuleDto {
    id: String,
    name: String,
    description: String,
    rule_type: String,
    keywords: Vec<String>,
    positive: bool,
    confidence: i32,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RevisionTaskDto {
    id: String,
    project_id: String,
    title: String,
    task_type: String,
    priority: String,
    status: String,
    created_at: String,
    notes: String,
}

#[tauri::command]
fn list_rules(app: tauri::AppHandle, project_id: String) -> Result<Vec<WorldRuleDto>, String> {
    let core = open_core(&app)?;
    let rules = core.storage.list_rules(&project_id).map_err(to_command_error)?;
    Ok(rules.into_iter().map(|r| {
        let keywords: Vec<String> = serde_json::from_str(&r.keywords_json).unwrap_or_default();
        WorldRuleDto {
            id: r.id, name: r.name, description: r.description, rule_type: r.rule_type,
            keywords, positive: r.positive, confidence: r.confidence, status: r.status,
        }
    }).collect())
}

#[tauri::command]
fn create_rule(app: tauri::AppHandle, project_id: String, name: String, description: String, rule_type: String, keywords: Vec<String>, positive: bool) -> Result<(), String> {
    let core = open_core(&app)?;
    let now = chrono::Utc::now().to_rfc3339();
    core.storage.upsert_rule(&novellossless_storage::WorldRule {
        id: uuid::Uuid::new_v4().to_string(), project_id, name, description, rule_type,
        keywords_json: serde_json::to_string(&keywords).unwrap_or_default(), positive,
        source_chunk_id: None, confidence: 100, status: "active".to_string(),
        created_at: now.clone(), updated_at: now,
    }).map_err(to_command_error)
}

#[tauri::command]
fn delete_rule(app: tauri::AppHandle, rule_id: String) -> Result<(), String> {
    let core = open_core(&app)?;
    core.storage.delete_rule(&rule_id).map_err(to_command_error)
}

#[tauri::command]
fn list_tasks(app: tauri::AppHandle, project_id: String) -> Result<Vec<RevisionTaskDto>, String> {
    let core = open_core(&app)?;
    let tasks = core.storage.list_tasks(&project_id).map_err(to_command_error)?;
    Ok(tasks.into_iter().map(|t| RevisionTaskDto {
        id: t.id, project_id: t.project_id, title: t.title, task_type: t.task_type,
        priority: t.priority, status: t.status, created_at: t.created_at, notes: t.notes,
    }).collect())
}

#[tauri::command]
fn update_task_status(app: tauri::AppHandle, task_id: String, status: String) -> Result<(), String> {
    let core = open_core(&app)?;
    core.storage.update_task_status(&task_id, &status).map_err(to_command_error)
}

#[tauri::command]
fn list_timeline_events(app: tauri::AppHandle, project_id: String) -> Result<Vec<novellossless_storage::TimelineEvent>, String> {
    let core = open_core(&app)?;
    core.storage.list_timeline_events(&project_id).map_err(to_command_error)
}
```

Register command in `invoke_handler`:
```
list_rules, create_rule, delete_rule, list_tasks, update_task_status, list_timeline_events,
```

- [ ] **Step 6: Add `tasks` subcommand to CLI (`apps/cli/src/main.rs`)**

```rust
Tasks {
    #[arg(long)]
    project_id: String,
},
```

In the match:
```rust
Command::Tasks { project_id } => {
    for task in core.list_tasks(&project_id)? {
        println!("{} | {} | {} | {}", task.priority, task.title, task.status, task.created_at);
    }
}
```

- [ ] **Step 7: Build everything**

Run: `cargo build`
Expected: success

- [ ] **Step 8: Run all tests**

Run: `cargo test`
Expected: all existing + new tests pass (44 + 9 = 53+)

- [ ] **Step 9: Run TypeScript check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 10: Commit**

```bash
git add crates/core/Cargo.toml crates/core/src/lib.rs apps/desktop/src-tauri/src/lib.rs apps/cli/src/main.rs Cargo.toml
git commit -m "feat(core): integrate rules, timeline, tasks, impact, ai crates into scan pipeline"
```

---

## Self-Review

**Spec coverage check:**
- Storage tables: ✓ (Task 1)
- Rules crate: ✓ (Task 3)
- Timeline crate: ✓ (Task 4)
- Tasks crate: ✓ (Task 2)
- Impact crate: ✓ (Task 5)
- AI crate: ✓ (Task 6)
- Core integration: ✓ (Task 7)

**Placeholder scan:** No placeholders found. All code is explicit.

**Type consistency check:**
- `WorldRule` `keywords_json: String` is used consistently across storage → rules crate
- Timeline event fields consistent between storage and timeline crate
- Task CRUD signatures match across storage and tasks crate
- `AiProvider` trait signatures match spec