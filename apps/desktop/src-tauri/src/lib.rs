use std::path::{Path, PathBuf};

use novellossless_core::{Dashboard, NovelCore, PrivacyStatus, ProfileInfo, ScanReport};
use novellossless_storage::{
    ContextPack, ContinuityIssue, ForeshadowItem, NarrativeNode, Project, ProjectSummary, SearchHit,
};
use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectDto {
    id: String,
    name: String,
    root_path: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummaryDto {
    project_id: String,
    document_count: i64,
    chunk_count: i64,
    total_words: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanReportDto {
    project_id: String,
    scanned_documents: usize,
    skipped_files: usize,
    summary: ProjectSummaryDto,
    analysis: AnalysisReportDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisReportDto {
    person_candidates: usize,
    place_candidates: usize,
    item_candidates: usize,
    foreshadow_candidates: usize,
    issue_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardDto {
    summary: ProjectSummaryDto,
    person_candidates: usize,
    place_candidates: usize,
    item_candidates: usize,
    foreshadow_candidates: usize,
    issue_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchHitDto {
    document_id: String,
    chunk_id: String,
    document_path: String,
    chunk_index: i64,
    title: String,
    snippet: String,
    start_offset: i64,
    end_offset: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NarrativeNodeDto {
    id: String,
    node_type: String,
    name: String,
    occurrence_count: i64,
    status: String,
    confidence: i64,
    source_chunk_id: String,
    source_title: String,
    source_path: String,
    source_snippet: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ForeshadowItemDto {
    id: String,
    title: String,
    foreshadow_type: String,
    status: String,
    risk_level: String,
    source_chunk_id: String,
    source_title: String,
    source_path: String,
    evidence: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContinuityIssueDto {
    id: String,
    issue_type: String,
    severity: String,
    title: String,
    description: String,
    evidence_json: String,
    suggested_actions_json: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextPackDto {
    id: String,
    project_id: String,
    title: String,
    target: String,
    content: String,
    format: String,
    source_refs_json: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrivacyStatusDto {
    offline_mode: bool,
    ai_enabled: bool,
    uploads_enabled: bool,
    clipboard_access: bool,
    screenshot_access: bool,
    keyboard_monitoring: bool,
    database_path: String,
    storage_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileInfoDto {
    id: String,
    name: String,
    version: String,
    description: String,
}

#[tauri::command]
fn list_projects(app: tauri::AppHandle) -> Result<Vec<ProjectDto>, String> {
    let core = open_core(&app)?;
    core.list_projects()
        .map(|projects| projects.into_iter().map(ProjectDto::from).collect())
        .map_err(to_command_error)
}

#[tauri::command]
fn import_project(app: tauri::AppHandle, name: String, path: String) -> Result<ProjectDto, String> {
    let core = open_core(&app)?;
    core.import_project(&name, Path::new(&path))
        .map(ProjectDto::from)
        .map_err(to_command_error)
}

#[tauri::command]
fn get_dashboard(app: tauri::AppHandle, project_id: String) -> Result<DashboardDto, String> {
    let core = open_core(&app)?;
    core.dashboard(&project_id)
        .map(DashboardDto::from)
        .map_err(to_command_error)
}

#[tauri::command]
fn scan_project(app: tauri::AppHandle, project_id: String) -> Result<ScanReportDto, String> {
    let core = open_core(&app)?;
    core.scan_project(&project_id)
        .map(ScanReportDto::from)
        .map_err(to_command_error)
}

#[tauri::command]
fn search_project(
    app: tauri::AppHandle,
    project_id: String,
    query: String,
    limit: i64,
) -> Result<Vec<SearchHitDto>, String> {
    let core = open_core(&app)?;
    core.search(&project_id, &query, limit)
        .map(|hits| hits.into_iter().map(SearchHitDto::from).collect())
        .map_err(to_command_error)
}

#[tauri::command]
fn list_candidates(
    app: tauri::AppHandle,
    project_id: String,
    node_type: Option<String>,
    limit: i64,
) -> Result<Vec<NarrativeNodeDto>, String> {
    let core = open_core(&app)?;
    core.list_candidates(&project_id, node_type.as_deref(), limit)
        .map(|items| items.into_iter().map(NarrativeNodeDto::from).collect())
        .map_err(to_command_error)
}

#[tauri::command]
fn list_foreshadows(
    app: tauri::AppHandle,
    project_id: String,
    limit: i64,
) -> Result<Vec<ForeshadowItemDto>, String> {
    let core = open_core(&app)?;
    core.list_foreshadows(&project_id, limit)
        .map(|items| items.into_iter().map(ForeshadowItemDto::from).collect())
        .map_err(to_command_error)
}

#[tauri::command]
fn list_issues(
    app: tauri::AppHandle,
    project_id: String,
    limit: i64,
) -> Result<Vec<ContinuityIssueDto>, String> {
    let core = open_core(&app)?;
    core.list_issues(&project_id, limit)
        .map(|items| items.into_iter().map(ContinuityIssueDto::from).collect())
        .map_err(to_command_error)
}

#[tauri::command]
fn update_candidate_status(
    app: tauri::AppHandle,
    id: String,
    status: String,
) -> Result<(), String> {
    let core = open_core(&app)?;
    core.update_candidate_status(&id, &status)
        .map_err(to_command_error)
}

#[tauri::command]
fn update_foreshadow_status(
    app: tauri::AppHandle,
    id: String,
    status: String,
) -> Result<(), String> {
    let core = open_core(&app)?;
    core.update_foreshadow_status(&id, &status)
        .map_err(to_command_error)
}

#[tauri::command]
fn update_issue_status(app: tauri::AppHandle, id: String, status: String) -> Result<(), String> {
    let core = open_core(&app)?;
    core.update_issue_status(&id, &status)
        .map_err(to_command_error)
}

#[tauri::command]
fn build_context_pack(
    app: tauri::AppHandle,
    project_id: String,
    query: String,
    limit: i64,
) -> Result<ContextPackDto, String> {
    let core = open_core(&app)?;
    core.build_context_pack(&project_id, &query, limit)
        .map(ContextPackDto::from)
        .map_err(to_command_error)
}

#[tauri::command]
fn get_project_summary(
    app: tauri::AppHandle,
    project_id: String,
) -> Result<ProjectSummaryDto, String> {
    let core = open_core(&app)?;
    core.project_summary(&project_id)
        .map(ProjectSummaryDto::from)
        .map_err(to_command_error)
}

#[tauri::command]
fn get_privacy_status(app: tauri::AppHandle) -> Result<PrivacyStatusDto, String> {
    let core = open_core(&app)?;
    let db_path = database_path(&app)?;
    Ok(PrivacyStatusDto::from(core.privacy_status(&db_path)))
}

#[tauri::command]
fn list_profiles(app: tauri::AppHandle) -> Result<Vec<ProfileInfoDto>, String> {
    let core = open_core(&app)?;
    let profiles_root = profiles_root().map_err(to_command_error)?;
    core.load_profiles(&profiles_root)
        .map(|profiles| profiles.into_iter().map(ProfileInfoDto::from).collect())
        .map_err(to_command_error)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_projects,
            import_project,
            get_dashboard,
            scan_project,
            search_project,
            list_candidates,
            list_foreshadows,
            list_issues,
            update_candidate_status,
            update_foreshadow_status,
            update_issue_status,
            build_context_pack,
            get_project_summary,
            get_privacy_status,
            list_profiles
        ])
        .run(tauri::generate_context!())
        .expect("failed to run novellossless desktop app");
}

fn open_core(app: &tauri::AppHandle) -> Result<NovelCore, String> {
    let db_path = database_path(app)?;
    NovelCore::open(&db_path).map_err(to_command_error)
}

fn database_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录：{error}"))?;
    std::fs::create_dir_all(&app_data)
        .map_err(|error| format!("无法创建应用数据目录 {}：{error}", app_data.display()))?;
    Ok(app_data.join("novellossless.db"))
}

fn profiles_root() -> Result<PathBuf, String> {
    let current_dir =
        std::env::current_dir().map_err(|error| format!("无法定位当前目录：{error}"))?;
    for ancestor in current_dir.ancestors() {
        let candidate = ancestor.join("profiles");
        if candidate
            .join("common_longform")
            .join("profile.toml")
            .exists()
        {
            return Ok(candidate);
        }
    }
    Ok(current_dir.join("profiles"))
}

fn to_command_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

impl From<Project> for ProjectDto {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
            root_path: project.root_path,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

impl From<ProjectSummary> for ProjectSummaryDto {
    fn from(summary: ProjectSummary) -> Self {
        Self {
            project_id: summary.project_id,
            document_count: summary.document_count,
            chunk_count: summary.chunk_count,
            total_words: summary.total_words,
        }
    }
}

impl From<Dashboard> for DashboardDto {
    fn from(dashboard: Dashboard) -> Self {
        Self {
            summary: ProjectSummaryDto::from(dashboard.summary),
            person_candidates: dashboard.person_candidates,
            place_candidates: dashboard.place_candidates,
            item_candidates: dashboard.item_candidates,
            foreshadow_candidates: dashboard.foreshadow_candidates,
            issue_count: dashboard.issue_count,
        }
    }
}

impl From<ScanReport> for ScanReportDto {
    fn from(report: ScanReport) -> Self {
        Self {
            project_id: report.project_id,
            scanned_documents: report.scanned_documents,
            skipped_files: report.skipped_files,
            summary: ProjectSummaryDto::from(report.summary),
            analysis: AnalysisReportDto {
                person_candidates: report.analysis.person_candidates,
                place_candidates: report.analysis.place_candidates,
                item_candidates: report.analysis.item_candidates,
                foreshadow_candidates: report.analysis.foreshadow_candidates,
                issue_count: report.analysis.issue_count,
            },
        }
    }
}

impl From<SearchHit> for SearchHitDto {
    fn from(hit: SearchHit) -> Self {
        Self {
            document_id: hit.document_id,
            chunk_id: hit.chunk_id,
            document_path: hit.document_path,
            chunk_index: hit.chunk_index,
            title: hit.title,
            snippet: hit.snippet,
            start_offset: hit.start_offset,
            end_offset: hit.end_offset,
        }
    }
}

impl From<NarrativeNode> for NarrativeNodeDto {
    fn from(node: NarrativeNode) -> Self {
        Self {
            id: node.id,
            node_type: node.node_type,
            name: node.name,
            occurrence_count: node.occurrence_count,
            status: node.status,
            confidence: node.confidence,
            source_chunk_id: node.source_chunk_id,
            source_title: node.source_title,
            source_path: node.source_path,
            source_snippet: node.source_snippet,
        }
    }
}

impl From<ForeshadowItem> for ForeshadowItemDto {
    fn from(item: ForeshadowItem) -> Self {
        Self {
            id: item.id,
            title: item.title,
            foreshadow_type: item.foreshadow_type,
            status: item.status,
            risk_level: item.risk_level,
            source_chunk_id: item.source_chunk_id,
            source_title: item.source_title,
            source_path: item.source_path,
            evidence: item.evidence,
        }
    }
}

impl From<ContinuityIssue> for ContinuityIssueDto {
    fn from(issue: ContinuityIssue) -> Self {
        Self {
            id: issue.id,
            issue_type: issue.issue_type,
            severity: issue.severity,
            title: issue.title,
            description: issue.description,
            evidence_json: issue.evidence_json,
            suggested_actions_json: issue.suggested_actions_json,
            status: issue.status,
        }
    }
}

impl From<ContextPack> for ContextPackDto {
    fn from(pack: ContextPack) -> Self {
        Self {
            id: pack.id,
            project_id: pack.project_id,
            title: pack.title,
            target: pack.target,
            content: pack.content,
            format: pack.format,
            source_refs_json: pack.source_refs_json,
            created_at: pack.created_at,
        }
    }
}

impl From<PrivacyStatus> for PrivacyStatusDto {
    fn from(status: PrivacyStatus) -> Self {
        Self {
            offline_mode: status.offline_mode,
            ai_enabled: status.ai_enabled,
            uploads_enabled: status.uploads_enabled,
            clipboard_access: status.clipboard_access,
            screenshot_access: status.screenshot_access,
            keyboard_monitoring: status.keyboard_monitoring,
            database_path: status.database_path,
            storage_mode: status.storage_mode,
        }
    }
}

impl From<ProfileInfo> for ProfileInfoDto {
    fn from(profile: ProfileInfo) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            version: profile.version,
            description: profile.description,
        }
    }
}
