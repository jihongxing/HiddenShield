import type { FormalReportExportResult } from "./tauri-api";

const STORAGE_KEY = "hs_formal_report_exports_v3";
const MAX_RECENT_REPORT_EXPORTS = 5;

function canUseStorage(): boolean {
  return typeof window !== "undefined" && typeof localStorage !== "undefined";
}

function normalizeReportExport(value: unknown): FormalReportExportResult | null {
  if (!value || typeof value !== "object") return null;
  const item = value as Partial<FormalReportExportResult>;
  if (
    typeof item.reportId !== "string" ||
    typeof item.reportType !== "string" ||
    typeof item.reportDir !== "string" ||
    typeof item.pdfPath !== "string" ||
    typeof item.jsonPath !== "string" ||
    typeof item.manifestPath !== "string" ||
    typeof item.exportedAt !== "string" ||
    typeof item.recordCount !== "number" ||
    typeof item.pdfGenerationMs !== "number" ||
    typeof item.pdfPageCount !== "number" ||
    typeof item.bundleVersion !== "number" ||
    (item.supersedesReportId !== null && typeof item.supersedesReportId !== "string")
  ) {
    return null;
  }

  return {
    reportId: item.reportId,
    reportType: item.reportType,
    reportDir: item.reportDir,
    pdfPath: item.pdfPath,
    jsonPath: item.jsonPath,
    manifestPath: item.manifestPath,
    exportedAt: item.exportedAt,
    recordCount: item.recordCount,
    pdfGenerationMs: item.pdfGenerationMs,
    pdfPageCount: item.pdfPageCount,
    bundleVersion: item.bundleVersion,
    supersedesReportId: item.supersedesReportId,
  };
}

export function loadRecentReportExports(): FormalReportExportResult[] {
  if (!canUseStorage()) return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map((item) => normalizeReportExport(item))
      .filter((item): item is FormalReportExportResult => item !== null)
      .slice(0, MAX_RECENT_REPORT_EXPORTS);
  } catch {
    return [];
  }
}

export function saveRecentReportExport(result: FormalReportExportResult): FormalReportExportResult[] {
  const next = [
    result,
    ...loadRecentReportExports().filter((item) => item.reportId !== result.reportId),
  ].slice(0, MAX_RECENT_REPORT_EXPORTS);

  if (canUseStorage()) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  }

  return next;
}
