use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
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
            "#,
        )?;

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
            SELECT id, issue_type, severity, title, description, evidence_json,
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
                issue_type: row.get(1)?,
                severity: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                evidence_json: row.get(5)?,
                suggested_actions_json: row.get(6)?,
                status: row.get(7)?,
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
                "WHERE project_id = ?1 AND document_id = ?2".to_string(),
                project_id.to_string(),
                d.to_string(),
            ),
            None => (
                "WHERE project_id = ?1".to_string(),
                project_id.to_string(),
                String::new(),
            ),
        };
        let sql = format!(
            "SELECT id, project_id, document_id, revision_type, old_content_hash, new_content_hash, \
             old_chunk_count, new_chunk_count, chunks_added, chunks_removed, chunks_modified, \
             diff_json, created_at FROM revision_history {clause} ORDER BY created_at DESC LIMIT ?3"
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
}
