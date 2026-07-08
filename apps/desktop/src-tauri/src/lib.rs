use std::path::{Path, PathBuf};
use std::sync::Mutex;

use novellossless_core::{
    Dashboard, DocumentTree, NovelCore, PrivacyStatus, ProgressReporter, ScanReport,
};
use novellossless_storage::{
    ContextPack, ContinuityIssue, FileScanLog, ForeshadowItem, NarrativeNode, Project,
    ProjectChunk, ProjectSummary, RevisionRecord, SearchHit,
};
use serde::Serialize;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

mod watcher;

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
struct ProfileManifestDto {
    id: String,
    name: String,
    version: String,
    description: String,
    enabled_by_default: bool,
    entity_types: Vec<String>,
    fact_types: Vec<String>,
    event_types: Vec<String>,
    metrics: Vec<String>,
    checks: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileMetricDto {
    id: String,
    profile_id: String,
    metric_type: String,
    document_id: Option<String>,
    value: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgePackDto {
    id: String,
    profile_id: String,
    pack_name: String,
    pack_type: String,
    entries: Vec<KnowledgeItemDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeItemDto {
    term: String,
    category: String,
    rank: String,
    note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingDto {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentDto {
    id: String,
    path: String,
    title: String,
    chapter_count: i64,
    word_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChunkDto {
    id: String,
    document_id: String,
    chunk_index: i64,
    title: String,
    content: String,
    start_offset: i64,
    word_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentTreeDto {
    documents: Vec<DocumentDto>,
    chunks: Vec<ChunkDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResultDto {
    scanned_documents: usize,
    skipped_files: usize,
    created: usize,
    modified: usize,
    unchanged: usize,
    deleted: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileScanLogDto {
    id: String,
    project_id: String,
    document_id: String,
    old_hash: Option<String>,
    new_hash: String,
    event_type: String,
    scanned_at: String,
    details: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RevisionRecordDto {
    id: String,
    project_id: String,
    document_id: String,
    revision_type: String,
    old_content_hash: Option<String>,
    new_content_hash: String,
    old_chunk_count: i64,
    new_chunk_count: i64,
    chunks_added: i64,
    chunks_removed: i64,
    chunks_modified: i64,
    diff_json: Option<String>,
    created_at: String,
}

struct WatcherState(Mutex<Option<crate::watcher::FileWatcher>>);

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
    let progress = TauriProgressReporter { app };
    core.scan_project_with_progress(&project_id, &progress)
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
fn get_available_profiles(app: tauri::AppHandle) -> Result<Vec<ProfileManifestDto>, String> {
    let core = open_core(&app)?;
    let manifests = core.get_available_profiles().map_err(to_command_error)?;
    Ok(manifests
        .into_iter()
        .map(|m| ProfileManifestDto {
            id: m.id,
            name: m.name,
            version: m.version,
            description: m.description,
            enabled_by_default: m.enabled_by_default.unwrap_or(false),
            entity_types: m.entities.types,
            fact_types: m.facts.types,
            event_types: m.events.types,
            metrics: m.metrics.enabled,
            checks: m.checks.enabled,
        })
        .collect())
}

#[tauri::command]
fn get_enabled_profiles(app: tauri::AppHandle, project_id: String) -> Result<Vec<String>, String> {
    let core = open_core(&app)?;
    core.get_enabled_profiles(&project_id)
        .map_err(to_command_error)
}

#[tauri::command]
fn set_enabled_profiles(
    app: tauri::AppHandle,
    project_id: String,
    profile_ids: Vec<String>,
) -> Result<(), String> {
    let core = open_core(&app)?;
    let ids: Vec<&str> = profile_ids.iter().map(|s| s.as_str()).collect();
    core.set_enabled_profiles(&project_id, &ids)
        .map_err(to_command_error)
}

#[tauri::command]
fn get_profile_metrics(
    app: tauri::AppHandle,
    project_id: String,
    profile_id: String,
) -> Result<Vec<ProfileMetricDto>, String> {
    let core = open_core(&app)?;
    core.get_profile_metrics(&project_id, &profile_id)
        .map(|metrics| {
            metrics
                .into_iter()
                .map(|m| ProfileMetricDto {
                    id: m.id,
                    profile_id: m.profile_id,
                    metric_type: m.metric_type,
                    document_id: m.document_id,
                    value: m.value,
                    created_at: m.created_at,
                })
                .collect()
        })
        .map_err(to_command_error)
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<Vec<SettingDto>, String> {
    let core = open_core(&app)?;
    Ok(core
        .get_settings()
        .into_iter()
        .map(|(k, v)| SettingDto { key: k, value: v })
        .collect())
}

#[tauri::command]
fn update_setting(app: tauri::AppHandle, key: String, value: String) -> Result<(), String> {
    let core = open_core(&app)?;
    core.update_setting(&key, &value).map_err(to_command_error)
}

#[tauri::command]
fn get_document_chunks(
    app: tauri::AppHandle,
    project_id: String,
) -> Result<DocumentTreeDto, String> {
    let core = open_core(&app)?;
    core.document_tree(&project_id, None)
        .map(DocumentTreeDto::from)
        .map_err(to_command_error)
}

#[tauri::command]
fn incremental_scan(app: tauri::AppHandle, project_id: String) -> Result<ScanResultDto, String> {
    let core = open_core(&app)?;
    let progress = TauriProgressReporter { app };
    core.incremental_scan_with_progress(&project_id, &progress)
        .map(ScanResultDto::from)
        .map_err(to_command_error)
}

#[tauri::command]
fn list_file_scans(
    app: tauri::AppHandle,
    project_id: String,
    limit: i64,
) -> Result<Vec<FileScanLogDto>, String> {
    let core = open_core(&app)?;
    core.list_file_scans(&project_id, limit)
        .map(|v| v.into_iter().map(FileScanLogDto::from).collect())
        .map_err(to_command_error)
}

#[tauri::command]
fn list_revisions(
    app: tauri::AppHandle,
    project_id: String,
    document_id: Option<String>,
    limit: i64,
) -> Result<Vec<RevisionRecordDto>, String> {
    let core = open_core(&app)?;
    core.list_revisions(&project_id, document_id.as_deref(), limit)
        .map(|v| v.into_iter().map(RevisionRecordDto::from).collect())
        .map_err(to_command_error)
}

#[tauri::command]
fn start_watching(app: tauri::AppHandle, project_id: String) -> Result<(), String> {
    let core = open_core(&app)?;
    let project = core
        .get_project(&project_id)
        .map_err(to_command_error)?
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
            if let Some(mut old) = w.replace(watcher) {
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
    if let Some(state) = app.try_state::<WatcherState>() {
        if let Ok(guard) = state.0.lock() {
            return Ok(guard.is_some());
        }
    }
    Ok(false)
}

#[tauri::command]
fn backup_database(app: tauri::AppHandle) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let db_path = database_path(&app)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_name = format!("novellossless-backup-{ts}.db");
    let dest = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法定位数据目录：{e}"))?
        .join(&backup_name);
    std::fs::copy(&db_path, &dest).map_err(|e| format!("备份失败：{e}"))?;
    Ok(dest.display().to_string())
}

#[tauri::command]
fn restore_database(app: tauri::AppHandle, source_path: String) -> Result<(), String> {
    let db_path = database_path(&app)?;
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err("备份文件不存在".to_string());
    }
    std::fs::copy(&source, &db_path).map_err(|e| format!("恢复失败：{e}"))?;
    Ok(())
}

#[tauri::command]
fn init_demo_project(app: tauri::AppHandle) -> Result<ProjectDto, String> {
    let core = open_core(&app)?;
    let demo_src = find_demo_root()?;
    let demo_dir = std::env::temp_dir().join(format!(
        "novellossless-demo-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ));
    std::fs::create_dir_all(&demo_dir).map_err(|e| format!("无法创建示例目录：{e}"))?;
    for entry in std::fs::read_dir(&demo_src).map_err(|e| format!("无法读取示例源：{e}"))? {
        let entry = entry.map_err(|e| format!("读取条目失败：{e}"))?;
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            let dest = demo_dir.join(entry.file_name());
            std::fs::copy(entry.path(), &dest).map_err(|e| format!("复制文件失败：{e}"))?;
        }
    }
    let project = core
        .import_project("三国演义（示例）", &demo_dir)
        .map_err(to_command_error)?;
    let _ = core.scan_project(&project.id);
    Ok(ProjectDto::from(project))
}

fn find_demo_root() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir().map_err(|e| format!("无法定位当前目录：{e}"))?;
    for ancestor in current_dir.ancestors() {
        let candidate = ancestor.join("profiles").join("demo");
        if candidate.join("001-桃园结义.txt").exists() {
            return Ok(candidate);
        }
    }
    Err("未找到示例项目文件 (profiles/demo/)".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(WatcherState(Mutex::new(None)))
        .setup(|app| {
            let show = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show)
                .separator()
                .item(&quit)
                .build()?;
            TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("novellossless")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
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
            get_available_profiles,
            get_enabled_profiles,
            set_enabled_profiles,
            get_profile_metrics,
            get_settings,
            update_setting,
            get_document_chunks,
            incremental_scan,
            list_file_scans,
            list_revisions,
            start_watching,
            stop_watching,
            watcher_status,
            backup_database,
            restore_database,
            init_demo_project
        ])
        .run(tauri::generate_context!())
        .expect("failed to run novellossless desktop app");
}

struct TauriProgressReporter {
    app: tauri::AppHandle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgressPayload {
    current: usize,
    total: usize,
    file: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanErrorPayload {
    file: String,
    message: String,
}

impl ProgressReporter for TauriProgressReporter {
    fn report(&self, current: usize, total: usize, file: &str) {
        let _ = self.app.emit(
            "scan-progress",
            ScanProgressPayload {
                current,
                total,
                file: file.to_string(),
            },
        );
    }

    fn error(&self, file: &str, message: &str) {
        let _ = self.app.emit(
            "scan-error",
            ScanErrorPayload {
                file: file.to_string(),
                message: message.to_string(),
            },
        );
    }
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

impl From<DocumentTree> for DocumentTreeDto {
    fn from(tree: DocumentTree) -> Self {
        Self {
            documents: tree.documents.into_iter().map(DocumentDto::from).collect(),
            chunks: tree.chunks.into_iter().map(ChunkDto::from).collect(),
        }
    }
}

impl From<novellossless_core::DocumentInfo> for DocumentDto {
    fn from(info: novellossless_core::DocumentInfo) -> Self {
        Self {
            id: info.id,
            path: info.path,
            title: info.title,
            chapter_count: info.chapter_count,
            word_count: info.word_count,
        }
    }
}

impl From<ProjectChunk> for ChunkDto {
    fn from(chunk: ProjectChunk) -> Self {
        Self {
            id: chunk.chunk_id,
            document_id: chunk.document_id,
            chunk_index: chunk.chunk_index,
            title: chunk.title,
            content: chunk.content,
            start_offset: chunk.start_offset,
            word_count: chunk.word_count,
        }
    }
}

impl From<novellossless_core::ScanResult> for ScanResultDto {
    fn from(r: novellossless_core::ScanResult) -> Self {
        Self {
            scanned_documents: r.scanned_documents,
            skipped_files: r.skipped_files,
            created: r.created,
            modified: r.modified,
            unchanged: r.unchanged,
            deleted: r.deleted,
            failed: r.failed,
        }
    }
}

impl From<FileScanLog> for FileScanLogDto {
    fn from(l: FileScanLog) -> Self {
        Self {
            id: l.id,
            project_id: l.project_id,
            document_id: l.document_id,
            old_hash: l.old_hash,
            new_hash: l.new_hash,
            event_type: l.event_type,
            scanned_at: l.scanned_at,
            details: l.details,
        }
    }
}

impl From<RevisionRecord> for RevisionRecordDto {
    fn from(r: RevisionRecord) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            document_id: r.document_id,
            revision_type: r.revision_type,
            old_content_hash: r.old_content_hash,
            new_content_hash: r.new_content_hash,
            old_chunk_count: r.old_chunk_count,
            new_chunk_count: r.new_chunk_count,
            chunks_added: r.chunks_added,
            chunks_removed: r.chunks_removed,
            chunks_modified: r.chunks_modified,
            diff_json: r.diff_json,
            created_at: r.created_at,
        }
    }
}
