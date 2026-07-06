import { useEffect, useState } from "react";
import { Network } from "lucide-react";
import { clsx } from "clsx";
import { listForeshadows, ForeshadowItem } from "../tauri";
import { StatusButtons } from "../components/StatusButtons";
import { InspectorPanel } from "../components/InspectorPanel";
import { riskLabel } from "../lib/helpers";

const riskColors: Record<string, string> = {
  high: "risk-high",
  medium: "risk-medium",
  low: "risk-low",
};

interface Props {
  projectId: string;
}

export function Foreshadows({ projectId }: Props) {
  const [items, setItems] = useState<ForeshadowItem[]>([]);
  const [selected, setSelected] = useState<ForeshadowItem | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listForeshadows(projectId, 50).then(setItems);
    }
  }, [projectId]);

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading"><h2>伏笔账本</h2><p>共 {items.length} 条</p></div>
          <div className="compact-list">
            {items.length > 0 ? (
              items.map((item) => (
                <article
                  className={clsx("compact-item", selected?.id === item.id && "compact-item-active")}
                  key={item.id}
                  onClick={() => setSelected(item)}
                >
                  <div>
                    <strong>{item.title}</strong>
                    <p>
                      <span className={riskColors[item.riskLevel] ?? ""}>
                        {riskLabel(item.riskLevel)}
                      </span>
                      {" · "}{item.sourcePath} · {item.evidence.slice(0, 60)}
                    </p>
                  </div>
                  <div className="row-actions">
                    <StatusButtons onConfirm={() => {}} onDismiss={() => {}} />
                  </div>
                </article>
              ))
            ) : (
              <div className="empty-state small">扫描后会显示伏笔和线索。</div>
            )}
          </div>
        </section>
      </div>
      {selected ? (
        <aside className="inspector">
          <section className="panel">
            <div className="panel-heading compact"><h2>详情</h2><Network size={22} /></div>
            <div className="evidence-meta">
              <div><span>类型</span><strong>{selected.foreshadowType}</strong></div>
              <div><span>风险</span><strong>{riskLabel(selected.riskLevel)}</strong></div>
              <div><span>状态</span><strong>{selected.status}</strong></div>
              <div><span>来源</span><strong>{selected.sourcePath}</strong></div>
              <div><span>章节</span><strong>{selected.sourceTitle}</strong></div>
            </div>
            <blockquote>{selected.evidence}</blockquote>
          </section>
        </aside>
      ) : (
        <InspectorPanel selectedHit={null} />
      )}
    </section>
  );
}
