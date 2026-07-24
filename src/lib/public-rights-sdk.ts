import {
  fetchPublicRights,
  fetchPublicRightsBatch,
  type PublicRightsBatchItem,
  type PublicRightsQueryResponse,
} from "./tauri-api";

export type PublicRightsSdkStatus = "ok" | "error";

export type PublicRightsSdkErrorCode =
  | "not_found"
  | "registry_unavailable"
  | "payload_invalid"
  | "manifest_conflict"
  | "backfill_pending"
  | "backfill_disputed"
  | "internal_error";

export interface PublicRightsPolicyResolution {
  trainingPolicy: string;
  trainingPolicyLabel: string;
  rightsManifestStatus: string;
  registryStatus: string;
  scanStatus: string;
  legalConclusion: false;
  requiresHumanReview: boolean;
  canTreatAsTrainingAllowed: false;
}

export interface PublicRightsSdkResult {
  status: PublicRightsSdkStatus;
  scan: PublicRightsQueryResponse | null;
  error: PublicRightsSdkErrorCode | null;
  warnings: string[];
  message: string;
  policy: PublicRightsPolicyResolution | null;
}

export interface PublicRightsBatchSdkItem extends PublicRightsSdkResult {
  watermarkUid: string;
}

export interface PublicRightsScanner {
  scanOne(watermarkUid: string): Promise<PublicRightsSdkResult>;
  scanBatch(watermarkUids: string[]): Promise<PublicRightsBatchSdkItem[]>;
  resolvePolicy(scanResult: PublicRightsQueryResponse): PublicRightsPolicyResolution;
  formatUserMessage(result: PublicRightsSdkResult): string;
}

export function createPublicRightsScanner(baseUrl: string): PublicRightsScanner {
  return {
    async scanOne(watermarkUid: string) {
      const uid = watermarkUid.trim();
      if (!uid) {
        return buildErrorResult(null, "not_found");
      }
      try {
        return buildOkResult(await fetchPublicRights(baseUrl, uid));
      } catch (error) {
        return buildErrorResult(null, classifyPublicRightsError(error));
      }
    },

    async scanBatch(watermarkUids: string[]) {
      const uniqueUids = [...new Set(watermarkUids.map((uid) => uid.trim()).filter(Boolean))];
      if (uniqueUids.length === 0) return [];
      try {
        const batch = await fetchPublicRightsBatch(baseUrl, uniqueUids);
        return batch.results.map(batchItemToSdkItem);
      } catch (error) {
        const code = classifyPublicRightsError(error);
        return uniqueUids.map((watermarkUid) => ({
          watermarkUid,
          ...buildErrorResult(null, code),
        }));
      }
    },

    resolvePolicy,

    formatUserMessage,
  };
}

export function resolvePolicy(scanResult: PublicRightsQueryResponse): PublicRightsPolicyResolution {
  const warnings = new Set(scanResult.warnings);
  const rightsManifestStatus = scanResult.rightsManifest?.status ?? "missing";
  return {
    trainingPolicy: scanResult.trainingPermission.policy,
    trainingPolicyLabel: scanResult.trainingPermission.label,
    rightsManifestStatus,
    registryStatus: scanResult.registry.registryStatus,
    scanStatus: scanResult.scanStatus,
    legalConclusion: false,
    requiresHumanReview:
      scanResult.scanStatus === "backfill_disputed" ||
      rightsManifestStatus === "disputed" ||
      warnings.has("registry_requires_human_review"),
    canTreatAsTrainingAllowed: false,
  };
}

export function formatUserMessage(result: PublicRightsSdkResult): string {
  if (result.status === "error") {
    return messageForError(result.error ?? "internal_error");
  }
  const scan = result.scan;
  if (!scan) return messageForError("internal_error");
  if (result.warnings.includes("backfill_pending")) {
    return "登记记录已找到，但公开权利 manifest 尚未完成回填。";
  }
  if (scan.scanStatus === "backfill_disputed") {
    return "公开权利声明需要人工处理，请以 registry 和人工核验为准。";
  }
  if (scan.scanStatus === "registry_revoked") {
    return "该公开权利声明已撤销，请不要按旧声明直接使用。";
  }
  if (scan.scanStatus === "registry_superseded") {
    return "该公开权利声明已有新版，请查看最新 registry 记录。";
  }
  if (scan.scanStatus === "watermark_only") {
    return "仅发现水印锚点，尚未查询到 active 公开权利 manifest。";
  }
  return "已读取创作者声明与 registry 快照；该结果不是法律授权结论。";
}

export function classifyPublicRightsError(error: unknown): PublicRightsSdkErrorCode {
  const message = String(error instanceof Error ? error.message : error).toLowerCase();
  if (message.includes("http 404") || message.includes("missing") || message.includes("not_found")) {
    return "not_found";
  }
  if (message.includes("http 401") || message.includes("http 403") || message.includes("unavailable")) {
    return "registry_unavailable";
  }
  if (message.includes("payload_invalid") || message.includes("auth")) {
    return "payload_invalid";
  }
  if (message.includes("conflict") || message.includes("disputed")) {
    return "manifest_conflict";
  }
  return "internal_error";
}

function batchItemToSdkItem(item: PublicRightsBatchItem): PublicRightsBatchSdkItem {
  if (item.status === "ok" && item.result) {
    return {
      watermarkUid: item.watermarkUid,
      ...buildOkResult(item.result),
    };
  }
  return {
    watermarkUid: item.watermarkUid,
    ...buildErrorResult(null, normalizeBatchErrorCode(item.errorCode)),
  };
}

function buildOkResult(scan: PublicRightsQueryResponse): PublicRightsSdkResult {
  const policy = resolvePolicy(scan);
  const error = errorCodeFromScan(scan);
  const status: PublicRightsSdkStatus = error ? "error" : "ok";
  const result: PublicRightsSdkResult = {
    status,
    scan,
    error,
    warnings: scan.warnings,
    message: "",
    policy,
  };
  result.message = formatUserMessage(result);
  return result;
}

function buildErrorResult(
  scan: PublicRightsQueryResponse | null,
  error: PublicRightsSdkErrorCode,
): PublicRightsSdkResult {
  const result: PublicRightsSdkResult = {
    status: "error",
    scan,
    error,
    warnings: scan?.warnings ?? [],
    message: "",
    policy: scan ? resolvePolicy(scan) : null,
  };
  result.message = formatUserMessage(result);
  return result;
}

function errorCodeFromScan(scan: PublicRightsQueryResponse): PublicRightsSdkErrorCode | null {
  if (scan.warnings.includes("backfill_pending")) return "backfill_pending";
  if (scan.scanStatus === "backfill_disputed") return "backfill_disputed";
  if (scan.scanStatus === "metadata_registry_conflict") return "manifest_conflict";
  if (scan.registry.payloadAuthStatus === "failed" || scan.registry.payloadAuthStatus === "invalid") {
    return "payload_invalid";
  }
  return null;
}

function normalizeBatchErrorCode(value: string | null): PublicRightsSdkErrorCode {
  switch (value) {
    case "not_found":
    case "registry_unavailable":
    case "payload_invalid":
    case "manifest_conflict":
    case "backfill_pending":
    case "backfill_disputed":
    case "internal_error":
      return value;
    case "bad_request":
      return "not_found";
    default:
      return "internal_error";
  }
}

function messageForError(error: PublicRightsSdkErrorCode): string {
  switch (error) {
    case "not_found":
      return "未找到公开 registry 记录。";
    case "registry_unavailable":
      return "公开 registry 暂不可用，请稍后重试。";
    case "payload_invalid":
      return "水印锚点认证失败，不能据此判断权利状态。";
    case "manifest_conflict":
      return "公开元数据与 registry 声明存在冲突，请人工核验。";
    case "backfill_pending":
      return "公开权利 manifest 尚未完成回填。";
    case "backfill_disputed":
      return "公开权利声明需要人工处理。";
    case "internal_error":
      return "公开权利查询暂时失败。";
  }
}
