import { Download, ShieldCheck } from "lucide-react";
import { clsx } from "clsx";
import { useState } from "react";
import { updateSetting, generateMarkdownReport, reloadAiProvider } from "../tauri";
import type { PrivacyStatus } from "../tauri";

interface PrivacyProps {
  privacy: PrivacyStatus;
  projectId?: string;
}

export function Privacy({ privacy, projectId }: PrivacyProps) {
  const [saving, setSaving] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [reportStatus, setReportStatus] = useState("");

  async function handleToggle(key: string) {
    const current = key === "aiEnabled" ? privacy.aiEnabled : key === "uploadsEnabled" ? privacy.uploadsEnabled : false;
    setSaving(key);
    setMessage("");
    try {
      const newValue = !current;
      const settingsKey = key === "aiEnabled" ? "ai_enabled" : "uploads_enabled";
      await updateSetting(settingsKey, String(newValue));
      if (key === "aiEnabled" && newValue) {
        await reloadAiProvider();
      }
      setMessage("设置已更新。");
    } catch (reason) {
      setMessage(`保存失败：${reason}`);
    } finally {
      setSaving(null);
    }
  }

  return (
    <div className="page">
      <div className="page-header">
        <h2>隐私中心</h2>
      </div>

      {message && (
        <div className="notice">
          <span>{message}</span>
        </div>
      )}

      <section className="settings-section">
        <h3 className="settings-section-title">数据安全</h3>
        <div style={{ maxWidth: 600, lineHeight: 1.7, marginBottom: "1.5rem" }}>
          <p>
            novellossless 默认以离线模式运行。正文内容和索引数据仅保存在本机，
            不会上传到任何外部服务。AI 分析和上传功能默认关闭。
          </p>
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section-title">当前状态</h3>
        <div className="privacy-list" style={{ marginTop: "1rem" }}>
          <PrivacyRow
            label="离线模式"
            value={privacy.offlineMode ? "已开启" : "已关闭"}
            icon={<ShieldCheck size={16} />}
          />
          <PrivacyRow
            label="AI 增强"
            value={privacy.aiEnabled ? "已开启" : "已关闭"}
            action={
              <button
                type="button"
                className={clsx("toggle", privacy.aiEnabled && "toggle-on")}
                onClick={() => handleToggle("aiEnabled")}
                disabled={saving === "aiEnabled"}
              >
                <div className="toggle-knob" />
              </button>
            }
          />
          <PrivacyRow
            label="上传正文"
            value={privacy.uploadsEnabled ? "已允许" : "不允许"}
            action={
              <button
                type="button"
                className={clsx("toggle", privacy.uploadsEnabled && "toggle-on")}
                onClick={() => handleToggle("uploadsEnabled")}
                disabled={saving === "uploadsEnabled"}
              >
                <div className="toggle-knob" />
              </button>
            }
          />
          <PrivacyRow
            label="剪贴板读取"
            value={privacy.clipboardAccess ? "允许" : "不读取"}
          />
          <PrivacyRow
            label="本地存储方式"
            value={privacy.storageMode}
          />
          <PrivacyRow
            label="数据库路径"
            value={privacy.databasePath}
          />
        </div>
      </section>

      {projectId && (
        <section className="settings-section">
          <h3 className="settings-section-title">操作</h3>
          <div style={{ display: "flex", gap: 8, marginTop: 4, flexWrap: "wrap" }}>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={async () => {
                setReportStatus("生成中…");
                try {
                  const pack = await generateMarkdownReport(projectId);
                  const blob = new Blob([pack.content], { type: "text/markdown" });
                  const url = URL.createObjectURL(blob);
                  const a = document.createElement("a");
                  a.href = url;
                  a.download = `privacy-report-${Date.now()}.md`;
                  a.click();
                  URL.revokeObjectURL(url);
                  setReportStatus("报告已下载。");
                } catch (e) {
                  setReportStatus(`生成失败：${e}`);
                }
              }}
            >
              <Download size={14} style={{ marginRight: 4 }} />
              导出隐私报告
            </button>
          </div>
          {reportStatus && (
            <p className="settings-desc" style={{ marginTop: 6 }}>{reportStatus}</p>
          )}
        </section>
      )}
    </div>
  );
}

function PrivacyRow({ label, value, icon, action }: { label: string; value: string; icon?: React.ReactNode; action?: React.ReactNode }) {
  return (
    <div className="privacy-row">
      <span>
        {icon && <span style={{ marginRight: 8, verticalAlign: "middle" }}>{icon}</span>}
        {label}
        <strong style={{ marginLeft: 8 }}>{value}</strong>
      </span>
      {action && <div className="settings-control">{action}</div>}
    </div>
  );
}
