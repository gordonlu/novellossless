import { useEffect, useState } from "react";
import { AlertTriangle, FileText } from "lucide-react";
import { clsx } from "clsx";
import { listIssues, updateIssueStatus, ContinuityIssue } from "../tauri";
import { StatusButtons } from "../components/StatusButtons";
import { InspectorPanel } from "../components/InspectorPanel";
import { severityLabel } from "../lib/helpers";

const severityColors: Record<string, string> = {
  serious: "severity-serious",
  high: "severity-high",
  medium: "severity-medium",
  low: "severity-low",
};

interface Props {
  projectId: string;
}

export function Issues({ projectId }: Props) {
  const [issues, setIssues] = useState<ContinuityIssue[]>([]);
  const [selected, setSelected] = useState<ContinuityIssue | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listIssues(projectId, 50).then(setIssues);
    }
  }, [projectId]);

  const handleStatus = async (id: string, status: string) => {
    try {
      await updateIssueStatus(id, status);
      const updated = await listIssues(projectId, 50);
      setIssues(updated);
      setSelected((prev) => (prev?.id === id ? updated.find((i) => i.id === id) ?? null : prev));
    } catch {
      // silent
    }
  };

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading"><h2>基础问题</h2><p>共 {issues.length} 条</p></div>
          <div className="compact-list">
            {issues.length > 0 ? (
              issues.map((issue) => (
                <article
                  className={clsx("compact-item issue", selected?.id === issue.id && "compact-item-active")}
                  key={issue.id}
                  onClick={() => setSelected(issue)}
                >
                  <div>
                    <strong>
                      {issue.title}
                      <span className={severityColors[issue.severity] ?? ""}>
                        {severityLabel(issue.severity)}
                      </span>
                    </strong>
                    <p>{issue.description}</p>
                  </div>
                  <div className="row-actions">
                    <StatusButtons
                      onConfirm={() => handleStatus(issue.id, "resolved")}
                      onDismiss={() => handleStatus(issue.id, "false_positive")}
                    />
                  </div>
                </article>
              ))
            ) : (
              <div className="empty-state small">扫描后会显示重复表达和明确属性冲突。</div>
            )}
          </div>
        </section>
      </div>
      {selected ? (
        <aside className="inspector">
          <section className="panel">
            <div className="panel-heading compact"><h2>详情</h2><AlertTriangle size={22} /></div>
            <div className="evidence-meta">
              <div><span>类型</span><strong>{selected.issueType}</strong></div>
              <div><span>严重度</span><strong>{severityLabel(selected.severity)}</strong></div>
              <div><span>状态</span><strong>{selected.status}</strong></div>
            </div>
            <blockquote>{selected.description}</blockquote>

            {(() => {
              let evidence: { snippet?: string; chapterTitle?: string; documentPath?: string; matchCount?: number }[] = [];
              try {
                evidence = JSON.parse(selected.evidenceJson);
              } catch { /* empty */ }
              if (evidence.length === 0) return null;
              return (
                <div style={{ marginTop: 12 }}>
                  <h4 style={{ fontSize: 12, fontWeight: 650, color: "var(--text-muted)", marginBottom: 6 }}>
                    来源证据 ({evidence.length})
                  </h4>
                  {evidence.map((item, idx) => (
                    <div key={idx} style={{ marginBottom: 6, padding: 6, borderRadius: 4, background: "var(--bg-hover)", fontSize: 12 }}>
                      {item.chapterTitle && (
                        <div style={{ fontWeight: 600, marginBottom: 2 }}>
                          <FileText size={11} style={{ marginRight: 4, verticalAlign: "middle" }} />
                          {item.chapterTitle}
                        </div>
                      )}
                      {item.snippet && (
                        <blockquote style={{ margin: "2px 0", padding: "2px 8px", borderLeft: "2px solid var(--border-light)", color: "var(--text-secondary)", lineHeight: 1.4 }}>
                          {item.snippet}
                        </blockquote>
                      )}
                    </div>
                  ))}
                </div>
              );
            })()}

            {selected.suggestedActionsJson && selected.suggestedActionsJson !== "[]" && (
              <div style={{ marginTop: 8 }}>
                <h4 style={{ fontSize: 12, fontWeight: 650, color: "var(--text-muted)", marginBottom: 4 }}>建议</h4>
                <p style={{ fontSize: 12, color: "var(--text-secondary)", fontStyle: "italic" }}>
                  {selected.suggestedActionsJson.replace(/^\["|"\]$|"|\\/g, "").replace(/","/g, "；")}
                </p>
              </div>
            )}
          </section>
        </aside>
      ) : (
        <InspectorPanel selectedHit={null} />
      )}
    </section>
  );
}
