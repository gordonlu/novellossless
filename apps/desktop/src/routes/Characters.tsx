import { useEffect, useState } from "react";
import { ChevronRight, UserRound } from "lucide-react";
import { clsx } from "clsx";
import { listCandidates, updateCandidateStatus, NarrativeNode } from "../tauri";
import { StatusButtons } from "../components/StatusButtons";
import { InspectorPanel } from "../components/InspectorPanel";
import { statusLabel } from "../lib/helpers";

interface Props {
  projectId: string;
}

export function Characters({ projectId }: Props) {
  const [characters, setCharacters] = useState<NarrativeNode[]>([]);
  const [selected, setSelected] = useState<NarrativeNode | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listCandidates(projectId, "person", 50).then(setCharacters);
    }
  }, [projectId]);

  const handleStatus = async (id: string, status: string) => {
    try {
      await updateCandidateStatus(id, status);
      const updated = await listCandidates(projectId, "person", 50);
      setCharacters(updated);
      setSelected((prev) => (prev?.id === id ? updated.find((c) => c.id === id) ?? null : prev));
    } catch {
      // silent
    }
  };

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading">
            <h2>人物卡</h2>
            <p>从正文提取的人物候选，共 {characters.length} 条</p>
          </div>
          <div className="compact-list">
            {characters.length > 0 ? (
              characters.map((c) => (
                <article
                  className={clsx("compact-item", selected?.id === c.id && "compact-item-active")}
                  key={c.id}
                  onClick={() => setSelected(c)}
                >
                  <div>
                    <strong>{c.name}</strong>
                    <p>出现 {c.occurrenceCount} 次 · {c.sourcePath} · {c.sourceTitle}</p>
                  </div>
                  <div className="row-actions">
                    <StatusButtons
                      onConfirm={() => handleStatus(c.id, "confirmed")}
                      onDismiss={() => handleStatus(c.id, "false_positive")}
                    />
                    <ChevronRight size={17} />
                  </div>
                </article>
              ))
            ) : (
              <div className="empty-state small">扫描后会显示人物候选。</div>
            )}
          </div>
        </section>
      </div>
      {selected ? (
        <aside className="inspector">
          <section className="panel">
            <div className="panel-heading compact">
              <h2>{selected.name}</h2>
              <UserRound size={22} />
            </div>
            <div className="evidence-meta">
              <div><span>类型</span><strong>人物</strong></div>
              <div><span>出现次数</span><strong>{selected.occurrenceCount}</strong></div>
              <div><span>置信度</span><strong>{selected.confidence}%</strong></div>
              <div><span>来源文件</span><strong>{selected.sourcePath}</strong></div>
              <div><span>首次出现</span><strong>{selected.sourceTitle}</strong></div>
              <div><span>状态</span><strong>{statusLabel(selected.status)}</strong></div>
            </div>
            <blockquote>{selected.sourceSnippet}</blockquote>
          </section>
        </aside>
      ) : (
        <InspectorPanel selectedHit={null} />
      )}
    </section>
  );
}
