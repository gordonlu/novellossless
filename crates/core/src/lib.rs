mod analysis;
mod profile;

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
    NewDocument, NewForeshadowItem, NewNarrativeNode, Project, ProjectSummary, SearchHit, Storage,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub project_id: String,
    pub scanned_documents: usize,
    pub skipped_files: usize,
    pub summary: ProjectSummary,
    pub analysis: AnalysisReport,
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

impl NovelCore {
    pub fn open(db_path: &Path) -> Result<Self> {
        let storage = Storage::open(db_path)?;
        let profiles_root = find_profiles_root();
        let profiles = load_profiles_from(&profiles_root);
        let analysis_rules = profile::load_analysis_rules(&profiles_root);
        Ok(Self {
            storage,
            profiles,
            extractor_rules: analysis_rules.extractors,
            people_config: analysis_rules.people,
        })
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
        let project = self
            .storage
            .get_project(project_id)?
            .with_context(|| format!("project not found: {project_id}"))?;
        let root = PathBuf::from(&project.root_path);
        let files = collect_text_files(&root)?;
        let mut scanned_documents = 0;
        let mut skipped_files = 0;

        let profile = self.profiles.first();
        let enable_chunking = profile.map(|p| p.rules.chapter_recognition).unwrap_or(true);

        for file in files {
            match self.scan_file(&project, &root, &file, enable_chunking) {
                Ok(()) => scanned_documents += 1,
                Err(_) => skipped_files += 1,
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

    pub fn privacy_status(&self, db_path: &Path) -> PrivacyStatus {
        PrivacyStatus {
            offline_mode: true,
            ai_enabled: false,
            uploads_enabled: false,
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
            extractors.push(Box::new(PersonExtractor::default()));
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
}
