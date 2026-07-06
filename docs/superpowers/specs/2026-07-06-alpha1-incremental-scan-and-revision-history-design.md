# Design: 增量扫描 + 文件监听 + 修订历史

> Alpha 1 — PRD Priority 4
> 2026-07-06

## 1. Overview

Extend the current full-scan loop with hash-gated incremental scanning, file system
watching, and per-document revision history including chunk-level diffs.

### Terminology

| Term | Meaning |
|------|---------|
| full scan | Walk all files, re-index everything unconditionally |
| incremental scan | Walk files, skip unchanged via SHA-256, re-index only changed files |
| file scan log | Row per file-per-scan recording event type (created/modified/unchanged/deleted) |
| revision history | Row per document-per-change recording old/new hashes and chunk-level diff summary |
| file watcher | OS-level recursive directory watch that fires `incremental_scan_file` on change |

### Principle

Correctness over performance — the extractor pipeline always runs on the full
chunk set after any file change, ensuring cross-chapter extractors (foreshadow,
conflict) see the complete picture.

---

## 2. Storage Layer — `crates/storage`

### 2.1 New tables

```sql
CREATE TABLE IF NOT EXISTS file_scan_log (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    old_hash TEXT,
    new_hash TEXT NOT NULL,
    event_type TEXT NOT NULL,       -- 'created' | 'modified' | 'unchanged' | 'deleted' | 'failed'
    scanned_at TEXT NOT NULL,       -- RFC 3339
    details TEXT                    -- JSON, e.g. error message on failure
);

CREATE TABLE IF NOT EXISTS revision_history (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    revision_type TEXT NOT NULL,    -- 'full_scan' | 'incremental'
    old_content_hash TEXT,
    new_content_hash TEXT NOT NULL,
    old_chunk_count INTEGER NOT NULL DEFAULT 0,
    new_chunk_count INTEGER NOT NULL DEFAULT 0,
    chunks_added INTEGER NOT NULL DEFAULT 0,
    chunks_removed INTEGER NOT NULL DEFAULT 0,
    chunks_modified INTEGER NOT NULL DEFAULT 0,
    diff_json TEXT,                 -- JSON array, see 2.2
    created_at TEXT NOT NULL
);
```

### 2.2 Chunk diff format (`diff_json`)

```json
[
  {
    "kind": "added",
    "index": 12,
    "title": "第十二章 暗流",
    "hash": "sha256..."
  },
  {
    "kind": "removed",
    "index": 8,
    "title": "第八章 旧雨",
    "hash": "sha256..."
  },
  {
    "kind": "modified",
    "index": 3,
    "old_title": "第三章 夜行",
    "new_title": "第三章 夜奔",
    "old_hash": "sha256...",
    "new_hash": "sha256..."
  }
]
```

Only hashes are stored, not full content. The old content is available from the
previous document_chunks row before upsert, but we do not persist it in the
revision record — the revision tells you *what* changed, and you can view the
current content in the content browser.

### 2.3 New Rust types

```rust
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

### 2.4 New storage methods

```rust
impl Storage {
    pub fn record_file_scan(&self, project_id: &str, document_id: &str,
        old_hash: Option<&str>, new_hash: &str, event_type: &str,
        details: Option<&str>) -> Result<String>;

    pub fn record_revision(&self, project_id: &str, document_id: &str,
        revision_type: &str, old_hash: Option<&str>, new_hash: &str,
        old_chunk_count: i64, new_chunk_count: i64,
        chunks_added: i64, chunks_removed: i64, chunks_modified: i64,
        diff_json: Option<&str>) -> Result<String>;

    pub fn list_file_scans(&self, project_id: &str, limit: i64)
        -> Result<Vec<FileScanLog>>;

    pub fn list_revisions(&self, project_id: &str,
        document_id: Option<&str>, limit: i64)
        -> Result<Vec<RevisionRecord>>;
}
```

---

## 3. Core Layer — `crates/core`

### 3.1 Chunk diff logic

New function in `crates/core/src/scan.rs` (or inline in lib.rs):

```rust
pub struct ChunkDiff {
    pub added: Vec<ChunkDiffEntry>,
    pub removed: Vec<ChunkDiffEntry>,
    pub modified: Vec<ModifiedEntry>,
}

pub struct ChunkDiffEntry {
    pub index: i64,
    pub title: String,
    pub hash: String,
}

pub struct ModifiedEntry {
    pub index: i64,
    pub old_title: String,
    pub new_title: String,
    pub old_hash: String,
    pub new_hash: String,
}

/// Compare old chunks (from DB) with new chunks (from parse)
/// to produce added/removed/modified lists.
pub fn diff_chunks(old_chunks: &[ProjectChunk], new_chunks: &[NewChunk]) -> ChunkDiff;
```

Logic by `chunk_index`:
- Only in old → removed
- Only in new → added
- Same index:
  - `title == new_title && content_hash == new_content_hash` → unchanged (skip)
  - Else → modified

### 3.2 Incremental scan

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

impl NovelCore {
    /// Walk all files, skip unchanged via hash, record scan log + revisions.
    pub fn incremental_scan(&self, project_id: &str) -> Result<ScanResult>;

    /// Scan a single changed file (called by the file watcher).
    pub fn incremental_scan_file(
        &self, project_id: &str, file_path: &Path
    ) -> Result<ScanResult>;
}
```

`incremental_scan` flow:

1. Collect text files (same as `scan_project`)
2. For each file:
   a. Compute SHA-256
   b. Look up existing document by (project_id, path) via `existing_document_id`
   c. **Not found** → full scan, record as `created`
   d. **Found, same hash** → skip, record as `unchanged`
   e. **Found, different hash** → full scan, compute chunk diff, record as `modified`
3. After file loop: detect DB documents whose path no longer exists → flag as `deleted`
4. Run `analyze_project` on full chunk set (same as today)
5. Return `ScanResult`

`incremental_scan_file` flow (for watcher):
1. Same as steps 2-4 for a single file
2. Also check if the file still exists (deleted event from watcher)

### 3.3 Existing scan_project remains

`scan_project` stays as-is for the initial import. `incremental_scan` is for
subsequent re-scans. The CLI can choose which to call.

---

## 4. File Watcher — `apps/desktop/src-tauri`

### 4.1 Dependency

```toml
[dependencies]
notify = "8"
```

### 4.2 Module: `src-tauri/src/watcher.rs`

```rust
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::collections::HashSet;

/// Owns the notify watcher and debounce logic.
pub struct FileWatcher {
    project_id: String,
    root: PathBuf,
    watcher: Option<RecommendedWatcher>,
    rx: mpsc::Receiver<Result<Event, notify::Error>>,
    debounce_ms: u64,
}

impl FileWatcher {
    pub fn start(project_id: &str, root: &Path, debounce_ms: u64) -> Result<Self>;
    pub fn stop(&mut self);
    pub fn is_running(&self) -> bool;
}
```

### 4.3 Debounce logic

1. `notify` fires raw events (Create, Modify, Remove, Rename)
2. Collect all events for `debounce_ms` (default 800ms)
3. If new events arrive during debounce, reset timer
4. After stable period, deduplicate by file path (last event wins)
5. Filter to supported extensions (.txt, .md, .markdown)
6. For each distinct path:
   - Create/Modify → call `incremental_scan_file`
   - Remove → mark document as deleted in scan log
   - Rename(from, to) → handle as remove + create
7. Debounce runs on a dedicated `std::thread::spawn` loop

### 4.4 Tauri commands

```rust
#[tauri::command]
fn cmd_start_watching(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<(), String>;

#[tauri::command]
fn cmd_stop_watching(
    state: tauri::State<'_, AppState>,
) -> Result<(), String>;
```

`AppState` gets a `Mutex<Option<FileWatcher>>` field.

### 4.5 Edge cases

- **File still open for writing** → `notify` fires on save; by the time we read, file is closed
- **Temporary editor swap files** → filter by extension + ignore files starting with `.`
- **Batch save (git checkout, rsync)** → debounce collects all events before scanning
- **Watcher dies** → `is_running()` returns false, UI shows warning, user can restart
- **Project switched** → stop old watcher, start new watcher

---

## 5. Tauri Commands

| Command | Input | Output | Purpose |
|---------|-------|--------|---------|
| `cmd_incremental_scan` | project_id | `ScanResult` | Manual incremental re-scan |
| `cmd_list_file_scans` | project_id, limit | `Vec<FileScanLog>` | Scan history for UI |
| `cmd_list_revisions` | project_id, document_id, limit | `Vec<RevisionRecord>` | Revision history per doc |
| `cmd_start_watching` | project_id | `()` | Start file watcher |
| `cmd_stop_watching` | — | `()` | Stop file watcher |
| `cmd_watcher_status` | — | `bool` | Is watcher running |

---

## 6. TypeScript API

```typescript
// apps/desktop/src/tauri.ts

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

export function incrementalScan(projectId: string): Promise<ScanResult>;
export function listFileScans(projectId: string, limit: number): Promise<FileScanLog[]>;
export function listRevisions(projectId: string, documentId: string | null, limit: number): Promise<RevisionRecord[]>;
export function startWatching(projectId: string): Promise<void>;
export function stopWatching(): Promise<void>;
export function watcherStatus(): Promise<boolean>;
```

---

## 7. UI — 改稿历史 Route

New route page `src/routes/RevisionHistory.tsx` at `/history`:

- Left column: timeline of file scan events (latest first)
  - Each event shows file path, event type (color-coded), timestamp
  - Click expands per-document revision history
- Right column (inspector): detail for selected revision
  - Old vs new chunk counts
  - List of added/removed/modified chunks
  - Click a chunk → navigate to content browser at that position

Navigation updated:
```typescript
const navigation = [
  ...
  { label: "改稿历史", icon: History, path: "/history" },
  ...
];
```

---

## 8. Testing

### Rust tests

```rust
// storage tests
fn records_and_lists_file_scan_logs();
fn records_and_lists_revision_history();
fn records_revision_with_chunk_diff();

// core tests
fn incremental_scan_skips_unchanged_files();
fn incremental_scan_detects_modified_file();
fn incremental_scan_detects_added_file();
fn incremental_scan_detects_deleted_document();
fn diff_chunks_detects_added_removed_modified();
```

### TypeScript verification
```
npx tsc --noEmit
```

### Integration
- Desktop app: manual test of watcher start/stop, change file, verify scan log
- CLI: `incremental_scan` command

---

## 9. Files Changed

| File | Change |
|------|--------|
| `crates/storage/src/lib.rs` | New tables, new methods, new types |
| `crates/core/src/lib.rs` | `incremental_scan`, `incremental_scan_file`, `ScanResult` |
| `crates/core/src/scan.rs` (new) | `diff_chunks`, `ChunkDiff` types |
| `apps/desktop/src-tauri/Cargo.toml` | Add `notify = "8"` |
| `apps/desktop/src-tauri/src/lib.rs` | New commands, `AppState` watcher field |
| `apps/desktop/src-tauri/src/watcher.rs` (new) | `FileWatcher` with debounce |
| `apps/desktop/src/tauri.ts` | New API functions + types |
| `apps/desktop/src/App.tsx` | Add `/history` route |
| `apps/desktop/src/routes/RevisionHistory.tsx` (new) | History page |
| `apps/desktop/src/styles.css` | History page styles |
