import { invoke } from "@tauri-apps/api/core";

export interface Project {
  id: string;
  name: string;
  rootPath: string;
  createdAt: string;
  updatedAt: string;
}

export interface ProjectSummary {
  projectId: string;
  documentCount: number;
  chunkCount: number;
  totalWords: number;
}

export interface AnalysisReport {
  personCandidates: number;
  placeCandidates: number;
  itemCandidates: number;
  foreshadowCandidates: number;
  issueCount: number;
}

export interface Dashboard {
  summary: ProjectSummary;
  personCandidates: number;
  placeCandidates: number;
  itemCandidates: number;
  foreshadowCandidates: number;
  issueCount: number;
}

export interface ScanReport {
  projectId: string;
  scannedDocuments: number;
  skippedFiles: number;
  summary: ProjectSummary;
  analysis: AnalysisReport;
}

export interface SearchHit {
  documentId: string;
  chunkId: string;
  documentPath: string;
  chunkIndex: number;
  title: string;
  snippet: string;
  startOffset: number;
  endOffset: number;
}

export interface NarrativeNode {
  id: string;
  nodeType: string;
  name: string;
  occurrenceCount: number;
  status: string;
  confidence: number;
  sourceChunkId: string;
  sourceTitle: string;
  sourcePath: string;
  sourceSnippet: string;
}

export interface ForeshadowItem {
  id: string;
  title: string;
  foreshadowType: string;
  status: string;
  riskLevel: string;
  sourceChunkId: string;
  sourceTitle: string;
  sourcePath: string;
  evidence: string;
}

export interface ContinuityIssue {
  id: string;
  issueType: string;
  severity: string;
  title: string;
  description: string;
  evidenceJson: string;
  suggestedActionsJson: string;
  status: string;
}

export interface ContextPack {
  id: string;
  projectId: string;
  title: string;
  target: string;
  content: string;
  format: string;
  sourceRefsJson: string;
  createdAt: string;
}

export interface PrivacyStatus {
  offlineMode: boolean;
  aiEnabled: boolean;
  uploadsEnabled: boolean;
  clipboardAccess: boolean;
  screenshotAccess: boolean;
  keyboardMonitoring: boolean;
  databasePath: string;
  storageMode: string;
}

export interface ProfileManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  enabledByDefault: boolean;
  entityTypes: string[];
  factTypes: string[];
  eventTypes: string[];
  metrics: string[];
  checks: string[];
}

export interface ProfileMetric {
  id: string;
  profileId: string;
  metricType: string;
  documentId: string | null;
  value: string;
  createdAt: string;
}

export interface DocumentInfo {
  id: string;
  path: string;
  title: string;
  chapterCount: number;
  wordCount: number;
}

export interface ChunkInfo {
  id: string;
  documentId: string;
  chunkIndex: number;
  title: string;
  content: string;
  startOffset: number;
  endOffset: number;
  wordCount: number;
}

export interface DocumentTree {
  documents: DocumentInfo[];
  chunks: ChunkInfo[];
}

export class DesktopRuntimeUnavailableError extends Error {
  constructor() {
    super("桌面运行时尚未连接。");
    this.name = "DesktopRuntimeUnavailableError";
  }
}

export function isDesktopRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function ensureDesktopRuntime() {
  if (!isDesktopRuntime()) {
    throw new DesktopRuntimeUnavailableError();
  }
}

export function listProjects() {
  ensureDesktopRuntime();
  return invoke<Project[]>("list_projects");
}

export function importProject(name: string, path: string) {
  ensureDesktopRuntime();
  return invoke<Project>("import_project", { name, path });
}

export function scanProject(projectId: string) {
  ensureDesktopRuntime();
  return invoke<ScanReport>("scan_project", { projectId });
}

export function getDashboard(projectId: string) {
  ensureDesktopRuntime();
  return invoke<Dashboard>("get_dashboard", { projectId });
}

export function searchProject(projectId: string, query: string, limit = 20) {
  ensureDesktopRuntime();
  return invoke<SearchHit[]>("search_project", { projectId, query, limit });
}

export function getProjectSummary(projectId: string) {
  ensureDesktopRuntime();
  return invoke<ProjectSummary>("get_project_summary", { projectId });
}

export function listCandidates(projectId: string, nodeType?: string, limit = 20) {
  ensureDesktopRuntime();
  return invoke<NarrativeNode[]>("list_candidates", { projectId, nodeType, limit });
}

export function listForeshadows(projectId: string, limit = 20) {
  ensureDesktopRuntime();
  return invoke<ForeshadowItem[]>("list_foreshadows", { projectId, limit });
}

export function listIssues(projectId: string, limit = 20, issueTypePrefix?: string) {
  ensureDesktopRuntime();
  return invoke<ContinuityIssue[]>("list_issues", { projectId, limit, issueTypePrefix });
}

export function updateCandidateStatus(id: string, status: string) {
  ensureDesktopRuntime();
  return invoke<void>("update_candidate_status", { id, status });
}

export function updateForeshadowStatus(id: string, status: string) {
  ensureDesktopRuntime();
  return invoke<void>("update_foreshadow_status", { id, status });
}

export function updateIssueStatus(id: string, status: string) {
  ensureDesktopRuntime();
  return invoke<void>("update_issue_status", { id, status });
}

export function buildContextPack(projectId: string, query: string, limit = 10) {
  ensureDesktopRuntime();
  return invoke<ContextPack>("build_context_pack", { projectId, query, limit });
}

export function listContextPacks(projectId: string) {
  ensureDesktopRuntime();
  return invoke<ContextPack[]>("list_context_packs", { projectId });
}

export function getContextPack(id: string) {
  ensureDesktopRuntime();
  return invoke<ContextPack | null>("get_context_pack", { id });
}

export function deleteContextPack(id: string) {
  ensureDesktopRuntime();
  return invoke<void>("delete_context_pack", { id });
}

export function generateMarkdownReport(projectId: string) {
  ensureDesktopRuntime();
  return invoke<ContextPack>("generate_markdown_report", { projectId });
}

export interface TimelineEvent {
  id: string;
  projectId: string;
  chunkId: string;
  chunkIndex: number;
  documentPath: string;
  title: string;
  orderIndex: number;
  timeExpression: string;
  estimatedOrder: number | null;
  participantsJson: string;
  location: string;
  isFlashback: boolean;
  confidence: number;
}

export function listTimelineEvents(projectId: string) {
  ensureDesktopRuntime();
  return invoke<TimelineEvent[]>("list_timeline_events", { projectId });
}

export function getPrivacyStatus() {
  ensureDesktopRuntime();
  return invoke<PrivacyStatus>("get_privacy_status");
}

export function getAvailableProfiles() {
  ensureDesktopRuntime();
  return invoke<ProfileManifest[]>("get_available_profiles");
}

export function getEnabledProfiles(projectId: string) {
  ensureDesktopRuntime();
  return invoke<string[]>("get_enabled_profiles", { projectId });
}

export function setEnabledProfiles(projectId: string, profileIds: string[]) {
  ensureDesktopRuntime();
  return invoke<void>("set_enabled_profiles", { projectId, profileIds });
}

export function getProfileMetrics(projectId: string, profileId: string) {
  ensureDesktopRuntime();
  return invoke<ProfileMetric[]>("get_profile_metrics", { projectId, profileId });
}

export function getDocumentChunks(projectId: string) {
  ensureDesktopRuntime();
  return invoke<DocumentTree>("get_document_chunks", { projectId });
}

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

export function incrementalScan(projectId: string) {
  ensureDesktopRuntime();
  return invoke<ScanResult>("incremental_scan", { projectId });
}

export function listFileScans(projectId: string, limit: number) {
  ensureDesktopRuntime();
  return invoke<FileScanLog[]>("list_file_scans", { projectId, limit });
}

export function listRevisions(projectId: string, documentId: string | null, limit: number) {
  ensureDesktopRuntime();
  return invoke<RevisionRecord[]>("list_revisions", { projectId, documentId, limit });
}

export function startWatching(projectId: string) {
  ensureDesktopRuntime();
  return invoke<void>("start_watching", { projectId });
}

export function stopWatching() {
  ensureDesktopRuntime();
  return invoke<void>("stop_watching");
}

export function watcherStatus() {
  ensureDesktopRuntime();
  return invoke<boolean>("watcher_status");
}

export interface BackupInfo {
  path: string;
  fileName: string;
  sizeBytes: number;
  createdAt: string;
}

export interface ScanRun {
  id: string;
  projectId: string;
  status: string;
  totalFiles: number;
  scannedFiles: number;
  createdAt: string;
}

export interface Setting {
  key: string;
  value: string;
}

export function getSettings() {
  ensureDesktopRuntime();
  return invoke<Setting[]>("get_settings");
}

export function updateSetting(key: string, value: string) {
  ensureDesktopRuntime();
  return invoke<void>("update_setting", { key, value });
}

export function backupDatabase() {
  ensureDesktopRuntime();
  return invoke<string>("backup_database");
}

export function restoreDatabase(sourcePath: string) {
  ensureDesktopRuntime();
  return invoke<void>("restore_database", { sourcePath });
}

export function initDemoProject() {
  ensureDesktopRuntime();
  return invoke<Project>("init_demo_project");
}

export function listBackups() {
  ensureDesktopRuntime();
  return invoke<BackupInfo[]>("list_backups");
}

export function deleteProject(projectId: string) {
  ensureDesktopRuntime();
  return invoke<void>("delete_project", { projectId });
}

export function getIncompleteScanRun(projectId: string) {
  ensureDesktopRuntime();
  return invoke<ScanRun | null>("get_incomplete_scan_run", { projectId });
}

export function resumeScan(projectId: string) {
  ensureDesktopRuntime();
  return invoke<ScanReport>("resume_scan", { projectId });
}

export interface KnowledgePackInfo {
  name: string;
  packType: string;
  entryCount: number;
  dynasties: string[];
  source: string;
}

export function listKnowledgePacks() {
  ensureDesktopRuntime();
  return invoke<KnowledgePackInfo[]>("list_knowledge_packs");
}

export function importKnowledgePack(sourcePath: string) {
  ensureDesktopRuntime();
  return invoke<string>("import_knowledge_pack", { sourcePath });
}

export function testAiConnection() {
  ensureDesktopRuntime();
  return invoke<string>("test_ai_connection");
}

export function reloadAiProvider() {
  ensureDesktopRuntime();
  return invoke("reload_ai_provider");
}

export function createTask(
  projectId: string,
  title: string,
  taskType: string,
  priority: string,
  relatedChunksJson: string,
  notes: string,
) {
  ensureDesktopRuntime();
  return invoke<string>("create_task", {
    projectId,
    title,
    taskType,
    priority,
    relatedChunksJson,
    notes,
  });
}
