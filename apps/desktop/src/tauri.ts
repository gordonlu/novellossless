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

export interface ProfileInfo {
  id: string;
  name: string;
  version: string;
  description: string;
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

export function listIssues(projectId: string, limit = 20) {
  ensureDesktopRuntime();
  return invoke<ContinuityIssue[]>("list_issues", { projectId, limit });
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

export function getPrivacyStatus() {
  ensureDesktopRuntime();
  return invoke<PrivacyStatus>("get_privacy_status");
}

export function listProfiles() {
  ensureDesktopRuntime();
  return invoke<ProfileInfo[]>("list_profiles");
}
