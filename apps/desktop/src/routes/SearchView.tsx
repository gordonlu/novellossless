import { useState } from "react";
import { ChevronRight, FileSearch, Search } from "lucide-react";
import { searchProject, SearchHit } from "../tauri";
import { InspectorPanel } from "../components/InspectorPanel";

interface Props {
  projectId: string;
}

export function SearchView({ projectId }: Props) {
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
        <InspectorPanel selectedHit={selected} />
      </aside>
    </section>
  );
}
