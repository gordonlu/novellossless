import {
  AlertTriangle,
  Archive,
  BookOpenText,
  CheckCircle2,
  ChevronRight,
  Clock3,
  Database,
  FileSearch,
  FolderOpen,
  Home,
  ListChecks,
  LockKeyhole,
  Network,
  Plus,
  RefreshCw,
  Search,
  ShieldCheck,
  Sparkles,
  UserRound,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { clsx } from "clsx";
import { useEffect, useMemo, useState } from "react";
import {
  buildContextPack,
  ContinuityIssue,
  ContextPack,
  Dashboard,
  DesktopRuntimeUnavailableError,
  ForeshadowItem,
  getDashboard,
  getPrivacyStatus,
  importProject,
  isDesktopRuntime,
  listCandidates,
  listForeshadows,
  listIssues,
  listProfiles,
  listProjects,
  NarrativeNode,
  PrivacyStatus,
  ProfileInfo,
  Project,
  ProjectSummary,
  scanProject,
  ScanReport,
  searchProject,
  SearchHit,
  updateCandidateStatus,
  updateForeshadowStatus,
  updateIssueStatus,
} from "./tauri";

const demoProject: Project = {
  id: "demo",
  name: "雨巷钟声",
  rootPath: "未导入项目",
  createdAt: "",
  updatedAt: "",
};

const navigation = [
  { label: "项目首页", icon: Home, active: true },
  { label: "正文", icon: BookOpenText },
  { label: "搜索", icon: Search },
  { label: "人物", icon: UserRound },
  { label: "伏笔", icon: Network },
  { label: "时间线", icon: Clock3 },
  { label: "冲突报告", icon: AlertTriangle },
  { label: "上下文包", icon: Archive },
  { label: "隐私中心", icon: LockKeyhole },
];

const emptySummary: ProjectSummary = {
  projectId: "demo",
  documentCount: 0,
  chunkCount: 0,
  totalWords: 0,
};

const emptyDashboard: Dashboard = {
  summary: emptySummary,
  personCandidates: 0,
  placeCandidates: 0,
  itemCandidates: 0,
  foreshadowCandidates: 0,
  issueCount: 0,
};

const sampleHits: SearchHit[] = [
  {
    documentId: "sample-1",
    chunkId: "sample-1",
    documentPath: "001-雨夜.txt",
    chunkIndex: 0,
    title: "第一章 雨夜",
    snippet: "林澈在[雨夜]里醒来，远处旧钟楼传来三声回响。",
    startOffset: 0,
    endOffset: 28,
  },
  {
    documentId: "sample-2",
    chunkId: "sample-2",
    documentPath: "002-伞下的人.txt",
    chunkIndex: 0,
    title: "第二章 伞下的人",
    snippet: "她说自己从未见过那枚铜钥匙，可[雨夜]的证词并不一致。",
    startOffset: 0,
    endOffset: 31,
  },
];

const sampleCandidates: NarrativeNode[] = [
  {
    id: "sample-person-1",
    nodeType: "person",
    name: "林澈",
    occurrenceCount: 3,
    status: "candidate",
    confidence: 80,
    sourceChunkId: "sample-1",
    sourceTitle: "第一章 雨夜",
    sourcePath: "001-雨夜.txt",
    sourceSnippet: "林澈在雨夜里醒来，远处旧钟楼传来三声回响。",
  },
  {
    id: "sample-item-1",
    nodeType: "item",
    name: "铜钥匙",
    occurrenceCount: 2,
    status: "candidate",
    confidence: 70,
    sourceChunkId: "sample-2",
    sourceTitle: "第二章 伞下的人",
    sourcePath: "002-伞下的人.txt",
    sourceSnippet: "她说自己从未见过那枚铜钥匙，可雨夜的证词并不一致。",
  },
];

const sampleForeshadows: ForeshadowItem[] = [
  {
    id: "sample-foreshadow-1",
    title: "她说自己从未见过那枚铜钥匙",
    foreshadowType: "explicit_clue",
    status: "candidate",
    riskLevel: "medium",
    sourceChunkId: "sample-2",
    sourceTitle: "第二章 伞下的人",
    sourcePath: "002-伞下的人.txt",
    evidence: "她说自己从未见过那枚铜钥匙，可雨夜的证词并不一致。",
  },
];

const sampleIssues: ContinuityIssue[] = [
  {
    id: "sample-issue-1",
    issueType: "repeat_expression",
    severity: "low",
    title: "“雨夜”反复出现",
    description: "“雨夜”在多个正文片段中重复出现，可在修订时确认是否有意保留。",
    evidenceJson: "[]",
    suggestedActionsJson: "[]",
    status: "open",
  },
];

const previewPrivacy: PrivacyStatus = {
  offlineMode: true,
  aiEnabled: false,
  uploadsEnabled: false,
  clipboardAccess: false,
  screenshotAccess: false,
  keyboardMonitoring: false,
  databasePath: "桌面应用中显示本地数据库路径",
  storageMode: "标准本地模式",
};

const previewProfiles: ProfileInfo[] = [
  {
    id: "common_longform",
    name: "通用长篇",
    version: "0.1.0",
    description: "章节识别、全文搜索、候选记忆和来源证据。",
  },
];

type BusyState = "idle" | "loading" | "import" | "scan" | "search" | "context";
type RuntimeMode = "desktop" | "preview";

export function App() {
  const [runtimeMode, setRuntimeMode] = useState<RuntimeMode>(
    isDesktopRuntime() ? "desktop" : "preview",
  );
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState<Project>(demoProject);
  const [dashboard, setDashboard] = useState<Dashboard>(emptyDashboard);
  const [lastScan, setLastScan] = useState<ScanReport | null>(null);
  const [folderPath, setFolderPath] = useState("");
  const [projectName, setProjectName] = useState("");
  const [query, setQuery] = useState("雨夜");
  const [hits, setHits] = useState<SearchHit[]>(sampleHits);
  const [selectedHit, setSelectedHit] = useState<SearchHit | null>(sampleHits[0]);
  const [candidates, setCandidates] = useState<NarrativeNode[]>(sampleCandidates);
  const [foreshadows, setForeshadows] = useState<ForeshadowItem[]>(sampleForeshadows);
  const [issues, setIssues] = useState<ContinuityIssue[]>(sampleIssues);
  const [privacy, setPrivacy] = useState<PrivacyStatus>(previewPrivacy);
  const [profiles, setProfiles] = useState<ProfileInfo[]>(previewProfiles);
  const [contextPack, setContextPack] = useState<ContextPack | null>(null);
  const [searchAttempted, setSearchAttempted] = useState(false);
  const [busy, setBusy] = useState<BusyState>("loading");
  const [notice, setNotice] = useState("离线模式已开启，正文仅保存在本机。");
  const [error, setError] = useState("");

  const hasRealProject = selectedProject.id !== "demo";

  useEffect(() => {
    let canceled = false;

    async function loadProjects() {
      setBusy("loading");
      try {
        const items = await listProjects();
        if (canceled) {
          return;
        }
        setRuntimeMode("desktop");
        setProjects(items);
        await loadGlobalP0();
        if (items.length > 0) {
          await selectProject(items[0], { quiet: true });
        } else {
          resetToEmptyProject();
          setNotice("尚未导入项目。选择小说目录后即可建立本地索引。");
        }
      } catch (reason) {
        if (canceled) {
          return;
        }
        if (reason instanceof DesktopRuntimeUnavailableError) {
          setRuntimeMode("preview");
          setNotice("桌面命令尚未连接，当前显示界面预览数据。");
          applyPreviewData();
        } else {
          setError(formatError(reason));
        }
      } finally {
        if (!canceled) {
          setBusy("idle");
        }
      }
    }

    loadProjects();

    return () => {
      canceled = true;
    };
  }, []);

  const projectTitle = selectedProject.name || "未命名项目";
  const statusLabel = hasRealProject ? "本地项目" : runtimeMode === "preview" ? "界面预览" : "等待导入";
  const selectedRootLabel = hasRealProject ? basename(selectedProject.rootPath) : "尚未选择目录";
  const scanReady = hasRealProject;

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

  async function loadGlobalP0() {
    const [privacyStatus, profileItems] = await Promise.all([getPrivacyStatus(), listProfiles()]);
    setPrivacy(privacyStatus);
    setProfiles(profileItems);
  }

  async function loadProjectP0(projectId: string) {
    const [nextDashboard, nextCandidates, nextForeshadows, nextIssues] = await Promise.all([
      getDashboard(projectId),
      listCandidates(projectId, undefined, 30),
      listForeshadows(projectId, 20),
      listIssues(projectId, 20),
    ]);
    setDashboard(nextDashboard);
    setCandidates(nextCandidates);
    setForeshadows(nextForeshadows);
    setIssues(nextIssues);
  }

  async function selectProject(project: Project, options?: { quiet?: boolean }) {
    setSelectedProject(project);
    setProjectName(project.name);
    setFolderPath("");
    setLastScan(null);
    setHits(project.id === "demo" ? sampleHits : []);
    setSelectedHit(project.id === "demo" ? sampleHits[0] : null);
    setContextPack(null);
    setSearchAttempted(false);

    if (project.id === "demo") {
      applyPreviewData();
    } else {
      await loadProjectP0(project.id);
    }

    if (!options?.quiet) {
      setNotice(`已切换到《${project.name}》。`);
    }
  }

  function applyPreviewData() {
    setDashboard({
      summary: emptySummary,
      personCandidates: 1,
      placeCandidates: 0,
      itemCandidates: 1,
      foreshadowCandidates: sampleForeshadows.length,
      issueCount: sampleIssues.length,
    });
    setHits(sampleHits);
    setSelectedHit(sampleHits[0]);
    setCandidates(sampleCandidates);
    setForeshadows(sampleForeshadows);
    setIssues(sampleIssues);
    setPrivacy(previewPrivacy);
    setProfiles(previewProfiles);
  }

  function resetToEmptyProject() {
    setSelectedProject(demoProject);
    setDashboard(emptyDashboard);
    setHits([]);
    setSelectedHit(null);
    setCandidates([]);
    setForeshadows([]);
    setIssues([]);
    setContextPack(null);
    setSearchAttempted(false);
  }

  async function refreshProjects() {
    if (runtimeMode === "preview") {
      setNotice("当前是界面预览。启动 Tauri 桌面应用后可读取真实项目。");
      return;
    }

    setBusy("loading");
    setError("");
    try {
      const items = await listProjects();
      setProjects(items);
      await loadGlobalP0();
      const current = items.find((item) => item.id === selectedProject.id);
      if (current) {
        setSelectedProject(current);
        await loadProjectP0(current.id);
      } else if (items[0]) {
        await selectProject(items[0]);
      } else {
        resetToEmptyProject();
        setNotice("尚未导入项目。选择小说目录后即可建立本地索引。");
      }
    } catch (reason) {
      setError(formatError(reason));
    } finally {
      setBusy("idle");
    }
  }

  async function chooseFolder() {
    setError("");
    if (runtimeMode === "preview") {
      setError("浏览器预览不能打开系统目录选择器。请在桌面应用中选择目录。");
      return;
    }

    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setFolderPath(selected);
      if (!projectName) {
        setProjectName(basename(selected) || "新小说项目");
      }
    }
  }

  async function handleImport() {
    if (!folderPath.trim()) {
      setError("请先选择小说项目目录。");
      return;
    }
    setBusy("import");
    setError("");
    try {
      const project = await importProject(projectName.trim() || "新小说项目", folderPath.trim());
      setSelectedProject(project);
      setProjects((items) => [project, ...items.filter((item) => item.id !== project.id)]);
      setHits([]);
      setSelectedHit(null);
      setContextPack(null);
      setSearchAttempted(false);
      await loadProjectP0(project.id);
      setNotice("项目已导入，可以开始本地扫描。");
    } catch (reason) {
      setError(formatError(reason));
    } finally {
      setBusy("idle");
    }
  }

  async function handleScan() {
    if (!hasRealProject) {
      setError("请先导入项目再扫描。");
      return;
    }
    setBusy("scan");
    setError("");
    try {
      const report = await scanProject(selectedProject.id);
      setLastScan(report);
      setDashboard({
        summary: report.summary,
        personCandidates: report.analysis.personCandidates,
        placeCandidates: report.analysis.placeCandidates,
        itemCandidates: report.analysis.itemCandidates,
        foreshadowCandidates: report.analysis.foreshadowCandidates,
        issueCount: report.analysis.issueCount,
      });
      await loadProjectLists(selectedProject.id);
      setNotice(
        `扫描完成：读取 ${report.scannedDocuments} 份文件，跳过 ${report.skippedFiles} 份文件。`,
      );
      await runSearch(query, selectedProject.id);
    } catch (reason) {
      setError(formatError(reason));
    } finally {
      setBusy("idle");
    }
  }

  async function loadProjectLists(projectId: string) {
    const [nextCandidates, nextForeshadows, nextIssues] = await Promise.all([
      listCandidates(projectId, undefined, 30),
      listForeshadows(projectId, 20),
      listIssues(projectId, 20),
    ]);
    setCandidates(nextCandidates);
    setForeshadows(nextForeshadows);
    setIssues(nextIssues);
  }

  async function runSearch(nextQuery = query, projectId = selectedProject.id) {
    const trimmedQuery = nextQuery.trim();
    if (!trimmedQuery) {
      setError("请输入要搜索的文字。");
      return;
    }

    if (!hasRealProject) {
      const previewHits = sampleHits.filter(
        (hit) => hit.title.includes(trimmedQuery) || plainSnippet(hit.snippet).includes(trimmedQuery),
      );
      setHits(previewHits);
      setSelectedHit(previewHits[0] ?? null);
      setSearchAttempted(true);
      setNotice(
        previewHits.length > 0
          ? `预览数据中找到 ${previewHits.length} 条来源片段。`
          : "预览数据中没有匹配片段。",
      );
      return;
    }

    setBusy("search");
    setError("");
    setSearchAttempted(true);
    try {
      const results = await searchProject(projectId, trimmedQuery, 20);
      setHits(results);
      setSelectedHit(results[0] ?? null);
      setNotice(results.length > 0 ? `找到 ${results.length} 条来源片段。` : "没有找到匹配片段。");
    } catch (reason) {
      setError(formatError(reason));
    } finally {
      setBusy("idle");
    }
  }

  async function handleBuildContextPack() {
    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
      setError("请输入上下文包关键词。");
      return;
    }

    setBusy("context");
    setError("");
    try {
      if (!hasRealProject) {
        const content = `# 上下文包：${trimmedQuery}\n\n> 当前是界面预览数据。\n\n## 1. 第二章 伞下的人\n\n- 来源文件：002-伞下的人.txt\n- 片段：第 1 段\n- 位置：0-31\n\n她说自己从未见过那枚铜钥匙，可雨夜的证词并不一致。\n`;
        setContextPack({
          id: "preview-context",
          projectId: "demo",
          title: `上下文包：${trimmedQuery}`,
          target: trimmedQuery,
          content,
          format: "markdown",
          sourceRefsJson: "[]",
          createdAt: "",
        });
        setNotice("已生成预览上下文包。");
        return;
      }

      const pack = await buildContextPack(selectedProject.id, trimmedQuery, 10);
      setContextPack(pack);
      setNotice(`已生成《${pack.title}》。`);
    } catch (reason) {
      setError(formatError(reason));
    } finally {
      setBusy("idle");
    }
  }

  async function handleStatus(kind: "candidate" | "foreshadow" | "issue", id: string, status: string) {
    if (runtimeMode === "preview") {
      applyLocalStatus(kind, id, status);
      setNotice("预览状态已更新。");
      return;
    }

    setError("");
    try {
      if (kind === "candidate") {
        await updateCandidateStatus(id, status);
      } else if (kind === "foreshadow") {
        await updateForeshadowStatus(id, status);
      } else {
        await updateIssueStatus(id, status);
      }
      applyLocalStatus(kind, id, status);
      setNotice("状态已保存。");
    } catch (reason) {
      setError(formatError(reason));
    }
  }

  function applyLocalStatus(kind: "candidate" | "foreshadow" | "issue", id: string, status: string) {
    if (kind === "candidate") {
      setCandidates((items) => items.map((item) => (item.id === id ? { ...item, status } : item)));
    } else if (kind === "foreshadow") {
      setForeshadows((items) => items.map((item) => (item.id === id ? { ...item, status } : item)));
    } else {
      setIssues((items) => items.map((item) => (item.id === id ? { ...item, status } : item)));
    }
  }

  function revealSelectedSource() {
    if (!selectedHit) {
      return;
    }

    setNotice(
      `来源定位：${selectedHit.documentPath}，第 ${selectedHit.chunkIndex + 1} 段，位置 ${selectedHit.startOffset}-${selectedHit.endOffset}。`,
    );
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">n</div>
          <div>
            <div className="brand-name">novellossless</div>
            <div className="brand-subtitle">本地创作记忆</div>
          </div>
        </div>

        <nav className="nav-list">
          {navigation.map((item) => (
            <button
              type="button"
              className={clsx("nav-item", item.active && "nav-item-active")}
              key={item.label}
            >
              <item.icon size={17} />
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <section className="project-switcher" aria-label="最近项目">
          <div className="sidebar-section-title">最近项目</div>
          {projects.length > 0 ? (
            projects.map((project) => (
              <button
                type="button"
                key={project.id}
                className={clsx("project-option", selectedProject.id === project.id && "project-option-active")}
                onClick={() => selectProject(project)}
              >
                <span>{project.name}</span>
                <small>{basename(project.rootPath)}</small>
              </button>
            ))
          ) : (
            <div className="project-empty">
              {runtimeMode === "preview" ? "桌面应用中会显示真实项目" : "导入后会显示在这里"}
            </div>
          )}
        </section>

        <div className="privacy-box">
          <ShieldCheck size={18} />
          <div>
            <strong>离线可用</strong>
            <p>不登录、不上传正文、不调用 AI。</p>
          </div>
        </div>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <div className="crumb">项目首页</div>
            <h1>《{projectTitle}》</h1>
            <div className="project-meta">根目录：{selectedRootLabel}</div>
          </div>
          <div className="topbar-actions">
            <span className="status-label">
              <CheckCircle2 size={15} />
              {statusLabel}
            </span>
            <button
              className="icon-button"
              type="button"
              title="刷新项目"
              onClick={refreshProjects}
              disabled={busy !== "idle"}
            >
              <RefreshCw size={18} />
            </button>
          </div>
        </header>

        <section className="notice-row">
          <div className="notice">
            <Database size={18} />
            <span>{notice}</span>
          </div>
          {error && <div className="error">{error}</div>}
        </section>

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
                  disabled={!scanReady || busy !== "idle"}
                >
                  <RefreshCw size={16} />
                  {busy === "scan" ? "正在扫描" : "开始扫描"}
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

            <section className="panel search-panel">
              <div className="panel-heading compact">
                <div>
                  <h2>全文搜索</h2>
                  <p>搜索正文片段，并保留可回溯的章节证据。</p>
                </div>
                <FileSearch size={22} />
              </div>

              <div className="search-row">
                <Search size={18} />
                <input
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      runSearch();
                    }
                  }}
                  placeholder="输入人物、物件、地点或句子"
                />
                <button type="button" onClick={() => runSearch()} disabled={busy !== "idle"}>
                  {busy === "search" ? "搜索中" : "搜索"}
                </button>
              </div>

              <div className="results-list">
                {hits.length > 0 ? (
                  hits.map((hit) => (
                    <button
                      type="button"
                      key={hit.chunkId}
                      className={clsx(
                        "result-item",
                        selectedHit?.chunkId === hit.chunkId && "result-item-active",
                      )}
                      onClick={() => setSelectedHit(hit)}
                    >
                      <div>
                        <strong>{hit.title}</strong>
                        <span className="result-meta">{sourceLabel(hit)}</span>
                        <p>{renderSnippet(hit.snippet)}</p>
                      </div>
                      <ChevronRight size={17} />
                    </button>
                  ))
                ) : (
                  <div className="empty-state">
                    <strong>{searchAttempted ? "没有找到匹配片段" : "等待搜索"}</strong>
                    <p>
                      {hasRealProject
                        ? "输入人物、物件、地点或原文句子后，可在右侧查看来源证据。"
                        : "导入项目后会搜索真实正文；当前仅用于界面预览。"}
                    </p>
                  </div>
                )}
              </div>
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
                    <p>根据当前搜索词生成 Markdown，所有内容保留来源。</p>
                  </div>
                  <Archive size={22} />
                </div>
                <button
                  type="button"
                  className="secondary-button strong"
                  onClick={handleBuildContextPack}
                  disabled={busy !== "idle"}
                >
                  <Archive size={16} />
                  {busy === "context" ? "正在生成" : "生成上下文包"}
                </button>
                {contextPack && <pre className="context-preview">{contextPack.content}</pre>}
              </section>
            </section>
          </div>

          <aside className="inspector">
            <section className="panel evidence-panel">
              <div className="panel-heading compact">
                <div>
                  <h2>来源证据</h2>
                  <p>所有提醒都必须能回到正文。</p>
                </div>
                <ListChecks size={22} />
              </div>

              <div className="evidence-source">{selectedHit?.title ?? "未选择片段"}</div>
              {selectedHit && (
                <div className="evidence-meta">
                  <div>
                    <span>来源文件</span>
                    <strong>{selectedHit.documentPath}</strong>
                  </div>
                  <div>
                    <span>片段位置</span>
                    <strong>
                      第 {selectedHit.chunkIndex + 1} 段 · {selectedHit.startOffset}-{selectedHit.endOffset}
                    </strong>
                  </div>
                </div>
              )}
              <blockquote>{selectedHit ? plainSnippet(selectedHit.snippet) : "选择搜索结果后查看来源。"}</blockquote>

              <div className="evidence-actions">
                <button
                  type="button"
                  className="primary-button full"
                  disabled={!selectedHit}
                  onClick={revealSelectedSource}
                >
                  查看来源
                </button>
                <button type="button" className="secondary-button full" disabled={!selectedHit}>
                  标记误报
                </button>
                <button type="button" className="secondary-button full" disabled={!selectedHit}>
                  创建任务
                </button>
              </div>
            </section>

            <section className="panel risk-panel">
              <div className="risk-header">
                <AlertTriangle size={18} />
                <span>今日重点</span>
              </div>
              <div className="risk-item amber">
                <strong>{issues.length > 0 ? "有待确认问题" : "等待扫描"}</strong>
                <p>
                  {issues.length > 0
                    ? `当前有 ${issues.length} 条基础问题需要确认。`
                    : "导入并扫描项目后，这里会显示高风险一致性问题。"}
                </p>
              </div>
              <div className="risk-item teal">
                <strong>上下文准备</strong>
                <p>搜索片段可以作为后续上下文包的证据来源。</p>
              </div>
            </section>

            <section className="panel privacy-panel">
              <div className="panel-heading compact">
                <div>
                  <h2>隐私中心</h2>
                  <p>默认只使用本机数据。</p>
                </div>
                <LockKeyhole size={22} />
              </div>
              <div className="privacy-list">
                <PrivacyRow label="离线模式" value={privacy.offlineMode ? "开启" : "关闭"} />
                <PrivacyRow label="AI 增强" value={privacy.aiEnabled ? "开启" : "关闭"} />
                <PrivacyRow label="上传正文" value={privacy.uploadsEnabled ? "允许" : "不允许"} />
                <PrivacyRow label="剪贴板读取" value={privacy.clipboardAccess ? "允许" : "不读取"} />
                <PrivacyRow label="本地存储" value={privacy.storageMode} />
                <PrivacyRow label="数据库" value={basename(privacy.databasePath)} />
              </div>
            </section>

            <section className="panel mode-panel">
              <div className="mode-icon">
                <Sparkles size={18} />
              </div>
              <div>
                <strong>{profiles[0]?.name ?? "通用长篇模式"}</strong>
                <p>{profiles[0]?.description || "已启用章节识别、全文搜索、证据保留。"}</p>
              </div>
            </section>
          </aside>
        </section>
      </main>
    </div>
  );
}

function StatusButtons({ onConfirm, onDismiss }: { onConfirm: () => void; onDismiss: () => void }) {
  return (
    <div className="status-actions">
      <button type="button" onClick={onConfirm}>
        确认
      </button>
      <button type="button" onClick={onDismiss}>
        误报
      </button>
    </div>
  );
}

function PrivacyRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="privacy-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function totalCandidateCount(dashboard: Dashboard) {
  return dashboard.personCandidates + dashboard.placeCandidates + dashboard.itemCandidates;
}

function basename(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function formatError(reason: unknown) {
  const message = reason instanceof Error ? reason.message : String(reason);
  return message.startsWith("操作失败") ? message : `操作失败：${message}`;
}

function candidateTypeLabel(nodeType: string) {
  if (nodeType === "person") return "人物";
  if (nodeType === "place") return "地点";
  if (nodeType === "item") return "物件";
  return "候选";
}

function riskLabel(risk: string) {
  if (risk === "high") return "高风险";
  if (risk === "medium") return "中风险";
  if (risk === "low") return "低风险";
  return "待确认";
}

function severityLabel(severity: string) {
  if (severity === "serious") return "严重";
  if (severity === "high") return "高";
  if (severity === "medium") return "中";
  if (severity === "low") return "低";
  return "信息";
}

function sourceLabel(hit: SearchHit) {
  return `${hit.documentPath} · 第 ${hit.chunkIndex + 1} 段`;
}

function plainSnippet(snippet: string) {
  return snippet.replace(/\[/g, "").replace(/\]/g, "");
}

function renderSnippet(snippet: string) {
  const segments = snippet.split(/(\[[^\]]+\])/g);
  return segments.map((segment, index) => {
    if (segment.startsWith("[") && segment.endsWith("]")) {
      return <mark key={`${segment}-${index}`}>{segment.slice(1, -1)}</mark>;
    }
    return <span key={`${segment}-${index}`}>{segment}</span>;
  });
}
