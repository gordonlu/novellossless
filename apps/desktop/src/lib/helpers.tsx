import type { Dashboard, SearchHit } from "../tauri";

export function totalCandidateCount(dashboard: Dashboard) {
  return dashboard.personCandidates + dashboard.placeCandidates + dashboard.itemCandidates;
}

export function basename(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

export function formatError(reason: unknown) {
  const message = reason instanceof Error ? reason.message : String(reason);
  return message.startsWith("操作失败") ? message : `操作失败：${message}`;
}

export function candidateTypeLabel(nodeType: string) {
  if (nodeType === "person") return "人物";
  if (nodeType === "place") return "地点";
  if (nodeType === "item") return "物件";
  return "候选";
}

export function riskLabel(risk: string) {
  if (risk === "high") return "高风险";
  if (risk === "medium") return "中风险";
  if (risk === "low") return "低风险";
  return "待确认";
}

export function statusLabel(status: string) {
  if (status === "confirmed") return "已确认";
  if (status === "dismissed") return "误报";
  return "候选";
}

export function severityLabel(severity: string) {
  if (severity === "serious") return "严重";
  if (severity === "high") return "高";
  if (severity === "medium") return "中";
  if (severity === "low") return "低";
  return "信息";
}

export function sourceLabel(hit: SearchHit) {
  return `${hit.documentPath} · 第 ${hit.chunkIndex + 1} 段`;
}

export function plainSnippet(snippet: string) {
  return snippet.replace(/\[/g, "").replace(/\]/g, "");
}

export function renderSnippet(snippet: string) {
  const segments = snippet.split(/(\[[^\]]+\])/g);
  return segments.map((segment, index) => {
    if (segment.startsWith("[") && segment.endsWith("]")) {
      return <mark key={`${segment}-${index}`}>{segment.slice(1, -1)}</mark>;
    }
    return <span key={`${segment}-${index}`}>{segment}</span>;
  });
}
