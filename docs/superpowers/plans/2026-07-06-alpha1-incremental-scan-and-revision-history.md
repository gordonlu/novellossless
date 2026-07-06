# 增量扫描 + 文件监听 + 修订历史 Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development to implement task-by-task.

**Goal:** Add hash-gated incremental scanning, `notify`-based file watching with 800ms debounce, and per-document revision history with chunk-level diffs.

**Architecture:** Storage owns new tables (`file_scan_log`, `revision_history`) and CRUD methods. Core owns chunk-diff logic (`diff_chunks`) and `incremental_scan`/`incremental_scan_file`. Desktop layer owns `FileWatcher` wrapping `notify` with debounce. Each Tauri command (including the watcher thread) opens its own `NovelCore` via the existing `open_core()` pattern — no shared mutable state.

**Tech Stack:** Rust (rusqlite, notify 8, sha2, serde_json), Tauri 2, React/TypeScript

## Global Constraints

- `notify = "8"` pinned in `apps/desktop/src-tauri/Cargo.toml` only
- New SQL tables use `IF NOT EXISTS`
- `deleted` column in `documents` table already exists (0/1)
- Existing `upsert_document_with_chunks` already sets `deleted = 0`
- All storage methods follow existing `self.conn.execute()` + `params![]` pattern
- `cargo test` must pass after each task
- `npx tsc --noEmit` must pass after UI tasks
- Keep UTF-8 / UTF-8 BOM / GB18030 support

---
## File Structure

### Create:
- `crates/core/src/scan.rs`
- `apps/desktop/src-tauri/src/watcher.rs`
- `apps/desktop/src/routes/RevisionHistory.tsx`

### Modify:
- `crates/storage/src/lib.rs`
- `crates/core/src/lib.rs`
- `apps/desktop/src-tauri/Cargo.toml`
- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src/tauri.ts`
- `apps/desktop/src/App.tsx`
- `apps/desktop/src/styles.css`
- `apps/cli/src/main.rs`

---

### Task 1: Storage — file_scan_log + revision_history tables and methods

**Files:**
- Modify: `crates/storage/src/lib.rs`

**Interfaces Produced:**
- `FileScanLog` struct, `RevisionRecord` struct
- `Storage::record_file_scan(project_id, document_id, old_hash, new_hash, event_type, details) -> Result<String>`
- `Storage::record_revision(project_id, document_id, revision_type, old_hash, new_hash, old_chunk_count, new_chunk_count, chunks_added, chunks_removed, chunks_modified, diff_json) -> Result<String>`
- `Storage::list_file_scans(project_id, limit) -> Result<Vec<FileScanLog>>`
- `Storage::list_revisions(project_id, document_id, limit) -> Result<Vec<RevisionRecord>>`
- `Storage::project_document_by_id(id) -> Result<ProjectDocument>` (add `content_hash` field)
- `Storage::document_chunks(document_id) -> Result<Vec<ProjectChunk>>`
- `Storage::mark_document_deleted(id) -> Result<()>`

- [ ] **Step 1: Add structs + `content_hash` to `ProjectDocument`**

After existing structs, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileScanLog {
    pub id: String,
    pub project_id: String,
    pub document_id: String,
    pub old_hash: Option<String>,
    pub new_hash: String,
    pub event_type: String,
    pub scanned_at: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRecord {
    pub id: String,
    pub project_id: String,
    pub document_id: String,
    pub revision_type: String,
    pub old_content_hash: Option<String>,
    pub new_content_hash: String,
    pub old_chunk_count: i64,
    pub new_chunk_count: i64,
    pub chunks_added: i64,
    pub chunks_removed: i64,
    pub chunks_modified: i64,
    pub diff_json: Option<String>,
    pub created_at: String,
}
```

Add `content_hash: String` field to `ProjectDocument`.

- [ ] **Step 2: Add migration SQL in `Storage::open`**

After the `context_packs` table creation, add:

```rust
self.conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS file_scan_log (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
        old_hash TEXT,
        new_hash TEXT NOT NULL,
        event_type TEXT NOT NULL,
        scanned_at TEXT NOT NULL,
        details TEXT
    );
    CREATE TABLE IF NOT EXISTS revision_history (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
        revision_type TEXT NOT NULL,
        old_content_hash TEXT,
        new_content_hash TEXT NOT NULL,
        old_chunk_count INTEGER NOT NULL DEFAULT 0,
        new_chunk_count INTEGER NOT NULL DEFAULT 0,
        chunks_added INTEGER NOT NULL DEFAULT 0,
        chunks_removed INTEGER NOT NULL DEFAULT 0,
        chunks_modified INTEGER NOT NULL DEFAULT 0,
        diff_json TEXT,
        created_at TEXT NOT NULL
    );",
)?;
```

- [ ] **Step 3: Add 6 storage methods to `impl Storage`**

```rust
pub fn record_file_scan(&self, project_id: &str, document_id: &str,
    old_hash: Option<&str>, new_hash: &str, event_type: &str,
    details: Option<&str>) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    self.conn.execute(
        "INSERT INTO file_scan_log (id, project_id, document_id, old_hash, new_hash, event_type, scanned_at, details)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, project_id, document_id, old_hash, new_hash, event_type, now, details],
    )?;
    Ok(id)
}

pub fn record_revision(&self, project_id: &str, document_id: &str,
    revision_type: &str, old_hash: Option<&str>, new_hash: &str,
    old_chunk_count: i64, new_chunk_count: i64,
    chunks_added: i64, chunks_removed: i64, chunks_modified: i64,
    diff_json: Option<&str>) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    self.conn.execute(
        "INSERT INTO revision_history (id, project_id, document_id, revision_type, old_content_hash, new_content_hash, old_chunk_count, new_chunk_count, chunks_added, chunks_removed, chunks_modified, diff_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![id, project_id, document_id, revision_type, old_hash, new_hash, old_chunk_count, new_chunk_count, chunks_added, chunks_removed, chunks_modified, diff_json, now],
    )?;
    Ok(id)
}

pub fn list_file_scans(&self, project_id: &str, limit: i64) -> Result<Vec<FileScanLog>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, project_id, document_id, old_hash, new_hash, event_type, scanned_at, details
         FROM file_scan_log WHERE project_id = ?1 ORDER BY scanned_at DESC LIMIT ?2",
    )?;
    stmt.query_map(params![project_id, limit], |row| {
        Ok(FileScanLog {
            id: row.get(0)?, project_id: row.get(1)?, document_id: row.get(2)?,
            old_hash: row.get(3)?, new_hash: row.get(4)?, event_type: row.get(5)?,
            scanned_at: row.get(6)?, details: row.get(7)?,
        })
    })?.collect()
}

pub fn list_revisions(&self, project_id: &str, document_id: Option<&str>, limit: i64) -> Result<Vec<RevisionRecord>> {
    let (clause, pid, doc): (String, String, String) = match document_id {
        Some(d) => ("WHERE project_id = ?1 AND document_id = ?2".into(), project_id.to_string(), d.to_string()),
        None => ("WHERE project_id = ?1".into(), project_id.to_string(), String::new()),
    };
    let sql = format!(
        "SELECT id, project_id, document_id, revision_type, old_content_hash, new_content_hash,
         old_chunk_count, new_chunk_count, chunks_added, chunks_removed, chunks_modified,
         diff_json, created_at FROM revision_history {clause} ORDER BY created_at DESC LIMIT ?3"
    );
    let mut stmt = self.conn.prepare(&sql)?;
    let rows = if document_id.is_some() {
        stmt.query_map(params![pid, doc, limit], |row| { Ok(RevisionRecord {
            id: row.get(0)?, project_id: row.get(1)?, document_id: row.get(2)?,
            revision_type: row.get(3)?, old_content_hash: row.get(4)?, new_content_hash: row.get(5)?,
            old_chunk_count: row.get(6)?, new_chunk_count: row.get(7)?,
            chunks_added: row.get(8)?, chunks_removed: row.get(9)?, chunks_modified: row.get(10)?,
            diff_json: row.get(11)?, created_at: row.get(12)?,
        })})?
    } else {
        stmt.query_map(params![pid, limit], |row| { Ok(RevisionRecord {
            id: row.get(0)?, project_id: row.get(1)?, document_id: row.get(2)?,
            revision_type: row.get(3)?, old_content_hash: row.get(4)?, new_content_hash: row.get(5)?,
            old_chunk_count: row.get(6)?, new_chunk_count: row.get(7)?,
            chunks_added: row.get(8)?, chunks_removed: row.get(9)?, chunks_modified: row.get(10)?,
            diff_json: row.get(11)?, created_at: row.get(12)?,
        })})?
    };
    rows.collect()
}

pub fn project_document_by_id(&self, id: &str) -> Result<ProjectDocument> {
    self.conn.query_row(
        "SELECT id, path, title, chapter_count, word_count, content_hash FROM documents WHERE id = ?1 AND deleted = 0",
        params![id],
        |row| Ok(ProjectDocument {
            id: row.get(0)?, path: row.get(1)?, title: row.get(2)?,
            chapter_count: row.get(3)?, word_count: row.get(4)?, content_hash: row.get(5)?,
        }),
    ).map_err(Into::into)
}

pub fn document_chunks(&self, document_id: &str) -> Result<Vec<ProjectChunk>> {
    let mut stmt = self.conn.prepare(
        "SELECT ch.document_id, ch.id, d.path, ch.chunk_index, ch.title, ch.content, ch.start_offset, ch.end_offset, ch.word_count
         FROM document_chunks ch JOIN documents d ON d.id = ch.document_id
         WHERE ch.document_id = ?1 ORDER BY ch.chunk_index ASC",
    )?;
    stmt.query_map(params![document_id], |row| Ok(ProjectChunk {
        document_id: row.get(0)?, chunk_id: row.get(1)?, document_path: row.get(2)?,
        chunk_index: row.get(3)?, title: row.get(4)?, content: row.get(5)?,
        start_offset: row.get(6)?, end_offset: row.get(7)?, word_count: row.get(8)?,
    }))?.collect()
}

pub fn mark_document_deleted(&self, id: &str) -> Result<()> {
    self.conn.execute("UPDATE documents SET deleted = 1 WHERE id = ?1", params![id])?;
    Ok(())
}
```

- [ ] **Step 4: Write tests**

Add these tests inside `#[cfg(test)] mod tests`:

```rust
fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
    let storage = test_storage()?;
    let project = storage.create_project(name, &format!("/tmp/{name}"))?;
    Ok((storage, project.id))
}

#[test]
fn records_and_lists_file_scan_logs() -> Result<()> {
    let (storage, pid) = test_storage_with_project("scan_log_test")?;
    let id = storage.record_file_scan(&pid, "doc-1", None, "abc", "created", None)?;
    assert!(!id.is_empty());
    let logs = storage.list_file_scans(&pid, 10)?;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].event_type, "created");
    assert_eq!(logs[0].new_hash, "abc");
    Ok(())
}

#[test]
fn records_and_lists_revisions() -> Result<()> {
    let (storage, pid) = test_storage_with_project("rev_test")?;
    let diff = r#"[{"kind":"modified","index":0}]"#;
    let id = storage.record_revision(&pid, "doc-1", "incremental",
        Some("old"), "new", 1, 1, 0, 0, 1, Some(diff))?;
    assert!(!id.is_empty());
    let revs = storage.list_revisions(&pid, Some("doc-1"), 10)?;
    assert_eq!(revs.len(), 1);
    assert_eq!(revs[0].chunks_modified, 1);
    Ok(())
}
```

- [ ] **Step 5: Run and commit**

```bash
cd /home/gordon/code/novellossless
cargo test -p novellossless-storage -- --test-threads=1 2>&1
git add -A && git commit -m "feat(storage): file_scan_log and revision_history tables"
```

---

### Task 2: Core — ChunkDiff + incremental_scan

**Files:**
- Create: `crates/core/src/scan.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces Produced:**
- `ChunkDiff`, `ChunkDiffEntry`, `ModifiedEntry` structs
- `pub fn diff_chunks(old: &[ProjectChunk], new: &[NewChunk]) -> ChunkDiff`
- `pub struct ScanResult { scanned_documents, skipped_files, created, modified, unchanged, deleted, failed }`
- `NovelCore::incremental_scan(project_id) -> Result<ScanResult>`
- `NovelCore::incremental_scan_file(project_id, file_path) -> Result<ScanResult>`

- [ ] **Step 1: Create `crates/core/src/scan.rs`**

```rust
use novellossless_storage::{NewChunk, ProjectChunk};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkDiff {
    pub added: Vec<ChunkDiffEntry>,
    pub removed: Vec<ChunkDiffEntry>,
    pub modified: Vec<ModifiedEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkDiffEntry {
    pub index: i64,
    pub title: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedEntry {
    pub index: i64,
    pub old_title: String,
    pub new_title: String,
    pub old_hash: String,
    pub new_hash: String,
}

pub fn diff_chunks(old_chunks: &[ProjectChunk], new_chunks: &[NewChunk]) -> ChunkDiff {
    let old_by_idx: HashMap<i64, &ProjectChunk> = old_chunks.iter().map(|c| (c.chunk_index, c)).collect();
    let new_by_idx: HashMap<i64, &NewChunk> = new_chunks.iter().map(|c| (c.chunk_index, c)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    for (&idx, nc) in &new_by_idx {
        match old_by_idx.get(&idx) {
            None => added.push(ChunkDiffEntry { index: idx, title: nc.title.clone(), hash: nc.content_hash.clone() }),
            Some(oc) if oc.title != nc.title || oc.content_hash != nc.content_hash => {
                modified.push(ModifiedEntry {
                    index: idx,
                    old_title: oc.title.clone(), new_title: nc.title.clone(),
                    old_hash: oc.content_hash.clone(), new_hash: nc.content_hash.clone(),
                });
            }
            _ => {}
        }
    }

    for (&idx, oc) in &old_by_idx {
        if !new_by_idx.contains_key(&idx) {
            removed.push(ChunkDiffEntry { index: idx, title: oc.title.clone(), hash: oc.content_hash.clone() });
        }
    }

    ChunkDiff { added, removed, modified }
}
```

Add `mod scan;` in `lib.rs`.

- [ ] **Step 2: Add `ScanResult` and helper to `lib.rs`**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    pub scanned_documents: usize,
    pub skipped_files: usize,
    pub created: usize,
    pub modified: usize,
    pub unchanged: usize,
    pub deleted: usize,
    pub failed: usize,
}

use std::collections::HashSet;

fn file_content_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let content = std::fs::read(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(hex::encode(Sha256::digest(&content)))
}
```

- [ ] **Step 3: Add `incremental_scan` and `incremental_scan_file` to `impl NovelCore`**

```rust
pub fn incremental_scan(&self, project_id: &str) -> Result<ScanResult> {
    let project = self.storage.get_project(project_id)?
        .with_context(|| format!("project not found: {project_id}"))?;
    let root = PathBuf::from(&project.root_path);
    let files = collect_text_files(&root)?;
    let profile = self.profiles.first();
    let enable_chunking = profile.map(|p| p.rules.chapter_recognition).unwrap_or(true);

    let mut created = 0usize;
    let mut modified = 0usize;
    let mut unchanged = 0usize;
    let mut failed = 0usize;
    let mut file_paths: HashSet<String> = HashSet::new();

    for file in &files {
        let relative = relative_document_path(&root, file);
        file_paths.insert(relative.clone());
        let hash = file_content_hash(file)?;

        match self.storage.existing_document_id(project_id, &relative)? {
            None => {
                match self.scan_file(&project, &root, file, enable_chunking) {
                    Ok(()) => {
                        created += 1;
                        if let Ok(Some(doc_id)) = self.storage.existing_document_id(project_id, &relative) {
                            let _ = self.storage.record_file_scan(project_id, &doc_id, None, &hash, "created", None);
                        }
                    }
                    Err(_) => failed += 1,
                }
            }
            Some(doc_id) => {
                let current_doc = self.storage.project_document_by_id(&doc_id)?;
                if current_doc.content_hash == hash {
                    unchanged += 1;
                    let _ = self.storage.record_file_scan(project_id, &doc_id, Some(&hash), &hash, "unchanged", None);
                } else {
                    let old_chunks = self.storage.document_chunks(&doc_id)?;
                    match self.scan_file(&project, &root, file, enable_chunking) {
                        Ok(()) => {
                            modified += 1;
                            let parsed = novellossless_parser::parse_document(file)?;
                            let chapters = if enable_chunking { parsed.chapters } else {
                                vec![novellossless_parser::Chapter {
                                    index: 0, title: parsed.title.clone(), start_offset: 0,
                                    end_offset: parsed.content.len(), content: parsed.content.clone(),
                                }]
                            };
                            let new_chunks: Vec<NewChunk> = chapters.iter().map(|ch| NewChunk {
                                chunk_index: ch.index as i64, title: ch.title.clone(),
                                start_offset: ch.start_offset as i64, end_offset: ch.end_offset as i64,
                                content: ch.content.clone(),
                                content_hash: sha256_hex(ch.content.as_bytes()),
                                word_count: count_words(&ch.content) as i64,
                            }).collect();
                            let diff = scan::diff_chunks(&old_chunks, &new_chunks);
                            let diff_arr: Vec<serde_json::Value> = {
                                let mut v = Vec::new();
                                for a in &diff.added { v.push(serde_json::json!({"kind":"added","index":a.index,"title":a.title})); }
                                for r in &diff.removed { v.push(serde_json::json!({"kind":"removed","index":r.index,"title":r.title})); }
                                for m in &diff.modified { v.push(serde_json::json!({"kind":"modified","index":m.index,"old_title":m.old_title,"new_title":m.new_title})); }
                                v
                            };
                            let diff_json = serde_json::to_string(&diff_arr).ok();
                            let _ = self.storage.record_file_scan(project_id, &doc_id, Some(&current_doc.content_hash), &hash, "modified", None);
                            let _ = self.storage.record_revision(project_id, &doc_id, "incremental",
                                Some(&current_doc.content_hash), &hash,
                                old_chunks.len() as i64, new_chunks.len() as i64,
                                diff.added.len() as i64, diff.removed.len() as i64, diff.modified.len() as i64,
                                diff_json.as_deref());
                        }
                        Err(_) => failed += 1,
                    }
                }
            }
        }
    }

    let mut deleted = 0usize;
    for doc in self.storage.project_documents(project_id)? {
        if !file_paths.contains(&doc.path) {
            self.storage.mark_document_deleted(&doc.id)?;
            let _ = self.storage.record_file_scan(project_id, &doc.id, Some(&doc.content_hash), &doc.content_hash, "deleted", None);
            deleted += 1;
        }
    }

    let _ = self.analyze_project(project_id)?;

    Ok(ScanResult {
        scanned_documents: created + modified,
        skipped_files: failed,
        created, modified, unchanged, deleted, failed,
    })
}

pub fn incremental_scan_file(&self, project_id: &str, file_path: &Path) -> Result<ScanResult> {
    let project = self.storage.get_project(project_id)?
        .with_context(|| format!("project not found: {project_id}"))?;
    let root = PathBuf::from(&project.root_path);
    let profile = self.profiles.first();
    let enable_chunking = profile.map(|p| p.rules.chapter_recognition).unwrap_or(true);
    let relative = relative_document_path(&root, file_path);
    let hash = file_content_hash(file_path)?;

    let mut result = ScanResult { scanned_documents: 0, skipped_files: 0, created: 0, modified: 0, unchanged: 0, deleted: 0, failed: 0 };

    match self.storage.existing_document_id(project_id, &relative)? {
        None => {
            match self.scan_file(&project, &root, file_path, enable_chunking) {
                Ok(()) => {
                    result.created = 1;
                    if let Ok(Some(doc_id)) = self.storage.existing_document_id(project_id, &relative) {
                        let _ = self.storage.record_file_scan(project_id, &doc_id, None, &hash, "created", None);
                    }
                }
                Err(_) => result.failed = 1,
            }
        }
        Some(doc_id) => {
            let current_doc = self.storage.project_document_by_id(&doc_id)?;
            if current_doc.content_hash == hash {
                result.unchanged = 1;
                let _ = self.storage.record_file_scan(project_id, &doc_id, Some(&hash), &hash, "unchanged", None);
            } else {
                let old_chunks = self.storage.document_chunks(&doc_id)?;
                match self.scan_file(&project, &root, file_path, enable_chunking) {
                    Ok(()) => {
                        result.modified = 1;
                        let parsed = novellossless_parser::parse_document(file_path)?;
                        let chapters = if enable_chunking { parsed.chapters } else {
                            vec![novellossless_parser::Chapter {
                                index: 0, title: parsed.title.clone(), start_offset: 0,
                                end_offset: parsed.content.len(), content: parsed.content.clone(),
                            }]
                        };
                        let new_chunks: Vec<NewChunk> = chapters.iter().map(|ch| NewChunk {
                            chunk_index: ch.index as i64, title: ch.title.clone(),
                            start_offset: ch.start_offset as i64, end_offset: ch.end_offset as i64,
                            content: ch.content.clone(),
                            content_hash: sha256_hex(ch.content.as_bytes()),
                            word_count: count_words(&ch.content) as i64,
                        }).collect();
                        let diff = scan::diff_chunks(&old_chunks, &new_chunks);
                        let diff_arr: Vec<serde_json::Value> = {
                            let mut v = Vec::new();
                            for a in &diff.added { v.push(serde_json::json!({"kind":"added","index":a.index,"title":a.title})); }
                            for r in &diff.removed { v.push(serde_json::json!({"kind":"removed","index":r.index,"title":r.title})); }
                            for m in &diff.modified { v.push(serde_json::json!({"kind":"modified","index":m.index,"old_title":m.old_title,"new_title":m.new_title})); }
                            v
                        };
                        let diff_json = serde_json::to_string(&diff_arr).ok();
                        let _ = self.storage.record_file_scan(project_id, &doc_id, Some(&current_doc.content_hash), &hash, "modified", None);
                        let _ = self.storage.record_revision(project_id, &doc_id, "incremental",
                            Some(&current_doc.content_hash), &hash,
                            old_chunks.len() as i64, new_chunks.len() as i64,
                            diff.added.len() as i64, diff.removed.len() as i64, diff.modified.len() as i64,
                            diff_json.as_deref());
                    }
                    Err(_) => result.failed = 1,
                }
            }
        }
    }

    result.scanned_documents = result.created + result.modified;
    let _ = self.analyze_project(project_id)?;
    Ok(result)
}
```

- [ ] **Step 4: Add `list_file_scans` and `list_revisions` pass-throughs to NovelCore**

```rust
pub fn list_file_scans(&self, project_id: &str, limit: i64) -> Result<Vec<novellossless_storage::FileScanLog>> {
    self.storage.list_file_scans(project_id, limit)
}

pub fn list_revisions(&self, project_id: &str, document_id: Option<&str>, limit: i64) -> Result<Vec<novellossless_storage::RevisionRecord>> {
    self.storage.list_revisions(project_id, document_id, limit)
}
```

- [ ] **Step 5: Write test for `diff_chunks`**

Add inside `#[cfg(test)]`:

```rust
#[test]
fn diff_chunks_detects_changes() {
    use crate::scan::diff_chunks;
    use novellossless_storage::{NewChunk, ProjectChunk};

    let old = vec![
        ProjectChunk { document_id: "d".into(), chunk_id: "c1".into(), document_path: "p".into(), chunk_index: 0, title: "A".into(), content: "aa".into(), start_offset: 0, end_offset: 2, word_count: 1 },
        ProjectChunk { document_id: "d".into(), chunk_id: "c2".into(), document_path: "p".into(), chunk_index: 1, title: "B".into(), content: "bb".into(), start_offset: 3, end_offset: 5, word_count: 1 },
    ];
    let new = vec![
        NewChunk { chunk_index: 0, title: "A".into(), start_offset: 0, end_offset: 2, content: "aa".into(), content_hash: "same".into(), word_count: 1 },
        NewChunk { chunk_index: 1, title: "B2".into(), start_offset: 3, end_offset: 6, content: "bbb".into(), content_hash: "diff".into(), word_count: 1 },
        NewChunk { chunk_index: 2, title: "C".into(), start_offset: 7, end_offset: 9, content: "cc".into(), content_hash: "new".into(), word_count: 1 },
    ];

    let diff = diff_chunks(&old, &new);
    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].index, 2);
    assert_eq!(diff.removed.len(), 0);
    assert_eq!(diff.modified.len(), 1);
    assert_eq!(diff.modified[0].old_title, "B");
    assert_eq!(diff.modified[0].new_title, "B2");
}
```

- [ ] **Step 6: Run and commit**

```bash
cd /home/gordon/code/novellossless
cargo test -- --test-threads=1 2>&1
git add -A && git commit -m "feat(core): chunk diff, incremental_scan, incremental_scan_file"
```

---

### Task 3: File Watcher (desktop layer)

**Files:**
- Create: `apps/desktop/src-tauri/src/watcher.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`

- [ ] **Step 1: Add `notify = "8"` to `apps/desktop/src-tauri/Cargo.toml`**

- [ ] **Step 2: Create `apps/desktop/src-tauri/src/watcher.rs`**

```rust
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const SUPPORTED: &[&str] = &["txt", "md", "markdown"];
const DEBOUNCE_MS: u64 = 800;

pub struct FileWatcher {
    project_id: String,
    root: PathBuf,
    watcher: Option<RecommendedWatcher>,
    running: Arc<Mutex<bool>>,
}

impl FileWatcher {
    pub fn start<F>(project_id: &str, root: &Path, on_change: F) -> Result<Self, String>
    where
        F: Fn(String, PathBuf) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
        use notify::Config;

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| format!("failed to create watcher: {e}"))?;

        watcher.watch(root, RecursiveMode::Recursive)
            .map_err(|e| format!("failed to watch {root:?}: {e}"))?;

        let running = Arc::new(Mutex::new(true));
        let r = running.clone();
        let pid = project_id.to_string();

        thread::spawn(move || {
            let mut pending: HashSet<PathBuf> = HashSet::new();
            let mut last_event = Instant::now();

            loop {
                if !*r.lock().unwrap() { break; }
                match rx.recv_timeout(Duration::from_millis(200)) {
                    Ok(Ok(event)) => {
                        for path in &event.paths {
                            if is_supported(path) {
                                pending.insert(path.clone());
                                last_event = Instant::now();
                            }
                        }
                    }
                    Ok(Err(_)) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if !pending.is_empty() && last_event.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
                            let batch: Vec<PathBuf> = pending.drain().collect();
                            for path in batch {
                                on_change(pid.clone(), path);
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok(Self { project_id: project_id.to_string(), root: root.to_path_buf(), watcher: Some(watcher), running })
    }

    pub fn stop(&mut self) {
        if let Ok(mut r) = self.running.lock() {
            *r = false;
        }
        if let Some(w) = self.watcher.take() {
            let _ = w.unwatch(&self.root);
            drop(w);
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.lock().map(|r| *r).unwrap_or(false)
    }
}

fn is_supported(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') || name.starts_with('~') || name.ends_with('~') || name.contains(".swp") {
            return false;
        }
    }
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED.contains(&e))
}
```

- [ ] **Step 3: Add `mod watcher;` to `lib.rs`**

```rust
mod watcher;
```

- [ ] **Step 4: Build and commit**

```bash
cd /home/gordon/code/novellossless
cargo build -p novellossless-desktop 2>&1 | grep -v "^warning:"
git add -A && git commit -m "feat(desktop): file watcher with 800ms debounce"
```

---

### Task 4: Tauri Commands

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Add DTOs for new types**

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResultDto {
    scanned_documents: usize, skipped_files: usize,
    created: usize, modified: usize, unchanged: usize, deleted: usize, failed: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileScanLogDto {
    id: String, project_id: String, document_id: String,
    old_hash: Option<String>, new_hash: String,
    event_type: String, scanned_at: String, details: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RevisionRecordDto {
    id: String, project_id: String, document_id: String,
    revision_type: String,
    old_content_hash: Option<String>, new_content_hash: String,
    old_chunk_count: i64, new_chunk_count: i64,
    chunks_added: i64, chunks_removed: i64, chunks_modified: i64,
    diff_json: Option<String>, created_at: String,
}
```

- [ ] **Step 2: Add From impls + commands**

```rust
impl From<novellossless_core::ScanResult> for ScanResultDto {
    fn from(r: novellossless_core::ScanResult) -> Self {
        Self {
            scanned_documents: r.scanned_documents, skipped_files: r.skipped_files,
            created: r.created, modified: r.modified, unchanged: r.unchanged, deleted: r.deleted, failed: r.failed,
        }
    }
}

impl From<novellossless_storage::FileScanLog> for FileScanLogDto {
    fn from(l: novellossless_storage::FileScanLog) -> Self {
        Self {
            id: l.id, project_id: l.project_id, document_id: l.document_id,
            old_hash: l.old_hash, new_hash: l.new_hash,
            event_type: l.event_type, scanned_at: l.scanned_at, details: l.details,
        }
    }
}

impl From<novellossless_storage::RevisionRecord> for RevisionRecordDto {
    fn from(r: novellossless_storage::RevisionRecord) -> Self {
        Self {
            id: r.id, project_id: r.project_id, document_id: r.document_id,
            revision_type: r.revision_type,
            old_content_hash: r.old_content_hash, new_content_hash: r.new_content_hash,
            old_chunk_count: r.old_chunk_count, new_chunk_count: r.new_chunk_count,
            chunks_added: r.chunks_added, chunks_removed: r.chunks_removed, chunks_modified: r.chunks_modified,
            diff_json: r.diff_json, created_at: r.created_at,
        }
    }
}

#[tauri::command]
fn incremental_scan(app: tauri::AppHandle, project_id: String) -> Result<ScanResultDto, String> {
    let core = open_core(&app)?;
    core.incremental_scan(&project_id).map(ScanResultDto::from).map_err(to_command_error)
}

#[tauri::command]
fn list_file_scans(app: tauri::AppHandle, project_id: String, limit: i64) -> Result<Vec<FileScanLogDto>, String> {
    let core = open_core(&app)?;
    core.list_file_scans(&project_id, limit)
        .map(|v| v.into_iter().map(FileScanLogDto::from).collect())
        .map_err(to_command_error)
}

#[tauri::command]
fn list_revisions(app: tauri::AppHandle, project_id: String, document_id: Option<String>, limit: i64) -> Result<Vec<RevisionRecordDto>, String> {
    let core = open_core(&app)?;
    core.list_revisions(&project_id, document_id.as_deref(), limit)
        .map(|v| v.into_iter().map(RevisionRecordDto::from).collect())
        .map_err(to_command_error)
}

use std::sync::Mutex;

struct WatcherState(Mutex<Option<crate::watcher::FileWatcher>>);

#[tauri::command]
fn start_watching(app: tauri::AppHandle, project_id: String) -> Result<(), String> {
    let core = open_core(&app)?;
    let project = core.get_project(&project_id).map_err(to_command_error)?
        .ok_or_else(|| "project not found".to_string())?;
    let root = PathBuf::from(&project.root_path);

    let app_handle = app.clone();
    let watcher = crate::watcher::FileWatcher::start(&project_id, &root, move |pid, path| {
        if let Ok(core) = open_core(&app_handle) {
            let _ = core.incremental_scan_file(&pid, &path);
        }
    })?;

    if let Some(state) = app.try_state::<WatcherState>() {
        if let Ok(mut w) = state.0.lock() {
            if let Some(old) = w.replace(watcher) {
                old.stop();
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn stop_watching(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<WatcherState>() {
        if let Ok(mut w) = state.0.lock() {
            if let Some(ref mut watcher) = *w {
                watcher.stop();
            }
            *w = None;
        }
    }
    Ok(())
}

#[tauri::command]
fn watcher_status(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(app.try_state::<WatcherState>()
        .and_then(|s| s.0.lock().ok())
        .map(|w| w.is_some())
        .unwrap_or(false))
}
```

Actually, the watcher lifetime management is tricky with the `open_core` pattern. Let me use a simpler approach: wrap `RecommendedWatcher` directly using `tauri::Manager` to store it in app state, and have the watcher thread call back into the core.

The watcher thread needs access to the `AppHandle` to open a new core. Let me capture `app_handle.clone()` in the closure:

```rust
#[tauri::command]
fn start_watching(app: tauri::AppHandle, project_id: String) -> Result<(), String> {
    let core = open_core(&app)?;
    let project = core.get_project(&project_id).map_err(to_command_error)?
        .ok_or_else(|| "project not found".to_string())?;
    let root = PathBuf::from(&project.root_path);

    let app_handle = app.clone();
    let watcher = crate::watcher::FileWatcher::start(&project_id, &root, move |pid, path| {
        if let Ok(core) = open_core(&app_handle) {
            let _ = core.incremental_scan_file(&pid, &path);
        }
    })?;

    app.manage(WatcherState(Mutex::new(Some(watcher))));
    Ok(())
}

#[tauri::command]
fn stop_watching(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<WatcherState>() {
        if let Ok(mut w) = state.0.lock() {
            if let Some(ref mut watcher) = *w {
                watcher.stop();
            }
            *w = None;
        }
    }
    Ok(())
}

#[tauri::command]
fn watcher_status(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(app.try_state::<WatcherState>()
        .and_then(|s| s.0.lock().ok())
        .map(|w| w.is_some())
        .unwrap_or(false))
}
```

With state:
```rust
struct WatcherState(Mutex<Option<crate::watcher::FileWatcher>>);
```

Register in `run()`: `.manage(WatcherState(Mutex::new(None)))`

- [ ] **Step 3: Register commands and state in `run()`**

Add `.manage(WatcherState(Mutex::new(None)))` and add to `generate_handler![]`:
```
incremental_scan, list_file_scans, list_revisions, start_watching, stop_watching, watcher_status
```

- [ ] **Step 4: Add `get_project` method to NovelCore**

```rust
pub fn get_project(&self, project_id: &str) -> Result<Option<novellossless_storage::Project>> {
    self.storage.get_project(project_id)
}
```

- [ ] **Step 5: Build and commit**

```bash
cd /home/gordon/code/novellossless
cargo build -p novellossless-desktop 2>&1
git add -A && git commit -m "feat(desktop): Tauri commands for incremental scan and watcher"
```

---

### Task 5: TypeScript API + 改稿History UI

**Files:**
- Modify: `apps/desktop/src/tauri.ts`
- Create: `apps/desktop/src/routes/RevisionHistory.tsx`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`

- [ ] **Step 1: Add types and functions to `tauri.ts`**

```typescript
export interface ScanResult {
  scannedDocuments: number;
  skippedFiles: number;
  created: number;
  modified: number;
  unchanged: number;
  deleted: number;
  failed: number;
}

export interface FileScanLog {
  id: string;
  projectId: string;
  documentId: string;
  oldHash: string | null;
  newHash: string;
  eventType: string;
  scannedAt: string;
  details: string | null;
}

export interface RevisionRecord {
  id: string;
  projectId: string;
  documentId: string;
  revisionType: string;
  oldContentHash: string | null;
  newContentHash: string;
  oldChunkCount: number;
  newChunkCount: number;
  chunksAdded: number;
  chunksRemoved: number;
  chunksModified: number;
  diffJson: string | null;
  createdAt: string;
}

export function incrementalScan(projectId: string): Promise<ScanResult> {
  return invoke("incremental_scan", { projectId });
}

export function listFileScans(projectId: string, limit: number): Promise<FileScanLog[]> {
  return invoke("list_file_scans", { projectId, limit });
}

export function listRevisions(projectId: string, documentId: string | null, limit: number): Promise<RevisionRecord[]> {
  return invoke("list_revisions", { projectId, documentId, limit });
}

export function startWatching(projectId: string): Promise<void> {
  return invoke("start_watching", { projectId });
}

export function stopWatching(): Promise<void> {
  return invoke("stop_watching");
}

export function watcherStatus(): Promise<boolean> {
  return invoke("watcher_status");
}
```

- [ ] **Step 2: Create `RevisionHistory.tsx`**

```tsx
import { useEffect, useState } from "react";
import { History, ChevronRight, RefreshCw, Play, Square } from "lucide-react";
import { listFileScans, listRevisions, incrementalScan, startWatching, stopWatching, watcherStatus, FileScanLog, RevisionRecord } from "../tauri";

interface Props {
  projectId: string;
}

const eventLabels: Record<string, string> = {
  created: "新建", modified: "修改", unchanged: "未变",
  deleted: "删除", failed: "失败",
};

const eventColors: Record<string, string> = {
  created: "evt-created", modified: "evt-modified",
  unchanged: "evt-unchanged", deleted: "evt-deleted", failed: "evt-failed",
};

export function RevisionHistory({ projectId }: Props) {
  const [scans, setScans] = useState<FileScanLog[]>([]);
  const [revisions, setRevisions] = useState<RevisionRecord[]>([]);
  const [selectedDoc, setSelectedDoc] = useState<string | null>(null);
  const [watching, setWatching] = useState(false);
  const [scanning, setScanning] = useState(false);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listFileScans(projectId, 100).then(setScans);
      watcherStatus().then(setWatching);
    }
  }, [projectId]);

  const handleSelectDoc = (docId: string) => {
    setSelectedDoc(docId);
    listRevisions(projectId, docId, 50).then(setRevisions);
  };

  const handleIncrementalScan = async () => {
    setScanning(true);
    await incrementalScan(projectId);
    const [newScans] = await Promise.all([listFileScans(projectId, 100)]);
    setScans(newScans);
    setScanning(false);
  };

  const toggleWatcher = async () => {
    if (watching) {
      await stopWatching();
      setWatching(false);
    } else {
      await startWatching(projectId);
      setWatching(true);
    }
  };

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading">
            <h2>改稿历史</h2>
            <p>共 {scans.length} 条扫描记录</p>
          </div>
          <div className="scan-toolbar">
            <button className="primary-button" onClick={handleIncrementalScan} disabled={scanning}>
              <RefreshCw size={15} />{scanning ? "扫描中..." : "增量扫描"}
            </button>
            <button className={`secondary-button ${watching ? "watching" : ""}`} onClick={toggleWatcher}>
              {watching ? <><Square size={15} /> 停止监听</> : <><Play size={15} /> 开始监听</>}
            </button>
          </div>
          <div className="compact-list">
            {scans.length > 0 ? scans.map((s) => (
              <article
                key={s.id}
                className={`compact-item ${selectedDoc === s.documentId ? "compact-item-active" : ""}`}
                onClick={() => handleSelectDoc(s.documentId)}
              >
                <div>
                  <strong>
                    {s.documentId.slice(0, 8)}
                    <span className={eventColors[s.eventType] ?? ""}>{eventLabels[s.eventType] ?? s.eventType}</span>
                  </strong>
                  <p>{s.eventType === "modified" ? `${s.oldHash?.slice(0, 8)} → ${s.newHash.slice(0, 8)}` : s.newHash.slice(0, 16)} · {s.scannedAt}</p>
                </div>
                <ChevronRight size={17} />
              </article>
            )) : (
              <div className="empty-state small">尚未扫描。</div>
            )}
          </div>
        </section>
      </div>
      <aside className="inspector">
        {selectedDoc && revisions.length > 0 ? (
          <section className="panel">
            <div className="panel-heading compact">
              <h2>修订详情</h2>
              <History size={22} />
            </div>
            {revisions.map((rev) => (
              <div key={rev.id} className="revision-card">
                <div className="revision-meta">
                  <span>{rev.revisionType}</span>
                  <strong>{new Date(rev.createdAt).toLocaleString("zh-CN")}</strong>
                </div>
                <div className="revision-stats">
                  <span>新增 {rev.chunksAdded} 段</span>
                  <span>删除 {rev.chunksRemoved} 段</span>
                  <span>修改 {rev.chunksModified} 段</span>
                </div>
                {rev.diffJson && <pre className="revision-diff">{JSON.stringify(JSON.parse(rev.diffJson), null, 2)}</pre>}
              </div>
            ))}
          </section>
        ) : (
          <div className="empty-state">选择一个文档查看修订详情。</div>
        )}
      </aside>
    </section>
  );
}
```

- [ ] **Step 3: Register route in App.tsx**

```tsx
import { RevisionHistory } from "./routes/RevisionHistory";
// ...
<Routes>
  // ...
  <Route path="/history" element={<RevisionHistory projectId={projectId} />} />
</Routes>
```

Add navigation entry:
```tsx
{ label: "改稿历史", icon: History, path: "/history" },
```

- [ ] **Step 4: Add CSS**

```css
.scan-toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.scan-toolbar button {
  display: flex;
  align-items: center;
  gap: 6px;
}

.revision-card {
  padding: 10px;
  margin-bottom: 8px;
  border: 1px solid #e2dccf;
  border-radius: 8px;
  background: #fffefb;
}

.revision-meta {
  display: flex;
  justify-content: space-between;
  margin-bottom: 6px;
  font-size: 12px;
  color: #817a70;
}

.revision-stats {
  display: flex;
  gap: 10px;
  font-size: 11px;
  color: #625f58;
}

.revision-diff {
  margin-top: 6px;
  padding: 6px;
  border-radius: 4px;
  background: #f7f5ef;
  font-size: 11px;
  overflow-x: auto;
}

.evt-created { background: #d4edda; color: #155724; }
.evt-modified { background: #fff3cd; color: #856404; }
.evt-unchanged { background: #e2e3e5; color: #383d41; }
.evt-deleted { background: #f8d7da; color: #721c24; }
.evt-failed { background: #f8d7da; color: #721c24; }
```

- [ ] **Step 5: Verify and commit**

```bash
cd /home/gordon/code/novellossless/apps/desktop
npx tsc --noEmit 2>&1
git add -A && git commit -m "feat(ui): revision history page and watcher controls"
```

---

### Task 6: CLI incremental-scan subcommand

**Files:**
- Modify: `apps/cli/src/main.rs`

- [ ] **Step 1: Add `incremental-scan` subcommand**

Read the existing CLI code and add:

```rust
("incremental-scan", Some(args)) => {
    let project_id = args.get_one::<String>("project-id")
        .or_else(|| args.get_one::<String>("project"))
        .or_else(|| project_id_from_name(&core)?)
        .ok_or_else(|| anyhow::anyhow!("请指定 --project-id 或 --project"))?;
    let report = core.incremental_scan(&project_id)?;
    println!("增量扫描完成：");
    println!("  已扫描: {}", report.scanned_documents);
    println!("  新建: {}", report.created);
    println!("  修改: {}", report.modified);
    println!("  未变: {}", report.unchanged);
    println!("  删除: {}", report.deleted);
    println!("  失败: {}", report.failed);
}
```

Add clap arg:
```rust
Subcommand::with_name("incremental-scan")
    .about("增量扫描 — 跳过未变更文件，记录变更")
    .arg(arg!(-p --"project-id" <PROJECT_ID> "项目 ID"))
    .arg(arg!(--project <NAME> "按名称查找项目"))
```

- [ ] **Step 2: Build and commit**

```bash
cd /home/gordon/code/novellossless
cargo build -p novellossless-cli 2>&1
git add -A && git commit -m "feat(cli): incremental-scan subcommand"
```
