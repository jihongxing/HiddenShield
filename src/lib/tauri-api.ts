export type Platform = "douyin" | "bilibili" | "xiaohongshu";
export type AppTab =
  | "workbench"
  | "process"
  | "verify"
  | "vault"
  | "batch"
  | "subscription"
  | "settings"
  | "help";

export interface SourceMeta {
  fileName: string;
  path: string;
  width: number;
  height: number;
  fps: number;
  durationSecs: number;
  durationConfirmed: boolean;
  sampleRate?: number | null;
  channels?: number | null;
  watermarkEligible?: boolean | null;
  fileSizeBytes: number;
  fileSizeMb: number;
  isHdr: boolean;
  colorProfile: string;
  sha256: string;
  fileType: string;
}

export interface HardwareInfo {
  preferredEncoder: string;
  availableEncoders: string[];
  toneMappingSupported: boolean;
  ffmpegStatus: string;
}

export interface VaultRecord {
  id: number;
  fileName: string;
  createdAt: string;
  watermarkUid: string;
  originalHash: string;
  resolution: string;
  durationSecs: number;
  isHdrSource: boolean;
  creatorDisplayName: string | null;
  outputDouyin: string | null;
  outputBilibili: string | null;
  outputXhs: string | null;
  hwEncoderUsed: string | null;
  processTimeMs: number | null;
  tsaTokenPath: string | null;
  networkTime: string | null;
  tsaSource: string | null;
  tsaRequestNonce: string | null;
  isAiGenerated: boolean;
  aiTrainingPermission: string | null;
  aiGenerationMethod: string | null;
  humanModificationLevel: string | null;
  authenticityClaim: string | null;
  customMetadata: string | null;
  outputDouyinHash: string | null;
  outputBilibiliHash: string | null;
  outputXhsHash: string | null;
  protectedCopyName: string | null;
  protectedCopyPath: string | null;
  protectedCopyHash: string | null;
  outputStrategy: string;
  workSourceDeclaration: string;
  trainingPermissionDeclaration: string;
  creationMethodDeclaration: string;
  humanEditLevelDeclaration: string;
  authenticityClaimDeclaration: string;
  customRightsStatement: string | null;
  parentWatermarkUid: string | null;
  revision: number;
  rewriteReason: string | null;
  writeVerificationStatus: string | null;
  writeVerificationMessage: string | null;
  writeVerificationAt: string | null;
  payloadProtocolVersion: number;
  payloadBytesLength: number;
  watermarkIdIssueMode: string;
  watermarkIdRegistryStatus: string;
  watermarkIdRegistryReceipt: string | null;
  payloadAuthStatus: string;
  videoNotaryId: string | null;
  videoNotaryAt: string | null;
  videoNotaryReceiptSignature: string | null;
  videoNotaryUsageLedgerId: string | null;
  videoFingerprintRoot: string | null;
  videoBundleSha256: string | null;
  videoBundleBytes: number | null;
  videoBundleSceneCount: number | null;
  videoBundleElapsedMs: number | null;
  videoFrameSamplePolicy: string | null;
  videoVisualTaskId: string | null;
  videoVisualCompletedAt: string | null;
  videoVisualStrategyDigest: string | null;
  videoVisualSelfCheckConfidence: number | null;
  videoVisualSelfCheckThreshold: number | null;
  videoVisualCheckedFrames: number | null;
  videoVisualMediaHash: string | null;
  videoVisualReceiptHash: string | null;
  videoVisualOutputBytes: number | null;
  videoVisualOutputContentType: string | null;
}

export function formatLocalDateTime(
  value: string | null | undefined,
  fallback = "未记录",
): string {
  const trimmed = value?.trim();
  if (!trimmed) return fallback;
  const date = new Date(trimmed);
  if (Number.isNaN(date.getTime())) return trimmed;
  return date.toLocaleString();
}

export function formatEvidenceTime(
  value: string | null | undefined,
  fallback = "未记录",
): string {
  const trimmed = value?.trim();
  if (!trimmed) return fallback;
  const localTime = formatLocalDateTime(trimmed, fallback);
  if (localTime === trimmed) return trimmed;
  return `${localTime}（原始回执: ${trimmed}）`;
}

export function formatCopyrightDateTime(
  value: string | null | undefined,
  fallback = "未记录",
): string {
  const trimmed = value?.trim();
  if (!trimmed) return fallback;
  const date = new Date(trimmed);
  if (Number.isNaN(date.getTime())) return trimmed;
  const pad = (part: number) => String(part).padStart(2, "0");
  return [
    date.getFullYear(),
    pad(date.getMonth() + 1),
    pad(date.getDate()),
  ].join("-") +
    ` ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

export function formatTimeProofService(value?: string | null): string {
  const trimmed = value?.trim();
  if (!trimmed) return "第三方时间戳服务";
  try {
    const host = new URL(trimmed).hostname.toLowerCase().replace(/^www\./, "");
    if (host === "freetsa.org" || host.endsWith(".freetsa.org")) {
      return "FreeTSA 时间戳服务";
    }
    return "第三方时间戳服务";
  } catch {
    return trimmed;
  }
}

export type EntitlementStatus = "free" | "trial" | "active" | "grace" | "expired";
export type EntitlementPlanKey = "base_unpaid" | "image_audio_annual";

export interface EntitlementState {
  status: EntitlementStatus;
  planName: string | null;
  planCode: string;
  planKey: EntitlementPlanKey;
  planLabel: "未付费" | "图片 / 音频年费";
  features: Record<string, boolean>;
  billingSource: string | null;
  subscriptionId: string | null;
  trialStartedAt: string | null;
  trialEndsAt: string | null;
  currentPeriodStartedAt: string | null;
  currentPeriodEndsAt: string | null;
  graceEndsAt: string | null;
  lastCheckedAt: string | null;
  updatedAt: string;
}

export interface UsageLedgerSummary {
  totalUnits: number;
  totalEvents: number;
  imageUnits: number;
  videoUnits: number;
  audioUnits: number;
  lastUsedAt: string | null;
  lastFeatureName: string | null;
  entitlement: EntitlementState;
}

export interface FormalReportExportResult {
  reportId: string;
  reportType: string;
  reportDir: string;
  pdfPath: string;
  jsonPath: string;
  manifestPath: string;
  exportedAt: string;
  recordCount: number;
  pdfGenerationMs: number;
  pdfPageCount: number;
  bundleVersion: number;
  supersedesReportId: string | null;
}

export interface OfflineLicenseStatus {
  status:
    | "none"
    | "active"
    | "expired"
    | "revoked"
    | "device_mismatch"
    | "invalid";
  installationId: string;
  installationCreatedAt: string;
  licenseId: string | null;
  keyId: string | null;
  productCode: string | null;
  issuedAt: string | null;
  notBefore: string | null;
  expiresAt: string | null;
  importedAt: string | null;
  revocationListSequence: number | null;
  errorCode: string | null;
  features: Record<string, boolean>;
}

export interface OfflineActivationRequestExport {
  token: string;
  installationId: string;
  outputPath: string | null;
}

export interface FormalReportVerifiedFile {
  path: string;
  expectedBytes: number;
  actualBytes: number | null;
  expectedSha256: string;
  actualSha256: string | null;
  status: "matched" | "mismatch" | "missing" | "unsafe_path" | string;
}

export interface FormalReportBundleVerificationResult {
  reportId: string | null;
  reportType: string | null;
  reportDir: string;
  verifiedAt: string;
  manifestSchemaVersion: number | null;
  bundleVersion: number | null;
  supersedesReportId: string | null;
  integrityStatus: "matched" | "mismatch" | string;
  manifestChainStatus: "matched" | "mismatch" | string;
  documentContractStatus: "matched" | "mismatch" | string;
  signatureStatus: "not_signed" | "present_unverified" | string;
  trustedTimeStatus: "not_timestamped" | "present_unverified" | string;
  files: FormalReportVerifiedFile[];
  message: string;
}

export interface RightsEvidencePackVerifiedAttachment {
  attachmentId: string;
  path: string;
  role: "original" | "working_copy" | "capture" | "external_receipt" | string;
  expectedBytes: number;
  actualBytes: number | null;
  expectedSha256: string;
  actualSha256: string | null;
  status: "matched" | "mismatch" | "missing" | "unsafe_path" | string;
}

export interface RightsEvidencePackVerificationResult {
  packId: string | null;
  caseId: string | null;
  caseDir: string;
  verifiedAt: string;
  manifestSchemaVersion: number | null;
  directoryContractStatus: "matched" | "mismatch" | string;
  attachmentIntegrityStatus: "matched" | "mismatch" | string;
  eventChainStatus: "matched" | "mismatch" | string;
  attachmentChainStatus: "matched" | "mismatch" | string;
  signatureStatus: "not_signed" | "present_unverified" | string;
  trustedTimeStatus: "not_timestamped" | "present_unverified" | string;
  declaredRootDigest: string | null;
  computedRootDigest: string | null;
  attachments: RightsEvidencePackVerifiedAttachment[];
  message: string;
}

export type AnonymousEventOutcome = "success" | "failure" | "crash" | "diagnostic";

export interface AnonymousFeedbackStatus {
  installId: string;
  sessionId: string;
  queuedEvents: number;
  queuedBytes: number;
  lastEventAt: string | null;
  lastFlushAt: string | null;
  lastFlushError: string | null;
  consecutiveFailures: number;
  nextRetryAt: string | null;
  lastAttemptAt: string | null;
  lastSuccessAt: string | null;
  telemetryEnabled: boolean;
  acknowledged: boolean;
  networkEnabled: boolean;
  endpointConfigured: boolean;
}

export interface AnonymousFlushResult {
  attemptedEvents: number;
  sentEvents: number;
  remainingEvents: number;
  endpointConfigured: boolean;
  flushedAt: string | null;
  message: string;
}

export interface MobileSyncStatus {
  enabled: boolean;
  listenPort: number;
  listenAddress: string;
  pairingCode: string;
  receivedEvents: number;
  latestEventAt: string | null;
  resolutionCount: number;
  latestResolution: SyncResolutionSummary | null;
}

export interface SyncResolutionSummary {
  resolvedAt: string;
  resolutionType: string;
  reason: string;
  watermarkUid: string;
  desktopHash: string | null;
  mobileHash: string | null;
  desktopRevision: number | null;
  mobileRevision: number | null;
}

export interface DesktopCloudSyncProfile {
  cloudBaseUrl: string;
  accountId: string;
  accountLabel: string;
  accessToken: string;
  refreshToken: string;
  workspaceId: string;
  workspaceName: string;
  deviceId: string;
  deviceName: string | null;
  devicePlatform: string | null;
  creatorProfileId: string;
  creatorDisplayName: string;
  entitlementId: string;
  entitlementLabel: string;
  entitlementStatus: string;
  entitlementPlanCode: string;
  entitlementPlanKey: EntitlementPlanKey;
  entitlementFeatures: Record<string, boolean>;
  syncPolicy: string;
  lastRemoteCursor: string | null;
  updatedAt: string;
}

export interface AuthChallengeResult {
  challengeId: string;
  deliveryChannel: string;
  expiresAt: string;
  message: string;
  fixtureCode?: string | null;
}

export interface AccountDevice {
  id: string;
  clientDeviceId: string;
  name: string;
  platform: string;
  appVersion: string;
  registered: boolean;
  autoSyncEnabled: boolean;
  isCurrent: boolean;
  activeSessionCount: number;
  lastSeenAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface RevokeDeviceResult {
  ok: boolean;
  deviceId: string;
  revokedSessionCount: number;
}

export interface CloudSyncBatchResult {
  accepted: number;
  acceptedEventIds: string[];
  nextCursor: string | null;
  resolutions: unknown;
}

export interface CloudQueueStatus {
  pending: number;
  syncing: number;
  failed: number;
  blocked: number;
  synced: number;
  retryExhausted: number;
  staleRecovered: number;
  lastAttemptAt: string | null;
  lastSuccessAt: string | null;
  lastFailureAt: string | null;
  nextRetryAt: string | null;
  lastError: string | null;
  lastErrorCode: string | null;
  lastHttpStatus: number | null;
  blockedReason: string | null;
}

export interface CloudQueueFlushResult {
  attempted: number;
  synced: number;
  failed: number;
  message: string;
}

export interface RepairWatermarkRecordResult {
  recordId: number;
  previousWatermarkUid: string;
  replacementWatermarkUid: string | null;
  jobId: string | null;
  status: string;
  message: string;
  protectedCopyPath: string | null;
  protectedCopyHash: string | null;
}

export interface CloudSyncChange {
  cursor: string | null;
  entityType: string;
  operation: string;
  sourceDevice: string | null;
  entity: Record<string, unknown>;
}

export interface CloudSyncChangesResult {
  nextCursor: string;
  changes: CloudSyncChange[];
}

export interface CloudPullResult {
  nextCursor: string;
  totalChanges: number;
  applied: number;
  skipped: number;
  importedQueueIds: string[];
}

export interface VideoFingerprintNotaryReceipt {
  schemaVersion: string;
  notaryId: string;
  watermarkUid: string;
  sourceHash: string;
  fingerprintRoot: string;
  notarizedAt: string;
  serverReceiptSignature: string;
  usageLedgerId: string;
}

export interface VideoFingerprintNotaryResult {
  receipt: VideoFingerprintNotaryReceipt;
  vaultRecord: VaultRecord;
}

export interface CloudVideoTaskRecord {
  taskId: string;
  status: string;
  capabilityLevel: string;
  watermarkUid: string;
  sourceHash: string;
  durationMs: number;
  strategyDigest: string | null;
  selfCheckThreshold: number | null;
  selfCheckConfidence: number | null;
  checkedFrames: number | null;
  watermarkedMediaHash: string | null;
  outputMediaBytes: number | null;
  outputMediaContentType: string | null;
  workerReceiptHash: string | null;
  serverReceiptSignature: string | null;
  usageLedgerId: string | null;
  completedAt: string | null;
}

export interface SaveL3VideoVisualTaskResult {
  task: CloudVideoTaskRecord;
  vaultRecord: VaultRecord;
  outputPath: string;
  outputSha256: string;
  cloudSync: CloudQueueFlushResult | null;
}

export interface CreateL3VideoVisualUploadTaskResult {
  task: CloudVideoTaskRecord;
  watermarkUid: string;
  sourceSha256: string;
  uploadedBytes: number;
  privacyBoundary: string;
  nextAction: string;
}

export interface BillingPaymentSession {
  paymentSessionId: string;
  provider: string;
  providerOrderId: string;
  paymentAction: BillingPaymentAction;
  expiresAt: string;
}

export interface BillingPaymentAction {
  type: string;
  qrCodeUrl: string | null;
  h5Url: string | null;
}

export interface BillingPaymentSessionStatus {
  paymentSessionId: string;
  provider: string;
  providerOrderId: string;
  status: string;
  planCode: string;
  billingCycle: string;
  expiresAt: string;
  lastCheckedAt: string | null;
  nextCheckAfter: string | null;
  checkAttempts: number;
  entitlement: {
    id: string;
    planName: string | null;
    planCode: string;
    planKey: EntitlementPlanKey;
    planLabel: "未付费" | "图片 / 音频年费";
    status: EntitlementStatus;
    features: Record<string, boolean>;
  };
}

export interface BillingPaymentSessionReconcileResult {
  paymentSessionId: string;
  status: string;
  message: string;
  entitlement: BillingPaymentSessionStatus["entitlement"];
}

export interface ReportPurchaseGrant {
  grantId: string;
  accountId: string;
  workspaceId: string;
  creatorProfileId: string;
  vaultRecordId: string;
  productCode: string;
  priceCents: number;
  currency: string;
  status: string;
  grantedAt: string;
  revokedAt: string | null;
}

export interface ReportPurchaseSession {
  paymentSessionId: string;
  provider: string;
  providerOrderId: string;
  productCode: string;
  priceCents: number;
  currency: string;
  paymentAction: BillingPaymentAction;
  expiresAt: string;
}

export interface ReportPurchaseSessionStatus {
  paymentSessionId: string;
  provider: string;
  providerOrderId: string;
  status: string;
  productCode: string;
  priceCents: number;
  currency: string;
  vaultRecordId: string;
  expiresAt: string;
  lastCheckedAt: string | null;
  nextCheckAfter: string | null;
  checkAttempts: number;
  grant: ReportPurchaseGrant | null;
}

export interface ReportPurchaseSessionReconcileResult {
  paymentSessionId: string;
  status: string;
  message: string;
  grant: ReportPurchaseGrant | null;
}

export interface VideoFingerprintBundleGeneration {
  bundlePath: string;
  bundleSha256: string;
  bundleBytes: number;
  sourceHash: string;
  watermarkUid: string;
  durationMs: number;
  sceneCount: number;
  frameSamplePolicy: string;
  elapsedMs: number;
}

export interface VerificationResult {
  matched: boolean;
  watermarkUid: string | null;
  confidence: number;
  matchedRecord: VaultRecord | null;
  summary: string;
  reasonCode: string;
  reasonDetail: string;
  disclaimer: string;
  tsaTokenPresent: boolean;
  tsaTokenVerified: boolean;
  tsaVerificationPath: TsaVerificationPath | null;
  tsaSource: string | null;
  networkTime: string | null;
  createdAt: string | null;
  originalHash: string | null;
  payloadProtocolVersion: number | null;
  payloadBytesLength: number | null;
  payloadAuthStatus: string | null;
  watermarkIdIssueMode: string | null;
  mediaPayloadRole: string | null;
  durationMs: number;
}

export interface RightsManifestResponse {
  rightsManifestId: string;
  watermarkUid: string;
  manifestVersion: number;
  status: string;
  trainingPolicy: string;
  workSourceDeclaration: string;
  creationMethodDeclaration: string;
  humanEditLevelDeclaration: string;
  authenticityClaimDeclaration: string;
  customTermsUrl: string | null;
  customTermsHash: string | null;
  standardMappings: Record<string, unknown>;
  manifestSha256: string;
  signature: string;
  signedBy: string;
  effectiveAt: string;
  supersededBy: string | null;
  revokedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface RightsManifestSummary {
  rightsManifestId: string;
  watermarkUid: string;
  manifestVersion: number;
  status: string;
  trainingPolicy: string;
  updatedAt: string;
}

export interface PublicRightsRegistrySnapshot {
  registryId: string;
  watermarkUid: string;
  registryStatus: string;
  registryProofHash: string;
  registryReceipt: string;
  payloadAuthStatus: string;
  watermarkIdIssueMode: string;
  payloadProtocolVersion: number;
  payloadBytesLength: number;
  parentWatermarkUid: string | null;
  revision: number;
  anchorProtocol: string;
  mediaPayloadRole: string;
  rightsSource: string;
  issuedAt: string;
  updatedAt: string;
}

export interface PublicRightsMetadata {
  c2pa: string;
  iptc: string;
  xmp: string;
  consistency: string;
  standardMappings: Record<string, unknown>;
}

export interface PublicTrainingPermissionSnapshot {
  policy: string;
  label: string;
  source: string;
  effectiveSource: string;
  legalConclusion: boolean;
}

export interface PublicRightsQueryResponse {
  watermarkUid: string;
  scanStatus: string;
  registry: PublicRightsRegistrySnapshot;
  rightsManifest: RightsManifestResponse | null;
  history: RightsManifestSummary[];
  publicMetadata: PublicRightsMetadata;
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

export interface EnterpriseAdminAuditEventQuery {
  operation?: string;
  outcome?: string;
  accountId?: string;
  apiKeyId?: string;
  fromOccurredAt?: string;
  toOccurredAt?: string;
  limit?: number;
}

export interface EnterpriseAdminAuditEventRecord {
  auditEventId: string;
  operation: string;
  outcome: string;
  endpoint: string;
  accountId: string | null;
  workspaceId: string | null;
  apiKeyId: string | null;
  targetId: string | null;
  reason: string;
  details: Record<string, unknown>;
  occurredAt: string;
}

export interface EnterpriseAdminAuditEventListResponse {
  events: EnterpriseAdminAuditEventRecord[];
  returned: number;
}

export interface EnterpriseApiKeyCreateRequest {
  accountId: string;
  workspaceId: string;
  creatorProfileId?: string | null;
  name: string;
  keyPrefix: string;
  keyHash: string;
  scopes: string[];
  createdByAccountId: string;
  expiresAt?: string | null;
}

export interface EnterpriseApiKeyRecord {
  apiKeyId: string;
  accountId: string;
  workspaceId: string;
  creatorProfileId: string | null;
  keyPrefix: string;
  name: string;
  status: string;
  scopes: string[];
  createdByAccountId: string;
  createdAt: string;
  lastUsedAt: string | null;
  expiresAt: string | null;
  revokedAt: string | null;
  revokedReason: string | null;
}

export interface EnterpriseApiKeyListQuery {
  accountId?: string;
  workspaceId?: string;
  status?: string;
  limit?: number;
}

export interface EnterpriseApiKeyListResponse {
  apiKeys: EnterpriseApiKeyRecord[];
  returned: number;
}

export interface EnterpriseQuotaBalanceInitRequest {
  accountId: string;
  workspaceId: string;
  quotaType: string;
  periodStart: string;
  periodEnd: string;
  includedUnits: number;
  overageAllowed: boolean;
  overageUnitPriceCents?: number | null;
  currency: string;
}

export interface EnterpriseQuotaBalanceRecord {
  quotaBalanceId: string;
  accountId: string;
  workspaceId: string;
  quotaType: string;
  periodStart: string;
  periodEnd: string;
  includedUnits: number;
  usedUnits: number;
  reservedUnits: number;
  overageAllowed: boolean;
  overageUnitPriceCents: number | null;
  currency: string;
  updatedAt: string;
}

export interface RewriteTargetInspectionResult {
  supported: boolean;
  fileKind: string;
  hasWatermark: boolean;
  watermarkUid: string | null;
  detectedRevision: number | null;
  nextRevision: number;
  parentWatermarkUid: string | null;
  rewriteReason: string | null;
  summary: string;
  reasonCode: string;
  reasonDetail: string;
}

export type TsaVerificationPath = "systemRoots" | "embeddedRoots";

export interface AIContentOptions {
  workSourceDeclaration: string;
  trainingPermissionDeclaration: string;
  creationMethodDeclaration: string;
  humanEditLevelDeclaration: string;
  authenticityClaimDeclaration: string;
  customRightsStatement?: string;
}

export interface TranscodeOptions {
  aspectStrategy: "letterbox" | "smart_crop";
  encodingMode: "fast_gpu" | "high_quality_cpu";
  aiContent?: AIContentOptions;
  allowRewrite: boolean;
  rewriteReason?: string;
}

export interface PipelineStartResult {
  pipelineId: string;
  summary: string;
}

export interface PipelineProgressPayload {
  pipelineId: string;
  stage: string;
  percent: number;
  platformPercents: Record<Platform, number>;
}

export interface SystemCheckResult {
  ffmpegAvailable: boolean;
  ffmpegVersion: string;
  gpuEncoderAvailable: boolean;
  gpuEncoderName: string;
  diskFreeMb: number;
  diskSufficient: boolean;
  outputDirWritable: boolean;
  outputDir: string;
}

export interface PreferencesStatus {
  defaultOutputDir: string | null;
  defaultOutputDirWritable: boolean;
  onboardingCompleted: boolean;
  autoUpdateEnabled: boolean;
}

export interface OutputFileInfo {
  platform: string;
  path: string;
  sizeMb: number;
  resolution: string;
  fps: number;
}

export interface WriteVerificationInfo {
  verified: boolean;
  watermarkUid: string;
  revision: number;
  message: string;
}

export interface PipelineCompletePayload {
  pipelineId: string;
  watermarkUid: string;
  processTimeMs: number;
  encoderUsed: string;
  outputs: OutputFileInfo[];
  vaultRecord: VaultRecord;
  writeVerification?: WriteVerificationInfo | null;
}

export interface SourceWarning {
  type: "info" | "warning";
  message: string;
}

export type LocalBatchJobStatus = "draft" | "queued" | "paused" | "cancelled";
export type LocalBatchItemStatus = "queued" | "running" | "verified" | "failed" | "cancelled";
export type LocalBatchMediaKind = "image" | "audio" | "unsupported";

export interface LocalBatchItem {
  id: string;
  jobId: string;
  inputRef: string;
  fileName: string;
  mediaKind: LocalBatchMediaKind;
  status: LocalBatchItemStatus;
  attempts: number;
  lastError: string | null;
  outputRef: string | null;
  vaultRecordId: number | null;
  writeVerificationStatus: string | null;
  writeVerificationMessage: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface LocalBatchJob {
  id: string;
  status: LocalBatchJobStatus;
  createdAt: string;
  updatedAt: string;
  entitlementPlanCode: string;
  entitlementStatus: string;
  items: LocalBatchItem[];
}

const platformOrder: Platform[] = ["douyin", "bilibili", "xiaohongshu"];

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function buildMockSource(path: string): SourceMeta {
  const baseName = path.split(/[\\/]/).pop() || "demo.mp4";
  const isHdr = /\.mov$/i.test(baseName) || /hdr/i.test(baseName);
  const lowerName = baseName.toLowerCase();

  const ext = baseName.split(".").pop()?.toLowerCase() ?? "";
  const imageExts = ["jpg", "jpeg", "png", "webp", "bmp", "tiff"];
  const audioExts = ["mp3", "wav", "flac", "aac", "ogg"];
  const fileType = imageExts.includes(ext) ? "image" : audioExts.includes(ext) ? "audio" : "video";
  const durationSecs =
    fileType === "audio" && lowerName.includes("short")
      ? 10
      : fileType === "audio" && lowerName.includes("long")
        ? 42
        : isHdr
          ? 74
          : 42;

  return {
    fileName: baseName,
    path,
    width: isHdr ? 3840 : 1920,
    height: isHdr ? 2160 : 1080,
    fps: isHdr ? 60 : 30,
    durationSecs,
    durationConfirmed: !lowerName.includes("unknown"),
    sampleRate: fileType === "audio" ? 44_100 : null,
    channels: fileType === "audio" ? 2 : null,
    fileSizeBytes: Math.round((isHdr ? 824.6 : 186.2) * 1024 * 1024),
    fileSizeMb: isHdr ? 824.6 : 186.2,
    isHdr,
    colorProfile: isHdr ? "BT.2020 / PQ" : "BT.709 / SDR",
    sha256: "9a6c64f07e0c13bbf501f0f61a68e18d64db4dce5f83f4cbe91e23ba9f92d0c5",
    fileType,
  };
}

const mockVault: VaultRecord[] = [
  {
    id: 1,
    fileName: "春日咖啡馆_VLOG.mov",
    createdAt: "2026-04-18 20:16:21",
    watermarkUid: "HS-26A47D91-CA8F13B4-A9C0D2E1-F3456789",
    originalHash: "4ca9c53d98f5d88f5a5cbdb8b9107c14c3d3b3c9f8e2b4af",
    resolution: "3840x2160",
    durationSecs: 74,
    isHdrSource: true,
    creatorDisplayName: "本机创作者",
    outputDouyin: null,
    outputBilibili: null,
    outputXhs: null,
    hwEncoderUsed: "h264_videotoolbox",
    processTimeMs: 48200,
    tsaTokenPath: "/mock/tsa/HS-26A47D91-CA8F13B4-A9C0D2E1-F3456789.tsr",
    networkTime: "2026-04-18T12:16:35Z",
    tsaSource: "https://freetsa.org/tsr",
    tsaRequestNonce: "mock-nonce",
    isAiGenerated: false,
    aiTrainingPermission: null,
    aiGenerationMethod: null,
    humanModificationLevel: null,
    authenticityClaim: null,
    customMetadata: null,
    outputDouyinHash: null,
    outputBilibiliHash: null,
    outputXhsHash: null,
    protectedCopyName: "春日咖啡馆_VLOG_保护副本.mp4",
    protectedCopyPath: "/output/春日咖啡馆_VLOG_保护副本.mp4",
    protectedCopyHash: "f8a8e4f22d7d5f0cdd7b8d7b5e8f8d7a4e0f5b3a2c1d0e9f1234567890abcdef",
    outputStrategy: "minimal_required_change",
    workSourceDeclaration: "unspecified",
    trainingPermissionDeclaration: "prohibited",
    creationMethodDeclaration: "unspecified",
    humanEditLevelDeclaration: "unspecified",
    authenticityClaimDeclaration: "unspecified",
    customRightsStatement: null,
    parentWatermarkUid: null,
    revision: 1,
    rewriteReason: null,
    writeVerificationStatus: "verified",
    writeVerificationMessage: "完成后验证已通过",
    writeVerificationAt: "2026-04-18 20:16:35",
    payloadProtocolVersion: 2,
    payloadBytesLength: 119,
    watermarkIdIssueMode: "offline_generated",
    watermarkIdRegistryStatus: "pending_registration",
    watermarkIdRegistryReceipt: null,
    payloadAuthStatus: "verified",
    videoNotaryId: null,
    videoNotaryAt: null,
    videoNotaryReceiptSignature: null,
    videoNotaryUsageLedgerId: null,
    videoFingerprintRoot: null,
    videoBundleSha256: null,
    videoBundleBytes: null,
    videoBundleSceneCount: null,
    videoBundleElapsedMs: null,
    videoFrameSamplePolicy: null,
    videoVisualTaskId: null,
    videoVisualCompletedAt: null,
    videoVisualStrategyDigest: null,
    videoVisualSelfCheckConfidence: null,
    videoVisualSelfCheckThreshold: null,
    videoVisualCheckedFrames: null,
    videoVisualMediaHash: null,
    videoVisualReceiptHash: null,
    videoVisualOutputBytes: null,
    videoVisualOutputContentType: null,
  },
  {
    id: 2,
    fileName: "品牌开箱_横屏.mp4",
    createdAt: "2026-04-17 23:42:09",
    watermarkUid: "HS-154B2EF8-90D2A6C4-B8E1F034-56789ABC",
    originalHash: "7e1958c0d0a2834328a28ee3dcc8e3806ad9faa2ce37ed09",
    resolution: "1920x1080",
    durationSecs: 311,
    isHdrSource: false,
    creatorDisplayName: null,
    outputDouyin: null,
    outputBilibili: null,
    outputXhs: null,
    hwEncoderUsed: null,
    processTimeMs: 126800,
    tsaTokenPath: null,
    networkTime: null,
    tsaSource: null,
    tsaRequestNonce: null,
    isAiGenerated: true,
    aiTrainingPermission: "commercial",
    aiGenerationMethod: "text_to_video",
    humanModificationLevel: "moderate",
    authenticityClaim: "based_on_reality",
    customMetadata: "示例内容",
    outputDouyinHash: null,
    outputBilibiliHash: "2f3e4d5c6b7a8990a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0",
    outputXhsHash: null,
    protectedCopyName: "品牌开箱_保护副本.mp4",
    protectedCopyPath: "/output/品牌开箱_保护副本.mp4",
    protectedCopyHash: "2f3e4d5c6b7a8990a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0",
    outputStrategy: "minimal_required_change",
    workSourceDeclaration: "ai_generated",
    trainingPermissionDeclaration: "commercial_allowed",
    creationMethodDeclaration: "text_to_video",
    humanEditLevelDeclaration: "moderate",
    authenticityClaimDeclaration: "based_on_reality",
    customRightsStatement: "示例内容",
    parentWatermarkUid: null,
    revision: 1,
    rewriteReason: null,
    writeVerificationStatus: "failed",
    writeVerificationMessage: "完成后验证未通过，请重新写入",
    writeVerificationAt: "2026-04-17 23:43:02",
    payloadProtocolVersion: 2,
    payloadBytesLength: 119,
    watermarkIdIssueMode: "offline_generated",
    watermarkIdRegistryStatus: "pending_registration",
    watermarkIdRegistryReceipt: null,
    payloadAuthStatus: "failed",
    videoNotaryId: null,
    videoNotaryAt: null,
    videoNotaryReceiptSignature: null,
    videoNotaryUsageLedgerId: null,
    videoFingerprintRoot: null,
    videoBundleSha256: null,
    videoBundleBytes: null,
    videoBundleSceneCount: null,
    videoBundleElapsedMs: null,
    videoFrameSamplePolicy: null,
    videoVisualTaskId: null,
    videoVisualCompletedAt: null,
    videoVisualStrategyDigest: null,
    videoVisualSelfCheckConfidence: null,
    videoVisualSelfCheckThreshold: null,
    videoVisualCheckedFrames: null,
    videoVisualMediaHash: null,
    videoVisualReceiptHash: null,
    videoVisualOutputBytes: null,
    videoVisualOutputContentType: null,
  },
];

const mockEntitlement: EntitlementState = {
  status: "free",
  planName: "未付费",
  planCode: "free",
  planKey: "base_unpaid",
  planLabel: "未付费",
  features: {
    cloud_sync: false,
    batch_processing: false,
    report_export: false,
    cloud_batch_processing: false,
    cloud_video_processing: false,
    priority_queue: false,
    team_workspace: false,
    api_access: false,
  },
  billingSource: null,
  subscriptionId: null,
  trialStartedAt: null,
  trialEndsAt: null,
  currentPeriodStartedAt: null,
  currentPeriodEndsAt: null,
  graceEndsAt: null,
  lastCheckedAt: null,
  updatedAt: "2026-05-03T00:00:00Z",
};

const mockUsageSummary: UsageLedgerSummary = {
  totalUnits: 2,
  totalEvents: 2,
  imageUnits: 0,
  videoUnits: 2,
  audioUnits: 0,
  lastUsedAt: "2026-05-03T00:00:00Z",
  lastFeatureName: "watermark_video",
  entitlement: mockEntitlement,
};

// ---------------------------------------------------------------------------
// IPC Functions
// ---------------------------------------------------------------------------

export async function systemCheck(inputPath?: string): Promise<SystemCheckResult> {
  if (!isTauriRuntime()) {
    return {
      ffmpegAvailable: true,
      ffmpegVersion: "ffmpeg version 6.1 (mock)",
      gpuEncoderAvailable: false,
      gpuEncoderName: "libx264",
      diskFreeMb: 52480,
      diskSufficient: true,
      outputDirWritable: true,
      outputDir: "/mock/output",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SystemCheckResult>("system_check", { inputPath });
}

export async function getPreferences(): Promise<PreferencesStatus> {
  if (!isTauriRuntime()) {
    return {
      defaultOutputDir: "/mock/output",
      defaultOutputDirWritable: true,
      onboardingCompleted: false,
      autoUpdateEnabled: true,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PreferencesStatus>("get_preferences");
}

export async function savePreferences(input: {
  defaultOutputDir?: string | null;
  onboardingCompleted?: boolean;
  autoUpdateEnabled?: boolean;
}): Promise<PreferencesStatus> {
  if (!isTauriRuntime()) {
    return {
      defaultOutputDir: input.defaultOutputDir ?? "/mock/output",
      defaultOutputDirWritable: true,
      onboardingCompleted: input.onboardingCompleted ?? true,
      autoUpdateEnabled: input.autoUpdateEnabled ?? true,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PreferencesStatus>("save_preferences", { input });
}

export async function openOutputDir(path: string): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_output_dir", { dirPath: path });
}

/** Check which file paths are missing/offline. Returns list of missing paths. */
export async function checkFilesExist(paths: string[]): Promise<string[]> {
  if (!isTauriRuntime()) return [];
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string[]>("check_files_exist", { paths });
}

export async function probeSource(path: string): Promise<SourceMeta> {
  if (!isTauriRuntime()) {
    return buildMockSource(path);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SourceMeta>("probe_source", { path });
}

export async function getHardwareInfo(): Promise<HardwareInfo> {
  if (!isTauriRuntime()) {
    return {
      preferredEncoder: "software",
      availableEncoders: ["libx264", "libx265"],
      toneMappingSupported: true,
      ffmpegStatus: "skeleton-mode",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<HardwareInfo>("get_hw_info");
}

export async function listVaultRecords(): Promise<VaultRecord[]> {
  if (!isTauriRuntime()) {
    return mockVault;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<VaultRecord[]>("list_vault_records");
}

export async function listLocalBatchJobs(): Promise<LocalBatchJob[]> {
  if (!isTauriRuntime()) {
    return [];
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<LocalBatchJob[]>("list_local_batch_jobs");
}

export async function saveLocalBatchJob(job: LocalBatchJob): Promise<LocalBatchJob> {
  if (!isTauriRuntime()) {
    return job;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<LocalBatchJob>("save_local_batch_job", { job });
}

export async function getEntitlementState(): Promise<EntitlementState> {
  if (!isTauriRuntime()) {
    return mockEntitlement;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<EntitlementState>("get_entitlement_state");
}

export async function setEntitlementState(state: EntitlementState): Promise<EntitlementState> {
  if (!isTauriRuntime()) {
    return state;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<EntitlementState>("set_entitlement_state", { entitlementState: state });
}

export async function getUsageLedgerSummary(): Promise<UsageLedgerSummary> {
  if (!isTauriRuntime()) {
    return mockUsageSummary;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<UsageLedgerSummary>("get_usage_ledger_summary");
}

export async function exportVaultFormalReport(recordId: number): Promise<FormalReportExportResult> {
  if (!isTauriRuntime()) {
    const now = new Date().toISOString();
    return {
      reportId: `hsr-mock-${recordId}`,
      reportType: "formal_report",
      reportDir: `/mock/reports/formal-report-${recordId}`,
      pdfPath: `/mock/reports/formal-report-${recordId}/report.pdf`,
      jsonPath: `/mock/reports/formal-report-${recordId}/report.json`,
      manifestPath: `/mock/reports/formal-report-${recordId}/manifest.json`,
      exportedAt: now,
      recordCount: 1,
      pdfGenerationMs: 420,
      pdfPageCount: 4,
      bundleVersion: 1,
      supersedesReportId: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<FormalReportExportResult>("export_vault_formal_report", {
    input: { recordId },
  });
}

export async function exportVaultBatchSummaryReport(): Promise<FormalReportExportResult> {
  if (!isTauriRuntime()) {
    const now = new Date().toISOString();
    return {
      reportId: "hsr-mock-batch",
      reportType: "batch_summary",
      reportDir: "/mock/reports/batch-summary",
      pdfPath: "/mock/reports/batch-summary/report.pdf",
      jsonPath: "/mock/reports/batch-summary/report.json",
      manifestPath: "/mock/reports/batch-summary/manifest.json",
      exportedAt: now,
      recordCount: mockVault.length,
      pdfGenerationMs: 580,
      pdfPageCount: 4,
      bundleVersion: 1,
      supersedesReportId: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<FormalReportExportResult>("export_vault_batch_summary_report");
}

export async function supplementVaultTrustedTime(recordId: number): Promise<VaultRecord> {
  if (!isTauriRuntime()) {
    const record = mockVault.find((item) => item.id === recordId);
    if (!record) throw new Error("版权记录不存在。");
    return {
      ...record,
      networkTime: new Date().toISOString(),
      tsaSource: "网络授时服务（预览）",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<VaultRecord>("supplement_vault_trusted_time", { recordId });
}

export async function getOfflineLicenseStatus(): Promise<OfflineLicenseStatus> {
  if (!isTauriRuntime()) {
    throw new Error("离线许可证状态仅在桌面应用中可用");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<OfflineLicenseStatus>("get_offline_license_status");
}

export async function exportOfflineActivationRequest(
  outputPath?: string,
): Promise<OfflineActivationRequestExport> {
  if (!isTauriRuntime()) {
    throw new Error("离线激活请求仅在桌面应用中可用");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<OfflineActivationRequestExport>(
    "export_offline_activation_request",
    { outputPath: outputPath ?? null },
  );
}

export async function importOfflineLicense(
  tokenOrPath: string,
): Promise<OfflineLicenseStatus> {
  if (!isTauriRuntime()) {
    throw new Error("离线许可证导入仅在桌面应用中可用");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<OfflineLicenseStatus>("import_offline_license", {
    tokenOrPath,
  });
}

export async function clearOfflineLicense(): Promise<OfflineLicenseStatus> {
  if (!isTauriRuntime()) {
    throw new Error("离线许可证清除仅在桌面应用中可用");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<OfflineLicenseStatus>("clear_offline_license");
}

export async function importOfflineRevocationList(
  tokenOrPath: string,
): Promise<OfflineLicenseStatus> {
  if (!isTauriRuntime()) {
    throw new Error("撤销列表导入仅在桌面应用中可用");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<OfflineLicenseStatus>("import_offline_revocation_list", {
    tokenOrPath,
  });
}

export async function importMobileReportHandoff(
  reportDir: string,
): Promise<FormalReportExportResult> {
  if (!isTauriRuntime()) {
    const now = new Date().toISOString();
    return {
      reportId: "hsr-mock-mobile-handoff-import",
      reportType: "formal_report",
      reportDir: "/mock/reports/mobile-handoff-import",
      pdfPath: "/mock/reports/mobile-handoff-import/report.pdf",
      jsonPath: "/mock/reports/mobile-handoff-import/report.json",
      manifestPath: "/mock/reports/mobile-handoff-import/manifest.json",
      exportedAt: now,
      recordCount: 1,
      pdfGenerationMs: 460,
      pdfPageCount: 4,
      bundleVersion: 1,
      supersedesReportId: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<FormalReportExportResult>("import_mobile_report_handoff", {
    input: { reportDir },
  });
}

export async function verifyFormalReportBundle(
  reportDir: string,
): Promise<FormalReportBundleVerificationResult> {
  if (!isTauriRuntime()) {
    return {
      reportId: "hsr-mock-verified",
      reportType: "formal_report",
      reportDir,
      verifiedAt: new Date().toISOString(),
      manifestSchemaVersion: 2,
      bundleVersion: 1,
      supersedesReportId: null,
      integrityStatus: "matched",
      manifestChainStatus: "matched",
      documentContractStatus: "matched",
      signatureStatus: "not_signed",
      trustedTimeStatus: "not_timestamped",
      files: [
        {
          path: "report.pdf",
          expectedBytes: 752612,
          actualBytes: 752612,
          expectedSha256: "mock-pdf-sha256",
          actualSha256: "mock-pdf-sha256",
          status: "matched",
        },
        {
          path: "report.json",
          expectedBytes: 4096,
          actualBytes: 4096,
          expectedSha256: "mock-json-sha256",
          actualSha256: "mock-json-sha256",
          status: "matched",
        },
      ],
      message: "报告包文件与 Manifest 摘要链匹配；当前报告包未签名。",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<FormalReportBundleVerificationResult>("verify_formal_report_bundle", {
    input: { reportDir },
  });
}

export async function verifyRightsEvidencePack(
  caseDir: string,
): Promise<RightsEvidencePackVerificationResult> {
  if (!isTauriRuntime()) {
    return {
      packId: "hsep-fixture-r4-0001",
      caseId: "case-fixture-r4-0001",
      caseDir,
      verifiedAt: new Date().toISOString(),
      manifestSchemaVersion: 1,
      directoryContractStatus: "matched",
      attachmentIntegrityStatus: "matched",
      eventChainStatus: "matched",
      attachmentChainStatus: "matched",
      signatureStatus: "not_signed",
      trustedTimeStatus: "not_timestamped",
      declaredRootDigest:
        "4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33",
      computedRootDigest:
        "4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33",
      attachments: [],
      message:
        "案件包目录、附件、采集事件链和附件链匹配；当前案件包未签名，也未获得包级可信时间。",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<RightsEvidencePackVerificationResult>(
    "verify_rights_evidence_pack",
    {
      input: { caseDir },
    },
  );
}

export interface PublicRightsEmbeddedImageExportResult {
  recordId: number;
  watermarkUid: string;
  sourcePath: string;
  outputPath: string;
  outputDir: string;
  fileFormat: string;
  embeddedStandards: string[];
  embeddedAt: string;
  outputSha256: string;
  c2paManifestStatus: string;
  c2paManifestHash: string | null;
  c2paSignerStatus: string;
  legalConclusion: boolean;
  boundary: string;
}

export async function exportPublicRightsEmbeddedImage(
  recordId: number,
  metadata: PublicRightsMetadataExport,
): Promise<PublicRightsEmbeddedImageExportResult> {
  if (!isTauriRuntime()) {
    return {
      recordId,
      watermarkUid: metadata.watermarkUid,
      sourcePath: "/mock/protected-copy.png",
      outputPath: `/mock/public-rights/${metadata.watermarkUid}.png`,
      outputDir: "/mock/public-rights",
      fileFormat: "png",
      embeddedStandards: ["XMP", "IPTC/PLUS JSON-LD mapping", "C2PA/CAWG JSON-LD mapping"],
      embeddedAt: new Date().toISOString(),
      outputSha256: "mock-sha256",
      c2paManifestStatus: "mock_embedded_c2pa_signed_manifest",
      c2paManifestHash: "mock-c2pa-manifest-hash",
      c2paSignerStatus: "mock_ephemeral_development_certificate_not_publicly_trusted",
      legalConclusion: false,
      boundary: "creator_declaration_registry_snapshot_not_legal_advice_public_metadata_copy",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PublicRightsEmbeddedImageExportResult>("export_public_rights_embedded_image", {
    input: { recordId, metadata },
  });
}

export async function getAnonymousFeedbackStatus(): Promise<AnonymousFeedbackStatus> {
  if (!isTauriRuntime()) {
    return {
      installId: "inst-mock",
      sessionId: "sess-mock",
      queuedEvents: 0,
      queuedBytes: 0,
      lastEventAt: null,
      lastFlushAt: null,
      lastFlushError: null,
      consecutiveFailures: 0,
      nextRetryAt: null,
      lastAttemptAt: null,
      lastSuccessAt: null,
      telemetryEnabled: true,
      acknowledged: true,
      networkEnabled: true,
      endpointConfigured: false,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AnonymousFeedbackStatus>("get_anonymous_feedback_status");
}

export async function flushAnonymousFeedbackQueue(): Promise<AnonymousFlushResult> {
  if (!isTauriRuntime()) {
    return {
      attemptedEvents: 0,
      sentEvents: 0,
      remainingEvents: 0,
      endpointConfigured: false,
      flushedAt: null,
      message: "mock flush",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AnonymousFlushResult>("flush_anonymous_feedback_queue");
}

export async function getMobileSyncStatus(): Promise<MobileSyncStatus> {
  if (!isTauriRuntime()) {
    return {
      enabled: true,
      listenPort: 47219,
      listenAddress: "http://0.0.0.0:47219",
      pairingCode: "123456",
      receivedEvents: 0,
      latestEventAt: null,
      resolutionCount: 0,
      latestResolution: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<MobileSyncStatus>("get_mobile_sync_status");
}

export async function regenerateMobilePairingCode(): Promise<string> {
  if (!isTauriRuntime()) return "654321";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("regenerate_mobile_pairing_code");
}

export async function getDesktopCloudSyncProfile(): Promise<DesktopCloudSyncProfile | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopCloudSyncProfile | null>("get_desktop_cloud_sync_profile");
}

function ensureTrailingSlash(value: string): string {
  const trimmed = value.trim();
  return trimmed.endsWith("/") ? trimmed : `${trimmed}/`;
}

export async function fetchPublicRights(
  baseUrl: string,
  watermarkUid: string,
): Promise<PublicRightsQueryResponse> {
  const endpoint = new URL(
    `/v1/public/rights/${encodeURIComponent(watermarkUid.trim())}`,
    ensureTrailingSlash(baseUrl),
  );
  const response = await fetch(endpoint);
  if (!response.ok) {
    throw new Error(`public rights query failed: HTTP ${response.status}`);
  }
  return response.json() as Promise<PublicRightsQueryResponse>;
}

export async function fetchPublicRightsBatch(
  baseUrl: string,
  watermarkUids: string[],
): Promise<PublicRightsBatchResponse> {
  const endpoint = new URL("/v1/public/rights/batch", ensureTrailingSlash(baseUrl));
  const response = await fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ watermarkUids }),
  });
  if (!response.ok) {
    throw new Error(`public rights batch query failed: HTTP ${response.status}`);
  }
  return response.json() as Promise<PublicRightsBatchResponse>;
}

export interface PublicRightsMetadataExport {
  watermarkUid: string;
  exportVersion: number;
  generatedAt: string;
  legalConclusion: boolean;
  boundary: string;
  manifestHash: string;
  contentCredentials: Record<string, unknown>;
  c2paAssertions: unknown[];
  iptc: Record<string, unknown>;
  xmp: Record<string, unknown>;
  jsonLd: Record<string, unknown>;
}

export async function fetchPublicRightsMetadata(
  baseUrl: string,
  watermarkUid: string,
): Promise<PublicRightsMetadataExport> {
  const endpoint = new URL(
    `/v1/public/rights/${encodeURIComponent(watermarkUid.trim())}/metadata`,
    ensureTrailingSlash(baseUrl),
  );
  const response = await fetch(endpoint);
  if (!response.ok) {
    throw new Error(`public rights metadata export failed: HTTP ${response.status}`);
  }
  return response.json() as Promise<PublicRightsMetadataExport>;
}

export async function fetchEnterpriseAdminAuditEvents(
  baseUrl: string,
  adminToken: string,
  query: EnterpriseAdminAuditEventQuery,
): Promise<EnterpriseAdminAuditEventListResponse> {
  const endpoint = new URL("/internal/enterprise/admin-audit-events", ensureTrailingSlash(baseUrl));
  appendOptionalSearchParam(endpoint.searchParams, "operation", query.operation);
  appendOptionalSearchParam(endpoint.searchParams, "outcome", query.outcome);
  appendOptionalSearchParam(endpoint.searchParams, "accountId", query.accountId);
  appendOptionalSearchParam(endpoint.searchParams, "apiKeyId", query.apiKeyId);
  appendOptionalSearchParam(endpoint.searchParams, "fromOccurredAt", query.fromOccurredAt);
  appendOptionalSearchParam(endpoint.searchParams, "toOccurredAt", query.toOccurredAt);
  appendOptionalSearchParam(endpoint.searchParams, "limit", query.limit);
  const response = await fetch(endpoint, {
    method: "GET",
    headers: {
      authorization: `Bearer ${adminToken.trim()}`,
    },
  });
  if (!response.ok) {
    throw new Error(`enterprise admin audit query failed: HTTP ${response.status}`);
  }
  return response.json() as Promise<EnterpriseAdminAuditEventListResponse>;
}

function enterpriseAdminHeaders(adminToken: string, json = false): HeadersInit {
  return {
    ...(json ? { "content-type": "application/json" } : {}),
    authorization: `Bearer ${adminToken.trim()}`,
  };
}

async function parseEnterpriseAdminResponse<T>(response: Response, action: string): Promise<T> {
  if (!response.ok) {
    throw new Error(`${action} failed: HTTP ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export async function createEnterpriseApiKeyInternal(
  baseUrl: string,
  adminToken: string,
  request: EnterpriseApiKeyCreateRequest,
): Promise<EnterpriseApiKeyRecord> {
  const endpoint = new URL("/internal/enterprise/api-keys", ensureTrailingSlash(baseUrl));
  const response = await fetch(endpoint, {
    method: "POST",
    headers: enterpriseAdminHeaders(adminToken, true),
    body: JSON.stringify(request),
  });
  return parseEnterpriseAdminResponse<EnterpriseApiKeyRecord>(response, "enterprise api key create");
}

export async function listEnterpriseApiKeysInternal(
  baseUrl: string,
  adminToken: string,
  query: EnterpriseApiKeyListQuery,
): Promise<EnterpriseApiKeyListResponse> {
  const endpoint = new URL("/internal/enterprise/api-keys", ensureTrailingSlash(baseUrl));
  appendOptionalSearchParam(endpoint.searchParams, "accountId", query.accountId);
  appendOptionalSearchParam(endpoint.searchParams, "workspaceId", query.workspaceId);
  appendOptionalSearchParam(endpoint.searchParams, "status", query.status);
  appendOptionalSearchParam(endpoint.searchParams, "limit", query.limit);
  const response = await fetch(endpoint, {
    method: "GET",
    headers: enterpriseAdminHeaders(adminToken),
  });
  return parseEnterpriseAdminResponse<EnterpriseApiKeyListResponse>(response, "enterprise api key list");
}

export async function getEnterpriseApiKeyInternal(
  baseUrl: string,
  adminToken: string,
  apiKeyId: string,
): Promise<EnterpriseApiKeyRecord> {
  const endpoint = new URL(
    `/internal/enterprise/api-keys/${encodeURIComponent(apiKeyId.trim())}`,
    ensureTrailingSlash(baseUrl),
  );
  const response = await fetch(endpoint, {
    method: "GET",
    headers: enterpriseAdminHeaders(adminToken),
  });
  return parseEnterpriseAdminResponse<EnterpriseApiKeyRecord>(response, "enterprise api key get");
}

export async function pauseEnterpriseApiKeyInternal(
  baseUrl: string,
  adminToken: string,
  apiKeyId: string,
  reason: string,
): Promise<EnterpriseApiKeyRecord> {
  const endpoint = new URL(
    `/internal/enterprise/api-keys/${encodeURIComponent(apiKeyId.trim())}/pause`,
    ensureTrailingSlash(baseUrl),
  );
  const response = await fetch(endpoint, {
    method: "POST",
    headers: enterpriseAdminHeaders(adminToken, true),
    body: JSON.stringify({ reason }),
  });
  return parseEnterpriseAdminResponse<EnterpriseApiKeyRecord>(response, "enterprise api key pause");
}

export async function revokeEnterpriseApiKeyInternal(
  baseUrl: string,
  adminToken: string,
  apiKeyId: string,
  reason: string,
): Promise<EnterpriseApiKeyRecord> {
  const endpoint = new URL(
    `/internal/enterprise/api-keys/${encodeURIComponent(apiKeyId.trim())}/revoke`,
    ensureTrailingSlash(baseUrl),
  );
  const response = await fetch(endpoint, {
    method: "POST",
    headers: enterpriseAdminHeaders(adminToken, true),
    body: JSON.stringify({ reason }),
  });
  return parseEnterpriseAdminResponse<EnterpriseApiKeyRecord>(response, "enterprise api key revoke");
}

export async function initializeEnterpriseQuotaBalanceInternal(
  baseUrl: string,
  adminToken: string,
  request: EnterpriseQuotaBalanceInitRequest,
): Promise<EnterpriseQuotaBalanceRecord> {
  const endpoint = new URL("/internal/enterprise/quota-balances", ensureTrailingSlash(baseUrl));
  const response = await fetch(endpoint, {
    method: "POST",
    headers: enterpriseAdminHeaders(adminToken, true),
    body: JSON.stringify(request),
  });
  return parseEnterpriseAdminResponse<EnterpriseQuotaBalanceRecord>(response, "enterprise quota balance init");
}

function appendOptionalSearchParam(
  params: URLSearchParams,
  key: string,
  value: string | number | null | undefined,
) {
  if (value === null || value === undefined) return;
  const serialized = String(value).trim();
  if (serialized) {
    params.set(key, serialized);
  }
}

export async function getDesktopCloudQueueStatus(): Promise<CloudQueueStatus> {
  if (!isTauriRuntime()) {
    return {
      pending: 0,
      syncing: 0,
      failed: 0,
      blocked: 0,
      synced: 0,
      retryExhausted: 0,
      staleRecovered: 0,
      lastAttemptAt: null,
      lastSuccessAt: null,
      lastFailureAt: null,
      nextRetryAt: null,
      lastError: null,
      lastErrorCode: null,
      lastHttpStatus: null,
      blockedReason: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudQueueStatus>("get_desktop_cloud_queue_status");
}

export async function signOutDesktopCloud(): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("sign_out_desktop_cloud");
}

export async function createDesktopAuthChallenge(identifier: string): Promise<AuthChallengeResult> {
  if (!isTauriRuntime()) {
    return {
      challengeId: `mock-challenge-${Date.now()}`,
      deliveryChannel: "fixture",
      expiresAt: new Date(Date.now() + 10 * 60 * 1000).toISOString(),
      message: "本地预览验证码为 000000。",
      fixtureCode: "000000",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AuthChallengeResult>("create_desktop_auth_challenge", {
    input: { identifier },
  });
}

export async function continueCloudAccount(
  identifier: string,
  password: string,
  creatorDisplayName: string,
  challengeId?: string | null,
  verificationCode?: string | null,
): Promise<DesktopCloudSyncProfile> {
  if (!isTauriRuntime()) {
    return {
      cloudBaseUrl: "http://127.0.0.1:43188",
      accountId: "acct_mock",
      accountLabel: identifier,
      accessToken: "mock-access-token",
      refreshToken: "mock-refresh-token",
      workspaceId: "ws_mock",
      workspaceName: "个人空间",
      deviceId: "desktop-mock",
      deviceName: "Desktop Mock",
      devicePlatform: "web",
      creatorProfileId: "creator_mock",
      creatorDisplayName,
      entitlementId: "ent_mock",
      entitlementLabel: "未付费",
      entitlementStatus: "free",
      entitlementPlanCode: "free",
      entitlementPlanKey: "base_unpaid",
      entitlementFeatures: {
        cloud_sync: false,
        batch_processing: false,
        report_export: false,
        cloud_batch_processing: false,
        cloud_video_processing: false,
        priority_queue: false,
        team_workspace: false,
        api_access: false,
      },
      syncPolicy: "blocked_by_entitlement",
      lastRemoteCursor: null,
      updatedAt: new Date().toISOString(),
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopCloudSyncProfile>("continue_cloud_account", {
    input: { identifier, password, creatorDisplayName, challengeId, verificationCode },
  });
}

export async function setDesktopCloudAutoSyncEnabled(
  enabled: boolean,
): Promise<DesktopCloudSyncProfile> {
  if (!isTauriRuntime()) {
    return {
      cloudBaseUrl: "http://127.0.0.1:43188",
      accountId: "acct_mock",
      accountLabel: "mock@example.com",
      accessToken: "mock-access-token",
      refreshToken: "mock-refresh-token",
      workspaceId: "ws_mock",
      workspaceName: "个人空间",
      deviceId: "desktop-mock",
      deviceName: "Desktop Mock",
      devicePlatform: "web",
      creatorProfileId: "creator_mock",
      creatorDisplayName: "本机创作者",
      entitlementId: "ent_creator",
      entitlementLabel: "图片 / 音频年费",
      entitlementStatus: "active",
      entitlementPlanCode: "creator",
      entitlementPlanKey: "image_audio_annual",
      entitlementFeatures: {
        cloud_sync: true,
        batch_processing: true,
        report_export: false,
      },
      syncPolicy: enabled ? "auto_cloud_vault" : "manual_local_only",
      lastRemoteCursor: null,
      updatedAt: new Date().toISOString(),
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopCloudSyncProfile>("set_desktop_cloud_auto_sync_enabled", {
    input: { enabled },
  });
}

export async function listDesktopCloudDevices(): Promise<AccountDevice[]> {
  if (!isTauriRuntime()) {
    const now = new Date().toISOString();
    return [
      {
        id: "desktop-mock",
        clientDeviceId: "desktop-mock",
        name: "Desktop Mock",
        platform: "web",
        appVersion: "0.1.0",
        registered: true,
        autoSyncEnabled: true,
        isCurrent: true,
        activeSessionCount: 1,
        lastSeenAt: now,
        createdAt: now,
        updatedAt: now,
      },
    ];
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AccountDevice[]>("list_desktop_cloud_devices");
}

export async function updateDesktopCloudDeviceName(
  deviceId: string,
  name: string,
): Promise<AccountDevice> {
  if (!isTauriRuntime()) {
    const now = new Date().toISOString();
    return {
      id: deviceId,
      clientDeviceId: deviceId,
      name,
      platform: "web",
      appVersion: "0.1.0",
      registered: true,
      autoSyncEnabled: true,
      isCurrent: true,
      activeSessionCount: 1,
      lastSeenAt: now,
      createdAt: now,
      updatedAt: now,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AccountDevice>("update_desktop_cloud_device_name", {
    input: { deviceId, name },
  });
}

export async function revokeDesktopCloudDevice(deviceId: string): Promise<RevokeDeviceResult> {
  if (!isTauriRuntime()) {
    return { ok: true, deviceId, revokedSessionCount: 1 };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<RevokeDeviceResult>("revoke_desktop_cloud_device", {
    input: { deviceId },
  });
}

export async function refreshDesktopAuthSession(): Promise<DesktopCloudSyncProfile> {
  if (!isTauriRuntime()) {
    return {
      cloudBaseUrl: "http://127.0.0.1:43188",
      accountId: "acct_mock",
      accountLabel: "mock@example.com",
      accessToken: "mock-access-token-refreshed",
      refreshToken: "mock-refresh-token-refreshed",
      workspaceId: "ws_mock",
      workspaceName: "个人空间",
      deviceId: "desktop-mock",
      deviceName: "Desktop Mock",
      devicePlatform: "web",
      creatorProfileId: "creator_mock",
      creatorDisplayName: "本机创作者",
      entitlementId: "ent_creator",
      entitlementLabel: "图片 / 音频年费",
      entitlementStatus: "active",
      entitlementPlanCode: "creator",
      entitlementPlanKey: "image_audio_annual",
      entitlementFeatures: {
        cloud_sync: true,
        batch_processing: true,
        report_export: false,
      },
      syncPolicy: "auto_cloud_vault",
      lastRemoteCursor: null,
      updatedAt: new Date().toISOString(),
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopCloudSyncProfile>("refresh_desktop_auth_session");
}

export async function refreshDesktopCloudAccountSnapshot(): Promise<DesktopCloudSyncProfile> {
  if (!isTauriRuntime()) {
    return {
      cloudBaseUrl: "http://127.0.0.1:43188",
      accountId: "acct_mock",
      accountLabel: "mock@example.com",
      accessToken: "mock-access-token",
      refreshToken: "mock-refresh-token",
      workspaceId: "ws_mock",
      workspaceName: "个人空间",
      deviceId: "desktop-mock",
      deviceName: "Desktop Mock",
      devicePlatform: "web",
      creatorProfileId: "creator_mock",
      creatorDisplayName: "本机创作者",
      entitlementId: "ent_creator",
      entitlementLabel: "图片 / 音频年费",
      entitlementStatus: "active",
      entitlementPlanCode: "creator",
      entitlementPlanKey: "image_audio_annual",
      entitlementFeatures: {
        cloud_sync: true,
        batch_processing: true,
        report_export: false,
      },
      syncPolicy: "auto_cloud_vault",
      lastRemoteCursor: null,
      updatedAt: new Date().toISOString(),
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopCloudSyncProfile>("refresh_desktop_cloud_account_snapshot");
}

export async function createBillingPaymentSession(
  planCode: "creator" | "studio",
  billingCycle: "monthly" | "yearly" = "monthly",
  preferredProvider = "wechat_pay",
): Promise<BillingPaymentSession> {
  if (!isTauriRuntime()) {
    return {
      paymentSessionId: `mock_${planCode}_${billingCycle}`,
      provider: preferredProvider ?? "unconfigured",
      providerOrderId: `mock_order_${planCode}`,
      paymentAction: {
        type: "qr_code",
        qrCodeUrl: `fixture://pay/${planCode}/${billingCycle}`,
        h5Url: null,
      },
      expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<BillingPaymentSession>("create_billing_payment_session", {
    input: {
      planCode,
      billingCycle,
      preferredProvider,
    },
  });
}

export async function getBillingPaymentSessionStatus(
  paymentSessionId: string,
): Promise<BillingPaymentSessionStatus> {
  if (!isTauriRuntime()) {
    return {
      paymentSessionId,
      provider: "fixture",
      providerOrderId: "mock_order_creator",
      status: "pending",
      planCode: "creator",
      billingCycle: "monthly",
      expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
      lastCheckedAt: null,
      nextCheckAfter: null,
      checkAttempts: 0,
      entitlement: {
        id: "ent_mock",
        planName: "未付费",
        planCode: "free",
        planKey: "base_unpaid",
        planLabel: "未付费",
        status: "free",
        features: mockEntitlement.features,
      },
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<BillingPaymentSessionStatus>("get_billing_payment_session_status", {
    input: { paymentSessionId },
  });
}

export async function reconcileBillingPaymentSession(
  paymentSessionId: string,
): Promise<BillingPaymentSessionReconcileResult> {
  if (!isTauriRuntime()) {
    return {
      paymentSessionId,
      status: "succeeded",
      message: "支付已确认，权益已生效。",
      entitlement: {
        id: "ent_mock_creator",
        planName: "图片 / 音频年费",
        planCode: "creator",
        planKey: "image_audio_annual",
        planLabel: "图片 / 音频年费",
        status: "active",
        features: {
          ...mockEntitlement.features,
          cloud_sync: true,
          batch_processing: true,
          report_export: false,
        },
      },
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<BillingPaymentSessionReconcileResult>("reconcile_billing_payment_session", {
    input: { paymentSessionId },
  });
}

export async function createReportPurchaseSession(
  vaultRecordId: number,
  productCode: "copyright_report_single" | "rights_evidence_pack_single",
  preferredProvider?: "fixture",
): Promise<ReportPurchaseSession> {
  if (!isTauriRuntime()) {
    return {
      paymentSessionId: `mock_report_${productCode}_${vaultRecordId}`,
      provider: preferredProvider ?? "unconfigured",
      providerOrderId: `mock_report_order_${vaultRecordId}`,
      productCode,
      priceCents: productCode === "copyright_report_single" ? 1990 : 4990,
      currency: "CNY",
      paymentAction: {
        type: "qr_code",
        qrCodeUrl: `fixture://pay/report/${productCode}/${vaultRecordId}`,
        h5Url: null,
      },
      expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ReportPurchaseSession>("create_report_purchase_session", {
    input: {
      vaultRecordId,
      productCode,
      preferredProvider,
    },
  });
}

export async function getReportPurchaseSessionStatus(
  paymentSessionId: string,
): Promise<ReportPurchaseSessionStatus> {
  if (!isTauriRuntime()) {
    return {
      paymentSessionId,
      provider: "fixture",
      providerOrderId: "mock_report_order",
      status: "created",
      productCode: "copyright_report_single",
      priceCents: 1990,
      currency: "CNY",
      vaultRecordId: "1",
      expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
      lastCheckedAt: null,
      nextCheckAfter: null,
      checkAttempts: 0,
      grant: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ReportPurchaseSessionStatus>("get_report_purchase_session_status", {
    input: { paymentSessionId },
  });
}

export async function reconcileReportPurchaseSession(
  paymentSessionId: string,
): Promise<ReportPurchaseSessionReconcileResult> {
  if (!isTauriRuntime()) {
    return {
      paymentSessionId,
      status: "succeeded",
      message: "支付已确认，报告授权已生效。",
      grant: {
        grantId: "mock_grant_1",
        accountId: "acct_mock",
        workspaceId: "ws_mock",
        creatorProfileId: "creator_mock",
        vaultRecordId: "1",
        productCode: "copyright_report_single",
        priceCents: 1990,
        currency: "CNY",
        status: "active",
        grantedAt: new Date().toISOString(),
        revokedAt: null,
      },
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ReportPurchaseSessionReconcileResult>("reconcile_report_purchase_session", {
    input: { paymentSessionId },
  });
}

export async function refreshBillingEntitlement(): Promise<DesktopCloudSyncProfile> {
  if (!isTauriRuntime()) {
    return {
      cloudBaseUrl: "http://127.0.0.1:43188",
      accountId: "acct_mock",
      accountLabel: "HiddenShield 账户",
      accessToken: "mock-access-token",
      refreshToken: "mock-refresh-token",
      workspaceId: "ws_mock",
      workspaceName: "个人空间",
      deviceId: "desktop-mock",
      deviceName: "Desktop Mock",
      devicePlatform: "web",
      creatorProfileId: "creator_mock",
      creatorDisplayName: "本机创作者",
      entitlementId: "ent_mock_creator",
      entitlementLabel: "图片 / 音频年费",
      entitlementStatus: "active",
      entitlementPlanCode: "creator",
      entitlementPlanKey: "image_audio_annual",
      entitlementFeatures: {
        cloud_sync: true,
        batch_processing: true,
        report_export: false,
        cloud_batch_processing: false,
        cloud_video_processing: false,
        priority_queue: false,
        team_workspace: false,
        api_access: false,
      },
      syncPolicy: "auto_cloud_vault",
      lastRemoteCursor: null,
      updatedAt: new Date().toISOString(),
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopCloudSyncProfile>("refresh_billing_entitlement");
}

export async function pushDesktopVaultRecordToCloud(
  baseUrl: string,
  accessToken: string,
  deviceId: string,
  workspaceId: string,
  recordId: number,
): Promise<CloudSyncBatchResult> {
  if (!isTauriRuntime()) {
    return {
      accepted: 1,
      acceptedEventIds: [`desktop-vault-${recordId}`],
      nextCursor: "cursor_mock",
      resolutions: [],
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudSyncBatchResult>("push_desktop_vault_record_to_cloud", {
    input: { baseUrl, accessToken, deviceId, workspaceId, recordId },
  });
}

export async function pushSavedDesktopVaultRecordToCloud(
  recordId: number,
): Promise<CloudSyncBatchResult> {
  if (!isTauriRuntime()) {
    return {
      accepted: 1,
      acceptedEventIds: [`desktop-vault-${recordId}`],
      nextCursor: "cursor_mock",
      resolutions: [],
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudSyncBatchResult>("push_saved_desktop_vault_record_to_cloud", {
    input: { recordId },
  });
}

export async function flushDesktopCloudSyncQueue(limit = 50): Promise<CloudQueueFlushResult> {
  if (!isTauriRuntime()) {
    return {
      attempted: 0,
      synced: 0,
      failed: 0,
      message: "mock flush",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudQueueFlushResult>("flush_desktop_cloud_sync_queue", {
    input: { limit },
  });
}

export async function pullSavedCloudChangesIntoDesktop(): Promise<CloudPullResult> {
  if (!isTauriRuntime()) {
    return {
      nextCursor: "cursor_mock",
      totalChanges: 0,
      applied: 0,
      skipped: 0,
      importedQueueIds: [],
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudPullResult>("pull_saved_cloud_changes_into_desktop");
}

export async function repairWatermarkRecordReissue(
  recordId: number,
  reason = "historical_duplicate_watermark_uid_repair",
): Promise<RepairWatermarkRecordResult> {
  if (!isTauriRuntime()) {
    return {
      recordId,
      previousWatermarkUid: "HS-11112222-33334444-55556666-77778888",
      replacementWatermarkUid: "HS-9999AAAA-BBBBCCCC-DDDDEEEE-FFFF0001",
      jobId: "wmreissue_mock",
      status: "repaired",
      message: "mock repair",
      protectedCopyPath: null,
      protectedCopyHash: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<RepairWatermarkRecordResult>("repair_watermark_record_reissue", {
    input: { recordId, reason },
  });
}

export async function fetchCloudChanges(
  baseUrl: string,
  accessToken: string,
  workspaceId: string,
  cursor?: string,
): Promise<CloudSyncChangesResult> {
  if (!isTauriRuntime()) {
    return { nextCursor: "cursor_mock", changes: [] };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudSyncChangesResult>("fetch_cloud_changes", {
    input: { baseUrl, accessToken, workspaceId, cursor },
  });
}

export async function createVideoFingerprintNotaryFromBundleFile(
  bundlePath: string,
  options: { title?: string; bundleElapsedMs?: number } = {},
): Promise<VideoFingerprintNotaryResult> {
  if (!isTauriRuntime()) {
    const runId = Date.now().toString(36);
    const receipt = {
      schemaVersion: "video_fingerprint_notary_receipt_v1",
      notaryId: `vfn_mock_${runId}`,
      watermarkUid: "wm-video-bundle-mock",
      sourceHash: "sha256:mock-source",
      fingerprintRoot: "sha256:mock-fingerprint-root",
      notarizedAt: new Date().toISOString(),
      serverReceiptSignature: "mock-server-receipt-signature",
      usageLedgerId: `usage_mock_${runId}`,
    };
    return {
      receipt,
      vaultRecord: {
        ...mockVault[0],
        id: Number.parseInt(runId.slice(-4), 36) || 99,
        fileName: options.title || "视频指纹存证",
        originalHash: "mock-source",
        watermarkUid: receipt.watermarkUid,
        writeVerificationStatus: "verified",
        writeVerificationMessage: "云端视频指纹存证已完成",
        writeVerificationAt: receipt.notarizedAt,
        payloadProtocolVersion: 2,
        payloadBytesLength: 119,
        watermarkIdIssueMode: "offline_generated",
        watermarkIdRegistryStatus: "pending_registration",
        watermarkIdRegistryReceipt: null,
        payloadAuthStatus: "verified",
        videoNotaryId: receipt.notaryId,
        videoNotaryAt: receipt.notarizedAt,
        videoNotaryReceiptSignature: receipt.serverReceiptSignature,
        videoNotaryUsageLedgerId: receipt.usageLedgerId,
        videoFingerprintRoot: receipt.fingerprintRoot,
        videoBundleSha256: "sha256:mock-bundle",
        videoBundleBytes: 4096,
        videoBundleSceneCount: 8,
        videoBundleElapsedMs: options.bundleElapsedMs ?? 1200,
        videoFrameSamplePolicy: "uniform_8_frames_v1",
      },
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<VideoFingerprintNotaryResult>("create_video_fingerprint_notary_from_bundle_file", {
    input: {
      bundlePath,
      title: options.title,
      bundleElapsedMs: options.bundleElapsedMs,
    },
  });
}

export async function saveL3VideoVisualTaskToVault(
  taskId: string,
  options: { title?: string } = {},
): Promise<SaveL3VideoVisualTaskResult> {
  if (!isTauriRuntime()) {
    const runId = Date.now().toString(36);
    const completedAt = new Date().toISOString();
    const task: CloudVideoTaskRecord = {
      taskId: taskId.trim() || `l3task_mock_${runId}`,
      status: "succeeded",
      capabilityLevel: "video_visual",
      watermarkUid: "HS-L3-MOCK-00000001",
      sourceHash: "sha256:mock-l3-source",
      durationMs: 42000,
      strategyDigest: "sha256:mock-l3-strategy",
      selfCheckThreshold: 0.9,
      selfCheckConfidence: 1,
      checkedFrames: 4,
      watermarkedMediaHash: "sha256:mock-l3-media",
      outputMediaBytes: 123456,
      outputMediaContentType: "video/mp4",
      workerReceiptHash: "sha256:mock-l3-worker-receipt",
      serverReceiptSignature: "mock-l3-server-receipt-signature",
      usageLedgerId: `usage_l3_mock_${runId}`,
      completedAt,
    };
    const vaultRecord: VaultRecord = {
      ...mockVault[0],
      id: Number.parseInt(runId.slice(-4), 36) || 199,
      fileName: options.title || "L3 视频画面盲水印成品",
      createdAt: completedAt,
      originalHash: "mock-l3-source",
      watermarkUid: task.watermarkUid,
      resolution: "L3 视频画面盲水印",
      durationSecs: 42,
      protectedCopyName: `${task.taskId}.l3-watermarked.mp4`,
      protectedCopyPath: "/mock/l3-output/mock-l3-watermarked.mp4",
      protectedCopyHash: "mock-l3-media",
      outputStrategy: "cloud_l3_video_visual_watermark",
      watermarkIdIssueMode: "server_reserved",
      watermarkIdRegistryStatus: "server_confirmed",
      watermarkIdRegistryReceipt: task.serverReceiptSignature,
      writeVerificationStatus: "verified",
      writeVerificationMessage: "L3 云端视频画面盲水印自检和签名下载哈希校验已通过",
      writeVerificationAt: completedAt,
      videoVisualTaskId: task.taskId,
      videoVisualCompletedAt: completedAt,
      videoVisualStrategyDigest: task.strategyDigest,
      videoVisualSelfCheckConfidence: task.selfCheckConfidence,
      videoVisualSelfCheckThreshold: task.selfCheckThreshold,
      videoVisualCheckedFrames: task.checkedFrames,
      videoVisualMediaHash: task.watermarkedMediaHash,
      videoVisualReceiptHash: task.workerReceiptHash,
      videoVisualOutputBytes: task.outputMediaBytes,
      videoVisualOutputContentType: task.outputMediaContentType,
    };
    return {
      task,
      vaultRecord,
      outputPath: "/mock/l3-output/mock-l3-watermarked.mp4",
      outputSha256: "sha256:mock-l3-media",
      cloudSync: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SaveL3VideoVisualTaskResult>("save_l3_video_visual_task_to_vault", {
    input: { taskId, title: options.title },
  });
}

export async function createL3VideoVisualUploadTask(
  inputPath: string,
  options: {
    title?: string;
    durationSecs?: number;
    width?: number;
    height?: number;
    frameCount?: number;
  } = {},
): Promise<CreateL3VideoVisualUploadTaskResult> {
  if (!isTauriRuntime()) {
    const runId = Date.now().toString(36);
    const task: CloudVideoTaskRecord = {
      taskId: `l3task_mock_upload_${runId}`,
      status: "queued",
      capabilityLevel: "hybrid_visual_watermark",
      watermarkUid: "HS-L3-MOCK-QUEUED-0001",
      sourceHash: "sha256:mock-l3-source-upload",
      durationMs: Math.max(1, Math.round((options.durationSecs ?? 42) * 1000)),
      strategyDigest: null,
      selfCheckThreshold: null,
      selfCheckConfidence: null,
      checkedFrames: null,
      watermarkedMediaHash: null,
      outputMediaBytes: null,
      outputMediaContentType: null,
      workerReceiptHash: null,
      serverReceiptSignature: null,
      usageLedgerId: null,
      completedAt: null,
    };
    return {
      task,
      watermarkUid: task.watermarkUid,
      sourceSha256: task.sourceHash,
      uploadedBytes: 4096,
      privacyBoundary: "signed_object_upload_only_no_local_path_no_raw_video_sync",
      nextAction: "等待 trusted worker 完成自检和收据固化；任务 succeeded 后再下载并保存版权库",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CreateL3VideoVisualUploadTaskResult>("create_l3_video_visual_upload_task", {
    input: {
      inputPath,
      title: options.title,
      durationSecs: options.durationSecs,
      width: options.width,
      height: options.height,
      frameCount: options.frameCount,
    },
  });
}

export async function generateVideoFingerprintBundle(
  inputPath: string,
): Promise<VideoFingerprintBundleGeneration> {
  if (!isTauriRuntime()) {
    const runId = Date.now().toString(36);
    return {
      bundlePath: `/mock/video_fingerprint_bundles/${runId}/bundle.json`,
      bundleSha256: "sha256:mock-bundle",
      bundleBytes: 4096,
      sourceHash: "sha256:mock-source",
      watermarkUid: "l2-mock-video",
      durationMs: 42000,
      sceneCount: 8,
      frameSamplePolicy: "uniform_8_frames_v1",
      elapsedMs: 1200,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<VideoFingerprintBundleGeneration>("generate_video_fingerprint_bundle", {
    input: { inputPath },
  });
}

export async function verifySuspect(path: string): Promise<VerificationResult> {
  if (!isTauriRuntime()) {
    const matchedRecord = mockVault[0];
    return {
      matched: true,
      watermarkUid: matchedRecord.watermarkUid,
      confidence: 0.88,
      matchedRecord,
      summary: "已匹配本地版权库中的作品记录。",
      reasonCode: "matched_original",
      reasonDetail: "样本中的版权编号与本地版权库记录完成绑定。",
      disclaimer: "本报告仅基于既定算法进行特征码技术提取，仅供参考，不代表任何司法鉴定意见。平台不对因本报告引发的连带法律责任负责。",
      tsaTokenPresent: true,
      tsaTokenVerified: true,
      tsaVerificationPath: "systemRoots",
      tsaSource: "https://freetsa.org/tsr",
      networkTime: "Sat, 19 Apr 2026 10:30:00 GMT",
      createdAt: "2026-04-19T10:30:00Z",
      originalHash: "b69956820610c86f72e051ae0c32a54e9af8bfca69361ba3093a38d24dbdaeaa",
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      payloadAuthStatus: "verified",
      watermarkIdIssueMode: "offline_generated",
      mediaPayloadRole: "v2_full_record",
      durationMs: 4200,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<VerificationResult>("verify_suspect", { path });
}

export async function verifySuspectReadonlyCandidate(path: string): Promise<VerificationResult> {
  if (!isTauriRuntime()) {
    const result = await verifySuspect(path);
    return {
      ...result,
      mediaPayloadRole: result.mediaPayloadRole ?? "v2_full_record",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<VerificationResult>("verify_suspect_readonly_candidate", { path });
}

export async function inspectRewriteTarget(path: string): Promise<RewriteTargetInspectionResult> {
  if (!isTauriRuntime()) {
    return {
      supported: true,
      fileKind: "image",
      hasWatermark: false,
      watermarkUid: null,
      detectedRevision: null,
      nextRevision: 1,
      parentWatermarkUid: null,
      rewriteReason: null,
      summary: "未找到已有版权记录，将按首次写入处理。",
      reasonCode: "first_write",
      reasonDetail: "写前检查没有读取到可验证记录；如果继续写入，会创建新的版权记录。",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<RewriteTargetInspectionResult>("inspect_rewrite_target", { path });
}

export async function startPipeline(
  inputPath: string,
  platforms: Platform[],
  options: TranscodeOptions,
): Promise<PipelineStartResult> {
  if (!isTauriRuntime()) {
    return {
      pipelineId: `mock-${Date.now()}`,
      summary: "模拟启动单一保护副本生成任务。",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PipelineStartResult>("start_pipeline", { inputPath, platforms, options });
}

export async function cancelPipeline(pipelineId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("cancel_pipeline", { pipelineId });
}

/** Check which pipelines are still active (for state reconciliation on focus). */
export async function checkActivePipelines(): Promise<string[]> {
  if (!isTauriRuntime()) return [];
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string[]>("check_active_pipelines");
}

export async function listenPipelineProgress(
  handler: (payload: PipelineProgressPayload) => void,
) {
  if (!isTauriRuntime()) return () => undefined;
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<PipelineProgressPayload>("pipeline-progress", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

export async function listenPipelineComplete(
  handler: (payload: PipelineCompletePayload) => void,
) {
  if (!isTauriRuntime()) return () => undefined;
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<PipelineCompletePayload>("pipeline-complete", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

export interface HwDegradationPayload {
  pipelineId: string;
  failedEncoder: string;
  fallbackEncoder: string;
  message: string;
}

export async function listenHwDegradation(
  handler: (payload: HwDegradationPayload) => void,
) {
  if (!isTauriRuntime()) return () => undefined;
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<HwDegradationPayload>("hw-degradation", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

export function createEmptyPlatformPercents() {
  return platformOrder.reduce(
    (acc, platform) => { acc[platform] = 0; return acc; },
    {} as Record<Platform, number>,
  );
}

// ---------------------------------------------------------------------------
// Pure Functions: Recommendations, Warnings, Summaries
// ---------------------------------------------------------------------------

export const MIN_AUDIO_PROTECTION_SECONDS = 30;
export const MAX_AUDIO_PROTECTION_SECONDS = 20 * 60;
export const MAX_AUDIO_PROTECTION_BYTES = 512 * 1024 * 1024;
export const MIN_SUPPORTED_AUDIO_SAMPLE_RATE = 8_000;
export const MAX_SUPPORTED_AUDIO_SAMPLE_RATE = 48_000;
export const MIN_SUPPORTED_AUDIO_CHANNELS = 1;
export const MAX_SUPPORTED_AUDIO_CHANNELS = 2;

export type AudioProtectionPreflightCode =
  | "ok"
  | "audio_duration_unknown"
  | "audio_too_short"
  | "audio_too_long"
  | "audio_file_too_large"
  | "audio_spec_unknown"
  | "audio_sample_rate_too_low"
  | "audio_sample_rate_too_high"
  | "audio_channels_unsupported";

export function standaloneAudioProtectionPreflight(meta: SourceMeta | null): AudioProtectionPreflightCode {
  if (!meta || meta.fileType !== "audio") return "ok";
  if (meta.durationConfirmed === false) return "audio_duration_unknown";
  if (meta.durationSecs < MIN_AUDIO_PROTECTION_SECONDS) return "audio_too_short";
  if (meta.durationSecs > MAX_AUDIO_PROTECTION_SECONDS) return "audio_too_long";
  if (meta.fileSizeBytes > MAX_AUDIO_PROTECTION_BYTES) return "audio_file_too_large";
  if (!meta.sampleRate || !meta.channels) return "audio_spec_unknown";
  if (meta.sampleRate < MIN_SUPPORTED_AUDIO_SAMPLE_RATE) return "audio_sample_rate_too_low";
  if (meta.sampleRate > MAX_SUPPORTED_AUDIO_SAMPLE_RATE) return "audio_sample_rate_too_high";
  if (
    meta.channels < MIN_SUPPORTED_AUDIO_CHANNELS ||
    meta.channels > MAX_SUPPORTED_AUDIO_CHANNELS
  ) return "audio_channels_unsupported";
  return "ok";
}

export function isStandaloneAudioTooShort(meta: SourceMeta | null): boolean {
  return standaloneAudioProtectionPreflight(meta) === "audio_too_short";
}

export function isStandaloneAudioDurationUnknown(meta: SourceMeta | null): boolean {
  return standaloneAudioProtectionPreflight(meta) === "audio_duration_unknown";
}

/** Legacy internal fallback for the historical video pipeline. Not exposed as product choice. */
export function recommendPlatforms(meta: SourceMeta): Platform[] {
  return meta.fileType === "video" ? ["bilibili"] : [];
}

/** Recommend the internal minimal-change write strategy. */
export function recommendStrategy(
  meta: SourceMeta,
  _platforms: Platform[],
  hwInfo: HardwareInfo,
): TranscodeOptions {
  const hasGpu = hwInfo.preferredEncoder !== "libx264" && hwInfo.preferredEncoder !== "libx265";
  const encodingMode: TranscodeOptions["encodingMode"] = hasGpu ? "fast_gpu" : "high_quality_cpu";
  const aspectStrategy: TranscodeOptions["aspectStrategy"] = "letterbox";

  return { aspectStrategy, encodingMode, allowRewrite: false };
}

/** Generate first-principles warnings for protected-copy writing. */
export function generateWarnings(meta: SourceMeta, _platforms: Platform[]): SourceWarning[] {
  const warnings: SourceWarning[] = [];

  if (meta.isHdr) {
    warnings.push({ type: "info", message: "当前素材是 HDR，生成保护副本时会按最小必要变更处理色彩兼容" });
  }

  if (meta.fileType === "video" && meta.durationSecs > 1800) {
    warnings.push({ type: "warning", message: "当前素材时长超过 30 分钟，预计处理时间较长" });
  }

  if (meta.fileType === "video" && meta.fps >= 60) {
    warnings.push({ type: "info", message: "当前素材帧率较高，将优先保持可验证性，不提供平台帧率预设" });
  }

  return warnings;
}

/** Build a copyright summary text for clipboard copy. */
export function buildCopyrightSummary(record: VaultRecord): string {
  const registryReceipt =
    shouldShowRegistryReceipt(record.watermarkIdRegistryStatus) &&
    record.watermarkIdRegistryReceipt?.trim()
      ? `登记收据编号: ${record.watermarkIdRegistryReceipt.trim()}`
      : "";
  const verificationMessage = formatSummaryVerificationMessage(record);
  const timeLines = buildSummaryTimeLines(record);

  return [
    `【HiddenShield 本地版权记录摘要】`,
    `【记录信息】`,
    `记录性质: HiddenShield 本地版权记录，非第三方公证、官方登记或法律权属结论`,
    `版权编号: ${record.watermarkUid}`,
    `版本次数: 第 ${record.revision} 次`,
    `创作者显示名称: ${record.creatorDisplayName?.trim() || "未声明"}`,
    `身份信息来源: 用户本地声明`,
    `身份核验状态: 未进行实名认证`,
    record.parentWatermarkUid ? `上一版编号: ${record.parentWatermarkUid}` : "",
    record.rewriteReason ? `更新说明: ${record.rewriteReason}` : "",
    `【写入与验证】`,
    record.writeVerificationStatus
      ? `保护副本验证: ${formatVerificationStatus(record.writeVerificationStatus)}`
      : "保护副本验证: 未完成",
    verificationMessage ? `验证说明: ${verificationMessage}` : "",
    `处理完成时间: ${formatCopyrightDateTime(record.createdAt)}`,
    record.writeVerificationAt
      ? `验证完成时间: ${formatCopyrightDateTime(record.writeVerificationAt)}`
      : "",
    `【编号与登记】`,
    `版权编号生成方式: ${formatWatermarkIssueMode(record.watermarkIdIssueMode)}`,
    `联网登记状态: ${formatRegistryStatus(record.watermarkIdRegistryStatus)}`,
    registryReceipt,
    `【文件证据】`,
    `原文件名: ${record.fileName}`,
    `作品指纹（SHA-256）: ${record.originalHash}`,
    `保护副本名称: ${record.protectedCopyName?.trim() || "未生成保护副本"}`,
    record.protectedCopyHash?.trim()
      ? `保护副本摘要（SHA-256）: ${record.protectedCopyHash.trim()}`
      : "",
    `输出策略: ${formatOutputStrategy(record.outputStrategy)}`,
    record.resolution?.trim() ? `图片尺寸: ${record.resolution.trim()}` : "",
    `【时间信息】`,
    ...timeLines,
    `【创作者声明】`,
    `以下内容由创作者声明，HiddenShield 不进行 AI 生成检测、真实性鉴定或法律授权判断。`,
    `作品来源声明: ${formatWorkSourceDeclaration(record.workSourceDeclaration)}`,
    `训练许可声明: ${formatTrainingPermissionDeclaration(record.trainingPermissionDeclaration)}`,
    `创作方式声明: ${formatDeclarationValue(record.creationMethodDeclaration)}`,
    `人工编辑声明: ${formatDeclarationValue(record.humanEditLevelDeclaration)}`,
    `真实性声明: ${formatAuthenticityClaimDeclaration(record.authenticityClaimDeclaration)}`,
    record.customRightsStatement?.trim()
      ? `自定义版权声明: ${record.customRightsStatement.trim()}`
      : "",
    `【技术验证信息】`,
    `水印协议版本: V${record.payloadProtocolVersion}`,
    `载荷完整性校验: ${formatPayloadAuthStatus(record.payloadAuthStatus)}`,
    `---`,
    `本摘要在本地生成；原始媒体和保护副本未上传。版权元数据是否同步，以当前同步设置和联网登记状态为准。`,
  ]
    .filter((line) => line.trim().length > 0)
    .join("\n");
}

function formatSummaryVerificationMessage(record: VaultRecord): string {
  if (record.writeVerificationStatus === "verified") {
    return "已从保护副本回读并验证版权编号，可由 HiddenShield 再次读取验证";
  }
  return record.writeVerificationStatus
    ? record.writeVerificationMessage?.trim() || ""
    : "";
}

function shouldShowRegistryReceipt(status?: string | null): boolean {
  return status === "server_confirmed" || status === "offline_confirmed";
}

function buildSummaryTimeLines(record: VaultRecord): string[] {
  const source = record.tsaSource?.trim();
  if (record.tsaTokenPath) {
    return [
      `时间依据: 第三方时间戳回执`,
      `第三方时间证明: 已获取第三方时间戳回执`,
      `可信时间: ${formatCopyrightDateTime(record.networkTime || record.createdAt)}`,
      `时间证明服务: ${source || "第三方时间戳服务"}`,
    ];
  }
  if (record.networkTime) {
    return [
      `时间依据: ${source ? `网络授时服务（${source}）` : "网络授时服务"}`,
      `网络授时时间: ${formatCopyrightDateTime(record.networkTime)}`,
      `第三方时间证明: 未获取`,
    ];
  }
  return [
    `时间依据: 本机系统时间`,
    `第三方时间证明: 未获取`,
  ];
}

export function formatOutputStrategy(value?: string | null): string {
  if (value === "minimal_required_change" || !value) return "最小必要变更";
  return value;
}

export function formatWatermarkIssueMode(value?: string | null): string {
  const map: Record<string, string> = {
    server_reserved: "后端预签发",
    server_confirmed: "后端已确认",
    server_reissued: "后端重新签发",
    offline_generated: "本地离线生成",
  };
  return value ? map[value] || value : "本地离线生成";
}

export function formatRegistryStatus(value?: string | null): string {
  const map: Record<string, string> = {
    pending_registration: "等待联网登记",
    reserved: "已预留，等待写入确认",
    server_confirmed: "后端已确认",
    offline_confirmed: "离线编号已补登记",
    conflict: "编号冲突",
    reissue_required: "需要重新签发",
    pending_registry_reconcile: "待登记仲裁",
  };
  return value ? map[value] || value : "等待联网登记";
}

export function formatPayloadAuthStatus(value?: string | null): string {
  const map: Record<string, string> = {
    verified: "已验证",
    failed: "验证失败",
    unverified: "未验证",
    pending_repair: "待修复",
  };
  return value ? map[value] || value : "未验证";
}

export function formatWorkSourceDeclaration(value?: string | null): string {
  const map: Record<string, string> = {
    unspecified: "未声明",
    human_created: "人工创作",
    ai_assisted: "AI 辅助",
    ai_generated: "AI 生成",
  };
  return value ? map[value] || value : "未声明";
}

export function formatTrainingPermissionDeclaration(value?: string | null): string {
  const map: Record<string, string> = {
    prohibited: "禁止模型训练",
    separate_authorization_required: "需单独授权",
    non_commercial_allowed: "允许非商业训练",
    commercial_allowed: "允许商业训练",
    unspecified: "未声明",
  };
  return value ? map[value] || value : "禁止模型训练";
}

export function formatAuthenticityClaimDeclaration(value?: string | null): string {
  const map: Record<string, string> = {
    unspecified: "未声明",
    synthetic: "虚构或合成",
    based_on_reality: "基于真实",
    creator_claimed_authentic: "创作者声明真实",
    authentic: "创作者声明真实",
  };
  return value ? map[value] || value : "未声明";
}

function formatDeclarationValue(value?: string | null): string {
  return value && value !== "unspecified" ? value : "未声明";
}

export function formatThirdPartyVerificationStatus(record: VaultRecord): string {
  if (record.tsaTokenPath) return "已获取第三方时间戳回执";
  if (record.networkTime) return "已完成网络授时";
  return "未启用第三方时间服务";
}

export function formatTrustedTimeSource(record: VaultRecord): string {
  if (record.tsaTokenPath) return record.tsaSource?.trim() || "第三方时间戳服务";
  if (record.networkTime) return record.tsaSource?.trim() || "网络授时服务";
  return "本机创建时间（非第三方证明）";
}

export function formatTrustedTime(record: VaultRecord): string {
  if (record.networkTime) return formatEvidenceTime(record.networkTime);
  return `${formatLocalDateTime(record.createdAt)}（本机时间）`;
}

function formatVerificationStatus(value: string): string {
  if (value === "verified") return "已通过";
  if (value === "failed") return "未通过";
  return value;
}

/** Build a basic verification summary text for clipboard copy. */
export const IMAGE_ORIENTATION_DETECTION_SCOPE =
  "桌面图片默认检测支持轴对齐、宽高各为原图 1/4 的裁切区域，以及 90/180/270 度旋转、85% 缩放、JPEG/WebP quality 75/60 的独立恢复。任意角度、多个扰动叠加、更低质量重编码或更大比例缩小不在当前承诺内。";

export const VAULT_DEEP_DETECTION_SCOPE =
  "版权库深度检测需要本机已有作品记录，用于对裁剪、尺寸变化等疑难样本做复核；它不是纯盲检测，也不会上传原始文件或保护副本。";

export function buildVerificationSummary(result: VerificationResult, filePath: string): string {
  const status = result.matched
    ? "已匹配版权记录"
    : result.confidence >= 0.95
      ? "已识别但未通过作品绑定"
      : result.confidence >= 0.5
        ? "疑似匹配"
        : "未找到对应记录";

  const lines = [
    `═══════════════════════════════════════════`,
    `       隐盾 HiddenShield 基础验证摘要`,
    `═══════════════════════════════════════════`,
    ``,
    `【验证结果】${status}`,
    `置信度: ${Math.round(result.confidence * 100)}%`,
    `验证耗时: ${formatDurationMs(result.durationMs)}`,
    result.watermarkUid ? `版权编号: ${result.watermarkUid}` : "",
    result.payloadProtocolVersion && result.payloadBytesLength
      ? `Payload 协议: V${result.payloadProtocolVersion} / ${result.payloadBytesLength} bytes`
      : "",
    result.payloadAuthStatus ? `Payload 认证状态: ${formatPayloadAuthStatus(result.payloadAuthStatus)}` : "",
    result.watermarkIdIssueMode ? `签发模式: ${result.watermarkIdIssueMode}` : "",
    result.mediaPayloadRole ? `媒体载荷角色: ${result.mediaPayloadRole}` : "",
    result.reasonCode ? `参考依据: ${result.reasonCode}` : "",
    result.reasonDetail ? `参考说明: ${result.reasonDetail}` : "",
    ``,
    `───────────── 文件信息 ─────────────`,
    `验证文件: ${filePath.split(/[\\/]/).pop() ?? filePath}`,
    result.originalHash ? `原文件作品指纹: ${result.originalHash}` : "",
    `检测时间: ${new Date().toLocaleString()}`,
    ``,
    `───────────── 检测范围 ─────────────`,
    IMAGE_ORIENTATION_DETECTION_SCOPE,
    VAULT_DEEP_DETECTION_SCOPE,
    ``,
  ];

  if (result.matched && result.matchedRecord) {
    const r = result.matchedRecord;
    lines.push(`───────────── 版权记录 ─────────────`);
    lines.push(`原始文件: ${r.fileName}`);
    lines.push(`版本次数: 第 ${r.revision} 次`);
    if (r.parentWatermarkUid) {
      lines.push(`上一版编号: ${r.parentWatermarkUid}`);
    }
    if (r.rewriteReason) {
      lines.push(`更新说明: ${r.rewriteReason}`);
    }
    lines.push(`入库时间: ${new Date(r.createdAt).toLocaleString()}`);
    if (r.resolution) {
      lines.push(`分辨率: ${r.resolution}`);
    }
    lines.push(`作品指纹: ${r.originalHash}`);
    lines.push(``);
  }

  if (result.tsaTokenPresent || result.networkTime) {
    lines.push(`───────────── 时间验证材料 ─────────────`);
    if (result.tsaTokenPresent && result.tsaSource) {
      lines.push(`可信时间回执: 已获取`);
      lines.push(`回执来源: ${result.tsaSource}`);
      lines.push(
        `状态: ${
          result.tsaTokenVerified
            ? getTsaVerificationLabel(result.tsaVerificationPath) ?? "已复验"
            : "已获取未复验"
        }`,
      );
    }
    if (result.networkTime) {
      lines.push(`网络授时 (GMT): ${result.networkTime}`);
      // Convert GMT to local time for readability
      const localTime = new Date(result.networkTime).toLocaleString();
      lines.push(`网络授时 (本地): ${localTime}`);
    }
    if (result.createdAt) {
      const localCreated = new Date(result.createdAt).toLocaleString();
      lines.push(`本地记录时间: ${localCreated}`);
    }
    if (!result.tsaTokenVerified) {
      lines.push(
        ``,
        `上述回执与网络授时仅作为补充验证材料，`,
        `   可信时间回执仍需完成独立验签后方可作为正式证明使用。`,
        ``,
      );
    } else {
      lines.push(
        ``,
        `可信时间回执：${getTsaVerificationLabel(result.tsaVerificationPath) ?? "已完成本地复验"}。`,
        ``,
      );
    }
  }

  lines.push(
    `───────────── 免责声明 ─────────────`,
    result.disclaimer,
    ``,
    `═══════════════════════════════════════════`,
    `本摘要由 HiddenShield v1.0 本地生成`,
    `数据未上传至任何服务器`,
    `═══════════════════════════════════════════`,
  );

  return lines.filter(l => l !== undefined).join("\n");
}

function formatDurationMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

export function getTsaVerificationLabel(path: TsaVerificationPath | null): string | null {
  if (path === "systemRoots") return "系统根已验证";
  if (path === "embeddedRoots") return "嵌入根已验证";
  return null;
}

// ---------------------------------------------------------------------------
// Telemetry & Data Management
// ---------------------------------------------------------------------------

export interface DataUsageInfo {
  ffmpegSizeMb: number;
  dbSizeMb: number;
  logSizeMb: number;
  totalSizeMb: number;
}

export interface UpdateInfo {
  available: boolean;
  version: string;
  body: string;
}

export async function getTelemetryEnabled(): Promise<boolean> {
  if (!isTauriRuntime()) return true;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("get_telemetry_enabled");
}

export async function setTelemetryEnabled(enabled: boolean): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("set_telemetry_enabled", { enabled });
}

export async function getTelemetryAcknowledged(): Promise<boolean> {
  if (!isTauriRuntime()) return true;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("get_telemetry_acknowledged");
}

export async function acknowledgeTelemetry(): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("acknowledge_telemetry");
}

export async function getNetworkEnabled(): Promise<boolean> {
  if (!isTauriRuntime()) return true;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("get_network_enabled");
}

export async function setNetworkEnabled(enabled: boolean): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("set_network_enabled", { enabled });
}

export async function exportCrashLog(): Promise<string> {
  if (!isTauriRuntime()) return "(mock) no crash logs";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("export_crash_log");
}

export async function getDataUsage(): Promise<DataUsageInfo> {
  if (!isTauriRuntime()) {
    return { ffmpegSizeMb: 85.2, dbSizeMb: 2.4, logSizeMb: 0.1, totalSizeMb: 87.7 };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DataUsageInfo>("get_data_usage");
}

export async function clearAllData(): Promise<string> {
  if (!isTauriRuntime()) return "所有数据已清除，可安全卸载";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("clear_all_data");
}

export async function clearCacheOnly(): Promise<string> {
  if (!isTauriRuntime()) return "缓存已清除，版权库数据已保留";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("clear_cache_only");
}

// ---------------------------------------------------------------------------
// Auto-Updater
// ---------------------------------------------------------------------------

export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!isTauriRuntime()) return null;
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await Promise.race([
    check(),
    new Promise<null>((resolve) => setTimeout(() => resolve(null), 10_000)),
  ]);
  if (!update) return null;
  return {
    available: true,
    version: update.version,
    body: update.body ?? "",
  };
}

export async function installUpdate(
  onProgress?: (downloaded: number, total: number | null) => void,
): Promise<void> {
  if (!isTauriRuntime()) return;
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return;

  let downloaded = 0;
  let contentLength: number | null = null;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      contentLength = event.data.contentLength ?? null;
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress?.(downloaded, contentLength);
    }
  });
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}

// ---------------------------------------------------------------------------
// Identity (Creator Seed)
// ---------------------------------------------------------------------------

export interface IdentityStatus {
  initialized: boolean;
  watermarkUidPreview: string | null;
  deviceIdHex: string | null;
  creatorDisplayName: string | null;
}

export async function getIdentityStatus(): Promise<IdentityStatus> {
  if (!isTauriRuntime()) {
    return {
      initialized: false,
      watermarkUidPreview: null,
      deviceIdHex: null,
      creatorDisplayName: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<IdentityStatus>("get_identity_status");
}

export async function setupIdentity(creatorInput: string): Promise<IdentityStatus> {
  if (!isTauriRuntime()) {
    return {
      initialized: true,
      watermarkUidPreview: "HS-MOCK-MOCK-MOCK",
      deviceIdHex: "deadbeef",
      creatorDisplayName: creatorInput,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<IdentityStatus>("setup_identity", { creatorInput });
}
