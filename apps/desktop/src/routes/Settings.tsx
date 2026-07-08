import { Check, Download, RefreshCw, Shield, Upload } from "lucide-react";
import { clsx } from "clsx";
import { useEffect, useState } from "react";
import { backupDatabase, getAvailableProfiles, getEnabledProfiles, getSettings, PrivacyStatus, ProfileManifest, restoreDatabase, setEnabledProfiles, updateSetting } from "../tauri";

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
                const input = document.createElement("input");
                input.type = "file";
                input.accept = ".db";
                input.onchange = async () => {
                  const file = input.files?.[0];
                  if (!file) return;
                  const path = (file as any).path;
                  if (path) {
                    await restoreDatabase(path);
                    setMessage("数据库已恢复。请重启应用。");
                  }
                };
                input.click();
              } catch (e) {
                setMessage(`恢复失败：${e}`);
              }
            }}
          >
            <Upload size={14} style={{ marginRight: 4 }} />
            恢复备份
          </button>
        </div>
      </section>

      {profiles.length > 0 && (
        <section className="settings-section">
          <h3 className="settings-section-title">创作模式</h3>
          {profiles.map((p) => (
            <div className="settings-row" key={p.id}>
              <div>
                <label>{p.name}</label>
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
