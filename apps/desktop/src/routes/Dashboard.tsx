import {
  AlertTriangle,
  Archive,
  CheckCircle2,
  FileDown,
  FolderOpen,
  Network,
  Plus,
  RefreshCw,
  Search,
  UserRound,
} from "lucide-react";
import { useMemo, useState } from "react";
import { InspectorPanel } from "../components/InspectorPanel";
import { StatusButtons } from "../components/StatusButtons";
import {
  candidateTypeLabel,
  formatError,
  riskLabel,
  severityLabel,
  totalCandidateCount,
} from "../lib/helpers";
import type {
  ContextPack,
  ContinuityIssue,
  Dashboard as DashboardData,
  ForeshadowItem,
  NarrativeNode,
  PrivacyStatus,
  ProfileManifest,
  ScanReport,
} from "../tauri";

type BusyState = "idle" | "loading" | "import" | "scan" | "search" | "context" | "report";

interface DashboardProps {
  projectName: string;
  setProjectName: (name: string) => void;
  folderPath: string;
  setFolderPath: (path: string) => void;
  dashboard: DashboardData;
  candidates: NarrativeNode[];
  foreshadows: ForeshadowItem[];
  issues: ContinuityIssue[];
  contextPack: ContextPack | null;
  lastScan: ScanReport | null;
  busy: BusyState;
  runtimeMode: string;
  hasRealProject: boolean;
  error: string;
  privacy: PrivacyStatus;
  profiles: ProfileManifest[];
  chooseFolder: () => void;
  handleImport: () => void;
  handleScan: () => void;
  handleBuildContextPack: (query: string) => void;
  handleGenerateReport: () => void;
  handleStatus: (kind: "candidate" | "foreshadow" | "issue", id: string, status: string) => void;
}

export function Dashboard(props: DashboardProps) {
  const {
    projectName,
    setProjectName,
    folderPath,
    setFolderPath,
    dashboard,
    candidates,
    foreshadows,
    issues,
    contextPack,
    lastScan,
    busy,
    runtimeMode,
    hasRealProject,
    error,
    privacy,
    profiles,
    chooseFolder,
    handleImport,
    handleScan,
    handleBuildContextPack,
    handleGenerateReport,
    handleStatus,
  } = props;

  const [contextQuery, setContextQuery] = useState("");

  const metrics = useMemo(
    () => [
      { label: "正文文件", value: dashboard.summary.documentCount, suffix: "份" },
      { label: "文本片段", value: dashboard.summary.chunkCount, suffix: "段" },
      { label: "总字数", value: dashboard.summary.totalWords, suffix: "字" },
      { label: "记忆候选", value: totalCandidateCount(dashboard), suffix: "条" },
      { label: "伏笔候选", value: dashboard.foreshadowCandidates, suffix: "条" },
      { label: "待看问题", value: dashboard.issueCount, suffix: "条" },
    ],
    [dashboard],
  );

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel import-panel">
          <div className="panel-heading">
            <div>
              <h2>项目导入与扫描</h2>
              <p>选择小说目录后，novellossless 只扫描该目录内的 TXT 和 Markdown。</p>
            </div>
            <FolderOpen size={22} />
          </div>

          <div className="import-form">
            <label>
              项目名称
              <input
                value={projectName}
                onChange={(event) => setProjectName(event.target.value)}
                placeholder="例如：雨巷钟声"
              />
            </label>
            <label className="path-field">
              小说目录
              <div className="path-row">
                <input
                  value={folderPath}
                  onChange={(event) => setFolderPath(event.target.value)}
                  placeholder="选择或粘贴本地目录"
                />
                <button type="button" className="secondary-button" onClick={chooseFolder}>
                  <FolderOpen size={16} />
                  浏览
                </button>
              </div>
            </label>
          </div>

          <div className="action-row">
            <button
              type="button"
              className="primary-button"
              onClick={handleImport}
              disabled={busy !== "idle" || runtimeMode === "preview"}
            >
              <Plus size={16} />
              {busy === "import" ? "正在导入" : "导入项目"}
            </button>
            <button
              type="button"
              className="secondary-button strong"
              onClick={handleScan}
              disabled={!hasRealProject || busy !== "idle"}
            >
              <RefreshCw size={16} />
              {busy === "scan" ? "正在扫描" : "开始扫描"}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={handleGenerateReport}
              disabled={!hasRealProject || busy !== "idle"}
            >
              <FileDown size={16} />
              导出报告
            </button>
          </div>

          {lastScan && (
            <div className="scan-summary">
              <CheckCircle2 size={16} />
              <span>
                本次读取 {lastScan.scannedDocuments} 份文件，跳过 {lastScan.skippedFiles} 份。
              </span>
            </div>
          )}
        </section>

        <section className="metric-grid six">
          {metrics.map((metric) => (
            <div className="metric-card" key={metric.label}>
              <span>{metric.label}</span>
              <strong>
                {metric.value.toLocaleString()}
                <small>{metric.suffix}</small>
              </strong>
            </div>
          ))}
        </section>

        <section className="p0-grid">
          <section className="panel p0-panel">
            <div className="panel-heading compact">
              <div>
                <h2>记忆候选</h2>
                <p>这些是系统从正文里提到的人物、地点和物件候选，尚不是事实。</p>
              </div>
              <UserRound size={22} />
            </div>
            <div className="compact-list">
              {candidates.length > 0 ? (
                candidates.slice(0, 6).map((candidate) => (
                  <article className="compact-item" key={candidate.id}>
                    <div>
                      <strong>
                        {candidate.name}
                        <span>{candidateTypeLabel(candidate.nodeType)}</span>
                      </strong>
                      <p>
                        出现 {candidate.occurrenceCount} 次 · {candidate.sourcePath} ·{" "}
                        {candidate.sourceTitle}
                      </p>
                    </div>
                    <StatusButtons
                      onConfirm={() => handleStatus("candidate", candidate.id, "confirmed")}
                      onDismiss={() => handleStatus("candidate", candidate.id, "false_positive")}
                    />
                  </article>
                ))
              ) : (
                <div className="empty-state small">扫描后会显示人物、地点和物件候选。</div>
              )}
            </div>
          </section>

          <section className="panel p0-panel">
            <div className="panel-heading compact">
              <div>
                <h2>伏笔候选</h2>
                <p>只记录明显线索或承诺，避免把推测当成结论。</p>
              </div>
              <Network size={22} />
            </div>
            <div className="compact-list">
              {foreshadows.length > 0 ? (
                foreshadows.slice(0, 5).map((item) => (
                  <article className="compact-item" key={item.id}>
                    <div>
                      <strong>{item.title}</strong>
                      <p>
                        {riskLabel(item.riskLevel)} · {item.sourcePath} · {item.evidence}
                      </p>
                    </div>
                    <StatusButtons
                      onConfirm={() => handleStatus("foreshadow", item.id, "confirmed")}
                      onDismiss={() => handleStatus("foreshadow", item.id, "false_positive")}
                    />
                  </article>
                ))
              ) : (
                <div className="empty-state small">扫描后会显示显式线索和承诺。</div>
              )}
            </div>
          </section>

          <section className="panel p0-panel">
            <div className="panel-heading compact">
              <div>
                <h2>基础问题</h2>
                <p>低打扰展示，需要作者确认后才进入修订处理。</p>
              </div>
              <AlertTriangle size={22} />
            </div>
            <div className="compact-list">
              {issues.length > 0 ? (
                issues.slice(0, 5).map((issue) => (
                  <article className="compact-item issue" key={issue.id}>
                    <div>
                      <strong>
                        {issue.title}
                        <span>{severityLabel(issue.severity)}</span>
                      </strong>
                      <p>{issue.description}</p>
                    </div>
                    <StatusButtons
                      onConfirm={() => handleStatus("issue", issue.id, "resolved")}
                      onDismiss={() => handleStatus("issue", issue.id, "false_positive")}
                    />
                  </article>
                ))
              ) : (
                <div className="empty-state small">扫描后会显示重复表达和明确属性冲突。</div>
              )}
            </div>
          </section>

          <section className="panel p0-panel context-panel">
            <div className="panel-heading compact">
              <div>
                <h2>上下文包</h2>
                <p>输入关键词生成 Markdown 上下文，所有内容保留来源。</p>
              </div>
              <Archive size={22} />
            </div>
            <div className="search-row">
              <Search size={18} />
              <input
                value={contextQuery}
                onChange={(e) => setContextQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleBuildContextPack(contextQuery)}
                placeholder="输入人物、地点或物件名称"
              />
              <button
                type="button"
                onClick={() => handleBuildContextPack(contextQuery)}
                disabled={busy !== "idle"}
              >
                {busy === "context" ? "正在生成" : "生成"}
              </button>
            </div>
            {contextPack && <pre className="context-preview">{contextPack.content}</pre>}
          </section>
        </section>
      </div>

      <InspectorPanel
        selectedHit={null}
        issuesCount={issues.length}
        privacy={privacy}
        profiles={profiles}
      />
    </section>
  );
}
