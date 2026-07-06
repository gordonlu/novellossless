import type { PrivacyStatus } from "../tauri";

interface PrivacyProps {
  privacy: PrivacyStatus;
}

export function Privacy({ privacy }: PrivacyProps) {
  return (
    <div className="panel" style={{ padding: "2rem" }}>
      <h2>隐私中心</h2>
      <div className="privacy-list" style={{ marginTop: "1rem" }}>
        <PrivacyRow label="离线模式" value={privacy.offlineMode ? "开启" : "关闭"} />
        <PrivacyRow label="AI 增强" value={privacy.aiEnabled ? "开启" : "关闭"} />
        <PrivacyRow label="上传正文" value={privacy.uploadsEnabled ? "允许" : "不允许"} />
        <PrivacyRow label="剪贴板读取" value={privacy.clipboardAccess ? "允许" : "不读取"} />
        <PrivacyRow label="本地存储" value={privacy.storageMode} />
      </div>
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
