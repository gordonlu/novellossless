import { Download, ExternalLink, Trash2, Upload } from "lucide-react";
import { clsx } from "clsx";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { open } from "@tauri-apps/plugin-dialog";
import { applyTheme } from "../App";
import { backupDatabase, BackupInfo, deleteProject, getAvailableProfiles, getEnabledProfiles, getSettings, listBackups, PrivacyStatus, ProfileManifest, restoreDatabase, setEnabledProfiles, updateSetting, testAiConnection, reloadAiProvider } from "../tauri";

interface Props {
  privacy: PrivacyStatus;
  projectId?: string;
}

interface SettingEntry {
  key: string;
  value: string;
  dirty: boolean;
}

const SETTING_LABELS: Record<string, string> = {
  theme: "主题",
  language: "语言",
  auto_scan: "自动扫描",
  auto_watch: "自动监听文件变更",
  ai_enabled: "允许 AI 分析",
  uploads_enabled: "上传正文",
  backup_enabled: "启用备份",
  backup_path: "备份目录",
};

const SETTING_DESCRIPTIONS: Record<string, string> = {
  theme: "深色 / 浅色",
  language: "界面语言",
  auto_scan: "打开项目后自动执行增量扫描",
  auto_watch: "自动监听小说目录的文件变更",
  ai_enabled: "允许通过 AI Provider 分析正文",
  uploads_enabled: "允许将正文内容上传到外部服务",
  backup_enabled: "定期备份数据库到指定目录",
  backup_path: "备份文件存放路径",
};

export function Settings({ privacy, projectId }: Props) {
  const [entries, setEntries] = useState<Record<string, SettingEntry>>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [profiles, setProfiles] = useState<ProfileManifest[]>([]);
  const [enabledIds, setEnabledIds] = useState<string[]>([]);
  const [profileSaving, setProfileSaving] = useState(false);
  const [aiTestResult, setAiTestResult] = useState("");
  const [aiTesting, setAiTesting] = useState(false);
  const [hasStoredKey, setHasStoredKey] = useState(false);

  useEffect(() => {
    if (entries["ai_api_key"]?.value === "stored-via-keyring") {
      setHasStoredKey(true);
    }
  }, [entries]);

  useEffect(() => {
    (async () => {
      try {
        const settings = await getSettings();
        const map: Record<string, SettingEntry> = {};
        for (const s of settings) {
          map[s.key] = { key: s.key, value: s.value, dirty: false };
        }
        setEntries(map);

        const available = await getAvailableProfiles();
        setProfiles(available);

        if (projectId) {
          const enabled = await getEnabledProfiles(projectId);
          setEnabledIds(enabled);
        }
      } catch {
        // preview mode
      } finally {
        setLoading(false);
      }
    })();
  }, [projectId]);

  async function handleToggle(key: string) {
    const current = entries[key]?.value === "true";
    const newValue = current ? "false" : "true";
    setSaving(key);
    setMessage("");
    try {
      await updateSetting(key, newValue);
      setEntries((prev) => ({
        ...prev,
        [key]: { key, value: newValue, dirty: false },
      }));
      if (key === "ai_enabled" && newValue === "true") {
        await reloadAiProvider();
      }
    } catch (reason) {
      setMessage(`保存失败：${reason}`);
    } finally {
      setSaving(null);
    }
  }

  if (loading) {
    return (
      <div className="page page-center">
        <p>加载中…</p>
      </div>
    );
  }

  const toggleValue = (key: string, defaultValue = "false"): boolean => {
    const entry = entries[key];
    return entry ? entry.value === "true" : defaultValue === "true";
  };

  return (
    <div className="page settings-page">
      <div className="page-header">
        <h2>设置</h2>
      </div>

      {message && (
        <div className="notice">
          <span>{message}</span>
        </div>
      )}

      <section className="settings-section">
        <h3 className="settings-section-title">界面</h3>
        <div className="settings-row">
          <div>
            <label>主题</label>
            <p className="settings-desc">{SETTING_DESCRIPTIONS["theme"]}</p>
          </div>
          <div className="settings-control">
            <select
              className="settings-select"
              value={entries["theme"]?.value || "dark"}
              onChange={async (e) => {
                const v = e.target.value;
                setSaving("theme");
                try {
                  await updateSetting("theme", v);
                  applyTheme(v);
                  setEntries((prev) => ({
                    ...prev,
                    theme: { key: "theme", value: v, dirty: false },
                  }));
                } catch {
                  // ignore
                } finally {
                  setSaving(null);
                }
              }}
            >
              <option value="dark">深色</option>
              <option value="light">浅色</option>
            </select>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section-title">扫描</h3>
        <div className="settings-row">
          <div>
            <label>自动扫描</label>
            <p className="settings-desc">{SETTING_DESCRIPTIONS["auto_scan"]}</p>
          </div>
          <button
            type="button"
            className={clsx("toggle", toggleValue("auto_scan", "true") && "toggle-on")}
            onClick={() => handleToggle("auto_scan")}
            disabled={saving === "auto_scan"}
          >
            <div className="toggle-knob" />
          </button>
        </div>
        <div className="settings-row">
          <div>
            <label>自动监听文件变更</label>
            <p className="settings-desc">{SETTING_DESCRIPTIONS["auto_watch"]}</p>
          </div>
          <button
            type="button"
            className={clsx("toggle", toggleValue("auto_watch") && "toggle-on")}
            onClick={() => handleToggle("auto_watch")}
            disabled={saving === "auto_watch"}
          >
            <div className="toggle-knob" />
          </button>
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section-title">隐私</h3>
        <div className="settings-row">
          <div>
            <label>允许 AI 分析</label>
            <p className="settings-desc">{SETTING_DESCRIPTIONS["ai_enabled"]}</p>
          </div>
          <button
            type="button"
            className={clsx("toggle", toggleValue("ai_enabled") && "toggle-on")}
            onClick={() => handleToggle("ai_enabled")}
            disabled={saving === "ai_enabled"}
          >
            <div className="toggle-knob" />
          </button>
        </div>
        <div className="settings-row">
          <div>
            <label>上传正文</label>
            <p className="settings-desc">{SETTING_DESCRIPTIONS["uploads_enabled"]}</p>
          </div>
          <button
            type="button"
            className={clsx("toggle", toggleValue("uploads_enabled") && "toggle-on")}
            onClick={() => handleToggle("uploads_enabled")}
            disabled={saving === "uploads_enabled"}
          >
            <div className="toggle-knob" />
          </button>
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section-title">AI 配置</h3>

        <div className="notice notice-block" style={{ marginBottom: 16 }}>
          <strong>数据隐私提醒：</strong>
          启用 AI 后，正文片段将通过你指定的 API 发送到外部服务（每次仅限 8000 字以内的相关片段，不会发送整本书）。
          OpenAI / DeepSeek / Anthropic 等<strong>纯 API</strong> 不会用你的数据训练模型。
          但<strong>Coding Plan（订阅制）</strong>服务会记录和分析对话内容用于产品改进，与纯 API 计费不同。
          如需确保数据绝对不离开本机，建议使用本地模型（
          <a href="https://ollama.com" target="_blank" rel="noopener">Ollama</a>、
          <a href="https://lmstudio.ai" target="_blank" rel="noopener">LM Studio</a>）。
        </div>

        <div className="settings-row">
          <div>
            <label>API 地址</label>
            <p className="settings-desc">
              本地模型填 <code>http://localhost:11434</code>，商业 API 填对应地址
            </p>
          </div>
          <div className="settings-control">
            <input
              className="settings-input"
              value={entries["ai_provider"]?.value || ""}
              placeholder="https://api.openai.com"
              onChange={async (e) => {
                const v = e.target.value;
                await updateSetting("ai_provider", v);
                setEntries((prev) => ({
                  ...prev,
                  ai_provider: { key: "ai_provider", value: v, dirty: false },
                }));
                await reloadAiProvider();
              }}
            />
          </div>
        </div>
        <div className="settings-row">
          <div>
            <label>API Key</label>
            <p className="settings-desc">API 密钥（存入系统密钥链，不落盘）。本地模型不需要</p>
          </div>
          <div className="settings-control">
            <input
              className="settings-input"
              type="password"
              value={hasStoredKey ? "" : (entries["ai_api_key"]?.value || "")}
              placeholder={hasStoredKey ? "••••••••（已存入系统密钥链）" : "sk-...（留空则不用 key）"}
              onChange={async (e) => {
                const v = e.target.value;
                if (!v) return;
                await updateSetting("ai_api_key", v);
                setEntries((prev) => ({
                  ...prev,
                  ai_api_key: { key: "ai_api_key", value: "stored-via-keyring", dirty: false },
                }));
                setHasStoredKey(true);
                await reloadAiProvider();
              }}
            />
          </div>
        </div>
        <div className="settings-row">
          <div>
            <label>模型</label>
            <p className="settings-desc">
              本地用 <code>qwen2.5:7b</code> / <code>llama3.2:3b</code>，商业用 <code>gpt-4o-mini</code> / <code>deepseek-chat</code>
            </p>
          </div>
          <div className="settings-control">
            <input
              className="settings-input"
              value={entries["ai_model"]?.value || ""}
              placeholder="gpt-4o-mini"
              onChange={async (e) => {
                const v = e.target.value;
                await updateSetting("ai_model", v);
                setEntries((prev) => ({
                  ...prev,
                  ai_model: { key: "ai_model", value: v, dirty: false },
                }));
                await reloadAiProvider();
              }}
            />
          </div>
        </div>
        <div className="settings-row">
          <div>
            <label>连接测试</label>
            <p className="settings-desc">{aiTestResult || "测试 AI API 是否可达"}</p>
          </div>
          <div className="settings-control">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={async () => {
                setAiTesting(true);
                setAiTestResult("测试中…");
                try {
                  const result = await testAiConnection();
                  setAiTestResult(result);
                } catch (e) {
                  setAiTestResult(`失败：${e}`);
                } finally {
                  setAiTesting(false);
                }
              }}
              disabled={aiTesting}
            >
              测试连接
            </button>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section-title">备份</h3>
        <div className="settings-row">
          <div>
            <label>启用备份</label>
            <p className="settings-desc">{SETTING_DESCRIPTIONS["backup_enabled"]}</p>
          </div>
          <button
            type="button"
            className={clsx("toggle", toggleValue("backup_enabled", "true") && "toggle-on")}
            onClick={() => handleToggle("backup_enabled")}
            disabled={saving === "backup_enabled"}
          >
            <div className="toggle-knob" />
          </button>
        </div>
        <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
          <button
            type="button"
            className="btn btn-secondary"
            onClick={async () => {
              setMessage("");
              try {
                const path = await backupDatabase();
                setMessage(`备份已保存到：${path}`);
              } catch (e) {
                setMessage(`备份失败：${e}`);
              }
            }}
          >
            <Download size={14} style={{ marginRight: 4 }} />
            立即备份
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            onClick={async () => {
              setMessage("");
              try {
                const selected = await open({
                  filters: [{ name: "Database", extensions: ["db"] }],
                  multiple: false,
                  directory: false,
                });
                if (selected) {
                  await restoreDatabase(selected);
                  setMessage("数据库已恢复。请重启应用。");
                }
              } catch (e) {
                setMessage(`恢复失败：${e}`);
              }
            }}
          >
            <Upload size={14} style={{ marginRight: 4 }} />
            恢复备份
          </button>
        </div>
        <BackupList />
      </section>

      {projectId && (
        <section className="settings-section">
          <h3 className="settings-section-title">项目管理</h3>
          <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
            <button
              type="button"
              className="btn btn-danger"
              onClick={async () => {
                if (!confirm("确定要删除此项目及其所有数据？此操作不可撤销。")) return;
                try {
                  await deleteProject(projectId);
                  setMessage("项目已删除。");
                  setTimeout(() => window.location.reload(), 1000);
                } catch (e) {
                  setMessage(`删除失败：${e}`);
                }
              }}
            >
              <Trash2 size={14} style={{ marginRight: 4 }} />
              删除此项目
            </button>
          </div>
        </section>
      )}

      {profiles.length > 0 && (
        <section className="settings-section">
          <h3 className="settings-section-title">创作模式</h3>
          {profiles.map((p) => (
            <div className="settings-row" key={p.id}>
              <div>
                <Link to={`/profiles/${p.id}`} style={{ display: "flex", alignItems: "center", gap: 6, textDecoration: "none", color: "inherit" }}>
                  <label style={{ cursor: "pointer" }}>{p.name}</label>
                  <ExternalLink size={12} style={{ color: "var(--text-muted)" }} />
                </Link>
                <p className="settings-desc">{p.description}</p>
              </div>
              <button
                type="button"
                className={clsx("toggle", enabledIds.includes(p.id) && "toggle-on")}
                onClick={async () => {
                  setProfileSaving(true);
                  try {
                    const next = enabledIds.includes(p.id)
                      ? enabledIds.filter((id) => id !== p.id)
                      : [...enabledIds, p.id];
                    await setEnabledProfiles(projectId!, next);
                    setEnabledIds(next);
                  } catch (e) {
                    setMessage(`保存失败：${e}`);
                  } finally {
                    setProfileSaving(false);
                  }
                }}
                disabled={profileSaving || !projectId}
              >
                <div className="toggle-knob" />
              </button>
            </div>
          ))}
        </section>
      )}
    </div>
  );
}

function BackupList() {
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const list = await listBackups();
        setBackups(list);
      } catch {
        // preview mode
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  if (loading) return null;
  if (backups.length === 0) return <p className="settings-desc" style={{ marginTop: 8 }}>尚无备份文件。</p>;

  return (
    <div style={{ marginTop: 12 }}>
      <p className="settings-desc" style={{ marginBottom: 6 }}>已有备份：</p>
      {backups.map((b) => (
        <div key={b.path} className="settings-row" style={{ padding: "6px 0" }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {b.fileName}
            </span>
            <span style={{ fontSize: 11, marginLeft: 8, color: "var(--text-muted)" }}>
              {(b.sizeBytes / 1024).toFixed(0)} KB · {b.createdAt.slice(0, 19).replace("T", " ")}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}
