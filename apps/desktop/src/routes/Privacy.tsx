import { ShieldCheck } from "lucide-react";
import { Link } from "react-router-dom";
import type { PrivacyStatus } from "../tauri";

interface PrivacyProps {
  privacy: PrivacyStatus;
}

export function Privacy({ privacy }: PrivacyProps) {
  return (
    <div className="page">
      <div className="page-header">
        <h2>隐私中心</h2>
      </div>

      <section className="settings-section">
        <h3 className="settings-section-title">数据安全</h3>
        <div style={{ maxWidth: 600, lineHeight: 1.7, marginBottom: "1.5rem" }}>
          <p>
            novellossless 默认以离线模式运行。正文内容和索引数据仅保存在本机，
            不会上传到任何外部服务。AI 分析和上传功能默认关闭，需要您在
            <Link to="/settings" style={{ textDecoration: "underline", margin: "0 4px" }}>设置</Link>
            中手动开启。
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
          />
          <PrivacyRow
            label="上传正文"
            value={privacy.uploadsEnabled ? "已允许" : "不允许"}
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

      <section className="settings-section">
        <h3 className="settings-section-title">配置</h3>
        <p style={{ maxWidth: 600, lineHeight: 1.7 }}>
          可在 <Link to="/settings" style={{ textDecoration: "underline" }}>设置页面</Link>{" "}
          中调整 AI 分析和上传权限。
        </p>
      </section>
    </div>
  );
}

function PrivacyRow({ label, value, icon }: { label: string; value: string; icon?: React.ReactNode }) {
  return (
    <div className="privacy-row">
      <span>
        {icon && <span style={{ marginRight: 8, verticalAlign: "middle" }}>{icon}</span>}
        {label}
      </span>
      <strong>{value}</strong>
    </div>
  );
}
