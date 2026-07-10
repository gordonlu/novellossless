import { useEffect, useState } from "react";
import { History, ChevronRight, RefreshCw, Play, Square } from "lucide-react";
import { getDocumentChunks, listFileScans, listRevisions, incrementalScan, startWatching, stopWatching, watcherStatus, FileScanLog, RevisionRecord } from "../tauri";

interface Props {
  projectId: string;
}

const eventLabels: Record<string, string> = {
  created: "新建", modified: "修改", unchanged: "未变",
  deleted: "删除", failed: "失败",
};

const eventColors: Record<string, string> = {
  created: "evt-created", modified: "evt-modified",
  unchanged: "evt-unchanged", deleted: "evt-deleted", failed: "evt-failed",
};

export function RevisionHistory({ projectId }: Props) {
  const [scans, setScans] = useState<FileScanLog[]>([]);
  const [revisions, setRevisions] = useState<RevisionRecord[]>([]);
  const [selectedDoc, setSelectedDoc] = useState<string | null>(null);
  const [watching, setWatching] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [docTitles, setDocTitles] = useState<Record<string, string>>({});

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listFileScans(projectId, 100).then((scans) => {
        setScans(scans);
        getDocumentChunks(projectId).then((tree) => {
          const map: Record<string, string> = {};
          for (const doc of tree.documents) {
            map[doc.id] = doc.title || doc.path;
          }
          setDocTitles(map);
        });
      });
      watcherStatus().then(setWatching);
    }
  }, [projectId]);

  const handleSelectDoc = (docId: string) => {
    setSelectedDoc(docId);
    listRevisions(projectId, docId, 50).then(setRevisions);
  };

  const handleIncrementalScan = async () => {
    setScanning(true);
    await incrementalScan(projectId);
    const newScans = await listFileScans(projectId, 100);
    setScans(newScans);
    setScanning(false);
  };

  const toggleWatcher = async () => {
    if (watching) {
      await stopWatching();
      setWatching(false);
    } else {
      await startWatching(projectId);
      setWatching(true);
    }
  };

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading">
            <h2>改稿历史</h2>
            <p>共 {scans.length} 条扫描记录</p>
          </div>
          <div className="scan-toolbar">
            <button className="primary-button" onClick={handleIncrementalScan} disabled={scanning}>
              <RefreshCw size={15} />{scanning ? "扫描中..." : "增量扫描"}
            </button>
            <button className={`secondary-button ${watching ? "watching" : ""}`} onClick={toggleWatcher}>
              {watching ? <><Square size={15} /> 停止监听</> : <><Play size={15} /> 开始监听</>}
            </button>
          </div>
          <div className="compact-list">
            {scans.length > 0 ? scans.map((s) => (
              <article
                key={s.id}
                className={`compact-item ${selectedDoc === s.documentId ? "compact-item-active" : ""}`}
                onClick={() => handleSelectDoc(s.documentId)}
              >
                <div>
                  <strong>
                    {docTitles[s.documentId] ?? s.documentId.slice(0, 8)}
                    <span className={eventColors[s.eventType] ?? ""}>{eventLabels[s.eventType] ?? s.eventType}</span>
                  </strong>
                  <p>{s.eventType === "modified" ? `${s.oldHash?.slice(0, 8)} → ${s.newHash.slice(0, 8)}` : s.newHash.slice(0, 16)} · {s.scannedAt}</p>
                </div>
                <ChevronRight size={17} />
              </article>
            )) : (
              <div className="empty-state small">尚未扫描。</div>
            )}
          </div>
        </section>
      </div>
      <aside className="inspector">
        {selectedDoc && revisions.length > 0 ? (
          <section className="panel">
            <div className="panel-heading compact">
              <h2>修订详情</h2>
              <History size={22} />
            </div>
            {revisions.map((rev) => (
              <div key={rev.id} className="revision-card">
                <div className="revision-meta">
                  <span>{rev.revisionType}</span>
                  <strong>{new Date(rev.createdAt).toLocaleString("zh-CN")}</strong>
                </div>
                <div className="revision-stats">
                  <span>新增 {rev.chunksAdded} 段</span>
                  <span>删除 {rev.chunksRemoved} 段</span>
                  <span>修改 {rev.chunksModified} 段</span>
                </div>
                {rev.diffJson && <pre className="revision-diff">{rev.diffJson}</pre>}
              </div>
            ))}
          </section>
        ) : (
          <div className="empty-state">选择一个文档查看修订详情。</div>
        )}
      </aside>
    </section>
  );
}
