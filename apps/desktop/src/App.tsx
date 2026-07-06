import { AlertTriangle, Archive, BookOpenText, CheckCircle2, Clock3, Database, Home, LockKeyhole, Network, RefreshCw, Search, ShieldCheck, UserRound } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { clsx } from "clsx";
import { useEffect, useState } from "react";
import { Link, Route, Routes, useLocation } from "react-router-dom";
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
import { basename, formatError, plainSnippet } from "./lib/helpers";
import { ContentView } from "./routes/ContentView";
import { ContextPack as ContextPackRoute } from "./routes/ContextPack";
import { Characters } from "./routes/Characters";
import { Dashboard as DashboardRoute } from "./routes/Dashboard";
import { Foreshadows } from "./routes/Foreshadows";
import { Issues } from "./routes/Issues";
import { Privacy } from "./routes/Privacy";
import { SearchView } from "./routes/SearchView";
import { Timeline } from "./routes/Timeline";

const demoProject: Project = {
  id: "demo",
  name: "雨巷钟声",
  rootPath: "未导入项目",
  createdAt: "",
  updatedAt: "",
};

const navigation = [
  { label: "项目首页", icon: Home, path: "/" },
  { label: "正文", icon: BookOpenText, path: "/content" },
  { label: "搜索", icon: Search, path: "/search" },
  { label: "人物", icon: UserRound, path: "/characters" },
  { label: "伏笔", icon: Network, path: "/foreshadows" },
  { label: "时间线", icon: Clock3, path: "/timeline" },
  { label: "冲突报告", icon: AlertTriangle, path: "/issues" },
  { label: "上下文包", icon: Archive, path: "/context-pack" },
  { label: "隐私中心", icon: LockKeyhole, path: "/privacy" },
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
  const location = useLocation();
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
            <Link
              to={item.path}
              className={clsx("nav-item", location.pathname === item.path && "nav-item-active")}
              key={item.label}
            >
              <item.icon size={17} />
              <span>{item.label}</span>
            </Link>
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

        <Routes>
          <Route
            path="/"
            element={
              <DashboardRoute
                projectName={projectName}
                setProjectName={setProjectName}
                folderPath={folderPath}
                setFolderPath={setFolderPath}
                dashboard={dashboard}
                candidates={candidates}
                foreshadows={foreshadows}
                issues={issues}
                query={query}
                setQuery={setQuery}
                hits={hits}
                selectedHit={selectedHit}
                setSelectedHit={setSelectedHit}
                contextPack={contextPack}
                searchAttempted={searchAttempted}
                lastScan={lastScan}
                busy={busy}
                runtimeMode={runtimeMode}
                hasRealProject={hasRealProject}
                error={error}
                privacy={privacy}
                profiles={profiles}
                chooseFolder={chooseFolder}
                handleImport={handleImport}
                handleScan={handleScan}
                runSearch={runSearch}
                handleBuildContextPack={handleBuildContextPack}
                handleStatus={handleStatus}
                revealSelectedSource={revealSelectedSource}
              />
            }
          />
          <Route path="/content" element={<ContentView />} />
          <Route path="/search" element={<SearchView />} />
          <Route path="/characters" element={<Characters />} />
          <Route path="/foreshadows" element={<Foreshadows />} />
          <Route path="/timeline" element={<Timeline />} />
          <Route path="/issues" element={<Issues />} />
          <Route path="/context-pack" element={<ContextPackRoute />} />
          <Route
            path="/privacy"
            element={<Privacy privacy={privacy} />}
          />
        </Routes>
      </main>
    </div>
  );
}
