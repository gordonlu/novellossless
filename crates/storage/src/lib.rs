use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDocument {
    pub path: String,
    pub kind: String,
    pub title: String,
    pub chapter_count: i64,
    pub content_hash: String,
    pub word_count: i64,
    pub encoding: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChunk {
    pub chunk_index: i64,
    pub title: String,
    pub start_offset: i64,
    pub end_offset: i64,
    pub content: String,
    pub content_hash: String,
    pub word_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub document_id: String,
    pub chunk_id: String,
    pub document_path: String,
    pub chunk_index: i64,
    pub title: String,
    pub snippet: String,
    pub start_offset: i64,
    pub end_offset: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub project_id: String,
    pub document_count: i64,
    pub chunk_count: i64,
    pub total_words: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChunk {
    pub document_id: String,
    pub chunk_id: String,
    pub document_path: String,
    pub chunk_index: i64,
    pub title: String,
    pub content: String,
    pub start_offset: i64,
    pub end_offset: i64,
    pub word_count: i64,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDocument {
    pub id: String,
    pub path: String,
    pub title: String,
    pub chapter_count: i64,
    pub word_count: i64,
    pub content_hash: String,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNarrativeNode {
    pub node_type: String,
    pub name: String,
    pub aliases_json: String,
    pub occurrence_count: i64,
    pub first_chunk_id: String,
    pub latest_chunk_id: String,
    pub confidence: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NarrativeNode {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub occurrence_count: i64,
    pub status: String,
    pub confidence: i64,
    pub source_chunk_id: String,
    pub source_title: String,
    pub source_path: String,
    pub source_snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewForeshadowItem {
    pub title: String,
    pub foreshadow_type: String,
    pub first_chunk_id: String,
    pub latest_chunk_id: String,
    pub risk_level: String,
    pub evidence: String,
    pub related_nodes_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeshadowItem {
    pub id: String,
    pub title: String,
    pub foreshadow_type: String,
    pub status: String,
    pub risk_level: String,
    pub source_chunk_id: String,
    pub source_title: String,
    pub source_path: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewContinuityIssue {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence_json: String,
    pub suggested_actions_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuityIssue {
    pub id: String,
    pub project_id: String,
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence_json: String,
    pub suggested_actions_json: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPack {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub target: String,
    pub content: String,
    pub format: String,
    pub source_refs_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct NewProfileMetric {
    pub profile_id: String,
    pub project_id: String,
    pub metric_type: String,
    pub document_id: Option<String>,
    pub value_json: String,
}

#[derive(Debug, Clone)]
pub struct ProfileMetric {
    pub id: String,
    pub profile_id: String,
    pub metric_type: String,
    pub document_id: Option<String>,
    pub value: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct KnowledgePackEntry {
    pub profile_id: String,
    pub pack_name: String,
    pub pack_type: String,
    pub entries_json: String,
    pub version: String,
}

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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScanRun {
    pub id: String,
    pub project_id: String,
    pub scan_type: String,
    pub status: String,
    pub total_files: i64,
    pub scanned_files: i64,
    pub scanned_paths: String,
    pub errors: String,
    pub started_at: String,
    pub completed_at: Option<String>,
}

pub struct NewScanRun {
    pub project_id: String,
    pub scan_type: String,
    pub total_files: i64,
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

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let conn =
            Connection::open(path).with_context(|| format!("failed to open {}", path.display()))?;
        let storage = Self { conn };
        storage.init()?;
        Ok(storage)
    }

    pub fn open_memory() -> Result<Self> {
        let storage = Self {
            conn: Connection::open_in_memory()?,
        };
        storage.init()?;
        Ok(storage)
    }

    pub fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                settings_json TEXT NOT NULL DEFAULT '{}',
                enabled_profiles_json TEXT NOT NULL DEFAULT '["common_longform"]',
                storage_mode TEXT NOT NULL DEFAULT 'local'
            );

            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                path TEXT NOT NULL,
                kind TEXT NOT NULL,
                title TEXT NOT NULL,
                chapter_count INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                word_count INTEGER NOT NULL,
                encoding TEXT NOT NULL,
                last_modified_at TEXT,
                indexed_at TEXT NOT NULL,
                deleted INTEGER NOT NULL DEFAULT 0,
                UNIQUE(project_id, path)
            );

            CREATE TABLE IF NOT EXISTS document_chunks (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                chunk_index INTEGER NOT NULL,
                title TEXT NOT NULL,
                start_offset INTEGER NOT NULL,
                end_offset INTEGER NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                word_count INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(document_id, chunk_index)
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS document_chunks_fts USING fts5(
                chunk_id UNINDEXED,
                project_id UNINDEXED,
                document_id UNINDEXED,
                title,
                content
            );

            CREATE TABLE IF NOT EXISTS narrative_nodes (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                node_type TEXT NOT NULL,
                name TEXT NOT NULL,
                aliases_json TEXT NOT NULL DEFAULT '[]',
                summary TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'candidate',
                confidence INTEGER NOT NULL,
                occurrence_count INTEGER NOT NULL DEFAULT 1,
                first_chunk_id TEXT NOT NULL REFERENCES document_chunks(id) ON DELETE CASCADE,
                latest_chunk_id TEXT NOT NULL REFERENCES document_chunks(id) ON DELETE CASCADE,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(project_id, node_type, name)
            );

            CREATE TABLE IF NOT EXISTS foreshadow_items (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                foreshadow_type TEXT NOT NULL,
                first_chunk_id TEXT NOT NULL REFERENCES document_chunks(id) ON DELETE CASCADE,
                latest_chunk_id TEXT NOT NULL REFERENCES document_chunks(id) ON DELETE CASCADE,
                related_nodes_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'candidate',
                risk_level TEXT NOT NULL,
                notes TEXT NOT NULL DEFAULT '',
                evidence TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(project_id, title, first_chunk_id)
            );

            CREATE TABLE IF NOT EXISTS continuity_issues (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                issue_type TEXT NOT NULL,
                severity TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                evidence_json TEXT NOT NULL,
                suggested_actions_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'open',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                resolved_at TEXT,
                UNIQUE(project_id, issue_type, title)
            );

            CREATE TABLE IF NOT EXISTS context_packs (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                target TEXT NOT NULL,
                content TEXT NOT NULL,
                format TEXT NOT NULL,
                source_refs_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS file_scan_log (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                old_hash TEXT,
                new_hash TEXT NOT NULL,
                event_type TEXT NOT NULL,
                scanned_at TEXT NOT NULL,
                details TEXT
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
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
            );

            CREATE TABLE IF NOT EXISTS profile_metrics (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                profile_id TEXT NOT NULL,
                metric_type TEXT NOT NULL,
                document_id TEXT REFERENCES documents(id) ON DELETE CASCADE,
                value_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS knowledge_packs (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                pack_name TEXT NOT NULL,
                pack_type TEXT NOT NULL,
                entries_json TEXT NOT NULL,
                version TEXT NOT NULL DEFAULT '0.1.0',
                created_at TEXT NOT NULL
            );

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

            CREATE TABLE IF NOT EXISTS scan_runs (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                scan_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                total_files INTEGER NOT NULL DEFAULT 0,
                scanned_files INTEGER NOT NULL DEFAULT 0,
                scanned_paths TEXT NOT NULL DEFAULT '[]',
                errors TEXT NOT NULL DEFAULT '[]',
                started_at TEXT NOT NULL,
                completed_at TEXT
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
            "#,
        )?;

        self.conn.pragma_update(None, "user_version", 1)?;

        Ok(())
    }

    pub fn create_project(&self, name: &str, root_path: &str) -> Result<Project> {
        let now = Utc::now().to_rfc3339();
        let project = Project {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            root_path: root_path.to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.conn.execute(
            r#"
            INSERT INTO projects (id, name, root_path, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                project.id,
                project.name,
                project.root_path,
                project.created_at,
                project.updated_at
            ],
        )?;

        Ok(project)
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        self.conn
            .query_row(
                "SELECT id, name, root_path, created_at, updated_at FROM projects WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        root_path: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, root_path, created_at, updated_at FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn upsert_document_with_chunks(
        &self,
        project_id: &str,
        document: &NewDocument,
        chunks: &[NewChunk],
    ) -> Result<String> {
        let document_id = self
            .existing_document_id(project_id, &document.path)?
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO documents (
                id, project_id, path, kind, title, chapter_count, content_hash,
                word_count, encoding, indexed_at, deleted
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)
            ON CONFLICT(project_id, path) DO UPDATE SET
                kind = excluded.kind,
                title = excluded.title,
                chapter_count = excluded.chapter_count,
                content_hash = excluded.content_hash,
                word_count = excluded.word_count,
                encoding = excluded.encoding,
                indexed_at = excluded.indexed_at,
                deleted = 0
            "#,
            params![
                document_id,
                project_id,
                document.path,
                document.kind,
                document.title,
                document.chapter_count,
                document.content_hash,
                document.word_count,
                document.encoding,
                now
            ],
        )?;

        self.conn.execute(
            "DELETE FROM document_chunks_fts WHERE document_id = ?1",
            params![document_id],
        )?;
        self.conn.execute(
            "DELETE FROM document_chunks WHERE document_id = ?1",
            params![document_id],
        )?;

        for chunk in chunks {
            let chunk_id = Uuid::new_v4().to_string();
            self.conn.execute(
                r#"
                INSERT INTO document_chunks (
                    id, project_id, document_id, chunk_index, title, start_offset,
                    end_offset, content, content_hash, word_count, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    chunk_id,
                    project_id,
                    document_id,
                    chunk.chunk_index,
                    chunk.title,
                    chunk.start_offset,
                    chunk.end_offset,
                    chunk.content,
                    chunk.content_hash,
                    chunk.word_count,
                    Utc::now().to_rfc3339()
                ],
            )?;
            self.conn.execute(
                r#"
                INSERT INTO document_chunks_fts (chunk_id, project_id, document_id, title, content)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    chunk_id,
                    project_id,
                    document_id,
                    chunk.title,
                    chunk.content
                ],
            )?;
        }

        self.conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), project_id],
        )?;

        Ok(document_id)
    }

    pub fn search(&self, project_id: &str, query: &str, limit: i64) -> Result<Vec<SearchHit>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let fts_hits = self.search_fts(project_id, query, limit)?;
        if !fts_hits.is_empty() {
            return Ok(fts_hits);
        }

        self.search_like(project_id, query, limit)
    }

    fn search_fts(&self, project_id: &str, query: &str, limit: i64) -> Result<Vec<SearchHit>> {
        let fts_query = fts5_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                document_chunks_fts.document_id,
                document_chunks_fts.chunk_id,
                documents.path,
                document_chunks.chunk_index,
                document_chunks_fts.title,
                snippet(document_chunks_fts, 4, '[', ']', '...', 18),
                document_chunks.start_offset,
                document_chunks.end_offset
            FROM document_chunks_fts
            JOIN document_chunks ON document_chunks.id = document_chunks_fts.chunk_id
            JOIN documents ON documents.id = document_chunks_fts.document_id
            WHERE document_chunks_fts.project_id = ?1 AND document_chunks_fts MATCH ?2
            LIMIT ?3
            "#,
        )?;

        let rows = stmt.query_map(params![project_id, fts_query, limit], |row| {
            Ok(SearchHit {
                document_id: row.get(0)?,
                chunk_id: row.get(1)?,
                document_path: row.get(2)?,
                chunk_index: row.get(3)?,
                title: row.get(4)?,
                snippet: row.get(5)?,
                start_offset: row.get(6)?,
                end_offset: row.get(7)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn search_like(&self, project_id: &str, query: &str, limit: i64) -> Result<Vec<SearchHit>> {
        let pattern = format!("%{}%", escape_like_query(query));
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                document_chunks.document_id,
                document_chunks.id,
                documents.path,
                document_chunks.chunk_index,
                document_chunks.title,
                document_chunks.content,
                document_chunks.start_offset,
                document_chunks.end_offset
            FROM document_chunks
            JOIN documents ON documents.id = document_chunks.document_id
            WHERE document_chunks.project_id = ?1 AND document_chunks.content LIKE ?2 ESCAPE '\'
            ORDER BY documents.path ASC, document_chunks.chunk_index ASC
            LIMIT ?3
            "#,
        )?;

        let rows = stmt.query_map(params![project_id, pattern, limit], |row| {
            let content: String = row.get(5)?;
            Ok(SearchHit {
                document_id: row.get(0)?,
                chunk_id: row.get(1)?,
                document_path: row.get(2)?,
                chunk_index: row.get(3)?,
                title: row.get(4)?,
                snippet: make_snippet(&content, query),
                start_offset: row.get(6)?,
                end_offset: row.get(7)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn project_summary(&self, project_id: &str) -> Result<ProjectSummary> {
        self.conn
            .query_row(
                r#"
                SELECT
                    ?1,
                    (SELECT COUNT(*) FROM documents WHERE project_id = ?1 AND deleted = 0),
                    (SELECT COUNT(*) FROM document_chunks WHERE project_id = ?1),
                    COALESCE((SELECT SUM(word_count) FROM documents WHERE project_id = ?1 AND deleted = 0), 0)
                "#,
                params![project_id],
                |row| {
                    Ok(ProjectSummary {
                        project_id: row.get(0)?,
                        document_count: row.get(1)?,
                        chunk_count: row.get(2)?,
                        total_words: row.get(3)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn project_chunks(&self, project_id: &str) -> Result<Vec<ProjectChunk>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                document_chunks.document_id,
                document_chunks.id,
                documents.path,
                document_chunks.chunk_index,
                document_chunks.title,
                document_chunks.content,
                document_chunks.start_offset,
                document_chunks.end_offset,
                document_chunks.word_count,
                document_chunks.content_hash
            FROM document_chunks
            JOIN documents ON documents.id = document_chunks.document_id
            WHERE document_chunks.project_id = ?1 AND documents.deleted = 0
            ORDER BY documents.path ASC, document_chunks.chunk_index ASC
            "#,
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(ProjectChunk {
                document_id: row.get(0)?,
                chunk_id: row.get(1)?,
                document_path: row.get(2)?,
                chunk_index: row.get(3)?,
                title: row.get(4)?,
                content: row.get(5)?,
                start_offset: row.get(6)?,
                end_offset: row.get(7)?,
                word_count: row.get(8)?,
                content_hash: row.get(9)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn project_documents(&self, project_id: &str) -> Result<Vec<ProjectDocument>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, path, title, chapter_count, word_count, content_hash
            FROM documents
            WHERE project_id = ?1 AND deleted = 0
            ORDER BY path ASC
            "#,
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(ProjectDocument {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                chapter_count: row.get(3)?,
                word_count: row.get(4)?,
                content_hash: row.get(5)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn upsert_narrative_nodes(
        &self,
        project_id: &str,
        nodes: &[NewNarrativeNode],
    ) -> Result<()> {
        for node in nodes {
            self.conn.execute(
                r#"
                INSERT INTO narrative_nodes (
                    id, project_id, node_type, name, aliases_json, confidence, occurrence_count,
                    first_chunk_id, latest_chunk_id, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
                ON CONFLICT(project_id, node_type, name) DO UPDATE SET
                    aliases_json = excluded.aliases_json,
                    confidence = excluded.confidence,
                    occurrence_count = excluded.occurrence_count,
                    latest_chunk_id = excluded.latest_chunk_id,
                    updated_at = excluded.updated_at
                "#,
                params![
                    Uuid::new_v4().to_string(),
                    project_id,
                    node.node_type,
                    node.name,
                    node.aliases_json,
                    node.confidence,
                    node.occurrence_count,
                    node.first_chunk_id,
                    node.latest_chunk_id,
                    Utc::now().to_rfc3339()
                ],
            )?;
        }

        Ok(())
    }

    pub fn upsert_foreshadow_items(
        &self,
        project_id: &str,
        items: &[NewForeshadowItem],
    ) -> Result<()> {
        for item in items {
            self.conn.execute(
                r#"
                INSERT INTO foreshadow_items (
                    id, project_id, title, foreshadow_type, first_chunk_id, latest_chunk_id,
                    risk_level, evidence, related_nodes_json, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
                ON CONFLICT(project_id, title, first_chunk_id) DO UPDATE SET
                    latest_chunk_id = excluded.latest_chunk_id,
                    risk_level = excluded.risk_level,
                    evidence = excluded.evidence,
                    related_nodes_json = excluded.related_nodes_json,
                    updated_at = excluded.updated_at
                "#,
                params![
                    Uuid::new_v4().to_string(),
                    project_id,
                    item.title,
                    item.foreshadow_type,
                    item.first_chunk_id,
                    item.latest_chunk_id,
                    item.risk_level,
                    item.evidence,
                    item.related_nodes_json,
                    Utc::now().to_rfc3339()
                ],
            )?;
        }

        Ok(())
    }

    pub fn upsert_continuity_issues(
        &self,
        project_id: &str,
        issues: &[NewContinuityIssue],
    ) -> Result<()> {
        for issue in issues {
            self.conn.execute(
                r#"
                INSERT INTO continuity_issues (
                    id, project_id, issue_type, severity, title, description,
                    evidence_json, suggested_actions_json, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
                ON CONFLICT(project_id, issue_type, title) DO UPDATE SET
                    severity = excluded.severity,
                    description = excluded.description,
                    evidence_json = excluded.evidence_json,
                    suggested_actions_json = excluded.suggested_actions_json,
                    updated_at = excluded.updated_at
                "#,
                params![
                    Uuid::new_v4().to_string(),
                    project_id,
                    issue.issue_type,
                    issue.severity,
                    issue.title,
                    issue.description,
                    issue.evidence_json,
                    issue.suggested_actions_json,
                    Utc::now().to_rfc3339()
                ],
            )?;
        }

        Ok(())
    }

    pub fn list_narrative_nodes(
        &self,
        project_id: &str,
        node_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<NarrativeNode>> {
        let mut sql = r#"
            SELECT
                narrative_nodes.id,
                narrative_nodes.node_type,
                narrative_nodes.name,
                narrative_nodes.occurrence_count,
                narrative_nodes.status,
                narrative_nodes.confidence,
                narrative_nodes.first_chunk_id,
                document_chunks.title,
                documents.path,
                document_chunks.content
            FROM narrative_nodes
            JOIN document_chunks ON document_chunks.id = narrative_nodes.first_chunk_id
            JOIN documents ON documents.id = document_chunks.document_id
            WHERE narrative_nodes.project_id = ?1
            "#
        .to_string();

        if node_type.is_some() {
            sql.push_str(" AND narrative_nodes.node_type = ?2");
        }
        sql.push_str(
            " ORDER BY narrative_nodes.occurrence_count DESC, narrative_nodes.name ASC LIMIT ?",
        );
        sql.push_str(if node_type.is_some() { "3" } else { "2" });

        let map_row = |row: &rusqlite::Row<'_>| {
            let source_content: String = row.get(9)?;
            Ok(NarrativeNode {
                id: row.get(0)?,
                node_type: row.get(1)?,
                name: row.get(2)?,
                occurrence_count: row.get(3)?,
                status: row.get(4)?,
                confidence: row.get(5)?,
                source_chunk_id: row.get(6)?,
                source_title: row.get(7)?,
                source_path: row.get(8)?,
                source_snippet: source_content.chars().take(80).collect(),
            })
        };

        if let Some(node_type) = node_type {
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(params![project_id, node_type, limit], map_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        } else {
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(params![project_id, limit], map_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        }
    }

    pub fn list_foreshadow_items(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<ForeshadowItem>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                foreshadow_items.id,
                foreshadow_items.title,
                foreshadow_items.foreshadow_type,
                foreshadow_items.status,
                foreshadow_items.risk_level,
                foreshadow_items.first_chunk_id,
                document_chunks.title,
                documents.path,
                foreshadow_items.evidence
            FROM foreshadow_items
            JOIN document_chunks ON document_chunks.id = foreshadow_items.first_chunk_id
            JOIN documents ON documents.id = document_chunks.document_id
            WHERE foreshadow_items.project_id = ?1
            ORDER BY foreshadow_items.updated_at DESC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![project_id, limit], |row| {
            Ok(ForeshadowItem {
                id: row.get(0)?,
                title: row.get(1)?,
                foreshadow_type: row.get(2)?,
                status: row.get(3)?,
                risk_level: row.get(4)?,
                source_chunk_id: row.get(5)?,
                source_title: row.get(6)?,
                source_path: row.get(7)?,
                evidence: row.get(8)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_continuity_issues(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<ContinuityIssue>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, project_id, issue_type, severity, title, description, evidence_json,
                   suggested_actions_json, status
            FROM continuity_issues
            WHERE project_id = ?1
            ORDER BY
                CASE severity
                    WHEN 'serious' THEN 0
                    WHEN 'high' THEN 1
                    WHEN 'medium' THEN 2
                    WHEN 'low' THEN 3
                    ELSE 4
                END,
                updated_at DESC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![project_id, limit], |row| {
            Ok(ContinuityIssue {
                id: row.get(0)?,
                project_id: row.get(1)?,
                issue_type: row.get(2)?,
                severity: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                evidence_json: row.get(6)?,
                suggested_actions_json: row.get(7)?,
                status: row.get(8)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn update_narrative_node_status(&self, id: &str, status: &str) -> Result<()> {
        self.update_status("narrative_nodes", id, status)
    }

    pub fn update_foreshadow_status(&self, id: &str, status: &str) -> Result<()> {
        self.update_status("foreshadow_items", id, status)
    }

    pub fn update_issue_status(&self, id: &str, status: &str) -> Result<()> {
        self.update_status("continuity_issues", id, status)
    }

    pub fn save_context_pack(
        &self,
        project_id: &str,
        title: &str,
        target: &str,
        content: &str,
        source_refs_json: &str,
    ) -> Result<ContextPack> {
        let pack = ContextPack {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            title: title.to_string(),
            target: target.to_string(),
            content: content.to_string(),
            format: "markdown".to_string(),
            source_refs_json: source_refs_json.to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        self.conn.execute(
            r#"
            INSERT INTO context_packs (
                id, project_id, title, target, content, format, source_refs_json, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                pack.id,
                pack.project_id,
                pack.title,
                pack.target,
                pack.content,
                pack.format,
                pack.source_refs_json,
                pack.created_at
            ],
        )?;

        Ok(pack)
    }

    pub fn create_scan_run(&self, run: &NewScanRun) -> Result<ScanRun> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let scan_run = ScanRun {
            id: id.clone(),
            project_id: run.project_id.clone(),
            scan_type: run.scan_type.clone(),
            status: "scanning".to_string(),
            total_files: run.total_files,
            scanned_files: 0,
            scanned_paths: "[]".to_string(),
            errors: "[]".to_string(),
            started_at: now.clone(),
            completed_at: None,
        };
        self.conn.execute(
            "INSERT INTO scan_runs (id, project_id, scan_type, status, total_files, scanned_files, scanned_paths, errors, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                scan_run.id, scan_run.project_id, scan_run.scan_type, scan_run.status,
                scan_run.total_files, scan_run.scanned_files, scan_run.scanned_paths,
                scan_run.errors, scan_run.started_at, scan_run.completed_at,
            ],
        )?;
        Ok(scan_run)
    }

    pub fn record_scan_file(&self, run_id: &str, path: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE scan_runs SET scanned_paths = json_set(scanned_paths, '$[#]', ?1), scanned_files = scanned_files + 1 WHERE id = ?2",
            params![path, run_id],
        )?;
        Ok(())
    }

    pub fn record_scan_error(&self, run_id: &str, path: &str, error: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE scan_runs SET errors = json_set(errors, '$[#]', json_object('path', ?1, 'error', ?2)) WHERE id = ?3",
            params![path, error, run_id],
        )?;
        Ok(())
    }

    pub fn update_scan_run_status(&self, id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE scan_runs SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn complete_scan_run(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE scan_runs SET status = 'completed', completed_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn get_scan_run(&self, id: &str) -> Result<Option<ScanRun>> {
        self.conn
            .query_row(
                "SELECT id, project_id, scan_type, status, total_files, scanned_files, scanned_paths, errors, started_at, completed_at FROM scan_runs WHERE id = ?1",
                params![id],
                |row| {
                    Ok(ScanRun {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        scan_type: row.get(2)?,
                        status: row.get(3)?,
                        total_files: row.get(4)?,
                        scanned_files: row.get(5)?,
                        scanned_paths: row.get(6)?,
                        errors: row.get(7)?,
                        started_at: row.get(8)?,
                        completed_at: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_latest_incomplete_scan_run(&self, project_id: &str) -> Result<Option<ScanRun>> {
        self.conn
            .query_row(
                "SELECT id, project_id, scan_type, status, total_files, scanned_files, scanned_paths, errors, started_at, completed_at
                 FROM scan_runs
                 WHERE project_id = ?1 AND status IN ('pending', 'scanning', 'analyzing')
                 ORDER BY started_at DESC LIMIT 1",
                params![project_id],
                |row| {
                    Ok(ScanRun {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        scan_type: row.get(2)?,
                        status: row.get(3)?,
                        total_files: row.get(4)?,
                        scanned_files: row.get(5)?,
                        scanned_paths: row.get(6)?,
                        errors: row.get(7)?,
                        started_at: row.get(8)?,
                        completed_at: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_scan_runs(&self, project_id: &str) -> Result<Vec<ScanRun>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, scan_type, status, total_files, scanned_files, scanned_paths, errors, started_at, completed_at
             FROM scan_runs WHERE project_id = ?1 ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(ScanRun {
                id: row.get(0)?,
                project_id: row.get(1)?,
                scan_type: row.get(2)?,
                status: row.get(3)?,
                total_files: row.get(4)?,
                scanned_files: row.get(5)?,
                scanned_paths: row.get(6)?,
                errors: row.get(7)?,
                started_at: row.get(8)?,
                completed_at: row.get(9)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn fail_incomplete_scan_runs(&self, project_id: &str, reason: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE scan_runs SET status = 'failed', errors = json_set(errors, '$[#]', json_object('path', '', 'error', ?1)) WHERE project_id = ?2 AND status IN ('pending', 'scanning', 'analyzing')",
            params![reason, project_id],
        )?;
        Ok(())
    }

    pub fn record_file_scan(
        &self,
        project_id: &str,
        document_id: &str,
        old_hash: Option<&str>,
        new_hash: &str,
        event_type: &str,
        details: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO file_scan_log (id, project_id, document_id, old_hash, new_hash, event_type, scanned_at, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, project_id, document_id, old_hash, new_hash, event_type, now, details],
        )?;
        Ok(id)
    }

    pub fn record_revision(
        &self,
        project_id: &str,
        document_id: &str,
        revision_type: &str,
        old_hash: Option<&str>,
        new_hash: &str,
        old_chunk_count: i64,
        new_chunk_count: i64,
        chunks_added: i64,
        chunks_removed: i64,
        chunks_modified: i64,
        diff_json: Option<&str>,
    ) -> Result<String> {
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
        let rows = stmt.query_map(params![project_id, limit], |row| {
            Ok(FileScanLog {
                id: row.get(0)?,
                project_id: row.get(1)?,
                document_id: row.get(2)?,
                old_hash: row.get(3)?,
                new_hash: row.get(4)?,
                event_type: row.get(5)?,
                scanned_at: row.get(6)?,
                details: row.get(7)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_revisions(
        &self,
        project_id: &str,
        document_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<RevisionRecord>> {
        let (clause, pid, doc) = match document_id {
            Some(d) => (
                "WHERE project_id = ?1 AND document_id = ?2 ORDER BY created_at DESC LIMIT ?3"
                    .to_string(),
                project_id.to_string(),
                d.to_string(),
            ),
            None => (
                "WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2".to_string(),
                project_id.to_string(),
                String::new(),
            ),
        };
        let sql = format!(
            "SELECT id, project_id, document_id, revision_type, old_content_hash, new_content_hash, \
             old_chunk_count, new_chunk_count, chunks_added, chunks_removed, chunks_modified, \
             diff_json, created_at FROM revision_history {clause}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(RevisionRecord {
                id: row.get(0)?,
                project_id: row.get(1)?,
                document_id: row.get(2)?,
                revision_type: row.get(3)?,
                old_content_hash: row.get(4)?,
                new_content_hash: row.get(5)?,
                old_chunk_count: row.get(6)?,
                new_chunk_count: row.get(7)?,
                chunks_added: row.get(8)?,
                chunks_removed: row.get(9)?,
                chunks_modified: row.get(10)?,
                diff_json: row.get(11)?,
                created_at: row.get(12)?,
            })
        };
        let rows = if document_id.is_some() {
            stmt.query_map(params![pid, doc, limit], map_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![pid, limit], map_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
    }

    pub fn project_document_by_id(&self, id: &str) -> Result<ProjectDocument> {
        self.conn
            .query_row(
                "SELECT id, path, title, chapter_count, word_count, content_hash FROM documents WHERE id = ?1 AND deleted = 0",
                params![id],
                |row| {
                    Ok(ProjectDocument {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        title: row.get(2)?,
                        chapter_count: row.get(3)?,
                        word_count: row.get(4)?,
                        content_hash: row.get(5)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn document_chunks(&self, document_id: &str) -> Result<Vec<ProjectChunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT ch.document_id, ch.id, d.path, ch.chunk_index, ch.title, ch.content, ch.start_offset, ch.end_offset, ch.word_count, ch.content_hash
             FROM document_chunks ch JOIN documents d ON d.id = ch.document_id
             WHERE ch.document_id = ?1 ORDER BY ch.chunk_index ASC",
        )?;
        let rows = stmt.query_map(params![document_id], |row| {
            Ok(ProjectChunk {
                document_id: row.get(0)?,
                chunk_id: row.get(1)?,
                document_path: row.get(2)?,
                chunk_index: row.get(3)?,
                title: row.get(4)?,
                content: row.get(5)?,
                start_offset: row.get(6)?,
                end_offset: row.get(7)?,
                word_count: row.get(8)?,
                content_hash: row.get(9)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn mark_document_deleted(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE documents SET deleted = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn existing_document_id(&self, project_id: &str, path: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT id FROM documents WHERE project_id = ?1 AND path = ?2 AND deleted = 0",
                params![project_id, path],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn delete_profile_metrics(&self, project_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM profile_metrics WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(())
    }

    pub fn upsert_profile_metric(&self, metric: &NewProfileMetric) -> Result<()> {
        self.conn.execute(
            "INSERT INTO profile_metrics (id, project_id, profile_id, metric_type, document_id, value_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                Uuid::new_v4().to_string(),
                metric.project_id,
                metric.profile_id,
                metric.metric_type,
                metric.document_id,
                metric.value_json,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn get_profile_metrics(
        &self,
        project_id: &str,
        profile_id: &str,
    ) -> Result<Vec<ProfileMetric>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, profile_id, metric_type, document_id, value_json, created_at
             FROM profile_metrics
             WHERE project_id = ?1 AND profile_id = ?2
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![project_id, profile_id], |row| {
            Ok(ProfileMetric {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                metric_type: row.get(2)?,
                document_id: row.get(3)?,
                value: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn get_project_profiles(&self, project_id: &str) -> Result<Vec<String>> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT enabled_profiles_json FROM projects WHERE id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .optional()?;
        let json = json.ok_or_else(|| anyhow::anyhow!("project not found: {project_id}"))?;
        if json.is_empty() {
            return Ok(vec!["common_longform".to_string()]);
        }
        serde_json::from_str(&json).map_err(|e| anyhow::anyhow!("parse enabled_profiles_json: {e}"))
    }

    pub fn set_project_profiles(&self, project_id: &str, profile_ids: &[&str]) -> Result<()> {
        let json = serde_json::to_string(profile_ids)?;
        self.conn.execute(
            "UPDATE projects SET enabled_profiles_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![json, Utc::now().to_rfc3339(), project_id],
        )?;
        Ok(())
    }

    pub fn upsert_knowledge_pack(&self, pack: &KnowledgePackEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO knowledge_packs (id, profile_id, pack_name, pack_type, entries_json, version, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                Uuid::new_v4().to_string(),
                pack.profile_id,
                pack.pack_name,
                pack.pack_type,
                pack.entries_json,
                pack.version,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn get_knowledge_packs(&self, profile_id: &str) -> Result<Vec<KnowledgePackEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT profile_id, pack_name, pack_type, entries_json, version
             FROM knowledge_packs WHERE profile_id = ?1",
        )?;
        let rows = stmt.query_map(params![profile_id], |row| {
            Ok(KnowledgePackEntry {
                profile_id: row.get(0)?,
                pack_name: row.get(1)?,
                pack_type: row.get(2)?,
                entries_json: row.get(3)?,
                version: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

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
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                rule_type: row.get(4)?,
                keywords_json: row.get(5)?,
                positive: row.get::<_, i32>(6)? != 0,
                source_chunk_id: row.get(7)?,
                confidence: row.get(8)?,
                status: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
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
        self.conn
            .execute("DELETE FROM world_rules WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_project_rules(&self, project_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM world_rules WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(())
    }

    pub fn delete_project_timeline_events(&self, project_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM timeline_events WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(())
    }

    pub fn delete_project_tasks(&self, project_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM revision_tasks WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(())
    }

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
                id: row.get(0)?,
                project_id: row.get(1)?,
                chunk_id: row.get(2)?,
                chunk_index: row.get(3)?,
                document_path: row.get(4)?,
                title: row.get(5)?,
                order_index: row.get(6)?,
                time_expression: row.get(7)?,
                estimated_order: row.get(8)?,
                participants_json: row.get(9)?,
                location: row.get(10)?,
                is_flashback: row.get::<_, i32>(11)? != 0,
                confidence: row.get(12)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

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
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                task_type: row.get(3)?,
                priority: row.get(4)?,
                source_issue_id: row.get(5)?,
                source_foreshadow_id: row.get(6)?,
                related_chunks_json: row.get(7)?,
                status: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                resolved_at: row.get(11)?,
                notes: row.get(12)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
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

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM app_settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn update_status(&self, table: &str, id: &str, status: &str) -> Result<()> {
        let allowed_table = matches!(
            table,
            "narrative_nodes" | "foreshadow_items" | "continuity_issues"
        );
        if !allowed_table {
            anyhow::bail!("unsupported status table: {table}");
        }

        let allowed_status = matches!(
            status,
            "candidate"
                | "confirmed"
                | "false_positive"
                | "intentional"
                | "deferred"
                | "discarded"
                | "open"
                | "resolved"
        );
        if !allowed_status {
            anyhow::bail!("unsupported status: {status}");
        }

        self.conn.execute(
            &format!("UPDATE {table} SET status = ?1, updated_at = ?2 WHERE id = ?3"),
            params![status, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }
}

fn fts5_query(query: &str) -> String {
    let cleaned = query
        .chars()
        .map(|ch| match ch {
            '"' | '\'' | '*' | '^' | '(' | ')' | '+' | '-' | '~' | '`' | ':' => ' ',
            value => value,
        })
        .collect::<String>();
    let phrase = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    if phrase.is_empty() {
        String::new()
    } else {
        format!("\"{phrase}\"")
    }
}

fn escape_like_query(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn make_snippet(content: &str, query: &str) -> String {
    let Some(byte_start) = content.find(query) else {
        return content.chars().take(60).collect();
    };

    let char_start = content[..byte_start].chars().count();
    let query_chars = query.chars().count();
    let prefix_start = char_start.saturating_sub(18);
    let suffix_end = char_start + query_chars + 18;
    let chars = content.chars().collect::<Vec<_>>();

    let mut snippet = String::new();
    if prefix_start > 0 {
        snippet.push_str("...");
    }
    snippet.extend(chars[prefix_start..char_start].iter());
    snippet.push('[');
    snippet.push_str(query);
    snippet.push(']');
    snippet.extend(chars[char_start + query_chars..chars.len().min(suffix_end)].iter());
    if suffix_end < chars.len() {
        snippet.push_str("...");
    }
    snippet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_searches_chunks() {
        let storage = Storage::open_memory().expect("storage opens");
        let project = storage
            .create_project("雨巷钟声", "D:/novels/rain")
            .expect("project created");

        storage
            .upsert_document_with_chunks(
                &project.id,
                &NewDocument {
                    path: "001.txt".to_string(),
                    kind: "text".to_string(),
                    title: "第一章".to_string(),
                    chapter_count: 1,
                    content_hash: "hash".to_string(),
                    word_count: 4,
                    encoding: "utf-8".to_string(),
                },
                &[NewChunk {
                    chunk_index: 0,
                    title: "第一章 雨夜".to_string(),
                    start_offset: 0,
                    end_offset: 12,
                    content: "林澈在雨夜醒来。".to_string(),
                    content_hash: "chunk-hash".to_string(),
                    word_count: 8,
                }],
            )
            .expect("document stored");

        let hits = storage
            .search(&project.id, "林澈", 10)
            .expect("search succeeds");

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "第一章 雨夜");
        assert_eq!(hits[0].document_path, "001.txt");
        assert_eq!(hits[0].chunk_index, 0);
        assert_eq!(hits[0].start_offset, 0);
        assert_eq!(hits[0].end_offset, 12);
        assert!(hits[0].snippet.contains("[林澈]"));
    }

    #[test]
    fn lists_projects_by_updated_time() {
        let storage = Storage::open_memory().expect("storage opens");
        storage
            .create_project("雨巷钟声", "D:/novels/rain")
            .expect("project created");

        let projects = storage.list_projects().expect("projects listed");

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "雨巷钟声");
    }

    fn test_storage() -> Result<Storage> {
        Storage::open_memory()
    }

    fn test_storage_with_project(name: &str) -> Result<(Storage, String)> {
        let storage = test_storage()?;
        let project = storage.create_project(name, &format!("/tmp/{name}"))?;
        Ok((storage, project.id))
    }

    fn seed_document(storage: &Storage, project_id: &str, path: &str) -> Result<String> {
        storage.upsert_document_with_chunks(
            project_id,
            &NewDocument {
                path: path.to_string(),
                kind: "text".to_string(),
                title: "第一章".to_string(),
                chapter_count: 1,
                content_hash: "abc".to_string(),
                word_count: 4,
                encoding: "utf-8".to_string(),
            },
            &[NewChunk {
                chunk_index: 0,
                title: "第一章".to_string(),
                start_offset: 0,
                end_offset: 4,
                content: "test".to_string(),
                content_hash: "chunk-hash".to_string(),
                word_count: 1,
            }],
        )
    }

    #[test]
    fn records_and_lists_file_scan_logs() -> Result<()> {
        let (storage, pid) = test_storage_with_project("scan_log_test")?;
        let doc_id = seed_document(&storage, &pid, "001.txt")?;
        let id = storage.record_file_scan(&pid, &doc_id, None, "abc", "created", None)?;
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
        let doc_id = seed_document(&storage, &pid, "001.txt")?;
        let diff = r#"[{"kind":"modified","index":0}]"#;
        let id = storage.record_revision(
            &pid,
            &doc_id,
            "incremental",
            Some("old"),
            "new",
            1,
            1,
            0,
            0,
            1,
            Some(diff),
        )?;
        assert!(!id.is_empty());
        let revs = storage.list_revisions(&pid, Some(&doc_id), 10)?;
        assert_eq!(revs.len(), 1);
        assert_eq!(revs[0].chunks_modified, 1);
        Ok(())
    }

    #[test]
    fn existing_document_id_excludes_deleted() -> Result<()> {
        let (storage, pid) = test_storage_with_project("deleted_test")?;
        let doc_id = seed_document(&storage, &pid, "gone.txt")?;
        storage.mark_document_deleted(&doc_id)?;
        let found = storage.existing_document_id(&pid, "gone.txt")?;
        assert!(found.is_none(), "deleted doc should not be returned");
        Ok(())
    }

    #[test]
    fn project_document_by_id_excludes_deleted() {
        let storage = Storage::open_memory().expect("storage opens");
        let project = storage
            .create_project("del_test", "/tmp/test")
            .expect("project created");
        let doc_id = storage
            .upsert_document_with_chunks(
                &project.id,
                &NewDocument {
                    path: "d.txt".into(),
                    kind: "text".into(),
                    title: "章".into(),
                    chapter_count: 1,
                    content_hash: "h".into(),
                    word_count: 1,
                    encoding: "utf-8".into(),
                },
                &[NewChunk {
                    chunk_index: 0,
                    title: "章".into(),
                    start_offset: 0,
                    end_offset: 1,
                    content: "a".into(),
                    content_hash: "h".into(),
                    word_count: 1,
                }],
            )
            .expect("doc stored");
        storage.mark_document_deleted(&doc_id).expect("deleted");
        let result = storage.project_document_by_id(&doc_id);
        assert!(result.is_err(), "deleted doc should error");
    }

    #[test]
    fn project_document_by_id_not_found() {
        let storage = Storage::open_memory().expect("storage opens");
        let result = storage.project_document_by_id("nonexistent");
        assert!(result.is_err(), "bogus id should error");
    }

    #[test]
    fn mark_document_deleted_idempotent() -> Result<()> {
        let (storage, pid) = test_storage_with_project("idempotent")?;
        let doc_id = seed_document(&storage, &pid, "again.txt")?;
        storage.mark_document_deleted(&doc_id)?;
        storage.mark_document_deleted(&doc_id)?;
        Ok(())
    }

    #[test]
    fn list_file_scans_empty_project() -> Result<()> {
        let (storage, pid) = test_storage_with_project("empty_scans")?;
        let logs = storage.list_file_scans(&pid, 10)?;
        assert!(logs.is_empty());
        Ok(())
    }

    #[test]
    fn list_revisions_without_document_id() -> Result<()> {
        let (storage, pid) = test_storage_with_project("all_revs")?;
        let d1 = seed_document(&storage, &pid, "a.txt")?;
        let d2 = seed_document(&storage, &pid, "b.txt")?;
        storage.record_revision(&pid, &d1, "incremental", None, "h1", 0, 1, 0, 0, 1, None)?;
        storage.record_revision(&pid, &d2, "incremental", None, "h2", 0, 1, 0, 0, 1, None)?;
        let revs = storage.list_revisions(&pid, None, 10)?;
        assert_eq!(revs.len(), 2);
        Ok(())
    }

    #[test]
    fn document_chunks_empty_for_nonexistent_document() -> Result<()> {
        let storage = test_storage()?;
        let chunks = storage.document_chunks("bogus")?;
        assert!(chunks.is_empty());
        Ok(())
    }

    #[test]
    fn treats_like_wildcards_as_literal_query_text() {
        let storage = Storage::open_memory().expect("storage opens");
        let project = storage
            .create_project("雨巷钟声", "D:/novels/rain")
            .expect("project created");

        storage
            .upsert_document_with_chunks(
                &project.id,
                &NewDocument {
                    path: "001.txt".to_string(),
                    kind: "text".to_string(),
                    title: "第一章".to_string(),
                    chapter_count: 1,
                    content_hash: "hash".to_string(),
                    word_count: 4,
                    encoding: "utf-8".to_string(),
                },
                &[NewChunk {
                    chunk_index: 0,
                    title: "第一章 雨夜".to_string(),
                    start_offset: 0,
                    end_offset: 12,
                    content: "林澈在雨夜醒来。".to_string(),
                    content_hash: "chunk-hash".to_string(),
                    word_count: 8,
                }],
            )
            .expect("document stored");

        let hits = storage
            .search(&project.id, "%", 10)
            .expect("search succeeds");

        assert!(hits.is_empty());
    }

    #[test]
    fn stores_and_retrieves_profile_metrics() -> Result<()> {
        let (storage, pid) = test_storage_with_project("profile_metrics_test")?;
        storage.upsert_profile_metric(&NewProfileMetric {
            profile_id: "shuangwen".into(),
            project_id: pid.clone(),
            metric_type: "爽点密度".into(),
            document_id: None,
            value_json: r#"{"value": 3.5, "unit": "per_1000_chars"}"#.into(),
        })?;
        let metrics = storage.get_profile_metrics(&pid, "shuangwen")?;
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].metric_type, "爽点密度");
        Ok(())
    }

    #[test]
    fn stores_and_loads_project_profiles() -> Result<()> {
        let (storage, pid) = test_storage_with_project("profiles_crud_test")?;
        let default = storage.get_project_profiles(&pid)?;
        assert_eq!(default, vec!["common_longform"]);
        storage.set_project_profiles(&pid, &["common_longform", "shuangwen"])?;
        let loaded = storage.get_project_profiles(&pid)?;
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains(&"shuangwen".to_string()));
        Ok(())
    }

    #[test]
    fn stores_and_retrieves_knowledge_packs() -> Result<()> {
        let storage = test_storage()?;
        storage.upsert_knowledge_pack(&KnowledgePackEntry {
            profile_id: "history".into(),
            pack_name: "tang_officials".into(),
            pack_type: "officials".into(),
            entries_json: r#"[{"term":"尚书","rank":"正三品"}]"#.into(),
            version: "0.1.0".into(),
        })?;
        let packs = storage.get_knowledge_packs("history")?;
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].pack_name, "tang_officials");
        Ok(())
    }

    #[test]
    fn stores_and_retrieves_rules() -> Result<()> {
        let (storage, pid) = test_storage_with_project("rules_test")?;
        storage.upsert_rule(&WorldRule {
            id: "r1".into(),
            project_id: pid.clone(),
            name: "魔法不能凭空制造生命".into(),
            description: "禁止用魔法创造生命".into(),
            rule_type: "world".into(),
            keywords_json: r#"["魔法","生命","创造"]"#.into(),
            positive: true,
            source_chunk_id: None,
            confidence: 100,
            status: "active".into(),
            created_at: "now".into(),
            updated_at: "now".into(),
        })?;
        let rules = storage.list_rules(&pid)?;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "魔法不能凭空制造生命");
        Ok(())
    }

    #[test]
    fn stores_and_lists_timeline_events() -> Result<()> {
        let (storage, pid) = test_storage_with_project("tl_test")?;
        let doc_id = seed_document(&storage, &pid, "002.txt")?;
        let chunks = storage.document_chunks(&doc_id)?;
        let chunk_id = chunks[0].chunk_id.clone();
        storage.upsert_timeline_event(&TimelineEvent {
            id: "t1".into(),
            project_id: pid.clone(),
            chunk_id: chunk_id,
            chunk_index: 0,
            document_path: "002.txt".into(),
            title: "第一章".into(),
            order_index: 1,
            time_expression: "三天后".into(),
            estimated_order: Some(3),
            participants_json: r#"["林澈"]"#.into(),
            location: "长安".into(),
            is_flashback: false,
            confidence: 50,
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
            project_id: pid.clone(),
            title: "检查战力倒退".into(),
            task_type: "conflict".into(),
            priority: "high".into(),
            source_issue_id: None,
            source_foreshadow_id: None,
            related_chunks_json: "[]".into(),
            notes: String::new(),
        })?;
        let tasks = storage.list_tasks(&pid)?;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "检查战力倒退");
        storage.update_task_status(&id, "resolved")?;
        let task = storage.get_task(&id)?.expect("exists");
        assert_eq!(task.status, "resolved");
        Ok(())
    }

    #[test]
    fn get_nonexistent_rule_returns_none() -> Result<()> {
        let storage = test_storage()?;
        let rule = storage.get_rule("nonexistent")?;
        assert!(rule.is_none());
        Ok(())
    }

    #[test]
    fn delete_nonexistent_rule_is_ok() -> Result<()> {
        let storage = test_storage()?;
        storage.delete_rule("nonexistent")?;
        Ok(())
    }

    #[test]
    fn list_tasks_empty_project() -> Result<()> {
        let (storage, pid) = test_storage_with_project("empty_tasks")?;
        let tasks = storage.list_tasks(&pid)?;
        assert!(tasks.is_empty());
        Ok(())
    }

    #[test]
    fn create_task_duplicate_source_id() -> Result<()> {
        let (storage, pid) = test_storage_with_project("dup_source")?;
        let id1 = storage.create_task(&NewRevisionTask {
            project_id: pid.clone(),
            title: "检查战力倒退".into(),
            task_type: "conflict".into(),
            priority: "high".into(),
            source_issue_id: Some("issue1".into()),
            source_foreshadow_id: None,
            related_chunks_json: "[]".into(),
            notes: String::new(),
        })?;
        let id2 = storage.create_task(&NewRevisionTask {
            project_id: pid.clone(),
            title: "另一个任务".into(),
            task_type: "foreshadow".into(),
            priority: "medium".into(),
            source_issue_id: Some("issue1".into()),
            source_foreshadow_id: None,
            related_chunks_json: "[]".into(),
            notes: String::new(),
        })?;
        assert_ne!(id1, id2, "should create separate tasks");
        let tasks = storage.list_tasks(&pid)?;
        assert_eq!(tasks.len(), 2);
        Ok(())
    }

    #[test]
    fn timeline_event_empty_participants() -> Result<()> {
        let (storage, pid) = test_storage_with_project("empty_part")?;
        let doc_id = seed_document(&storage, &pid, "001.txt")?;
        let chunks = storage.document_chunks(&doc_id)?;
        let chunk_id = chunks[0].chunk_id.clone();
        storage.upsert_timeline_event(&TimelineEvent {
            id: "t1".into(),
            project_id: pid.clone(),
            chunk_id,
            chunk_index: 0,
            document_path: "001.txt".into(),
            title: "第一章".into(),
            order_index: 1,
            time_expression: String::new(),
            estimated_order: None,
            participants_json: String::new(),
            location: String::new(),
            is_flashback: false,
            confidence: 50,
        })?;
        let events = storage.list_timeline_events(&pid)?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].participants_json, "");
        Ok(())
    }

    #[test]
    fn search_handles_empty_query() -> Result<()> {
        let (storage, pid) = test_storage_with_project("empty_search")?;
        let hits_empty = storage.search(&pid, "", 10)?;
        assert!(hits_empty.is_empty());
        let hits_whitespace = storage.search(&pid, "  ", 10)?;
        assert!(hits_whitespace.is_empty());
        Ok(())
    }

    #[test]
    fn search_handles_special_characters() -> Result<()> {
        let (storage, pid) = test_storage_with_project("special_search")?;
        seed_document(&storage, &pid, "001.txt")?;
        let hits_pct = storage.search(&pid, "%", 10)?;
        let hits_underscore = storage.search(&pid, "_", 10)?;
        let hits_backslash = storage.search(&pid, "\\", 10)?;
        assert!(hits_pct.is_empty());
        assert!(hits_underscore.is_empty());
        assert!(hits_backslash.is_empty());
        Ok(())
    }

    #[test]
    fn upsert_timeline_event_duplicate_id() -> Result<()> {
        let (storage, pid) = test_storage_with_project("dup_event")?;
        let doc_id = seed_document(&storage, &pid, "001.txt")?;
        let chunks = storage.document_chunks(&doc_id)?;
        let chunk_id = chunks[0].chunk_id.clone();
        storage.upsert_timeline_event(&TimelineEvent {
            id: "same_id".into(),
            project_id: pid.clone(),
            chunk_id: chunk_id.clone(),
            chunk_index: 0,
            document_path: "001.txt".into(),
            title: "第一章".into(),
            order_index: 1,
            time_expression: String::new(),
            estimated_order: None,
            participants_json: "[]".into(),
            location: String::new(),
            is_flashback: false,
            confidence: 50,
        })?;
        let result = storage.upsert_timeline_event(&TimelineEvent {
            id: "same_id".into(),
            project_id: pid.clone(),
            chunk_id,
            chunk_index: 0,
            document_path: "001.txt".into(),
            title: "第一章".into(),
            order_index: 1,
            time_expression: String::new(),
            estimated_order: None,
            participants_json: "[]".into(),
            location: String::new(),
            is_flashback: false,
            confidence: 50,
        });
        assert!(result.is_err(), "duplicate PK should fail");
        Ok(())
    }
}
