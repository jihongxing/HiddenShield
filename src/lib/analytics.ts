/**
 * 本地埋点分析模块 — 零依赖，数据存 localStorage。
 * 用于 MVP 阶段测试用户付费意愿（按钮点击率）。
 */

const STORAGE_KEY = "hs_analytics";

export interface ClickEvent {
  action: string;
  timestamp: number;
}

export interface AnalyticsSummary {
  totalClicks: number;
  firstClick: string | null;
  lastClick: string | null;
  clicksByDay: Record<string, number>;
}

function getEvents(): ClickEvent[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveEvents(events: ClickEvent[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(events));
}

/** 记录一次点击事件 */
// TODO: 接入远程上报服务时，在此函数内追加一行 fetch 将事件发送到后端统计接口
export function trackClick(action: string) {
  const events = getEvents();
  events.push({ action, timestamp: Date.now() });
  saveEvents(events);
}

/** 获取指定 action 的点击统计摘要 */
export function getClickSummary(action: string): AnalyticsSummary {
  const events = getEvents().filter((e) => e.action === action);

  if (events.length === 0) {
    return { totalClicks: 0, firstClick: null, lastClick: null, clicksByDay: {} };
  }

  const clicksByDay: Record<string, number> = {};
  for (const e of events) {
    const day = new Date(e.timestamp).toISOString().slice(0, 10);
    clicksByDay[day] = (clicksByDay[day] || 0) + 1;
  }

  return {
    totalClicks: events.length,
    firstClick: new Date(events[0].timestamp).toLocaleString(),
    lastClick: new Date(events[events.length - 1].timestamp).toLocaleString(),
    clicksByDay,
  };
}

/** 获取所有 action 的汇总 */
export function getAllSummaries(): Record<string, AnalyticsSummary> {
  const events = getEvents();
  const actions = [...new Set(events.map((e) => e.action))];
  const result: Record<string, AnalyticsSummary> = {};
  for (const action of actions) {
    result[action] = getClickSummary(action);
  }
  return result;
}
