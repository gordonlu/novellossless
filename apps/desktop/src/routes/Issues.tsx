import { useEffect, useState } from "react";
import { AlertTriangle } from "lucide-react";
import { clsx } from "clsx";
import { listIssues, ContinuityIssue } from "../tauri";
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
                    <StatusButtons onConfirm={() => {}} onDismiss={() => {}} />
                  </div>
                </article>
              ))
            ) : (
              <div className="empty-state small">扫描后会显示重复表达和明确属性冲突。</div>
            )}
          </div>
        </section>
      </div>
      <aside className="inspector">
        {selected ? (
          <section className="panel">
            <div className="panel-heading compact"><h2>详情</h2><AlertTriangle size={22} /></div>
            <div className="evidence-meta">
              <div><span>类型</span><strong>{selected.issueType}</strong></div>
              <div><span>严重度</span><strong>{severityLabel(selected.severity)}</strong></div>
              <div><span>状态</span><strong>{selected.status}</strong></div>
            </div>
            <blockquote>{selected.description}</blockquote>
          </section>
        ) : (
          <InspectorPanel selectedHit={null} />
        )}
      </aside>
    </section>
  );
}
