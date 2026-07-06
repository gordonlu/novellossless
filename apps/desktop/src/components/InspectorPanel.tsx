import { AlertTriangle, ListChecks, LockKeyhole, Sparkles } from "lucide-react";
import type { ProfileInfo, PrivacyStatus, SearchHit } from "../tauri";
import { basename, plainSnippet } from "../lib/helpers";

interface InspectorPanelProps {
  selectedHit: SearchHit | null;
  onRevealSource: () => void;
  issuesCount: number;
  privacy: PrivacyStatus;
  profiles: ProfileInfo[];
}

export function InspectorPanel({ selectedHit, onRevealSource, issuesCount, privacy, profiles }: InspectorPanelProps) {
  return (
    <aside className="inspector">
      <section className="panel evidence-panel">
        <div className="panel-heading compact">
          <div>
            <h2>来源证据</h2>
            <p>所有提醒都必须能回到正文。</p>
          </div>
          <ListChecks size={22} />
        </div>

        <div className="evidence-source">{selectedHit?.title ?? "未选择片段"}</div>
        {selectedHit && (
          <div className="evidence-meta">
            <div>
              <span>来源文件</span>
              <strong>{selectedHit.documentPath}</strong>
            </div>
            <div>
              <span>片段位置</span>
              <strong>
                第 {selectedHit.chunkIndex + 1} 段 · {selectedHit.startOffset}-{selectedHit.endOffset}
              </strong>
            </div>
          </div>
        )}
        <blockquote>{selectedHit ? plainSnippet(selectedHit.snippet) : "选择搜索结果后查看来源。"}</blockquote>

        <div className="evidence-actions">
          <button
            type="button"
            className="primary-button full"
            disabled={!selectedHit}
            onClick={onRevealSource}
          >
            查看来源
          </button>
          <button type="button" className="secondary-button full" disabled={!selectedHit}>
            标记误报
          </button>
          <button type="button" className="secondary-button full" disabled={!selectedHit}>
            创建任务
          </button>
        </div>
      </section>

      <section className="panel risk-panel">
        <div className="risk-header">
          <AlertTriangle size={18} />
          <span>今日重点</span>
        </div>
        <div className="risk-item amber">
          <strong>{issuesCount > 0 ? "有待确认问题" : "等待扫描"}</strong>
          <p>
            {issuesCount > 0
              ? `当前有 ${issuesCount} 条基础问题需要确认。`
              : "导入并扫描项目后，这里会显示高风险一致性问题。"}
          </p>
        </div>
        <div className="risk-item teal">
          <strong>上下文准备</strong>
          <p>搜索片段可以作为后续上下文包的证据来源。</p>
        </div>
      </section>

      <section className="panel privacy-panel">
        <div className="panel-heading compact">
          <div>
            <h2>隐私中心</h2>
            <p>默认只使用本机数据。</p>
          </div>
          <LockKeyhole size={22} />
        </div>
        <div className="privacy-list">
          <PrivacyRow label="离线模式" value={privacy.offlineMode ? "开启" : "关闭"} />
          <PrivacyRow label="AI 增强" value={privacy.aiEnabled ? "开启" : "关闭"} />
          <PrivacyRow label="上传正文" value={privacy.uploadsEnabled ? "允许" : "不允许"} />
          <PrivacyRow label="剪贴板读取" value={privacy.clipboardAccess ? "允许" : "不读取"} />
          <PrivacyRow label="本地存储" value={privacy.storageMode} />
          <PrivacyRow label="数据库" value={basename(privacy.databasePath)} />
        </div>
      </section>

      <section className="panel mode-panel">
        <div className="mode-icon">
          <Sparkles size={18} />
        </div>
        <div>
          <strong>{profiles[0]?.name ?? "通用长篇模式"}</strong>
          <p>{profiles[0]?.description || "已启用章节识别、全文搜索、证据保留。"}</p>
        </div>
      </section>
    </aside>
  );
}

function PrivacyRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="privacy-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
