import { useEffect, useState } from "react";
import { Package, Plus, Trash2, RefreshCw, ChevronRight, FileDown } from "lucide-react";
import { clsx } from "clsx";
import { listContextPacks, deleteContextPack, buildContextPack, generateMarkdownReport } from "../tauri";
import type { ContextPack as ContextPackData } from "../tauri";

interface Props {
  projectId: string;
}

export function ContextPack({ projectId }: Props) {
  const [packs, setPacks] = useState<ContextPackData[]>([]);
  const [selected, setSelected] = useState<ContextPackData | null>(null);
  const [query, setQuery] = useState("");
  const [building, setBuilding] = useState(false);
  const [generating, setGenerating] = useState(false);

  const loadPacks = () => {
    if (projectId && projectId !== "demo") {
      listContextPacks(projectId).then(setPacks);
    }
  };

  useEffect(loadPacks, [projectId]);

  const handleBuild = async () => {
    if (!query.trim()) return;
    setBuilding(true);
    try {
      const pack = await buildContextPack(projectId, query.trim(), 10);
      setPacks((prev) => [pack, ...prev]);
      setSelected(pack);
      setQuery("");
    } catch {
      // silent
    }
    setBuilding(false);
  };

  const handleGenerateReport = async () => {
    setGenerating(true);
    try {
      const pack = await generateMarkdownReport(projectId);
      setPacks((prev) => [pack, ...prev]);
      setSelected(pack);
    } catch {
      // silent
    }
    setGenerating(false);
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteContextPack(id);
      setPacks((prev) => prev.filter((p) => p.id !== id));
      if (selected?.id === id) setSelected(null);
    } catch {
      // silent
    }
  };

  const handleDownload = () => {
    if (!selected) return;
    const blob = new Blob([selected.content], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${selected.title}.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading">
            <h2>上下文包</h2>
            <p>共 {packs.length} 个包</p>
          </div>

          <div className="context-pack-toolbar" style={{ display: "flex", gap: "0.5rem", padding: "0.75rem 1rem", flexWrap: "wrap" }}>
            <input
              type="text"
              placeholder="输入关键词生成上下文包..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleBuild()}
              style={{ flex: 1, minWidth: 180 }}
            />
            <button className="primary-button" onClick={handleBuild} disabled={building || !query.trim()}>
              <Plus size={15} />{building ? "生成中..." : "生成"}
            </button>
            <button className="secondary-button" onClick={handleGenerateReport} disabled={generating}>
              <RefreshCw size={15} />{generating ? "生成中..." : "分析报告"}
            </button>
          </div>

          <div className="compact-list">
            {packs.length > 0 ? (
              packs.map((p) => (
                <article
                  className={clsx("compact-item", selected?.id === p.id && "compact-item-active")}
                  key={p.id}
                  onClick={() => setSelected(p)}
                >
                  <div>
                    <strong>{p.title}</strong>
                    <p>{p.target !== "report" ? `查询：${p.target}` : "项目分析报告"} · {p.createdAt}</p>
                  </div>
                  <div className="row-actions">
                    <button
                      className="icon-button"
                      onClick={(e) => { e.stopPropagation(); handleDelete(p.id); }}
                      title="删除"
                    >
                      <Trash2 size={15} />
                    </button>
                    <ChevronRight size={17} />
                  </div>
                </article>
              ))
            ) : (
              <div className="empty-state small">还没有上下文包。输入关键词生成一个。</div>
            )}
          </div>
        </section>
      </div>
      {selected ? (
        <aside className="inspector">
          <section className="panel">
            <div className="panel-heading compact">
              <h2>{selected.title}</h2>
              <button className="icon-button" onClick={handleDownload} title="下载 Markdown">
                <FileDown size={18} />
              </button>
            </div>
            <div className="evidence-meta">
              <div><span>格式</span><strong>{selected.format}</strong></div>
              <div><span>生成时间</span><strong>{selected.createdAt}</strong></div>
            </div>
            <pre className="context-preview">{selected.content}</pre>
          </section>
        </aside>
      ) : (
        <aside className="inspector">
          <div className="panel">
            <div className="empty-state">
              <Package size={32} />
              <p>选择一个上下文包查看内容。</p>
            </div>
          </div>
        </aside>
      )}
    </section>
  );
}