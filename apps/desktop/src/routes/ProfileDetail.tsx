import { useEffect, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import { getAvailableProfiles, getProfileMetrics, getEnabledProfiles, listKnowledgePacks, type KnowledgePackInfo, type ProfileManifest, type ProfileMetric } from "../tauri";

interface Props {
  projectId?: string;
}

export function ProfileDetail({ projectId }: Props) {
  const { profileId } = useParams();
  const [profile, setProfile] = useState<ProfileManifest | null>(null);
  const [metrics, setMetrics] = useState<ProfileMetric[]>([]);
  const [enabled, setEnabled] = useState(false);
  const [knowledgePacks, setKnowledgePacks] = useState<KnowledgePackInfo[]>([]);

  useEffect(() => {
    if (!profileId) return;
    (async () => {
      const all = await getAvailableProfiles();
      const found = all.find((p: ProfileManifest) => p.id === profileId);
      setProfile(found || null);

      if (projectId) {
        const enabledIds = await getEnabledProfiles(projectId);
        setEnabled(enabledIds.includes(profileId));
        try {
          setMetrics(await getProfileMetrics(projectId, profileId));
        } catch { /* no metrics yet */ }
      }
    })();
  }, [profileId, projectId]);

  useEffect(() => {
    if (!profileId) return;
    (async () => {
      try {
        setKnowledgePacks(await listKnowledgePacks());
      } catch { /* ok */ }
    })();
  }, [profileId]);

  if (!profile) {
    return (
      <div className="page page-center">
        <p>未找到该创作模式。</p>
        <Link to="/settings" className="btn btn-secondary" style={{ marginTop: 12, display: "inline-flex" }}>
          ← 返回设置
        </Link>
      </div>
    );
  }

  return (
    <div className="page">
      <div className="page-header">
        <Link to="/settings" style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 13, color: "var(--text-secondary)", marginBottom: 8 }}>
          <ArrowLeft size={14} /> 返回设置
        </Link>
        <h2>{profile.name}</h2>
        <p className="settings-desc" style={{ marginTop: 4 }}>{profile.description}</p>
      </div>

      <section className="settings-section">
        <h3 className="settings-section-title">基本信息</h3>
        <div className="privacy-list" style={{ marginTop: 8 }}>
          <ProfileInfoRow label="ID" value={profile.id} />
          <ProfileInfoRow label="版本" value={profile.version} />
          <ProfileInfoRow label="默认启用" value={profile.enabledByDefault ? "是" : "否"} />
          <ProfileInfoRow label="当前项目" value={projectId ? (enabled ? "已启用" : "未启用") : "未选择项目"} />
        </div>
      </section>

      {profile.entityTypes.length > 0 && (
        <section className="settings-section">
          <h3 className="settings-section-title">实体类型</h3>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 8 }}>
            {profile.entityTypes.map((t) => (
              <span key={t} style={{ padding: "3px 10px", background: "var(--accent-bg)", borderRadius: 999, fontSize: 12, color: "var(--accent)" }}>{t}</span>
            ))}
          </div>
        </section>
      )}

      {profile.metrics.length > 0 && (
        <section className="settings-section">
          <h3 className="settings-section-title">指标</h3>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 8 }}>
            {profile.metrics.map((m) => (
              <span key={m} style={{ padding: "3px 10px", border: "1px solid var(--border-light)", borderRadius: 6, fontSize: 12 }}>{m}</span>
            ))}
          </div>
        </section>
      )}

      {profile.checks.length > 0 && (
        <section className="settings-section">
          <h3 className="settings-section-title">检查项</h3>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 8 }}>
            {profile.checks.map((c) => (
              <span key={c} style={{ padding: "3px 10px", border: "1px solid var(--border-light)", borderRadius: 6, fontSize: 12 }}>{c}</span>
            ))}
          </div>
        </section>
      )}

      {metrics.length > 0 && (
        <section className="settings-section">
          <h3 className="settings-section-title">指标数据</h3>
          <div className="metric-grid" style={{ marginTop: 8, gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))" }}>
            {metrics.map((m) => (
              <div key={m.id} className="metric-card" style={{ padding: "10px 12px" }}>
                <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{m.metricType}</span>
                <strong style={{ fontSize: 18, fontWeight: 700, marginTop: 4, display: "block", fontVariantNumeric: "tabular-nums" }}>{m.value}</strong>
              </div>
            ))}
          </div>
        </section>
      )}

      {knowledgePacks.length > 0 && (
        <section className="settings-section">
          <h3 className="settings-section-title">知识包</h3>
          <div style={{ marginTop: 8 }}>
            {knowledgePacks.map((kp) => (
              <div key={kp.name} className="settings-row" style={{ padding: "8px 0" }}>
                <div>
                  <strong style={{ fontSize: 13 }}>{kp.name}</strong>
                  <p className="settings-desc">
                    {kp.entryCount} 条 · {kp.dynasties.join(", ")}
                    {kp.source === "user" && <span style={{ marginLeft: 6, padding: "1px 6px", borderRadius: 4, background: "var(--accent-bg)", fontSize: 11 }}>用户导入</span>}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function ProfileInfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="privacy-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
