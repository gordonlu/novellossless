mod profile;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::profile::{ExtractorRules, PeopleConfig, ProfileConfig};

use anyhow::{Context, Result};
use novellossless_parser::parse_document;
use novellossless_storage::{
    ContextPack, ContinuityIssue, ForeshadowItem, NarrativeNode, NewChunk, NewContinuityIssue,
    NewDocument, NewForeshadowItem, NewNarrativeNode, Project, ProjectChunk, ProjectSummary,
    SearchHit, Storage,
};
use regex::Regex;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateKind {
    Person,
    Place,
    Item,
}

impl CandidateKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Place => "place",
            Self::Item => "item",
        }
    }
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
        let rules = &self.extractor_rules;

        let people = if rules.people {
            extract_candidates(&chunks, CandidateKind::Person)?
        } else {
            Vec::new()
        };
        let places = if rules.places {
            extract_candidates(&chunks, CandidateKind::Place)?
        } else {
            Vec::new()
        };
        let items = if rules.items {
            extract_candidates(&chunks, CandidateKind::Item)?
        } else {
            Vec::new()
        };
        let foreshadows = if rules.foreshadows {
            extract_foreshadows(&chunks)
        } else {
            Vec::new()
        };
        let issues = if rules.eye_color_conflicts || rules.repeat_expressions {
            extract_issues(&chunks)?
        } else {
            Vec::new()
        };

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

fn extract_candidates(
    chunks: &[ProjectChunk],
    kind: CandidateKind,
) -> Result<Vec<NewNarrativeNode>> {
    let mut seen = BTreeMap::<String, CandidateAccumulator>::new();
    let patterns = candidate_patterns(kind)?;

    for chunk in chunks {
        for pattern in &patterns {
            for captures in pattern.captures_iter(&chunk.content) {
                let Some(raw) = captures.get(1).map(|value| value.as_str()) else {
                    continue;
                };
                let name = normalize_candidate(raw, kind);
                if !is_candidate_name(&name, kind) {
                    continue;
                }

                seen.entry(name.clone())
                    .or_insert_with(|| CandidateAccumulator {
                        count: 0,
                        first_chunk_id: chunk.chunk_id.clone(),
                        latest_chunk_id: chunk.chunk_id.clone(),
                    });
                if let Some(entry) = seen.get_mut(&name) {
                    entry.count += 1;
                    entry.latest_chunk_id = chunk.chunk_id.clone();
                }
            }
        }
    }

    Ok(seen
        .into_iter()
        .filter(|(_, entry)| entry.count >= min_candidate_count(kind))
        .map(|(name, entry)| NewNarrativeNode {
            node_type: kind.as_str().to_string(),
            name,
            occurrence_count: entry.count,
            first_chunk_id: entry.first_chunk_id,
            latest_chunk_id: entry.latest_chunk_id,
            confidence: candidate_confidence(entry.count),
        })
        .collect())
}

fn candidate_patterns(kind: CandidateKind) -> Result<Vec<Regex>> {
    let raw_patterns = match kind {
        CandidateKind::Person => vec![
            r"([\p{Han}]{2,4})(?:说|问|道|喊|低声|笑道|看着|走进|转身)",
            r"(?:向|对|跟)([\p{Han}]{2,4})(?:说|问|道)",
        ],
        CandidateKind::Place => vec![
            r"([\p{Han}]{1,6}(?:城|镇|村|街|巷|楼|塔|宫|殿|府|山|谷|阁|院|桥|寺|观|港|站|基地|星球|舰船))",
        ],
        CandidateKind::Item => vec![
            r"([\p{Han}]{0,4}(?:钥匙|信|戒指|刀|剑|书|照片|芯片|卷轴|玉佩|伞|令牌|地图|药瓶|手札|玉简))",
            r"(?:拿起|藏起|交给|寻找|丢失|夺走|握住)([\p{Han}]{1,6})",
        ],
    };

    raw_patterns
        .into_iter()
        .map(Regex::new)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn normalize_candidate(raw: &str, kind: CandidateKind) -> String {
    let trimmed = raw.trim_matches(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '，' | '。' | '、' | '：' | '；' | '“' | '”' | '"' | '\'' | '《' | '》'
            )
    });

    match kind {
        CandidateKind::Person => trimmed.to_string(),
        CandidateKind::Place => trim_after_context_verb(
            trimmed,
            &[
                "城", "镇", "村", "街", "巷", "楼", "塔", "宫", "殿", "府", "山", "谷", "阁", "院",
                "桥", "寺", "观", "港", "站", "基地", "星球", "舰船",
            ],
        ),
        CandidateKind::Item => trim_quantity_prefix(&trim_after_context_verb(
            trimmed,
            &[
                "钥匙", "信", "戒指", "刀", "剑", "书", "照片", "芯片", "卷轴", "玉佩", "伞",
                "令牌", "地图", "药瓶", "手札", "玉简",
            ],
        )),
    }
}

fn trim_quantity_prefix(value: &str) -> String {
    for prefix in [
        "那枚", "这枚", "一枚", "那把", "这把", "一把", "那封", "这封", "一封",
    ] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            return stripped.to_string();
        }
    }
    value.to_string()
}

fn trim_after_context_verb(raw: &str, suffixes: &[&str]) -> String {
    let context_verbs = [
        "说", "问", "道", "看着", "走进", "拿起", "藏起", "交给", "寻找", "丢失", "夺走", "握住",
        "回到", "离开", "来到",
    ];
    let mut value = raw.to_string();
    for verb in context_verbs {
        if let Some((_, right)) = value.rsplit_once(verb) {
            if !right.is_empty() {
                value = right.to_string();
                break;
            }
        }
    }

    for suffix in suffixes {
        if let Some(pos) = value.rfind(suffix) {
            let end = pos + suffix.len();
            value.truncate(end);
            break;
        }
    }

    value
}

fn is_candidate_name(name: &str, kind: CandidateKind) -> bool {
    if name.chars().count() < 2 {
        return false;
    }

    let stopwords = [
        "自己", "什么", "这里", "那里", "哪里", "这个", "那个", "他们", "我们", "你们", "没有",
        "不是", "已经", "突然",
    ];
    if stopwords.contains(&name) {
        return false;
    }

    match kind {
        CandidateKind::Person => !name.ends_with("里") && !name.ends_with("中"),
        CandidateKind::Place | CandidateKind::Item => true,
    }
}

fn min_candidate_count(kind: CandidateKind) -> i64 {
    match kind {
        CandidateKind::Person => 1,
        CandidateKind::Place => 1,
        CandidateKind::Item => 1,
    }
}

fn candidate_confidence(count: i64) -> i64 {
    (50 + count.saturating_mul(10)).min(90)
}

fn extract_foreshadows(chunks: &[ProjectChunk]) -> Vec<NewForeshadowItem> {
    let markers = [
        "秘密",
        "线索",
        "预感",
        "总觉得",
        "似乎",
        "好像",
        "日后",
        "终有一日",
        "钥匙",
        "信物",
        "谜",
    ];
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();

    for chunk in chunks {
        for sentence in split_sentences(&chunk.content) {
            if !markers.iter().any(|marker| sentence.contains(marker)) {
                continue;
            }
            let title = sentence.chars().take(28).collect::<String>();
            let key = format!("{}:{}", chunk.chunk_id, title);
            if !seen.insert(key) {
                continue;
            }
            items.push(NewForeshadowItem {
                title,
                foreshadow_type: "explicit_clue".to_string(),
                first_chunk_id: chunk.chunk_id.clone(),
                latest_chunk_id: chunk.chunk_id.clone(),
                risk_level: "medium".to_string(),
                evidence: sentence.chars().take(120).collect(),
            });
        }
    }

    items
}

fn extract_issues(chunks: &[ProjectChunk]) -> Result<Vec<NewContinuityIssue>> {
    let mut issues = Vec::new();
    issues.extend(extract_eye_color_conflicts(chunks)?);
    issues.extend(extract_repeat_expression_issues(chunks));
    Ok(issues)
}

fn extract_eye_color_conflicts(chunks: &[ProjectChunk]) -> Result<Vec<NewContinuityIssue>> {
    let pattern = Regex::new(
        r"([\p{Han}]{2,4}).{0,12}(黑色|灰蓝色|蓝色|褐色|金色|红色|琥珀色).{0,8}(?:眼睛|眼眸|眸子)",
    )?;
    let mut facts = HashMap::<String, BTreeMap<String, Vec<&ProjectChunk>>>::new();

    for chunk in chunks {
        for captures in pattern.captures_iter(&chunk.content) {
            let Some(person) = captures
                .get(1)
                .map(|value| normalize_candidate(value.as_str(), CandidateKind::Person))
            else {
                continue;
            };
            let Some(color) = captures.get(2).map(|value| value.as_str().to_string()) else {
                continue;
            };
            facts
                .entry(person)
                .or_default()
                .entry(color)
                .or_default()
                .push(chunk);
        }
    }

    let mut issues = Vec::new();
    for (person, colors) in facts {
        if colors.len() < 2 {
            continue;
        }
        let evidence = colors
            .iter()
            .filter_map(|(color, chunks)| {
                chunks.first().map(|chunk| {
                    json!({
                        "color": color,
                        "chunk_id": chunk.chunk_id,
                        "title": chunk.title,
                        "document_path": chunk.document_path,
                        "snippet": chunk.content.chars().take(100).collect::<String>(),
                    })
                })
            })
            .collect::<Vec<_>>();
        issues.push(NewContinuityIssue {
            issue_type: "character_attribute_conflict".to_string(),
            severity: "high".to_string(),
            title: format!("{person} 的眼睛颜色可能前后不一致"),
            description: format!("{person} 出现了多个眼睛颜色候选，请依据正文确认。"),
            evidence_json: serde_json::to_string(&evidence)?,
            suggested_actions_json: serde_json::to_string(&json!([
                "保持旧设定",
                "接受新设定",
                "标记为伪装",
                "标记为角色认知",
                "标记误报"
            ]))?,
        });
    }

    Ok(issues)
}

fn extract_repeat_expression_issues(chunks: &[ProjectChunk]) -> Vec<NewContinuityIssue> {
    let watched_terms = ["雨夜", "沉默", "钟声", "秘密", "黑暗"];
    let mut issues = Vec::new();

    for term in watched_terms {
        let hits = chunks
            .iter()
            .filter(|chunk| chunk.content.contains(term))
            .collect::<Vec<_>>();
        if hits.len() < 3 {
            continue;
        }

        let evidence = hits
            .iter()
            .take(5)
            .map(|chunk| {
                json!({
                    "chunk_id": chunk.chunk_id,
                    "title": chunk.title,
                    "document_path": chunk.document_path,
                    "snippet": make_local_snippet(&chunk.content, term),
                })
            })
            .collect::<Vec<_>>();

        issues.push(NewContinuityIssue {
            issue_type: "repeat_expression".to_string(),
            severity: "low".to_string(),
            title: format!("“{term}”反复出现"),
            description: format!(
                "“{term}”在多个正文片段中重复出现，可在周报或修订时检查是否有意为之。"
            ),
            evidence_json: serde_json::to_string(&evidence).unwrap_or_else(|_| "[]".to_string()),
            suggested_actions_json: serde_json::to_string(&json!([
                "稍后处理",
                "标记为有意为之",
                "创建修订任务",
                "标记误报"
            ]))
            .unwrap_or_else(|_| "[]".to_string()),
        });
    }

    issues
}

fn split_sentences(content: &str) -> Vec<String> {
    content
        .split(['。', '！', '？', '\n'])
        .map(str::trim)
        .filter(|value| value.chars().count() >= 8)
        .map(ToString::to_string)
        .collect()
}

fn make_local_snippet(content: &str, query: &str) -> String {
    content
        .find(query)
        .map(|byte_start| {
            let char_start = content[..byte_start].chars().count();
            let chars = content.chars().collect::<Vec<_>>();
            let prefix = char_start.saturating_sub(18);
            let suffix = (char_start + query.chars().count() + 18).min(chars.len());
            chars[prefix..suffix].iter().collect()
        })
        .unwrap_or_else(|| content.chars().take(60).collect())
}

fn plain_snippet(snippet: &str) -> String {
    snippet.replace(['[', ']'], "")
}

fn find_profiles_root() -> PathBuf {
    if let Ok(current_dir) = std::env::current_dir() {
        for ancestor in current_dir.ancestors() {
            let candidate = ancestor.join("profiles");
            if candidate.join("common_longform").join("profile.toml").exists() {
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

#[derive(Debug)]
struct CandidateAccumulator {
    count: i64,
    first_chunk_id: String,
    latest_chunk_id: String,
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
