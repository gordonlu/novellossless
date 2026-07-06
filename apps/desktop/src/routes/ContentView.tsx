import { useEffect, useState } from "react";
import { BookOpenText, ChevronRight } from "lucide-react";
import { getDocumentChunks, DocumentInfo, ChunkInfo } from "../tauri";

interface Props {
  projectId: string;
}

export function ContentView({ projectId }: Props) {
  const [documents, setDocuments] = useState<DocumentInfo[]>([]);
  const [selectedDoc, setSelectedDoc] = useState<string | null>(null);
  const [chunks, setChunks] = useState<ChunkInfo[]>([]);
  const [selectedChunk, setSelectedChunk] = useState<ChunkInfo | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      getDocumentChunks(projectId).then((tree) => {
        setDocuments(tree.documents);
        setChunks(tree.chunks);
      });
    }
  }, [projectId]);

  const filteredChunks = selectedDoc
    ? chunks.filter((c) => c.documentId === selectedDoc)
    : chunks;

  return (
    <section className="content-grid">
      <div className="primary-column">
        <div className="panel">
          <div className="panel-heading"><h2>正文</h2></div>
          <div className="doc-list">
            {documents.map((doc) => (
              <button
                key={doc.id}
                className={`doc-item ${selectedDoc === doc.id ? "doc-item-active" : ""}`}
                onClick={() => setSelectedDoc(doc.id)}
              >
                <BookOpenText size={16} />
                <span>{doc.title}</span>
                <small>{doc.chapterCount} 章</small>
              </button>
            ))}
          </div>
          <div className="chunk-list">
            {filteredChunks.map((chunk) => (
              <button
                key={chunk.id}
                className={`chunk-item ${selectedChunk?.id === chunk.id ? "chunk-item-active" : ""}`}
                onClick={() => setSelectedChunk(chunk)}
              >
                {chunk.title}
                <ChevronRight size={14} />
              </button>
            ))}
          </div>
        </div>
      </div>
      <aside className="inspector">
        {selectedChunk ? (
          <section className="panel">
            <h2>{selectedChunk.title}</h2>
            <div className="evidence-meta">
              <div><span>位置</span><strong>{selectedChunk.startOffset}</strong></div>
              <div><span>字数</span><strong>{selectedChunk.wordCount}</strong></div>
            </div>
            <div className="content-text">{selectedChunk.content}</div>
          </section>
        ) : (
          <div className="empty-state">选择章节后查看正文。</div>
        )}
      </aside>
    </section>
  );
}
