/**
 * 本地埋点分析模块。
 * - 默认本地保存，便于离线和调试。
 * - 提供轻量去重，避免同类错误刷屏。
 * - 保留旧版 click 事件兼容迁移。
 */

export type AnalyticsOutcome = "click" | "start" | "success" | "failure" | "cancel" | "diagnostic";
export type AnalyticsMediaType = "image" | "video" | "audio" | "system" | "unknown";
export type EntitlementStatus = "free" | "trial" | "active" | "grace" | "expired";

const STORAGE_KEY = "hs_analytics_v2";
const LEGACY_STORAGE_KEY = "hs_analytics";
const DEDUPE_KEY = "hs_analytics_guard_v1";
const MAX_EVENTS = 500;
const MAX_GUARDS = 64;
const CLICK_DEDUPE_MS = 1_500;
const START_DEDUPE_MS = 10_000;
const SUCCESS_DEDUPE_MS = 10_000;
const FAILURE_DEDUPE_MS = 5 * 60_000;
const DIAGNOSTIC_DEDUPE_MS = 5 * 60_000;

export interface AnalyticsEvent {
  eventId: string;
  action: string;
  outcome: AnalyticsOutcome;
  timestamp: number;
  featureName?: string;
  mediaType?: AnalyticsMediaType;
  durationMs?: number;
  errorCode?: string;
  entitlementStatus?: EntitlementStatus;
  source?: string;
}

export interface AnalyticsSummary {
  totalClicks: number;
  firstClick: string | null;
  lastClick: string | null;
  clicksByDay: Record<string, number>;
}

export interface AnalyticsOverview {
  totalEvents: number;
  clickEvents: number;
  startEvents: number;
  successEvents: number;
  failureEvents: number;
  cancelEvents: number;
  diagnosticEvents: number;
  uniqueActions: number;
  lastEventAt: string | null;
  topActions: Array<{ action: string; total: number }>;
  conversionRate: number;
  failureRate: number;
}

export interface RiskSnapshot {
  level: "low" | "medium" | "high";
  reasons: string[];
  failureRate: number;
  retryPressure: number;
  repeatedErrorCount: number;
}

interface LegacyClickEvent {
  action: string;
  timestamp: number;
}

interface DedupeGuardEntry {
  fingerprint: string;
  timestamp: number;
}

interface AnalyticsContext {
  featureName?: string;
  mediaType?: AnalyticsMediaType;
  durationMs?: number;
  errorCode?: string;
  entitlementStatus?: EntitlementStatus;
  source?: string;
}

function canUseStorage(): boolean {
  return typeof window !== "undefined" && typeof localStorage !== "undefined";
}

function readJson<T>(key: string, fallback: T): T {
  if (!canUseStorage()) return fallback;
  try {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) as T : fallback;
  } catch {
    return fallback;
  }
}

function writeJson(key: string, value: unknown) {
  if (!canUseStorage()) return;
  localStorage.setItem(key, JSON.stringify(value));
}

function createEventId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `evt-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

function normalizeEvent(event: Partial<AnalyticsEvent> & { action: string; timestamp: number }): AnalyticsEvent {
  return {
    eventId: event.eventId ?? createEventId(),
    action: event.action,
    outcome: event.outcome ?? "click",
    timestamp: event.timestamp,
    featureName: event.featureName,
    mediaType: event.mediaType,
    durationMs: event.durationMs,
    errorCode: event.errorCode,
    entitlementStatus: event.entitlementStatus,
    source: event.source,
  };
}

function loadEvents(): AnalyticsEvent[] {
  const current = readJson<AnalyticsEvent[]>(STORAGE_KEY, []);
  if (current.length > 0) {
    return current.map((event) => normalizeEvent(event));
  }

  const legacy = readJson<LegacyClickEvent[]>(LEGACY_STORAGE_KEY, []);
  if (legacy.length === 0) return [];

  const migrated = legacy.map((event) => normalizeEvent({
    action: event.action,
    timestamp: event.timestamp,
    outcome: "click",
  }));
  writeJson(STORAGE_KEY, migrated);
  return migrated;
}

function saveEvents(events: AnalyticsEvent[]) {
  const trimmed = events.slice(-MAX_EVENTS);
  writeJson(STORAGE_KEY, trimmed);
}

function loadGuards(): DedupeGuardEntry[] {
  const guards = readJson<DedupeGuardEntry[]>(DEDUPE_KEY, []);
  return guards.filter((guard) => guard && typeof guard.fingerprint === "string" && typeof guard.timestamp === "number");
}

function saveGuards(guards: DedupeGuardEntry[]) {
  writeJson(DEDUPE_KEY, guards.slice(-MAX_GUARDS));
}

function dedupeWindowFor(outcome: AnalyticsOutcome): number {
  switch (outcome) {
    case "failure":
      return FAILURE_DEDUPE_MS;
    case "diagnostic":
      return DIAGNOSTIC_DEDUPE_MS;
    case "start":
      return START_DEDUPE_MS;
    case "success":
      return SUCCESS_DEDUPE_MS;
    default:
      return CLICK_DEDUPE_MS;
  }
}

function fingerprintFor(event: AnalyticsEvent): string {
  return [
    event.action,
    event.outcome,
    event.featureName ?? "",
    event.mediaType ?? "",
    event.errorCode ?? "",
    event.entitlementStatus ?? "",
    event.source ?? "",
  ].join("|");
}

function shouldRecord(event: AnalyticsEvent): boolean {
  if (!canUseStorage()) return true;

  const windowMs = dedupeWindowFor(event.outcome);
  const now = Date.now();
  const fingerprint = fingerprintFor(event);
  const guards = loadGuards().filter((guard) => now - guard.timestamp < 24 * 60 * 60 * 1000);
  const duplicate = guards.find((guard) => guard.fingerprint === fingerprint && now - guard.timestamp < windowMs);
  if (duplicate) {
    saveGuards(guards);
    return false;
  }

  guards.push({ fingerprint, timestamp: now });
  saveGuards(guards);
  return true;
}

function storeEvent(event: AnalyticsEvent) {
  if (!canUseStorage()) return;
  if (!shouldRecord(event)) return;
  const events = loadEvents();
  events.push(event);
  saveEvents(events);
}

/** 记录一次点击事件。 */
export function trackClick(action: string, context: AnalyticsContext = {}) {
  storeEvent({
    eventId: createEventId(),
    action,
    outcome: "click",
    timestamp: Date.now(),
    ...context,
  });
}

/** 记录一次功能埋点。 */
export function trackFeatureEvent(
  featureName: string,
  outcome: Exclude<AnalyticsOutcome, "click">,
  context: AnalyticsContext = {},
) {
  storeEvent({
    eventId: createEventId(),
    action: featureName,
    outcome,
    timestamp: Date.now(),
    featureName,
    ...context,
  });
}

/** 记录权益快照，避免重复上报时刷屏。 */
export function trackEntitlementSnapshot(status: EntitlementStatus, source = "app") {
  trackFeatureEvent("entitlement_snapshot", "diagnostic", {
    entitlementStatus: status,
    source,
    mediaType: "system",
  });
}

/** 获取指定 action 的点击统计摘要。 */
export function getClickSummary(action: string): AnalyticsSummary {
  const events = loadEvents().filter((e) => e.action === action);

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

/** 获取所有 action 的汇总。 */
export function getAllSummaries(): Record<string, AnalyticsSummary> {
  const events = loadEvents();
  const actions = [...new Set(events.map((e) => e.action))];
  const result: Record<string, AnalyticsSummary> = {};
  for (const action of actions) {
    result[action] = getClickSummary(action);
  }
  return result;
}

export function getAnalyticsOverview(): AnalyticsOverview {
  const events = loadEvents();
  const totalEvents = events.length;
  const clickEvents = events.filter((event) => event.outcome === "click").length;
  const startEvents = events.filter((event) => event.outcome === "start").length;
  const successEvents = events.filter((event) => event.outcome === "success").length;
  const failureEvents = events.filter((event) => event.outcome === "failure").length;
  const cancelEvents = events.filter((event) => event.outcome === "cancel").length;
  const diagnosticEvents = events.filter((event) => event.outcome === "diagnostic").length;
  const lastEventAt = events.length > 0 ? new Date(events[events.length - 1].timestamp).toLocaleString() : null;

  const actionCounts = new Map<string, number>();
  for (const event of events) {
    actionCounts.set(event.action, (actionCounts.get(event.action) ?? 0) + 1);
  }

  const topActions = [...actionCounts.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([action, total]) => ({ action, total }));

  const conversionRate = startEvents > 0 ? successEvents / startEvents : 0;
  const failureRate = (failureEvents + diagnosticEvents) / Math.max(totalEvents, 1);

  return {
    totalEvents,
    clickEvents,
    startEvents,
    successEvents,
    failureEvents,
    cancelEvents,
    diagnosticEvents,
    uniqueActions: actionCounts.size,
    lastEventAt,
    topActions,
    conversionRate,
    failureRate,
  };
}

export function getRiskSnapshot(): RiskSnapshot {
  const overview = getAnalyticsOverview();
  const repeatedErrorCount = countRepeatedErrors();
  const retryPressure = overview.failureEvents + overview.diagnosticEvents + overview.cancelEvents;
  const reasons: string[] = [];

  if (overview.failureRate >= 0.25) {
    reasons.push("失败/诊断占比偏高");
  }
  if (repeatedErrorCount >= 3) {
    reasons.push("存在重复错误聚集");
  }
  if (retryPressure >= 8) {
    reasons.push("重试与中断次数偏高");
  }

  let level: RiskSnapshot["level"] = "low";
  if (reasons.length >= 2 || overview.failureRate >= 0.35 || repeatedErrorCount >= 5) {
    level = "high";
  } else if (reasons.length >= 1 || overview.failureRate >= 0.15 || retryPressure >= 4) {
    level = "medium";
  }

  return {
    level,
    reasons,
    failureRate: overview.failureRate,
    retryPressure,
    repeatedErrorCount,
  };
}

function countRepeatedErrors(): number {
  const events = loadEvents().filter((event) => event.outcome === "failure" || event.outcome === "diagnostic");
  const grouped = new Map<string, number>();

  for (const event of events) {
    const key = [
      event.action,
      event.featureName ?? "",
      event.mediaType ?? "",
      event.errorCode ?? "",
      event.entitlementStatus ?? "",
    ].join("|");
    grouped.set(key, (grouped.get(key) ?? 0) + 1);
  }

  let repeated = 0;
  for (const count of grouped.values()) {
    if (count > 1) repeated += count - 1;
  }
  return repeated;
}
