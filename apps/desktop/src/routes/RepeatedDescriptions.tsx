import { FileText } from "lucide-react";
import { clsx } from "clsx";
import { useEffect, useState } from "react";
import { listIssues, updateIssueStatus, ContinuityIssue } from "../tauri";

interface Props {
  projectId?: string;
}

const DETECTOR_LABELS: Record<string, string> = {
  repeated_paragraph: "重复场景描写",
  repeated_expression: "高频重复表达",
  repeated_action: "重复动作",
  repeated_dialogue: "重复对白引导模式",
  repeated_psych_density: "过密心理描写",
};

const DETECTOR_ICONS: Record<string, string> = {
  repeated_paragraph: "📄",
  repeated_expression: "🔁",
  repeated_action: "🔄",
  repeated_dialogue: "💬",
  repeated_psych_density: "🧠",
};

export function RepeatedDescriptions({ projectId }: Props) {
  const [issues, setIssues] = useState<ContinuityIssue[]>([]);
  const [selected, setSelected] = useState<ContinuityIssue | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!projectId) return;
    (async () => {
      try {
        const items = await listIssues(projectId, 50, "repeated_");
        setIssues(items);
      } catch {
        // preview mode
      } finally {
        setLoading(false);
      }
    })();
  }, [projectId]);

  async function handleStatus(id: string, status: string) {
    try {
      await updateIssueStatus(id, status);
      setIssues((prev) =>
        prev.map((i) => (i.id === id ? { ...i, status } : i)),
      );
    } catch {
      // ignore
    }
  }

  if (loading) {
    return (
      <div className="page page-center">
        <p>加载中…</p>
      </div>
    );
  }

  const grouped = groupBy(issues, (i) => i.issueType);
  const detectorTypes = Object.keys(grouped).sort();

  return (
    <div className="page">
      <div className="page-header">
        <h2>重复描写</h2>
      </div>

      {detectorTypes.length === 0 && (
        <div className="empty-state small">
          <strong>尚未检测到重复描写</strong>
          <p>扫描后会检测重复场景、高频表达、重复动作、对白模式和过密心理描写。</p>
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1fr)", gap: 16, alignItems: "start" }}>
        <div>
          {detectorTypes.map((detectorType) => {
            const items = grouped[detectorType];
            const label = DETECTOR_LABELS[detectorType] || detectorType;
            const icon = DETECTOR_ICONS[detectorType] || "📋";
            return (
              <section key={detectorType} style={{ marginBottom: 16 }}>
                <h3 style={{ fontSize: 13, fontWeight: 650, marginBottom: 8, color: "var(--text-secondary)" }}>
                  {icon} {label}
                  <span style={{ marginLeft: 6, fontSize: 11, color: "var(--text-muted)" }}>
                    {items.length}
                  </span>
                </h3>
                <div className="compact-list">
                  {items.map((issue) => {
                    let evidence: { chunkId?: string; chapterTitle?: string; documentPath?: string; snippet?: string }[] = [];
                    try {
                      evidence = JSON.parse(issue.evidenceJson);
                    } catch { /* empty */ }
                    return (
                      <div
                        key={issue.id}
                        className={clsx("compact-item", issue.severity === "info" && "issue", selected?.id === issue.id && "compact-item-active")}
                        onClick={() => setSelected(selected?.id === issue.id ? null : issue)}
                      >
                        <div>
                          <strong>{issue.title}</strong>
                          <p>{issue.description}</p>
                          {evidence.length > 0 && (
                            <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>
                              {evidence.length} 处来源
                            </p>
                          )}
                        </div>
                        <div className="row-actions">
                          <button
                            type="button"
                            className="btn btn-secondary"
                            onClick={(e) => { e.stopPropagation(); handleStatus(issue.id, "resolved"); }}
                            title="标记已解决"
                          >
                            ✓
                          </button>
                          <button
                            type="button"
                            className="btn btn-secondary"
                            onClick={(e) => { e.stopPropagation(); handleStatus(issue.id, "dismissed"); }}
                            title="忽略"
                          >
                            ✕
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </section>
            );
          })}
        </div>

        <div className="inspector">
          {selected ? (
            <div className="panel" style={{ padding: 16 }}>
              <h3 style={{ fontSize: 14, fontWeight: 700, marginBottom: 8 }}>{selected.title}</h3>
              <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
                <span className="badge badge-issue">{selected.severity}</span>
                <span className="badge">{DETECTOR_LABELS[selected.issueType] || selected.issueType}</span>
                <span className="badge">{selected.status}</span>
              </div>
              <p style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.6, marginBottom: 12 }}>
                {selected.description}
              </p>

              {(() => {
                let evidence: { chunkId?: string; chapterTitle?: string; documentPath?: string; snippet?: string; matchCount?: number }[] = [];
                try {
                  evidence = JSON.parse(selected.evidenceJson);
                } catch { /* empty */ }
                if (evidence.length === 0) return null;
                return (
                  <div>
                    <h4 style={{ fontSize: 12, fontWeight: 650, color: "var(--text-muted)", marginBottom: 8 }}>
                      来源证据 ({evidence.length})
                    </h4>
                    {evidence.map((item, idx) => (
                      <div key={idx} style={{ marginBottom: 8, padding: 8, borderRadius: 6, background: "var(--bg-hover)", fontSize: 12 }}>
                        {item.chapterTitle && (
                          <div style={{ fontWeight: 600, marginBottom: 4, color: "var(--text-primary)" }}>
                            <FileText size={12} style={{ marginRight: 4, verticalAlign: "middle" }} />
                            {item.chapterTitle}
                          </div>
                        )}
                        {item.snippet && (
                          <blockquote style={{ margin: "4px 0", padding: "4px 8px", borderLeft: "2px solid var(--border-light)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
                            {item.snippet}
                          </blockquote>
                        )}
                        {item.matchCount !== undefined && (
                          <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
                            出现 {item.matchCount} 次
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                );
              })()}

              {selected.suggestedActionsJson && (
                <div style={{ marginTop: 8 }}>
                  <h4 style={{ fontSize: 12, fontWeight: 650, color: "var(--text-muted)", marginBottom: 4 }}>
                    建议
                  </h4>
                  <p style={{ fontSize: 12, color: "var(--text-secondary)", fontStyle: "italic" }}>
                    {selected.suggestedActionsJson.replace(/^"|"$/g, "")}
                  </p>
                </div>
              )}

              <div style={{ display: "flex", gap: 6, marginTop: 16 }}>
                <button type="button" className="btn btn-secondary" onClick={() => handleStatus(selected.id, "resolved")}>
                  标记已解决
                </button>
                <button type="button" className="btn btn-secondary" onClick={() => handleStatus(selected.id, "dismissed")}>
                  忽略
                </button>
              </div>
            </div>
          ) : (
            <div className="empty-state small">
              <strong>选择一个重复描写查看详情</strong>
              <p>左侧按检测类型分组展示。</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function groupBy<T>(items: T[], fn: (item: T) => string): Record<string, T[]> {
  const map: Record<string, T[]> = {};
  for (const item of items) {
    const key = fn(item);
    if (!map[key]) map[key] = [];
    map[key].push(item);
  }
  return map;
}
