export type PublicRightsSdkStatus = "ok" | "error";

export type PublicRightsSdkErrorCode =
  | "not_found"
  | "registry_unavailable"
  | "payload_invalid"
  | "manifest_conflict"
  | "backfill_pending"
  | "backfill_disputed"
  | "rate_limited"
  | "quota_exhausted"
  | "scope_denied"
  | "api_key_invalid"
  | "internal_error";

export interface PublicRightsRegistrySnapshot {
  registryStatus: string;
  registryProofHash: string;
  payloadAuthStatus: string;
  anchorProtocol: string;
}

export interface PublicTrainingPermissionSnapshot {
  policy: string;
  label: string;
  source: string;
  effectiveSource: string;
  legalConclusion: false;
}

export interface PublicRightsQueryResponse {
  watermarkUid: string;
  scanStatus: string;
  registry: PublicRightsRegistrySnapshot;
  rightsManifest?: { status: string } | null;
  trainingPermission: PublicTrainingPermissionSnapshot;
  warnings: string[];
  resolvedAt: string;
}

export interface PublicRightsBatchItem {
  watermarkUid: string;
  status: string;
  errorCode: string | null;
  result?: PublicRightsQueryResponse | null;
  resolvedAt: string;
}

export interface PublicRightsBatchResponse {
  results: PublicRightsBatchItem[];
  resolvedAt: string;
}

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

export interface PublicRightsScannerOptions {
  baseUrl: string;
  apiKey?: string;
  fetchImpl?: typeof fetch;
}

export interface PublicRightsScanner {
  scanOne(watermarkUid: string): Promise<PublicRightsSdkResult>;
  scanBatch(watermarkUids: string[]): Promise<PublicRightsBatchSdkItem[]>;
  resolvePolicy(scanResult: PublicRightsQueryResponse): PublicRightsPolicyResolution;
  formatUserMessage(result: PublicRightsSdkResult): string;
}

export function createPublicRightsScanner(options: PublicRightsScannerOptions): PublicRightsScanner {
  const baseUrl = options.baseUrl.replace(/\/$/, "");
  const fetcher = options.fetchImpl ?? fetch;
  return {
    async scanOne(watermarkUid: string) {
      const uid = watermarkUid.trim();
      if (!uid) return buildErrorResult(null, "not_found");
      try {
        const response = await requestJson<PublicRightsQueryResponse>(
          fetcher,
          `${baseUrl}/v1/public/rights/${encodeURIComponent(uid)}`,
          "GET",
          undefined,
          options.apiKey,
        );
        return buildOkResult(response);
      } catch (error) {
        return buildErrorResult(null, classifyPublicRightsError(error));
      }
    },

    async scanBatch(watermarkUids: string[]) {
      const uniqueUids = [...new Set(watermarkUids.map((uid) => uid.trim()).filter(Boolean))];
      if (uniqueUids.length === 0) return [];
      const path = options.apiKey
        ? "/v1/enterprise/public-rights/batch"
        : "/v1/public/rights/batch";
      try {
        const response = await requestJson<PublicRightsBatchResponse | { batch: PublicRightsBatchResponse }>(
          fetcher,
          `${baseUrl}${path}`,
          "POST",
          options.apiKey
            ? { watermarkUids: uniqueUids, idempotencyKey: `sdk_${Date.now()}` }
            : { watermarkUids: uniqueUids },
          options.apiKey,
        );
        const batch = "batch" in response ? response.batch : response;
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
  if (result.status === "error") return messageForError(result.error ?? "internal_error");
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
  if (message.includes("404") || message.includes("not_found") || message.includes("missing")) {
    return "not_found";
  }
  if (message.includes("429") || message.includes("rate_limited")) return "rate_limited";
  if (message.includes("quota_exhausted") || message.includes("402")) return "quota_exhausted";
  if (message.includes("scope_denied")) return "scope_denied";
  if (message.includes("401") || message.includes("api_key_invalid")) return "api_key_invalid";
  if (message.includes("403") || message.includes("unavailable")) return "registry_unavailable";
  if (message.includes("payload_invalid") || message.includes("auth")) return "payload_invalid";
  if (message.includes("conflict") || message.includes("disputed")) return "manifest_conflict";
  return "internal_error";
}

function batchItemToSdkItem(item: PublicRightsBatchItem): PublicRightsBatchSdkItem {
  if (item.status === "ok" && item.result) {
    return { watermarkUid: item.watermarkUid, ...buildOkResult(item.result) };
  }
  return {
    watermarkUid: item.watermarkUid,
    ...buildErrorResult(null, normalizeBatchErrorCode(item.errorCode)),
  };
}

function buildOkResult(scan: PublicRightsQueryResponse): PublicRightsSdkResult {
  const policy = resolvePolicy(scan);
  const error = errorCodeFromScan(scan);
  const result: PublicRightsSdkResult = {
    status: error ? "error" : "ok",
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
    case "rate_limited":
    case "quota_exhausted":
    case "scope_denied":
    case "api_key_invalid":
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
    case "rate_limited":
      return "企业公开扫描已触发限流，请稍后重试。";
    case "quota_exhausted":
      return "企业公开扫描额度不足。";
    case "scope_denied":
      return "企业 API key 未包含公开批量扫描权限。";
    case "api_key_invalid":
      return "企业 API key 无效或已过期。";
    case "internal_error":
      return "公开权利查询暂时失败。";
  }
}

async function requestJson<T>(
  fetcher: typeof fetch,
  url: string,
  method: "GET" | "POST",
  body?: unknown,
  apiKey?: string,
): Promise<T> {
  const headers: Record<string, string> = { accept: "application/json" };
  if (body !== undefined) headers["content-type"] = "application/json";
  if (apiKey) headers.authorization = `Bearer ${apiKey}`;
  const response = await fetcher(url, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${text}`);
  }
  return JSON.parse(text) as T;
}
