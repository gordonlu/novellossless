import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { ChevronRight, FileSearch, Search } from "lucide-react";
import { createTask, searchProject, SearchHit } from "../tauri";
import { InspectorPanel } from "../components/InspectorPanel";

interface Props {
  projectId: string;
}

export function SearchView({ projectId }: Props) {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [selected, setSelected] = useState<SearchHit | null>(null);
  const [loading, setLoading] = useState(false);
  const [attempted, setAttempted] = useState(false);

  async function handleSearch() {
    const trimmed = query.trim();
    if (!trimmed || !projectId || projectId === "demo") return;
    setLoading(true);
    setAttempted(true);
    try {
      const results = await searchProject(projectId, trimmed, 20);
      setHits(results);
      setSelected(results[0] ?? null);
    } finally {
      setLoading(false);
    }
  }

  const handleRevealSource = () => {
    if (!selected) return;
    navigate("/content", { state: { revealDocId: selected.documentId, revealChunkId: selected.chunkId } });
  };

  const handleCreateTask = async () => {
    if (!selected) return;
    try {
      const title = `跟进：${selected.title}`;
      const relatedChunks = JSON.stringify([selected.chunkId]);
      const notes = `来源：${selected.documentPath}\n摘要：${selected.snippet}`;
      await createTask(projectId, title, "followup", "medium", relatedChunks, notes);
      alert("任务已创建。");
    } catch {
      alert("创建任务失败。");
    }
  };

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel search-panel">
          <div className="panel-heading compact">
            <h2>全文搜索</h2>
            <FileSearch size={22} />
          </div>
          <div className="search-row">
            <Search size={18} />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
              placeholder="输入人物、物件、地点或句子"
            />
            <button onClick={handleSearch} disabled={loading}>
              {loading ? "搜索中" : "搜索"}
            </button>
          </div>
          <div className="results-list">
            {hits.length > 0 ? (
              hits.map((hit) => (
                <button
                  type="button"
                  key={hit.chunkId}
                  className={`result-item ${selected?.chunkId === hit.chunkId ? "result-item-active" : ""}`}
                  onClick={() => setSelected(hit)}
                >
                  <div>
                    <strong>{hit.title}</strong>
                    <span className="result-meta">{hit.documentPath} · 第 {hit.chunkIndex + 1} 段</span>
                    <p>{hit.snippet}</p>
                  </div>
                  <ChevronRight size={17} />
                </button>
              ))
            ) : (
              <div className="empty-state">
                <strong>{attempted ? "没有匹配片段" : "等待搜索"}</strong>
                <p>输入人物、物件、地点或原文句子后，可在右侧查看来源证据。</p>
              </div>
            )}
          </div>
        </section>
      </div>
      <aside className="inspector">
        <InspectorPanel
          selectedHit={selected}
          onRevealSource={handleRevealSource}
          onCreateTask={handleCreateTask}
          onDismiss={() => setSelected(null)}
        />
      </aside>
    </section>
  );
}
