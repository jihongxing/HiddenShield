import { createHash } from "node:crypto";

export const AI_TRANSPARENCY_SDK_VERSION = "0.1.0";
export const CONFIRMED_MARKED_IMAGE = "confirmed_marked_image" as const;
export const MAX_GENERATED_IMAGE_BYTES = 64 * 1024 * 1024;

export type IssuerMode =
  | "hiddenshield_managed"
  | "customer_managed"
  | "platform_signed";

export type AiTransparencyErrorCategory =
  | "authorization"
  | "entitlement"
  | "validation"
  | "conflict"
  | "integrity"
  | "availability"
  | "internal";

export type AiTransparencyErrorCode =
  | "credential_invalid"
  | "credential_inactive"
  | "credential_expired"
  | "credential_scope_denied"
  | "license_inactive"
  | "license_expired"
  | "profile_not_entitled"
  | "admission_invalid"
  | "admission_expired"
  | "session_invalid"
  | "session_conflict"
  | "image_invalid"
  | "marked_image_digest_mismatch"
  | "confirm_conflict"
  | "metering_receipt_invalid"
  | "request_timeout"
  | "service_unavailable"
  | "invalid_response"
  | "internal_error";

export class AiTransparencySdkError extends Error {
  readonly code: AiTransparencyErrorCode;
  readonly category: AiTransparencyErrorCategory;
  readonly retryable: boolean;
  readonly httpStatus: number | null;
  readonly requestId: string | null;

  constructor(input: {
    code: AiTransparencyErrorCode;
    category: AiTransparencyErrorCategory;
    message: string;
    retryable?: boolean;
    httpStatus?: number | null;
    requestId?: string | null;
  }) {
    super(input.message);
    this.name = "AiTransparencySdkError";
    this.code = input.code;
    this.category = input.category;
    this.retryable = input.retryable ?? false;
    this.httpStatus = input.httpStatus ?? null;
    this.requestId = input.requestId ?? null;
  }
}

export interface ProductionProfileAdmissionRequest {
  licenseId: string;
  tenantId: string;
  workspaceId: string;
  issuerMode: IssuerMode;
  regulatoryProfileId: string;
  technicalProfileIds: string[];
}

export interface ProductionProfileAdmission {
  admissionId: string;
  status: "admitted";
  environment: "production";
  licenseId: string;
  tenantId: string;
  workspaceId: string;
  issuerMode: IssuerMode;
  regulatoryProfileId: string;
  technicalProfileIds: string[];
  entitlementVersionId: string;
  entitlementDigest: string;
  expiresAt: string;
}

export interface CreateGenerationSessionRequest {
  admission: ProductionProfileAdmission;
  idempotencyKey: string;
  generationEventId: string;
  subjectReference: string;
  contentType?: "image/png";
}

export interface GenerationSession {
  markingSessionId: string;
  admissionId: string;
  licenseId: string;
  entitlementDigest: string;
  status: "ready_to_upload";
  watermarkUid: string;
  contentType: "image/png";
  expiresAt: string;
}

export interface SubmitGeneratedImageRequest {
  session: GenerationSession;
  imageBytes: Uint8Array;
}

export interface MarkedImageSubmission {
  markingSessionId: string;
  licenseId: string;
  entitlementDigest: string;
  status: "ready_to_confirm";
  watermarkUid: string;
  contentType: "image/png";
  originalFileSha256: string;
  markedFileSha256: string;
  markedImageBytes: Uint8Array;
  confirmationToken: string;
  markerEvidenceDigest: string;
  explicitLabelReceiptDigest: string;
}

export interface ConfirmGeneratedAssetRequest {
  submission: MarkedImageSubmission;
  idempotencyKey: string;
}

export interface ConfirmedMarkedImageMeteringReceipt {
  receiptId: string;
  ledgerEntryId: string;
  licenseId: string;
  markingSessionId: string;
  meteringUnit: typeof CONFIRMED_MARKED_IMAGE;
  quantity: 1;
  ledgerStatus: "committed";
  committedAt: string;
  replayed: boolean;
}

export interface ConfirmedGeneratedAsset {
  status: "confirmed";
  manifestId: string;
  markingSessionId: string;
  watermarkUid: string;
  verificationUrl: string;
  profileStatus: "applied";
  explicitLabel: {
    text: string;
    requiredSurface: "platform_ui" | "exported_file" | "both";
  };
  meteringReceipt: ConfirmedMarkedImageMeteringReceipt;
}

export interface AiTransparencyTransport {
  admitProductionProfile(
    request: ProductionProfileAdmissionRequest,
  ): Promise<unknown>;
  createGenerationSession(request: {
    admissionId: string;
    idempotencyKey: string;
    generationEventId: string;
    subjectReference: string;
    contentType: "image/png";
  }): Promise<unknown>;
  submitGeneratedImage(request: {
    markingSessionId: string;
    contentType: "image/png";
    originalFileSha256: string;
    imageBytes: Uint8Array;
  }): Promise<unknown>;
  confirmGeneratedAsset(request: {
    markingSessionId: string;
    confirmationToken: string;
    markedFileSha256: string;
    idempotencyKey: string;
  }): Promise<unknown>;
}

export interface AiTransparencySdk {
  admitProductionProfile(
    request: ProductionProfileAdmissionRequest,
  ): Promise<ProductionProfileAdmission>;
  createGenerationSession(
    request: CreateGenerationSessionRequest,
  ): Promise<GenerationSession>;
  submitGeneratedImage(
    request: SubmitGeneratedImageRequest,
  ): Promise<MarkedImageSubmission>;
  confirmGeneratedAsset(
    request: ConfirmGeneratedAssetRequest,
  ): Promise<ConfirmedGeneratedAsset>;
}

export interface AiTransparencySdkOptions {
  baseUrl: string;
  credential: string;
  requestTimeoutMs?: number;
  fetchImpl?: typeof fetch;
  transport?: AiTransparencyTransport;
}

export interface MarkAndConfirmGeneratedImageRequest {
  admission: ProductionProfileAdmissionRequest;
  idempotencyKey: string;
  generationEventId: string;
  subjectReference: string;
  imageBytes: Uint8Array;
}

export interface MarkAndConfirmGeneratedImageResult {
  admission: ProductionProfileAdmission;
  session: GenerationSession;
  submission: MarkedImageSubmission;
  confirmation: ConfirmedGeneratedAsset;
}

export interface AiTransparencyPlatformFacade {
  admit(
    request: ProductionProfileAdmissionRequest,
  ): Promise<ProductionProfileAdmission>;
  createSession(
    request: CreateGenerationSessionRequest,
  ): Promise<GenerationSession>;
  submitImage(
    request: SubmitGeneratedImageRequest,
  ): Promise<MarkedImageSubmission>;
  confirm(
    request: ConfirmGeneratedAssetRequest,
  ): Promise<ConfirmedGeneratedAsset>;
  markAndConfirmGeneratedImage(
    request: MarkAndConfirmGeneratedImageRequest,
  ): Promise<MarkAndConfirmGeneratedImageResult>;
}

export interface AiTransparencyPlatformApiRequest {
  method: string;
  path: string;
  headers?: Readonly<Record<string, string>>;
  body: unknown;
}

export interface AiTransparencyPlatformApiResponse {
  statusCode: number;
  headers: Readonly<Record<string, string>>;
  body: unknown;
}

export interface AiTransparencyPlatformApiFacade {
  handle(
    request: AiTransparencyPlatformApiRequest,
  ): Promise<AiTransparencyPlatformApiResponse>;
}

export interface AiTransparencyPlatformApiFacadeOptions {
  sdk: AiTransparencySdk;
  authorize(
    request: AiTransparencyPlatformApiRequest,
  ): boolean | Promise<boolean>;
}

export function createAiTransparencySdk(
  options: AiTransparencySdkOptions,
): AiTransparencySdk {
  validateOptions(options);
  const transport =
    options.transport ??
    createFetchTransport({
      baseUrl: options.baseUrl,
      credential: options.credential,
      requestTimeoutMs: options.requestTimeoutMs ?? 10_000,
      fetchImpl: options.fetchImpl ?? fetch,
    });

  return {
    async admitProductionProfile(request) {
      validateAdmissionRequest(request);
      const response = await invokeTransport(() =>
        transport.admitProductionProfile(request),
      );
      return parseAdmission(response, request);
    },

    async createGenerationSession(request) {
      validateAdmission(request.admission);
      assertNotExpired(request.admission.expiresAt, "admission_expired");
      assertNonEmpty(request.idempotencyKey, "idempotencyKey");
      assertNonEmpty(request.generationEventId, "generationEventId");
      assertNonEmpty(request.subjectReference, "subjectReference");
      const response = await invokeTransport(() =>
        transport.createGenerationSession({
          admissionId: request.admission.admissionId,
          idempotencyKey: request.idempotencyKey,
          generationEventId: request.generationEventId,
          subjectReference: request.subjectReference,
          contentType: request.contentType ?? "image/png",
        }),
      );
      return parseSession(response, request.admission);
    },

    async submitGeneratedImage(request) {
      validateSession(request.session);
      assertNotExpired(request.session.expiresAt, "session_invalid");
      if (!(request.imageBytes instanceof Uint8Array) || request.imageBytes.byteLength === 0) {
        throw sdkError("image_invalid", "validation", "PNG bytes are required");
      }
      if (request.imageBytes.byteLength > MAX_GENERATED_IMAGE_BYTES) {
        throw sdkError(
          "image_invalid",
          "validation",
          "PNG exceeds the 64 MiB SDK limit",
        );
      }
      if (!isPng(request.imageBytes)) {
        throw sdkError("image_invalid", "validation", "Only image/png is accepted");
      }
      const originalFileSha256 = sha256Hex(request.imageBytes);
      const response = await invokeTransport(() =>
        transport.submitGeneratedImage({
          markingSessionId: request.session.markingSessionId,
          contentType: "image/png",
          originalFileSha256,
          imageBytes: request.imageBytes,
        }),
      );
      return parseSubmission(
        response,
        request.session,
        originalFileSha256,
      );
    },

    async confirmGeneratedAsset(request) {
      validateSubmission(request.submission);
      assertNonEmpty(request.idempotencyKey, "idempotencyKey");
      const response = await invokeTransport(() =>
        transport.confirmGeneratedAsset({
          markingSessionId: request.submission.markingSessionId,
          confirmationToken: request.submission.confirmationToken,
          markedFileSha256: request.submission.markedFileSha256,
          idempotencyKey: request.idempotencyKey,
        }),
      );
      return parseConfirmation(response, request.submission);
    },
  };
}

export function createAiTransparencyPlatformFacade(
  sdk: AiTransparencySdk,
): AiTransparencyPlatformFacade {
  return {
    admit: (request) => sdk.admitProductionProfile(request),
    createSession: (request) => sdk.createGenerationSession(request),
    submitImage: (request) => sdk.submitGeneratedImage(request),
    confirm: (request) => sdk.confirmGeneratedAsset(request),
    async markAndConfirmGeneratedImage(request) {
      const admission = await sdk.admitProductionProfile(request.admission);
      const session = await sdk.createGenerationSession({
        admission,
        idempotencyKey: request.idempotencyKey,
        generationEventId: request.generationEventId,
        subjectReference: request.subjectReference,
      });
      const submission = await sdk.submitGeneratedImage({
        session,
        imageBytes: request.imageBytes,
      });
      const confirmation = await sdk.confirmGeneratedAsset({
        submission,
        idempotencyKey: request.idempotencyKey,
      });
      return { admission, session, submission, confirmation };
    },
  };
}

export function createAiTransparencyPlatformApiFacade(
  options: AiTransparencyPlatformApiFacadeOptions,
): AiTransparencyPlatformApiFacade {
  return {
    async handle(request) {
      const headers = {
        "content-type": "application/json",
        "cache-control": "no-store",
      };
      try {
        const authorized = await options.authorize(request);
        if (!authorized) {
          return {
            statusCode: 401,
            headers,
            body: {
              errorCode: "credential_invalid",
              category: "authorization",
              retryable: false,
            },
          };
        }
        if (request.method.toUpperCase() !== "POST") {
          return {
            statusCode: 405,
            headers,
            body: {
              errorCode: "admission_invalid",
              category: "validation",
              retryable: false,
            },
          };
        }
        if (request.path === "/v1/ai-transparency/admissions") {
          const body = parseFacadeAdmissionRequest(request.body);
          return {
            statusCode: 201,
            headers,
            body: await options.sdk.admitProductionProfile(body),
          };
        }
        if (request.path === "/v1/ai-transparency/sessions") {
          const body = parseFacadeSessionRequest(request.body);
          return {
            statusCode: 201,
            headers,
            body: await options.sdk.createGenerationSession(body),
          };
        }
        if (request.path === "/v1/ai-transparency/images/mark") {
          const body = parseFacadeImageRequest(request.body);
          const result = await options.sdk.submitGeneratedImage(body);
          const { markedImageBytes, ...metadata } = result;
          return {
            statusCode: 200,
            headers,
            body: {
              ...metadata,
              markedImageBase64:
                Buffer.from(markedImageBytes).toString("base64"),
            },
          };
        }
        if (request.path === "/v1/ai-transparency/images/confirm") {
          const body = parseFacadeConfirmRequest(request.body);
          return {
            statusCode: 200,
            headers,
            body: await options.sdk.confirmGeneratedAsset(body),
          };
        }
        return {
          statusCode: 404,
          headers,
          body: {
            errorCode: "admission_invalid",
            category: "validation",
            retryable: false,
          },
        };
      } catch (error) {
        const sdkFailure =
          error instanceof AiTransparencySdkError
            ? error
            : sdkError(
                "internal_error",
                "internal",
                "Platform facade failed closed",
              );
        return {
          statusCode:
            sdkFailure.httpStatus ?? statusForCategory(sdkFailure.category),
          headers,
          body: {
            errorCode: sdkFailure.code,
            category: sdkFailure.category,
            retryable: sdkFailure.retryable,
            requestId: sdkFailure.requestId,
          },
        };
      }
    },
  };
}

interface FetchTransportOptions {
  baseUrl: string;
  credential: string;
  requestTimeoutMs: number;
  fetchImpl: typeof fetch;
}

function createFetchTransport(
  options: FetchTransportOptions,
): AiTransparencyTransport {
  const baseUrl = options.baseUrl.replace(/\/+$/, "");
  const postJson = async (path: string, body: unknown): Promise<unknown> => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), options.requestTimeoutMs);
    try {
      const response = await options.fetchImpl(`${baseUrl}${path}`, {
        method: "POST",
        headers: {
          authorization: `Bearer ${options.credential}`,
          "content-type": "application/json",
          "x-hiddenshield-sdk-version": AI_TRANSPARENCY_SDK_VERSION,
        },
        body: JSON.stringify(body),
        signal: controller.signal,
      });
      const requestId = response.headers.get("x-request-id");
      const payload = await parseJsonResponse(response, requestId);
      if (!response.ok) {
        throw mapRemoteError(response.status, requestId, payload);
      }
      return payload;
    } catch (error) {
      if (error instanceof AiTransparencySdkError) throw error;
      if (error instanceof Error && error.name === "AbortError") {
        throw sdkError(
          "request_timeout",
          "availability",
          "HiddenShield request timed out",
          true,
        );
      }
      throw sdkError(
        "service_unavailable",
        "availability",
        "HiddenShield service is unavailable",
        true,
      );
    } finally {
      clearTimeout(timer);
    }
  };

  return {
    admitProductionProfile: (request) =>
      postJson("/v1/ai-transparency/admissions", {
        ...request,
        environment: "production",
        mediaType: "image",
      }),
    createGenerationSession: (request) =>
      postJson("/v1/ai-transparency/sessions", request),
    submitGeneratedImage: (request) =>
      postJson("/v1/ai-transparency/images/mark", {
        markingSessionId: request.markingSessionId,
        contentType: request.contentType,
        originalFileSha256: request.originalFileSha256,
        imageBase64: Buffer.from(request.imageBytes).toString("base64"),
      }),
    confirmGeneratedAsset: (request) =>
      postJson("/v1/ai-transparency/images/confirm", request),
  };
}

async function parseJsonResponse(
  response: Response,
  requestId: string | null,
): Promise<unknown> {
  try {
    return await response.json();
  } catch {
    throw new AiTransparencySdkError({
      code: "invalid_response",
      category: "internal",
      message: "HiddenShield returned a non-JSON response",
      retryable: false,
      httpStatus: response.status,
      requestId,
    });
  }
}

function mapRemoteError(
  httpStatus: number,
  requestId: string | null,
  payload: unknown,
): AiTransparencySdkError {
  const object = asRecord(payload);
  const remoteCode =
    typeof object?.errorCode === "string" ? object.errorCode : "internal_error";
  const code = normalizeRemoteCode(remoteCode);
  const category = categoryForCode(code);
  return new AiTransparencySdkError({
    code,
    category,
    message:
      typeof object?.message === "string"
        ? object.message
        : "HiddenShield rejected the request",
    retryable:
      typeof object?.retryable === "boolean"
        ? object.retryable
        : category === "availability",
    httpStatus,
    requestId,
  });
}

async function invokeTransport<T>(operation: () => Promise<T>): Promise<T> {
  try {
    return await operation();
  } catch (error) {
    if (error instanceof AiTransparencySdkError) throw error;
    throw sdkError(
      "service_unavailable",
      "availability",
      "HiddenShield transport failed closed",
      true,
    );
  }
}

function validateOptions(options: AiTransparencySdkOptions): void {
  assertNonEmpty(options.baseUrl, "baseUrl");
  assertNonEmpty(options.credential, "credential");
  if (
    /example|changeme|placeholder/i.test(options.credential) ||
    options.credential.length < 16
  ) {
    throw sdkError(
      "credential_invalid",
      "authorization",
      "A non-placeholder server credential is required",
    );
  }
  if (!options.transport && !/^https:\/\//i.test(options.baseUrl)) {
    throw sdkError(
      "credential_invalid",
      "authorization",
      "Production API baseUrl must use HTTPS",
    );
  }
  const timeout = options.requestTimeoutMs ?? 10_000;
  if (!Number.isInteger(timeout) || timeout < 100 || timeout > 30_000) {
    throw sdkError(
      "admission_invalid",
      "validation",
      "requestTimeoutMs must be between 100 and 30000",
    );
  }
}

function validateAdmissionRequest(
  request: ProductionProfileAdmissionRequest,
): void {
  assertNonEmpty(request.licenseId, "licenseId");
  assertNonEmpty(request.tenantId, "tenantId");
  assertNonEmpty(request.workspaceId, "workspaceId");
  assertNonEmpty(request.regulatoryProfileId, "regulatoryProfileId");
  if (
    !["hiddenshield_managed", "customer_managed", "platform_signed"].includes(
      request.issuerMode,
    )
  ) {
    throw sdkError("admission_invalid", "validation", "issuerMode is invalid");
  }
  if (
    !Array.isArray(request.technicalProfileIds) ||
    request.technicalProfileIds.length === 0 ||
    request.technicalProfileIds.some(
      (profileId) => typeof profileId !== "string" || !profileId.trim(),
    )
  ) {
    throw sdkError(
      "admission_invalid",
      "validation",
      "technicalProfileIds are required",
    );
  }
}

function parseFacadeAdmissionRequest(
  value: unknown,
): ProductionProfileAdmissionRequest {
  const object = requireRecord(value);
  const request: ProductionProfileAdmissionRequest = {
    licenseId: requireString(object.licenseId, "admission_invalid"),
    tenantId: requireString(object.tenantId, "admission_invalid"),
    workspaceId: requireString(object.workspaceId, "admission_invalid"),
    issuerMode: requireString(
      object.issuerMode,
      "admission_invalid",
    ) as IssuerMode,
    regulatoryProfileId: requireString(
      object.regulatoryProfileId,
      "admission_invalid",
    ),
    technicalProfileIds: requireStringArray(
      object.technicalProfileIds,
      "admission_invalid",
    ),
  };
  validateAdmissionRequest(request);
  return request;
}

function parseFacadeSessionRequest(
  value: unknown,
): CreateGenerationSessionRequest {
  const object = requireRecord(value);
  const admission = parseFacadeAdmission(object.admission);
  return {
    admission,
    idempotencyKey: requireString(
      object.idempotencyKey,
      "admission_invalid",
    ),
    generationEventId: requireString(
      object.generationEventId,
      "admission_invalid",
    ),
    subjectReference: requireString(
      object.subjectReference,
      "admission_invalid",
    ),
    contentType: "image/png",
  };
}

function parseFacadeImageRequest(value: unknown): SubmitGeneratedImageRequest {
  const object = requireRecord(value);
  const session = parseFacadeSession(object.session);
  const imageBase64 = requireString(object.imageBase64, "image_invalid");
  const imageBytes = Uint8Array.from(Buffer.from(imageBase64, "base64"));
  if (imageBytes.byteLength === 0) {
    throw sdkError("image_invalid", "validation", "imageBase64 is invalid");
  }
  return { session, imageBytes };
}

function parseFacadeConfirmRequest(
  value: unknown,
): ConfirmGeneratedAssetRequest {
  const object = requireRecord(value);
  return {
    submission: parseFacadeSubmission(object.submission),
    idempotencyKey: requireString(
      object.idempotencyKey,
      "admission_invalid",
    ),
  };
}

function parseFacadeAdmission(value: unknown): ProductionProfileAdmission {
  const object = requireRecord(value);
  const admission: ProductionProfileAdmission = {
    admissionId: requireString(object.admissionId, "admission_invalid"),
    status: requireString(object.status, "admission_invalid") as "admitted",
    environment: requireString(
      object.environment,
      "admission_invalid",
    ) as "production",
    licenseId: requireString(object.licenseId, "admission_invalid"),
    tenantId: requireString(object.tenantId, "admission_invalid"),
    workspaceId: requireString(object.workspaceId, "admission_invalid"),
    issuerMode: requireString(
      object.issuerMode,
      "admission_invalid",
    ) as IssuerMode,
    regulatoryProfileId: requireString(
      object.regulatoryProfileId,
      "admission_invalid",
    ),
    technicalProfileIds: requireStringArray(
      object.technicalProfileIds,
      "admission_invalid",
    ),
    entitlementVersionId: requireString(
      object.entitlementVersionId,
      "admission_invalid",
    ),
    entitlementDigest: requireSha256(
      object.entitlementDigest,
      "admission_invalid",
    ),
    expiresAt: requireTimestamp(object.expiresAt, "admission_invalid"),
  };
  validateAdmission(admission);
  return admission;
}

function parseFacadeSession(value: unknown): GenerationSession {
  const object = requireRecord(value);
  const session: GenerationSession = {
    markingSessionId: requireString(object.markingSessionId, "session_invalid"),
    admissionId: requireString(object.admissionId, "session_invalid"),
    licenseId: requireString(object.licenseId, "session_invalid"),
    entitlementDigest: requireSha256(
      object.entitlementDigest,
      "session_invalid",
    ),
    status: requireString(object.status, "session_invalid") as "ready_to_upload",
    watermarkUid: requireString(object.watermarkUid, "session_invalid"),
    contentType: requireString(
      object.contentType,
      "session_invalid",
    ) as "image/png",
    expiresAt: requireTimestamp(object.expiresAt, "session_invalid"),
  };
  validateSession(session);
  return session;
}

function parseFacadeSubmission(value: unknown): MarkedImageSubmission {
  const object = requireRecord(value);
  const markedImageBase64 = requireString(
    object.markedImageBase64,
    "image_invalid",
  );
  const submission: MarkedImageSubmission = {
    markingSessionId: requireString(
      object.markingSessionId,
      "session_invalid",
    ),
    licenseId: requireString(object.licenseId, "session_invalid"),
    entitlementDigest: requireSha256(
      object.entitlementDigest,
      "session_invalid",
    ),
    status: requireString(
      object.status,
      "session_invalid",
    ) as "ready_to_confirm",
    watermarkUid: requireString(object.watermarkUid, "session_invalid"),
    contentType: requireString(object.contentType, "image_invalid") as "image/png",
    originalFileSha256: requireSha256(
      object.originalFileSha256,
      "marked_image_digest_mismatch",
    ),
    markedFileSha256: requireSha256(
      object.markedFileSha256,
      "marked_image_digest_mismatch",
    ),
    markedImageBytes: Uint8Array.from(
      Buffer.from(markedImageBase64, "base64"),
    ),
    confirmationToken: requireString(
      object.confirmationToken,
      "invalid_response",
    ),
    markerEvidenceDigest: requireSha256(
      object.markerEvidenceDigest,
      "invalid_response",
    ),
    explicitLabelReceiptDigest: requireSha256(
      object.explicitLabelReceiptDigest,
      "invalid_response",
    ),
  };
  validateSubmission(submission);
  return submission;
}

function parseAdmission(
  value: unknown,
  request: ProductionProfileAdmissionRequest,
): ProductionProfileAdmission {
  const object = requireRecord(value);
  requireExact(object.status, "admitted", "admission_invalid");
  requireExact(object.environment, "production", "admission_invalid");
  requireExact(object.licenseId, request.licenseId, "admission_invalid");
  requireExact(object.tenantId, request.tenantId, "admission_invalid");
  requireExact(object.workspaceId, request.workspaceId, "admission_invalid");
  requireExact(object.issuerMode, request.issuerMode, "admission_invalid");
  requireExact(
    object.regulatoryProfileId,
    request.regulatoryProfileId,
    "profile_not_entitled",
  );
  const technicalProfileIds = requireStringArray(
    object.technicalProfileIds,
    "profile_not_entitled",
  );
  if (
    technicalProfileIds.length !== request.technicalProfileIds.length ||
    request.technicalProfileIds.some(
      (profileId) => !technicalProfileIds.includes(profileId),
    )
  ) {
    throw sdkError(
      "profile_not_entitled",
      "entitlement",
      "Technical Profile entitlement response does not match the request",
    );
  }
  const admission: ProductionProfileAdmission = {
    admissionId: requireString(object.admissionId, "admission_invalid"),
    status: "admitted",
    environment: "production",
    licenseId: request.licenseId,
    tenantId: request.tenantId,
    workspaceId: request.workspaceId,
    issuerMode: request.issuerMode,
    regulatoryProfileId: request.regulatoryProfileId,
    technicalProfileIds,
    entitlementVersionId: requireString(
      object.entitlementVersionId,
      "admission_invalid",
    ),
    entitlementDigest: requireSha256(
      object.entitlementDigest,
      "admission_invalid",
    ),
    expiresAt: requireTimestamp(object.expiresAt, "admission_invalid"),
  };
  assertNotExpired(admission.expiresAt, "admission_expired");
  return admission;
}

function validateAdmission(admission: ProductionProfileAdmission): void {
  if (
    admission.status !== "admitted" ||
    admission.environment !== "production" ||
    !admission.admissionId ||
    !admission.licenseId ||
    !admission.tenantId ||
    !admission.workspaceId ||
    !admission.regulatoryProfileId ||
    !["hiddenshield_managed", "customer_managed", "platform_signed"].includes(
      admission.issuerMode,
    ) ||
    admission.technicalProfileIds.length === 0 ||
    !admission.entitlementVersionId ||
    !isSha256(admission.entitlementDigest)
  ) {
    throw sdkError(
      "admission_invalid",
      "entitlement",
      "A valid production admission is required",
    );
  }
}

function parseSession(
  value: unknown,
  admission: ProductionProfileAdmission,
): GenerationSession {
  const object = requireRecord(value);
  requireExact(object.status, "ready_to_upload", "session_invalid");
  requireExact(object.admissionId, admission.admissionId, "session_invalid");
  requireExact(object.licenseId, admission.licenseId, "session_invalid");
  requireExact(
    object.entitlementDigest,
    admission.entitlementDigest,
    "session_invalid",
  );
  requireExact(object.contentType, "image/png", "session_invalid");
  const session: GenerationSession = {
    markingSessionId: requireString(object.markingSessionId, "session_invalid"),
    admissionId: admission.admissionId,
    licenseId: admission.licenseId,
    entitlementDigest: admission.entitlementDigest,
    status: "ready_to_upload",
    watermarkUid: requireString(object.watermarkUid, "session_invalid"),
    contentType: "image/png",
    expiresAt: requireTimestamp(object.expiresAt, "session_invalid"),
  };
  assertNotExpired(session.expiresAt, "session_invalid");
  return session;
}

function validateSession(session: GenerationSession): void {
  if (
    session.status !== "ready_to_upload" ||
    session.contentType !== "image/png" ||
    !session.markingSessionId ||
    !session.admissionId ||
    !session.licenseId ||
    !isSha256(session.entitlementDigest) ||
    !session.watermarkUid
  ) {
    throw sdkError(
      "session_invalid",
      "validation",
      "A ready_to_upload marking session is required",
    );
  }
}

function parseSubmission(
  value: unknown,
  session: GenerationSession,
  originalFileSha256: string,
): MarkedImageSubmission {
  const object = requireRecord(value);
  requireExact(object.status, "ready_to_confirm", "session_invalid");
  requireExact(
    object.markingSessionId,
    session.markingSessionId,
    "session_invalid",
  );
  requireExact(object.licenseId, session.licenseId, "session_invalid");
  requireExact(
    object.entitlementDigest,
    session.entitlementDigest,
    "session_invalid",
  );
  requireExact(object.watermarkUid, session.watermarkUid, "session_invalid");
  requireExact(object.contentType, "image/png", "image_invalid");
  requireExact(
    object.originalFileSha256,
    originalFileSha256,
    "marked_image_digest_mismatch",
  );
  const markedImageBase64 = requireString(
    object.markedImageBase64,
    "invalid_response",
  );
  const markedImageBytes = Uint8Array.from(
    Buffer.from(markedImageBase64, "base64"),
  );
  if (markedImageBytes.byteLength === 0 || !isPng(markedImageBytes)) {
    throw sdkError(
      "image_invalid",
      "integrity",
      "Marked image bytes are not a PNG",
    );
  }
  const markedFileSha256 = requireSha256(
    object.markedFileSha256,
    "marked_image_digest_mismatch",
  );
  if (sha256Hex(markedImageBytes) !== markedFileSha256) {
    throw sdkError(
      "marked_image_digest_mismatch",
      "integrity",
      "Marked image digest verification failed",
    );
  }
  return {
    markingSessionId: session.markingSessionId,
    licenseId: session.licenseId,
    entitlementDigest: session.entitlementDigest,
    status: "ready_to_confirm",
    watermarkUid: session.watermarkUid,
    contentType: "image/png",
    originalFileSha256,
    markedFileSha256,
    markedImageBytes,
    confirmationToken: requireString(
      object.confirmationToken,
      "invalid_response",
    ),
    markerEvidenceDigest: requireSha256(
      object.markerEvidenceDigest,
      "invalid_response",
    ),
    explicitLabelReceiptDigest: requireSha256(
      object.explicitLabelReceiptDigest,
      "invalid_response",
    ),
  };
}

function validateSubmission(submission: MarkedImageSubmission): void {
  if (
    submission.status !== "ready_to_confirm" ||
    submission.contentType !== "image/png" ||
    !submission.confirmationToken ||
    !submission.licenseId ||
    !isSha256(submission.entitlementDigest) ||
    !isSha256(submission.originalFileSha256) ||
    !isSha256(submission.markedFileSha256) ||
    !isSha256(submission.markerEvidenceDigest) ||
    !isSha256(submission.explicitLabelReceiptDigest) ||
    sha256Hex(submission.markedImageBytes) !== submission.markedFileSha256
  ) {
    throw sdkError(
      "marked_image_digest_mismatch",
      "integrity",
      "A verified ready_to_confirm submission is required",
    );
  }
}

function parseConfirmation(
  value: unknown,
  submission: MarkedImageSubmission,
): ConfirmedGeneratedAsset {
  const object = requireRecord(value);
  requireExact(object.status, "confirmed", "confirm_conflict");
  requireExact(
    object.markingSessionId,
    submission.markingSessionId,
    "confirm_conflict",
  );
  requireExact(object.watermarkUid, submission.watermarkUid, "confirm_conflict");
  requireExact(object.profileStatus, "applied", "profile_not_entitled");
  const explicitLabel = requireRecord(object.explicitLabel);
  const requiredSurface = requireString(
    explicitLabel.requiredSurface,
    "invalid_response",
  );
  if (!["platform_ui", "exported_file", "both"].includes(requiredSurface)) {
    throw sdkError(
      "invalid_response",
      "internal",
      "Explicit label surface is invalid",
    );
  }
  const receiptObject = requireRecord(object.meteringReceipt);
  requireExact(
    receiptObject.meteringUnit,
    CONFIRMED_MARKED_IMAGE,
    "metering_receipt_invalid",
  );
  requireExact(receiptObject.quantity, 1, "metering_receipt_invalid");
  requireExact(
    receiptObject.ledgerStatus,
    "committed",
    "metering_receipt_invalid",
  );
  requireExact(
    receiptObject.markingSessionId,
    submission.markingSessionId,
    "metering_receipt_invalid",
  );
  requireExact(
    receiptObject.licenseId,
    submission.licenseId,
    "metering_receipt_invalid",
  );
  return {
    status: "confirmed",
    manifestId: requireString(object.manifestId, "invalid_response"),
    markingSessionId: submission.markingSessionId,
    watermarkUid: submission.watermarkUid,
    verificationUrl: requireHttpsUrl(object.verificationUrl),
    profileStatus: "applied",
    explicitLabel: {
      text: requireString(explicitLabel.text, "invalid_response"),
      requiredSurface: requiredSurface as
        | "platform_ui"
        | "exported_file"
        | "both",
    },
    meteringReceipt: {
      receiptId: requireString(receiptObject.receiptId, "metering_receipt_invalid"),
      ledgerEntryId: requireString(
        receiptObject.ledgerEntryId,
        "metering_receipt_invalid",
      ),
      licenseId: submission.licenseId,
      markingSessionId: submission.markingSessionId,
      meteringUnit: CONFIRMED_MARKED_IMAGE,
      quantity: 1,
      ledgerStatus: "committed",
      committedAt: requireTimestamp(
        receiptObject.committedAt,
        "metering_receipt_invalid",
      ),
      replayed: requireBoolean(
        receiptObject.replayed,
        "metering_receipt_invalid",
      ),
    },
  };
}

function normalizeRemoteCode(code: string): AiTransparencyErrorCode {
  const aliases: Record<string, AiTransparencyErrorCode> = {
    ai_credential_unauthorized: "credential_invalid",
    ai_credential_inactive: "credential_inactive",
    ai_credential_expired: "credential_expired",
    ai_credential_scope_denied: "credential_scope_denied",
    ai_credential_environment_mismatch: "credential_invalid",
    ai_credential_issuer_mode_denied: "profile_not_entitled",
    ai_license_inactive: "license_inactive",
    ai_license_expired: "license_expired",
    ai_environment_mismatch: "admission_invalid",
    ai_profile_not_entitled: "profile_not_entitled",
    ai_idempotency_conflict: "session_conflict",
    ai_session_state_invalid: "session_invalid",
    ai_confirmation_conflict: "confirm_conflict",
    ai_executor_session_invalid: "session_invalid",
    ai_executor_profile_invalid: "profile_not_entitled",
    ai_executor_confirm_rejected: "confirm_conflict",
    ai_subject_digest_invalid: "marked_image_digest_mismatch",
    ai_evidence_invalid: "image_invalid",
    ai_marker_requirement_failed: "image_invalid",
    ai_explicit_label_requirement_failed: "profile_not_entitled",
  };
  return (
    aliases[code] ??
    (isErrorCode(code) ? code : "internal_error")
  );
}

function isErrorCode(code: string): code is AiTransparencyErrorCode {
  return [
    "credential_invalid",
    "credential_inactive",
    "credential_expired",
    "credential_scope_denied",
    "license_inactive",
    "license_expired",
    "profile_not_entitled",
    "admission_invalid",
    "admission_expired",
    "session_invalid",
    "session_conflict",
    "image_invalid",
    "marked_image_digest_mismatch",
    "confirm_conflict",
    "metering_receipt_invalid",
    "request_timeout",
    "service_unavailable",
    "invalid_response",
    "internal_error",
  ].includes(code);
}

function categoryForCode(
  code: AiTransparencyErrorCode,
): AiTransparencyErrorCategory {
  if (code.startsWith("credential_")) return "authorization";
  if (
    code.startsWith("license_") ||
    code === "profile_not_entitled" ||
    code.startsWith("admission_")
  ) {
    return "entitlement";
  }
  if (code.includes("digest") || code === "metering_receipt_invalid") {
    return "integrity";
  }
  if (code.includes("conflict")) return "conflict";
  if (code === "request_timeout" || code === "service_unavailable") {
    return "availability";
  }
  if (code === "internal_error" || code === "invalid_response") return "internal";
  return "validation";
}

function statusForCategory(category: AiTransparencyErrorCategory): number {
  switch (category) {
    case "authorization":
      return 401;
    case "entitlement":
      return 403;
    case "validation":
    case "integrity":
      return 422;
    case "conflict":
      return 409;
    case "availability":
      return 503;
    case "internal":
      return 500;
  }
}

function requireRecord(value: unknown): Record<string, unknown> {
  const object = asRecord(value);
  if (!object) {
    throw sdkError(
      "invalid_response",
      "internal",
      "HiddenShield response must be an object",
    );
  }
  return object;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function requireString(
  value: unknown,
  code: AiTransparencyErrorCode,
): string {
  if (typeof value !== "string" || !value.trim()) {
    throw sdkError(code, categoryForCode(code), "Required response field is missing");
  }
  return value;
}

function requireStringArray(
  value: unknown,
  code: AiTransparencyErrorCode,
): string[] {
  if (
    !Array.isArray(value) ||
    value.length === 0 ||
    value.some((item) => typeof item !== "string" || !item.trim())
  ) {
    throw sdkError(code, categoryForCode(code), "Required Profile list is invalid");
  }
  return [...new Set(value as string[])];
}

function requireBoolean(
  value: unknown,
  code: AiTransparencyErrorCode,
): boolean {
  if (typeof value !== "boolean") {
    throw sdkError(code, categoryForCode(code), "Required boolean is missing");
  }
  return value;
}

function requireSha256(
  value: unknown,
  code: AiTransparencyErrorCode,
): string {
  const digest = requireString(value, code);
  if (!isSha256(digest)) {
    throw sdkError(code, categoryForCode(code), "SHA-256 digest is invalid");
  }
  return digest;
}

function requireTimestamp(
  value: unknown,
  code: AiTransparencyErrorCode,
): string {
  const timestamp = requireString(value, code);
  if (!Number.isFinite(Date.parse(timestamp))) {
    throw sdkError(code, categoryForCode(code), "Timestamp is invalid");
  }
  return timestamp;
}

function requireHttpsUrl(value: unknown): string {
  const url = requireString(value, "invalid_response");
  if (!/^https:\/\//i.test(url)) {
    throw sdkError(
      "invalid_response",
      "internal",
      "Verification URL must use HTTPS",
    );
  }
  return url;
}

function requireExact(
  actual: unknown,
  expected: unknown,
  code: AiTransparencyErrorCode,
): void {
  if (actual !== expected) {
    throw sdkError(
      code,
      categoryForCode(code),
      "HiddenShield response binding mismatch",
    );
  }
}

function assertNonEmpty(value: string, field: string): void {
  if (typeof value !== "string" || !value.trim()) {
    throw sdkError(
      "admission_invalid",
      "validation",
      `${field} is required`,
    );
  }
}

function assertNotExpired(
  expiresAt: string,
  code: AiTransparencyErrorCode,
): void {
  if (!Number.isFinite(Date.parse(expiresAt)) || Date.parse(expiresAt) <= Date.now()) {
    throw sdkError(code, categoryForCode(code), "Authorization has expired");
  }
}

function isPng(bytes: Uint8Array): boolean {
  return (
    bytes.byteLength >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47 &&
    bytes[4] === 0x0d &&
    bytes[5] === 0x0a &&
    bytes[6] === 0x1a &&
    bytes[7] === 0x0a
  );
}

function isSha256(value: string): boolean {
  return /^[a-f0-9]{64}$/.test(value);
}

function sha256Hex(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function sdkError(
  code: AiTransparencyErrorCode,
  category: AiTransparencyErrorCategory,
  message: string,
  retryable = false,
): AiTransparencySdkError {
  return new AiTransparencySdkError({
    code,
    category,
    message,
    retryable,
  });
}
