import { useEffect, useState } from "react";
import { Clock, MapPin, Users, Rewind, ChevronRight } from "lucide-react";
import { clsx } from "clsx";
import { listTimelineEvents, TimelineEvent } from "../tauri";

interface Props {
  projectId: string;
}

export function Timeline({ projectId }: Props) {
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [selected, setSelected] = useState<TimelineEvent | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listTimelineEvents(projectId).then(setEvents);
    }
  }, [projectId]);

  const flashbackCount = events.filter((e) => e.isFlashback).length;

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading">
            <h2>时间线</h2>
            <p>共 {events.length} 个事件{flashbackCount > 0 ? `，其中 ${flashbackCount} 个倒叙` : ""}</p>
          </div>
          <div className="compact-list">
            {events.length > 0 ? (
              events.map((e) => (
                <article
                  className={clsx("compact-item", selected?.id === e.id && "compact-item-active")}
                  key={e.id}
                  onClick={() => setSelected(e)}
                >
                  <div className="compact-item-main">
                    <div className="compact-item-order">
                      <span className="order-badge">#{e.orderIndex}</span>
                    </div>
                    <div className="compact-item-body">
                      <strong>
                        {e.title}
                        {e.isFlashback && <span className="flashback-tag">倒叙</span>}
                      </strong>
                      <p>
                        {e.timeExpression && <><Clock size={12} /> {e.timeExpression} · </>}
                        {e.location && <><MapPin size={12} /> {e.location} · </>}
                        {e.documentPath}
                      </p>
                    </div>
                  </div>
                  <ChevronRight size={17} />
                </article>
              ))
            ) : (
              <div className="empty-state small">扫描后会显示提取的时间线事件。</div>
            )}
          </div>
        </section>
      </div>
      {selected ? (
        <aside className="inspector">
          <section className="panel">
            <div className="panel-heading compact">
              <h2>事件详情</h2>
              <Clock size={22} />
            </div>
            <div className="evidence-meta">
              <div><span>事件标题</span><strong>{selected.title}</strong></div>
              <div><span>时间顺序</span><strong>#{selected.orderIndex}</strong></div>
              <div><span>时间表述</span><strong>{selected.timeExpression || "无"}</strong></div>
              <div><span>地点</span><strong>{selected.location || "未指定"}</strong></div>
              <div><span>是否倒叙</span><strong>{selected.isFlashback ? "是" : "否"}</strong></div>
              <div><span>置信度</span><strong>{selected.confidence}%</strong></div>
              <div><span>来源文件</span><strong>{selected.documentPath}</strong></div>
              {selected.participantsJson && selected.participantsJson !== "[]" && (
                <div>
                  <span>参与者</span>
                  <strong>
                    <Users size={14} />
                    {(JSON.parse(selected.participantsJson) as string[]).join("、")}
                  </strong>
                </div>
              )}
              {selected.estimatedOrder != null && (
                <div><span>估计位置</span><strong>{selected.estimatedOrder}</strong></div>
              )}
            </div>
          </section>
        </aside>
      ) : (
        <aside className="inspector">
          <div className="panel">
            <div className="empty-state">
              <Rewind size={32} />
              <p>选择事件后查看详情。</p>
            </div>
          </div>
        </aside>
      )}
    </section>
  );
}