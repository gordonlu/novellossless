mod analysis;
mod profile;
mod scan;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::analysis::extractor::{ChunkInfo, Extraction, Extractor};
use crate::analysis::{
    EyeColorConflictExtractor, ForeshadowExtractor, ItemExtractor, PersonExtractor, PlaceExtractor,
    RepeatExpressionExtractor,
};
use crate::profile::{ExtractorRules, PeopleConfig, ProfileConfig};

use anyhow::{Context, Result};
use novellossless_parser::parse_document;
use novellossless_storage::{
    ContextPack, ContinuityIssue, ForeshadowItem, NarrativeNode, NewChunk, NewContinuityIssue,
    NewDocument, NewForeshadowItem, NewNarrativeNode, Project, ProjectChunk, ProjectSummary,
    SearchHit, Storage,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

pub struct NovelCore {
    storage: Storage,
    profiles: Vec<ProfileConfig>,
    extractor_rules: ExtractorRules,
    people_config: PeopleConfig,
}

pub trait ProgressReporter {
    fn report(&self, current: usize, total: usize, file: &str);
    fn error(&self, file: &str, message: &str);
}

pub struct NoopProgress;
impl ProgressReporter for NoopProgress {
    fn report(&self, _current: usize, _total: usize, _file: &str) {}
    fn error(&self, _file: &str, _message: &str) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub project_id: String,
    pub scanned_documents: usize,
    pub skipped_files: usize,
    pub summary: ProjectSummary,
    pub analysis: AnalysisReport,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisReport {
    pub person_candidates: usize,
    pub place_candidates: usize,
    pub item_candidates: usize,
    pub foreshadow_candidates: usize,
    pub issue_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dashboard {
    pub summary: ProjectSummary,
    pub person_candidates: usize,
    pub place_candidates: usize,
    pub item_candidates: usize,
    pub foreshadow_candidates: usize,
    pub issue_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyStatus {
    pub offline_mode: bool,
    pub ai_enabled: bool,
    pub uploads_enabled: bool,
    pub clipboard_access: bool,
    pub screenshot_access: bool,
    pub keyboard_monitoring: bool,
    pub database_path: String,
    pub storage_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInfo {
    pub id: String,
    pub path: String,
    pub title: String,
    pub chapter_count: i64,
    pub word_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTree {
    pub documents: Vec<DocumentInfo>,
    pub chunks: Vec<ProjectChunk>,
}

impl NovelCore {
    pub fn open(db_path: &Path) -> Result<Self> {
        let storage = Storage::open(db_path)?;
        let profiles_root = find_profiles_root();
        let profiles = load_profiles_from(&profiles_root);
        let analysis_rules = profile::load_analysis_rules(&profiles_root);
        let core = Self {
            storage,
            profiles,
            extractor_rules: analysis_rules.extractors,
            people_config: analysis_rules.people,
        };
        core.seed_default_settings().ok();
        Ok(core)
    }

    fn seed_default_settings(&self) -> Result<()> {
        let defaults: [(&str, &str); 8] = [
            ("language", "zh-CN"),
            ("theme", "dark"),
            ("auto_scan", "true"),
            ("auto_watch", "false"),
            ("ai_enabled", "false"),
            ("uploads_enabled", "false"),
            ("backup_enabled", "true"),
            ("backup_path", ""),
        ];
        for (key, value) in &defaults {
            if self.storage.get_setting(key)?.is_none() {
                self.storage.set_setting(key, value)?;
            }
        }
        Ok(())
    }

    pub fn from_storage(storage: Storage) -> Self {
        let profiles_root = find_profiles_root();
        let profiles = load_profiles_from(&profiles_root);
        let analysis_rules = profile::load_analysis_rules(&profiles_root);
        Self {
            storage,
            profiles,
            extractor_rules: analysis_rules.extractors,
            people_config: analysis_rules.people,
        }
    }

    pub fn import_project(&self, name: &str, root_path: &Path) -> Result<Project> {
        let canonical_root = root_path
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", root_path.display()))?;
        ensure_supported_root(&canonical_root)?;

        self.storage
            .create_project(name, &canonical_root.to_string_lossy())
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        self.storage.list_projects()
    }

    pub fn get_project(&self, project_id: &str) -> Result<Option<Project>> {
        self.storage.get_project(project_id)
    }

    pub fn project_summary(&self, project_id: &str) -> Result<ProjectSummary> {
        self.storage.project_summary(project_id)
    }

    pub fn dashboard(&self, project_id: &str) -> Result<Dashboard> {
        let summary = self.project_summary(project_id)?;
        Ok(Dashboard {
            summary,
            person_candidates: self
                .list_candidates(project_id, Some("person"), 1_000)?
                .len(),
            place_candidates: self
                .list_candidates(project_id, Some("place"), 1_000)?
                .len(),
            item_candidates: self.list_candidates(project_id, Some("item"), 1_000)?.len(),
            foreshadow_candidates: self.list_foreshadows(project_id, 1_000)?.len(),
            issue_count: self.list_issues(project_id, 1_000)?.len(),
        })
    }

    pub fn scan_project(&self, project_id: &str) -> Result<ScanReport> {
        self.scan_project_with_progress(project_id, &NoopProgress)
    }

    pub fn scan_project_with_progress(
        &self,
        project_id: &str,
        progress: &dyn ProgressReporter,
    ) -> Result<ScanReport> {
        let project = self
            .storage
            .get_project(project_id)?
            .with_context(|| format!("project not found: {project_id}"))?;
        let root = PathBuf::from(&project.root_path);
        let files = collect_text_files(&root)?;
        let total = files.len();
        let mut scanned_documents = 0;
        let mut skipped_files = 0;

        let profile = self.profiles.first();
        let enable_chunking = profile.map(|p| p.rules.chapter_recognition).unwrap_or(true);

        for (i, file) in files.iter().enumerate() {
            let relative = relative_document_path(&root, file);
            progress.report(i + 1, total, &relative);
            match self.scan_file(&project, &root, file, enable_chunking) {
                Ok(()) => scanned_documents += 1,
                Err(e) => {
                    progress.error(&relative, &e.to_string());
                    skipped_files += 1;
                }
            }
        }

        let analysis = self.analyze_project(project_id)?;
        let summary = self.storage.project_summary(project_id)?;
        Ok(ScanReport {
            project_id: project_id.to_string(),
            scanned_documents,
            skipped_files,
            summary,
            analysis,
        })
    }

    pub fn incremental_scan(&self, project_id: &str) -> Result<ScanResult> {
        self.incremental_scan_with_progress(project_id, &NoopProgress)
    }

    pub fn incremental_scan_with_progress(
        &self,
        project_id: &str,
        progress: &dyn ProgressReporter,
    ) -> Result<ScanResult> {
        let project = self
            .storage
            .get_project(project_id)?
            .with_context(|| format!("project not found: {project_id}"))?;
        let root = PathBuf::from(&project.root_path);
        let files = collect_text_files(&root)?;
        let total = files.len();
        let profile = self.profiles.first();
        let enable_chunking = profile.map(|p| p.rules.chapter_recognition).unwrap_or(true);

        let mut created = 0usize;
        let mut modified = 0usize;
        let mut unchanged = 0usize;
        let mut failed = 0usize;
        let mut file_paths: HashSet<String> = HashSet::new();

        for (i, file) in files.iter().enumerate() {
            let relative = relative_document_path(&root, file);
            progress.report(i + 1, total, &relative);
            file_paths.insert(relative.clone());
            let hash = file_content_hash(file)?;

            match self.storage.existing_document_id(project_id, &relative)? {
                None => match self.scan_file(&project, &root, file, enable_chunking) {
                    Ok(()) => {
                        created += 1;
                        if let Ok(Some(doc_id)) =
                            self.storage.existing_document_id(project_id, &relative)
                        {
                            let _ = self.storage.record_file_scan(
                                project_id, &doc_id, None, &hash, "created", None,
                            );
                        }
                    }
                    Err(e) => {
                        progress.error(&relative, &e.to_string());
                        failed += 1;
                    }
                },
                Some(doc_id) => {
                    let current_doc = self.storage.project_document_by_id(&doc_id)?;
                    if current_doc.content_hash == hash {
                        unchanged += 1;
                        let _ = self.storage.record_file_scan(
                            project_id,
                            &doc_id,
                            Some(&hash),
                            &hash,
                            "unchanged",
                            None,
                        );
                    } else {
                        let old_chunks = self.storage.document_chunks(&doc_id)?;
                        match self.scan_file(&project, &root, file, enable_chunking) {
                            Ok(()) => {
                                modified += 1;
                                let parsed = novellossless_parser::parse_document(file)?;
                                let chapters = if enable_chunking {
                                    parsed.chapters
                                } else {
                                    vec![novellossless_parser::Chapter {
                                        index: 0,
                                        title: parsed.title.clone(),
                                        start_offset: 0,
                                        end_offset: parsed.content.len(),
                                        content: parsed.content.clone(),
                                    }]
                                };
                                let new_chunks: Vec<NewChunk> = chapters
                                    .iter()
                                    .map(|ch| NewChunk {
                                        chunk_index: ch.index as i64,
                                        title: ch.title.clone(),
                                        start_offset: ch.start_offset as i64,
                                        end_offset: ch.end_offset as i64,
                                        content: ch.content.clone(),
                                        content_hash: sha256_hex(ch.content.as_bytes()),
                                        word_count: count_words(&ch.content) as i64,
                                    })
                                    .collect();
                                let diff = scan::diff_chunks(&old_chunks, &new_chunks);
                                let diff_arr: Vec<serde_json::Value> = {
                                    let mut v = Vec::new();
                                    for a in &diff.added {
                                        v.push(serde_json::json!({"kind":"added","index":a.index,"title":a.title,"hash":a.hash}));
                                    }
                                    for r in &diff.removed {
                                        v.push(serde_json::json!({"kind":"removed","index":r.index,"title":r.title,"hash":r.hash}));
                                    }
                                    for m in &diff.modified {
                                        v.push(serde_json::json!({"kind":"modified","index":m.index,"old_title":m.old_title,"new_title":m.new_title,"old_hash":m.old_hash,"new_hash":m.new_hash}));
                                    }
                                    v
                                };
                                let diff_json = serde_json::to_string(&diff_arr).ok();
                                let _ = self.storage.record_file_scan(
                                    project_id,
                                    &doc_id,
                                    Some(&current_doc.content_hash),
                                    &hash,
                                    "modified",
                                    None,
                                );
                                let _ = self.storage.record_revision(
                                    project_id,
                                    &doc_id,
                                    "incremental",
                                    Some(&current_doc.content_hash),
                                    &hash,
                                    old_chunks.len() as i64,
                                    new_chunks.len() as i64,
                                    diff.added.len() as i64,
                                    diff.removed.len() as i64,
                                    diff.modified.len() as i64,
                                    diff_json.as_deref(),
                                );
                            }
                            Err(e) => {
                                progress.error(&relative, &e.to_string());
                                failed += 1;
                            }
                        }
                    }
                }
            }
        }

        let mut deleted = 0usize;
        for doc in self.storage.project_documents(project_id)? {
            if !file_paths.contains(&doc.path) {
                self.storage.mark_document_deleted(&doc.id)?;
                let _ = self.storage.record_file_scan(
                    project_id,
                    &doc.id,
                    Some(&doc.content_hash),
                    &doc.content_hash,
                    "deleted",
                    None,
                );
                deleted += 1;
            }
        }

        let _ = self.analyze_project(project_id)?;

        Ok(ScanResult {
            scanned_documents: created + modified,
            skipped_files: failed,
            created,
            modified,
            unchanged,
            deleted,
            failed,
        })
    }

    pub fn incremental_scan_file(&self, project_id: &str, file_path: &Path) -> Result<ScanResult> {
        let project = self
            .storage
            .get_project(project_id)?
            .with_context(|| format!("project not found: {project_id}"))?;
        let root = PathBuf::from(&project.root_path);
        let profile = self.profiles.first();
        let enable_chunking = profile.map(|p| p.rules.chapter_recognition).unwrap_or(true);
        let relative = relative_document_path(&root, file_path);
        let hash = file_content_hash(file_path)?;

        let mut result = ScanResult {
            scanned_documents: 0,
            skipped_files: 0,
            created: 0,
            modified: 0,
            unchanged: 0,
            deleted: 0,
            failed: 0,
        };

        match self.storage.existing_document_id(project_id, &relative)? {
            None => match self.scan_file(&project, &root, file_path, enable_chunking) {
                Ok(()) => {
                    result.created = 1;
                    if let Ok(Some(doc_id)) =
                        self.storage.existing_document_id(project_id, &relative)
                    {
                        let _ = self
                            .storage
                            .record_file_scan(project_id, &doc_id, None, &hash, "created", None);
                    }
                }
                Err(_) => result.failed = 1,
            },
            Some(doc_id) => {
                let current_doc = self.storage.project_document_by_id(&doc_id)?;
                if current_doc.content_hash == hash {
                    result.unchanged = 1;
                    let _ = self.storage.record_file_scan(
                        project_id,
                        &doc_id,
                        Some(&hash),
                        &hash,
                        "unchanged",
                        None,
                    );
                } else {
                    let old_chunks = self.storage.document_chunks(&doc_id)?;
                    match self.scan_file(&project, &root, file_path, enable_chunking) {
                        Ok(()) => {
                            result.modified = 1;
                            let parsed = novellossless_parser::parse_document(file_path)?;
                            let chapters = if enable_chunking {
                                parsed.chapters
                            } else {
                                vec![novellossless_parser::Chapter {
                                    index: 0,
                                    title: parsed.title.clone(),
                                    start_offset: 0,
                                    end_offset: parsed.content.len(),
                                    content: parsed.content.clone(),
                                }]
                            };
                            let new_chunks: Vec<NewChunk> = chapters
                                .iter()
                                .map(|ch| NewChunk {
                                    chunk_index: ch.index as i64,
                                    title: ch.title.clone(),
                                    start_offset: ch.start_offset as i64,
                                    end_offset: ch.end_offset as i64,
                                    content: ch.content.clone(),
                                    content_hash: sha256_hex(ch.content.as_bytes()),
                                    word_count: count_words(&ch.content) as i64,
                                })
                                .collect();
                            let diff = scan::diff_chunks(&old_chunks, &new_chunks);
                            let diff_arr: Vec<serde_json::Value> = {
                                let mut v = Vec::new();
                                for a in &diff.added {
                                    v.push(serde_json::json!({"kind":"added","index":a.index,"title":a.title}));
                                }
                                for r in &diff.removed {
                                    v.push(serde_json::json!({"kind":"removed","index":r.index,"title":r.title}));
                                }
                                for m in &diff.modified {
                                    v.push(serde_json::json!({"kind":"modified","index":m.index,"old_title":m.old_title,"new_title":m.new_title}));
                                }
                                v
                            };
                            let diff_json = serde_json::to_string(&diff_arr).ok();
                            let _ = self.storage.record_file_scan(
                                project_id,
                                &doc_id,
                                Some(&current_doc.content_hash),
                                &hash,
                                "modified",
                                None,
                            );
                            let _ = self.storage.record_revision(
                                project_id,
                                &doc_id,
                                "incremental",
                                Some(&current_doc.content_hash),
                                &hash,
                                old_chunks.len() as i64,
                                new_chunks.len() as i64,
                                diff.added.len() as i64,
                                diff.removed.len() as i64,
                                diff.modified.len() as i64,
                                diff_json.as_deref(),
                            );
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

    pub fn list_file_scans(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<novellossless_storage::FileScanLog>> {
        self.storage.list_file_scans(project_id, limit)
    }

    pub fn list_revisions(
        &self,
        project_id: &str,
        document_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<novellossless_storage::RevisionRecord>> {
        self.storage.list_revisions(project_id, document_id, limit)
    }

    pub fn search(&self, project_id: &str, query: &str, limit: i64) -> Result<Vec<SearchHit>> {
        self.storage.search(project_id, query, limit)
    }

    pub fn list_candidates(
        &self,
        project_id: &str,
        node_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<NarrativeNode>> {
        self.storage
            .list_narrative_nodes(project_id, node_type, limit)
    }

    pub fn list_foreshadows(&self, project_id: &str, limit: i64) -> Result<Vec<ForeshadowItem>> {
        self.storage.list_foreshadow_items(project_id, limit)
    }

    pub fn list_issues(&self, project_id: &str, limit: i64) -> Result<Vec<ContinuityIssue>> {
        self.storage.list_continuity_issues(project_id, limit)
    }

    pub fn document_tree(
        &self,
        project_id: &str,
        _document_id: Option<&str>,
    ) -> Result<DocumentTree> {
        let documents = self
            .storage
            .project_documents(project_id)?
            .into_iter()
            .map(|d| DocumentInfo {
                id: d.id,
                path: d.path,
                title: d.title,
                chapter_count: d.chapter_count,
                word_count: d.word_count,
            })
            .collect();

        let chunks = self.storage.project_chunks(project_id)?;

        Ok(DocumentTree { documents, chunks })
    }

    pub fn update_candidate_status(&self, id: &str, status: &str) -> Result<()> {
        self.storage.update_narrative_node_status(id, status)
    }

    pub fn update_foreshadow_status(&self, id: &str, status: &str) -> Result<()> {
        self.storage.update_foreshadow_status(id, status)
    }

    pub fn update_issue_status(&self, id: &str, status: &str) -> Result<()> {
        self.storage.update_issue_status(id, status)
    }

    pub fn build_context_pack(
        &self,
        project_id: &str,
        query: &str,
        limit: i64,
    ) -> Result<ContextPack> {
        let hits = self.search(project_id, query, limit)?;
        let title = if query.trim().is_empty() {
            "上下文包".to_string()
        } else {
            format!("上下文包：{}", query.trim())
        };

        let mut content = String::new();
        content.push_str("# ");
        content.push_str(&title);
        content.push_str("\n\n");
        content.push_str("> 本上下文包只来自本地索引片段。原始正文仍以项目文件为准。\n\n");

        if hits.is_empty() {
            content.push_str("未找到匹配片段。\n");
        } else {
            for (index, hit) in hits.iter().enumerate() {
                content.push_str(&format!(
                    "## {}. {}\n\n- 来源文件：{}\n- 片段：第 {} 段\n- 位置：{}-{}\n\n{}\n\n",
                    index + 1,
                    hit.title,
                    hit.document_path,
                    hit.chunk_index + 1,
                    hit.start_offset,
                    hit.end_offset,
                    plain_snippet(&hit.snippet)
                ));
            }
        }

        let source_refs = hits
            .iter()
            .map(|hit| {
                json!({
                    "document_id": hit.document_id,
                    "chunk_id": hit.chunk_id,
                    "document_path": hit.document_path,
                    "title": hit.title,
                    "start_offset": hit.start_offset,
                    "end_offset": hit.end_offset,
                })
            })
            .collect::<Vec<_>>();

        self.storage.save_context_pack(
            project_id,
            &title,
            query,
            &content,
            &serde_json::to_string(&source_refs)?,
        )
    }

    pub fn get_settings(&self) -> Vec<(String, String)> {
        self.storage.get_all_settings().unwrap_or_default()
    }

    pub fn update_setting(&self, key: &str, value: &str) -> Result<()> {
        self.storage.set_setting(key, value).map_err(Into::into)
    }

    pub fn privacy_status(&self, db_path: &Path) -> PrivacyStatus {
        let settings: std::collections::HashMap<String, String> =
            self.get_settings().into_iter().collect();
        let to_bool = |key: &str, default: bool| -> bool {
            settings.get(key).map_or(default, |v| v == "true")
        };
        PrivacyStatus {
            offline_mode: true,
            ai_enabled: to_bool("ai_enabled", false),
            uploads_enabled: to_bool("uploads_enabled", false),
            clipboard_access: false,
            screenshot_access: false,
            keyboard_monitoring: false,
            database_path: db_path.display().to_string(),
            storage_mode: "标准本地模式".to_string(),
        }
    }

    pub fn load_profiles(&self, _profiles_root: &Path) -> Result<Vec<ProfileInfo>> {
        Ok(self
            .profiles
            .iter()
            .map(|p| ProfileInfo {
                id: p.id.clone(),
                name: p.name.clone(),
                version: "0.1.0".to_string(),
                description: String::new(),
            })
            .collect())
    }

    fn scan_file(
        &self,
        project: &Project,
        root: &Path,
        file: &Path,
        enable_chunking: bool,
    ) -> Result<()> {
        let parsed = parse_document(file)?;
        let relative_path = relative_document_path(root, file);
        let kind = document_kind(file);

        let chapters = if enable_chunking {
            parsed.chapters
        } else {
            vec![parsed.chapters.into_iter().next().unwrap_or_else(|| {
                novellossless_parser::Chapter {
                    index: 0,
                    title: parsed.title.clone(),
                    start_offset: 0,
                    end_offset: parsed.content.len(),
                    content: parsed.content.clone(),
                }
            })]
        };

        let chunks = chapters
            .iter()
            .map(|chapter| NewChunk {
                chunk_index: chapter.index as i64,
                title: chapter.title.clone(),
                start_offset: chapter.start_offset as i64,
                end_offset: chapter.end_offset as i64,
                content: chapter.content.clone(),
                content_hash: sha256_hex(chapter.content.as_bytes()),
                word_count: count_words(&chapter.content) as i64,
            })
            .collect::<Vec<_>>();

        self.storage.upsert_document_with_chunks(
            &project.id,
            &NewDocument {
                path: relative_path,
                kind,
                title: parsed.title,
                chapter_count: chunks.len() as i64,
                content_hash: sha256_hex(parsed.content.as_bytes()),
                word_count: count_words(&parsed.content) as i64,
                encoding: parsed.encoding,
            },
            &chunks,
        )?;

        Ok(())
    }

    fn analyze_project(&self, project_id: &str) -> Result<AnalysisReport> {
        let chunks = self.storage.project_chunks(project_id)?;
        let chunk_info: Vec<ChunkInfo> = chunks
            .iter()
            .map(|c| ChunkInfo {
                document_id: c.document_id.clone(),
                chunk_id: c.chunk_id.clone(),
                document_path: c.document_path.clone(),
                chunk_index: c.chunk_index,
                title: c.title.clone(),
                content: c.content.clone(),
                start_offset: c.start_offset,
                end_offset: c.end_offset,
            })
            .collect();

        let mut extractors: Vec<Box<dyn Extractor>> = Vec::new();
        let rules = &self.extractor_rules;

        if rules.people {
            extractors.push(Box::new(PersonExtractor::new(self.people_config.clone())));
        }
        if rules.places {
            extractors.push(Box::new(PlaceExtractor::default()));
        }
        if rules.items {
            extractors.push(Box::new(ItemExtractor::default()));
        }
        if rules.foreshadows {
            extractors.push(Box::new(ForeshadowExtractor::default()));
        }
        if rules.eye_color_conflicts {
            extractors.push(Box::new(EyeColorConflictExtractor::default()));
        }
        if rules.repeat_expressions {
            extractors.push(Box::new(RepeatExpressionExtractor::default()));
        }

        let mut people = Vec::new();
        let mut places = Vec::new();
        let mut items = Vec::new();
        let mut foreshadows = Vec::new();
        let mut issues = Vec::new();

        for extractor in &extractors {
            for extraction in extractor.extract(&chunk_info) {
                match extraction {
                    Extraction::Candidate(c) => {
                        let node = NewNarrativeNode {
                            node_type: c.node_type,
                            name: c.name,
                            aliases_json: serde_json::to_string(&c.aliases).unwrap_or_default(),
                            occurrence_count: c.occurrence_count,
                            first_chunk_id: c.first_chunk_id,
                            latest_chunk_id: c.latest_chunk_id,
                            confidence: c.confidence,
                        };
                        match node.node_type.as_str() {
                            "person" => people.push(node),
                            "place" => places.push(node),
                            "item" => items.push(node),
                            _ => {}
                        }
                    }
                    Extraction::Foreshadow(f) => foreshadows.push(NewForeshadowItem {
                        title: f.title,
                        foreshadow_type: f.foreshadow_type,
                        first_chunk_id: f.first_chunk_id,
                        latest_chunk_id: f.latest_chunk_id,
                        risk_level: f.risk_level,
                        evidence: f.evidence,
                        related_nodes_json: serde_json::to_string(&f.related_nodes)
                            .unwrap_or_default(),
                    }),
                    Extraction::Issue(iss) => issues.push(NewContinuityIssue {
                        issue_type: iss.issue_type,
                        severity: iss.severity,
                        title: iss.title,
                        description: iss.description,
                        evidence_json: iss.evidence_json,
                        suggested_actions_json: iss.suggested_actions_json,
                    }),
                }
            }
        }

        self.storage.upsert_narrative_nodes(project_id, &people)?;
        self.storage.upsert_narrative_nodes(project_id, &places)?;
        self.storage.upsert_narrative_nodes(project_id, &items)?;
        self.storage
            .upsert_foreshadow_items(project_id, &foreshadows)?;
        self.storage.upsert_continuity_issues(project_id, &issues)?;

        Ok(AnalysisReport {
            person_candidates: people.len(),
            place_candidates: places.len(),
            item_candidates: items.len(),
            foreshadow_candidates: foreshadows.len(),
            issue_count: issues.len(),
        })
    }
}

fn ensure_supported_root(path: &Path) -> Result<()> {
    if path.is_dir() || path.is_file() {
        return Ok(());
    }

    anyhow::bail!("project root must be an existing file or directory");
}

fn collect_text_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(if is_supported_text_file(root) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        });
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() && is_supported_text_file(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn is_supported_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "txt" | "md" | "markdown"
            )
        })
        .unwrap_or(false)
}

fn document_kind(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("md" | "markdown") => "markdown".to_string(),
        _ => "text".to_string(),
    }
}

fn relative_document_path(root: &Path, file: &Path) -> String {
    if root.is_file() {
        return file
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
    }

    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

fn count_words(content: &str) -> usize {
    content
        .chars()
        .filter(|value| !value.is_whitespace())
        .count()
}

fn file_content_hash(path: &Path) -> Result<String> {
    let content =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(sha256_hex(&content))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn plain_snippet(snippet: &str) -> String {
    snippet.replace(['[', ']'], "")
}

fn find_profiles_root() -> PathBuf {
    if let Ok(current_dir) = std::env::current_dir() {
        for ancestor in current_dir.ancestors() {
            let candidate = ancestor.join("profiles");
            if candidate
                .join("common_longform")
                .join("profile.toml")
                .exists()
            {
                return candidate;
            }
        }
    }
    PathBuf::from("profiles")
}

fn load_profiles_from(profiles_root: &Path) -> Vec<ProfileConfig> {
    let common = profiles_root.join("common_longform").join("profile.toml");
    if !common.exists() {
        return Vec::new();
    }
    let content = match std::fs::read_to_string(&common) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    toml::from_str::<ProfileConfig>(&content)
        .map(|p| vec![p])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use novellossless_storage::Storage;

    #[test]
    fn imports_scans_searches_and_analyzes_a_directory_project() {
        let temp = tempfile::tempdir().expect("tempdir");
        let novel_dir = temp.path().join("novel");
        fs::create_dir(&novel_dir).expect("novel dir");
        fs::write(
            novel_dir.join("001.txt"),
            "第一章 雨夜\n林澈说他在雨夜醒来。沈微问那枚铜钥匙在哪里。\n第二章 钟声\n林澈看着旧钟楼，钟声再次响起。沈微说铜钥匙是秘密。",
        )
        .expect("write text");
        fs::write(novel_dir.join("notes.pdf"), "ignored").expect("write ignored");

        let core = NovelCore::from_storage(Storage::open_memory().expect("storage"));
        let project = core
            .import_project("雨巷钟声", &novel_dir)
            .expect("project import");
        let report = core.scan_project(&project.id).expect("scan");

        assert_eq!(report.scanned_documents, 1);
        assert_eq!(report.summary.document_count, 1);
        assert_eq!(report.summary.chunk_count, 2);
        assert!(report.analysis.person_candidates >= 1);
        assert!(report.analysis.item_candidates >= 1);
        assert!(report.analysis.foreshadow_candidates >= 1);

        let hits = core.search(&project.id, "钟声", 10).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "第二章 钟声");
        assert_eq!(hits[0].document_path, "001.txt");

        let candidates = core
            .list_candidates(&project.id, Some("person"), 10)
            .expect("candidates");
        assert!(candidates.iter().any(|candidate| candidate.name == "林澈"));

        let context = core
            .build_context_pack(&project.id, "铜钥匙", 5)
            .expect("context pack");
        assert!(context.content.contains("铜钥匙"));
        assert!(context.content.contains("来源文件"));
    }

    #[test]
    fn diff_chunks_detects_changes() {
        use crate::scan::diff_chunks;
        use novellossless_storage::{NewChunk, ProjectChunk};

        let old = vec![
            ProjectChunk {
                document_id: "d".into(),
                chunk_id: "c1".into(),
                document_path: "p".into(),
                chunk_index: 0,
                title: "A".into(),
                content: "aa".into(),
                start_offset: 0,
                end_offset: 2,
                word_count: 1,
                content_hash: "same".into(),
            },
            ProjectChunk {
                document_id: "d".into(),
                chunk_id: "c2".into(),
                document_path: "p".into(),
                chunk_index: 1,
                title: "B".into(),
                content: "bb".into(),
                start_offset: 3,
                end_offset: 5,
                word_count: 1,
                content_hash: "old_b".into(),
            },
        ];
        let new = vec![
            NewChunk {
                chunk_index: 0,
                title: "A".into(),
                start_offset: 0,
                end_offset: 2,
                content: "aa".into(),
                content_hash: "same".into(),
                word_count: 1,
            },
            NewChunk {
                chunk_index: 1,
                title: "B2".into(),
                start_offset: 3,
                end_offset: 6,
                content: "bbb".into(),
                content_hash: "diff".into(),
                word_count: 1,
            },
            NewChunk {
                chunk_index: 2,
                title: "C".into(),
                start_offset: 7,
                end_offset: 9,
                content: "cc".into(),
                content_hash: "new".into(),
                word_count: 1,
            },
        ];

        let diff = diff_chunks(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].index, 2);
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].old_title, "B");
        assert_eq!(diff.modified[0].new_title, "B2");
    }

    #[test]
    fn diff_chunks_handles_no_changes() {
        use crate::scan::diff_chunks;
        use novellossless_storage::{NewChunk, ProjectChunk};

        let old = vec![ProjectChunk {
            document_id: "d".into(),
            chunk_id: "c1".into(),
            document_path: "p".into(),
            chunk_index: 0,
            title: "A".into(),
            content: "aa".into(),
            start_offset: 0,
            end_offset: 2,
            word_count: 1,
            content_hash: "same".into(),
        }];
        let new = vec![NewChunk {
            chunk_index: 0,
            title: "A".into(),
            start_offset: 0,
            end_offset: 2,
            content: "aa".into(),
            content_hash: "same".into(),
            word_count: 1,
        }];

        let diff = diff_chunks(&old, &new);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn diff_chunks_handles_empty_input() {
        use crate::scan::diff_chunks;
        use novellossless_storage::NewChunk;

        let old = vec![];
        let new: Vec<NewChunk> = vec![];
        let diff = diff_chunks(&old, &new);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn diff_chunks_handles_removed_only() {
        use crate::scan::diff_chunks;
        use novellossless_storage::{NewChunk, ProjectChunk};

        let old = vec![
            ProjectChunk {
                document_id: "d".into(),
                chunk_id: "c1".into(),
                document_path: "p".into(),
                chunk_index: 0,
                title: "A".into(),
                content: "aa".into(),
                start_offset: 0,
                end_offset: 2,
                word_count: 1,
                content_hash: "h1".into(),
            },
            ProjectChunk {
                document_id: "d".into(),
                chunk_id: "c2".into(),
                document_path: "p".into(),
                chunk_index: 1,
                title: "B".into(),
                content: "bb".into(),
                start_offset: 3,
                end_offset: 5,
                word_count: 1,
                content_hash: "h2".into(),
            },
        ];
        let new: Vec<NewChunk> = vec![];

        let diff = diff_chunks(&old, &new);
        assert!(diff.added.is_empty());
        assert_eq!(diff.removed.len(), 2);
        let removed_indices: Vec<i64> = diff.removed.iter().map(|r| r.index).collect();
        assert!(removed_indices.contains(&0));
        assert!(removed_indices.contains(&1));
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn diff_chunks_handles_added_only() {
        use crate::scan::diff_chunks;
        use novellossless_storage::{NewChunk, ProjectChunk};

        let old: Vec<ProjectChunk> = vec![];
        let new = vec![NewChunk {
            chunk_index: 0,
            title: "A".into(),
            start_offset: 0,
            end_offset: 2,
            content: "aa".into(),
            content_hash: "h1".into(),
            word_count: 1,
        }];

        let diff = diff_chunks(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn incremental_scan_skips_unchanged_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let novel_dir = temp.path().join("novel");
        std::fs::create_dir(&novel_dir).expect("dir");
        std::fs::write(novel_dir.join("001.txt"), "第一章 雨夜\n内容不变。").expect("write");

        let core = NovelCore::from_storage(
            novellossless_storage::Storage::open_memory().expect("storage"),
        );
        let project = core.import_project("test", &novel_dir).expect("import");
        let first = core.incremental_scan(&project.id).expect("first scan");
        assert_eq!(first.scanned_documents, 1);
        assert_eq!(first.created, 1);
        assert_eq!(first.modified, 0);
        assert_eq!(first.unchanged, 0);

        let second = core.incremental_scan(&project.id).expect("second scan");
        assert_eq!(second.scanned_documents, 0);
        assert_eq!(second.created, 0);
        assert_eq!(second.modified, 0);
        assert_eq!(second.unchanged, 1);
        assert_eq!(second.deleted, 0);
        assert_eq!(second.failed, 0);
    }

    fn person_aliases_are_merged() {
        let temp = tempfile::tempdir().expect("tempdir");
        let novel_dir = temp.path().join("novel");
        std::fs::create_dir(&novel_dir).expect("dir");
        std::fs::write(
            novel_dir.join("001.txt"),
            "第一章 雨夜\n林澈说他在雨夜醒来。林兄，你怎么在这里？",
        )
        .expect("write");

        let core = NovelCore::from_storage(Storage::open_memory().expect("storage"));
        let project = core.import_project("test", &novel_dir).expect("import");
        core.scan_project(&project.id).expect("scan");

        let candidates = core
            .list_candidates(&project.id, Some("person"), 10)
            .expect("list");
        let linche = candidates
            .iter()
            .find(|c| c.name == "林澈")
            .expect("林澈 found");
        assert!(linche.occurrence_count >= 2);
    }
}
