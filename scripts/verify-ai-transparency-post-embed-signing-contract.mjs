import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const root = 'docs/contracts/ai-transparency-post-embed-signing';
const commandSchema = readJson(`${root}/production-post-embed-signing-command-v1.schema.json`);
const receiptSchema = readJson(`${root}/production-post-embed-signing-receipt-v1.schema.json`);
const artifactReceiptSchema = readJson(
  `${root}/production-post-embed-artifact-receipt-v1.schema.json`,
);
const profileSchema = readJson(`${root}/production-post-embed-signing-profile-v1.schema.json`);
const base = readJson(`${root}/base-production-input-v1.json`);
const baseArtifactReceipts = readJson(`${root}/base-production-artifact-receipts-v1.json`);
const fixtureNames = [
  'success-v1.fixture.json',
  'signer-rejected-v1.fixture.json',
  'receipt-hash-mismatch-v1.fixture.json',
  'c2pa-readback-failure-v1.fixture.json',
  'v3-readback-failure-v1.fixture.json',
  'confirm-rollback-v1.fixture.json',
  'duplicate-replay-v1.fixture.json',
  'concurrent-reservation-v1.fixture.json',
  'artifact-finalize-recovery-v1.fixture.json',
  'crash-after-reservation-v1.fixture.json',
  'crash-after-signer-v1.fixture.json',
  'crash-after-artifact-stage-v1.fixture.json',
  'crash-after-confirm-v1.fixture.json',
];
const fixtures = Object.fromEntries(
  fixtureNames.map((name) => [name, readJson(`${root}/${name}`)]),
);

assertSchemaIds();
assertRequiredFields(commandSchema, base.command, 'command');
assertRequiredFields(profileSchema, base.profile, 'profile');
assertRequiredFields(
  receiptSchema.$defs.authorizationReceipt,
  base.authorizationReceipt,
  'authorization receipt',
);
assertRequiredFields(receiptSchema.$defs.signerReceipt, base.signerReceipt, 'signer receipt');
assertRequiredFields(
  artifactReceiptSchema,
  baseArtifactReceipts.stageReceipt,
  'artifact stage receipt',
);
assertRequiredFields(
  artifactReceiptSchema,
  baseArtifactReceipts.finalizeReceipt,
  'artifact finalize receipt',
);
assertBaseBindings();
assertAdapterReceiptBindings();
assertFixtureSet();
assertSuccess();
assertSignerRejected();
assertReceiptHashMismatch();
assertC2paReadbackFailure();
assertV3ReadbackFailure();
assertConfirmRollback();
assertDuplicateReplay();
assertConcurrentReservation();
assertArtifactFinalizeRecovery();
assertCrashRecoveryFixtures();

console.log(
  `AI Transparency post-embed signing contract passed (${fixtureNames.length} fixtures)`,
);

function assertSchemaIds() {
  assert(
    commandSchema.$id.endsWith('production-post-embed-signing-command-v1.schema.json'),
    'command schema ID must remain frozen',
  );
  assert(
    receiptSchema.$id.endsWith('production-post-embed-signing-receipt-v1.schema.json'),
    'receipt schema ID must remain frozen',
  );
  assert(
    profileSchema.$id.endsWith('production-post-embed-signing-profile-v1.schema.json'),
    'Profile schema ID must remain frozen',
  );
  assert(
    artifactReceiptSchema.$id.endsWith(
      'production-post-embed-artifact-receipt-v1.schema.json',
    ),
    'artifact receipt schema ID must remain frozen',
  );
  assert(
    commandSchema.properties.schemaVersion.const ===
      'hs-ai-production-post-embed-signing-command-v1',
    'command schema version must remain frozen',
  );
  assert(
    profileSchema.properties.allowEphemeralSigner.const === false,
    'production Profile must forbid ephemeral signers',
  );
}

function assertAdapterReceiptBindings() {
  const { signerReceipt } = base;
  const { stageReceipt, finalizeReceipt } = baseArtifactReceipts;
  assert(
    signerReceipt.signerInvocationKey === stageReceipt.signerInvocationKey &&
      signerReceipt.signerInvocationKey === finalizeReceipt.signerInvocationKey,
    'signer and object-store receipts must bind the same signer invocation key',
  );
  assert(
    signerReceipt.finalSignedPngSha256 === stageReceipt.finalSignedPngSha256 &&
      signerReceipt.finalSignedPngSha256 === finalizeReceipt.finalSignedPngSha256,
    'signer and object-store receipts must bind the same final PNG hash',
  );
  assert(
    stageReceipt.executionId === base.command.executionId &&
      finalizeReceipt.executionId === base.command.executionId &&
      stageReceipt.idempotencyKey === base.command.idempotencyKey &&
      finalizeReceipt.idempotencyKey === base.command.idempotencyKey,
    'artifact receipts must bind execution and command idempotency',
  );
  assert(
    stageReceipt.operation === 'stage' &&
      stageReceipt.durabilityStatus === 'staged' &&
      finalizeReceipt.operation === 'finalize' &&
      finalizeReceipt.durabilityStatus === 'finalized' &&
      stageReceipt.artifactRef === finalizeReceipt.artifactRef &&
      stageReceipt.objectVersion === finalizeReceipt.objectVersion,
    'stage and finalize receipts must bind one durable object version',
  );
  assert(
    signerReceipt.idempotencyDisposition === 'created' &&
      typeof signerReceipt.billableInvocationId === 'string' &&
      signerReceipt.billableInvocationId.length > 0 &&
      typeof signerReceipt.signerResultRef === 'string' &&
      signerReceipt.signerResultRef.length > 0,
    'signer receipt must freeze provider idempotency and billable invocation identity',
  );
}

function assertBaseBindings() {
  const { command, profile, authorizationReceipt, signerReceipt } = base;
  assertWatermarkUid(command.watermarkUid);
  for (const digest of [
    command.unsignedV3PngSha256,
    command.signerCredentialRefDigest,
    command.requestDigest,
    profile.entitlementDigest,
    authorizationReceipt.profileEntitlementDigest,
    authorizationReceipt.unsignedV3PngSha256,
    authorizationReceipt.signerCredentialRefDigest,
    authorizationReceipt.scopeDigest,
    signerReceipt.profileEntitlementDigest,
    signerReceipt.unsignedV3PngSha256,
    signerReceipt.finalSignedPngSha256,
    signerReceipt.c2paClaimDigest,
    signerReceipt.certificateChainDigest,
    signerReceipt.signerInvocationKey,
  ]) {
    assertDigest(digest);
  }
  assert(
    command.requestedProfileIds.includes('hiddenshield_v3_image_anchor_v1') &&
      command.requestedProfileIds.includes('c2pa_post_embed_signing_v1') &&
      command.requestedProfileIds.includes(profile.regionalProfileId),
    'command must request V3, post-embed C2PA, and regional Profiles',
  );
  assert(
    profile.status === 'active' &&
      profile.mediaType === 'image/png' &&
      profile.claimType === 'ai_generated' &&
      profile.issuerMode === 'production_platform' &&
      profile.signingOrder === 'watermark_then_c2pa' &&
      profile.allowEphemeralSigner === false,
    'Profile must freeze production watermark-then-C2PA semantics',
  );
  assert(
    command.profileEntitlementVersion === profile.profileEntitlementVersion &&
      authorizationReceipt.profileEntitlementDigest === profile.entitlementDigest &&
      signerReceipt.profileEntitlementDigest === profile.entitlementDigest,
    'command and receipts must bind the same Profile entitlement',
  );
  assert(
    command.unsignedV3PngSha256 === authorizationReceipt.unsignedV3PngSha256 &&
      command.unsignedV3PngSha256 === signerReceipt.unsignedV3PngSha256,
    'command and receipts must bind the same unsigned V3 PNG',
  );
  assert(
    command.signerCredentialRefDigest === authorizationReceipt.signerCredentialRefDigest,
    'authorization receipt must bind the signer credential reference digest',
  );
  assert(
    authorizationReceipt.receiptId === command.authorizationReceiptId &&
      authorizationReceipt.operation === 'ai_transparency_post_embed_c2pa_sign' &&
      authorizationReceipt.role === 'ai_transparency_production_signer',
    'authorization receipt must match the command operation and role',
  );
  assert(
    Date.parse(authorizationReceipt.expiresAt) > Date.parse(authorizationReceipt.issuedAt),
    'authorization receipt expiry must follow issuance',
  );
  assert(
    signerReceipt.operation === 'c2pa_post_embed_sign' &&
      signerReceipt.watermarkUid === command.watermarkUid &&
      profile.allowedSignatureAlgorithms.includes(signerReceipt.signatureAlgorithm),
    'signer receipt must match the watermark and allowed algorithm',
  );
  assert(
    signerReceipt.signerInvocationKey === stableSignerInvocationKey(command),
    'signer receipt must bind the deterministic signer invocation key',
  );
}

function assertFixtureSet() {
  const expectedTypes = [
    'success',
    'signer_rejected',
    'receipt_hash_mismatch',
    'c2pa_readback_failure',
    'v3_readback_failure',
    'confirm_rollback',
    'duplicate_replay',
    'concurrent_reservation',
    'artifact_finalize_recovery',
    'crash_after_reservation',
    'crash_after_signer',
    'crash_after_artifact_stage',
    'crash_after_confirm',
  ];
  const actualTypes = fixtureNames.map((name) => fixtures[name].fixtureType);
  assert(
    JSON.stringify(actualTypes) === JSON.stringify(expectedTypes),
    'fixture types must remain complete and ordered',
  );
  for (const [name, fixture] of Object.entries(fixtures)) {
    assert(fixture.schemaVersion === 1, `${name}: schemaVersion must be 1`);
    assert(
      fixture.baseInput === 'base-production-input-v1.json',
      `${name}: fixture must use the frozen base input`,
    );
  }
}

function assertSuccess() {
  const fixture = fixtures['success-v1.fixture.json'];
  assert(
    fixture.observed.computedFinalSignedPngSha256 === base.signerReceipt.finalSignedPngSha256,
    'success must bind computed final hash to signer receipt',
  );
  assert(
    fixture.observed.c2paReadback.activeManifestPresent &&
      fixture.observed.c2paReadback.hardBindingValid &&
      fixture.observed.c2paReadback.validationFindings.length === 0,
    'success must pass production C2PA readback without findings',
  );
  assert(
    fixture.observed.v3Readback.watermarkUid === base.command.watermarkUid &&
      fixture.observed.v3Readback.protocolVersion === 3 &&
      fixture.observed.v3Readback.payloadBytesLength === 39 &&
      fixture.observed.v3Readback.payloadAuthStatus === 'verified',
    'success must preserve verified V3/39 readback',
  );
  assert(
    fixture.observed.confirmStatus === 'committed' &&
      fixture.expected.artifactReturned &&
      fixture.expected.finalHashBoundEverywhere &&
      fixture.expected.committedConfirmedMarkedImageCount === 1 &&
      fixture.expected.customerMeteringQuantity === 1 &&
      fixture.expected.orphanSigningEventCreated === false,
    'success must confirm once, return the final artifact, and meter once',
  );
}

function assertSignerRejected() {
  const fixture = fixtures['signer-rejected-v1.fixture.json'];
  assert(
    fixture.overrides.signerReceipt === null &&
      fixture.observed.signerStatus === 'rejected' &&
      fixture.observed.confirmStatus === 'not_started',
    'signer rejection must stop before receipt and confirm',
  );
  assertFailureIsNonMetered(fixture, false);
}

function assertReceiptHashMismatch() {
  const fixture = fixtures['receipt-hash-mismatch-v1.fixture.json'];
  assert(
    fixture.observed.computedFinalSignedPngSha256 !==
      fixture.observed.receiptFinalSignedPngSha256,
    'receipt/hash mismatch fixture must contain conflicting hashes',
  );
  assert(
    fixture.observed.receiptFinalSignedPngSha256 ===
      base.signerReceipt.finalSignedPngSha256,
    'receipt/hash mismatch must compare against the frozen signer receipt',
  );
  assertFailureIsNonMetered(fixture, true);
}

function assertC2paReadbackFailure() {
  const fixture = fixtures['c2pa-readback-failure-v1.fixture.json'];
  assert(
    fixture.observed.c2paReadback.activeManifestPresent &&
      fixture.observed.c2paReadback.hardBindingValid === false &&
      fixture.observed.c2paReadback.validationFindings.includes('content_hash_mismatch'),
    'C2PA readback failure must detect invalid hard binding',
  );
  assertFailureIsNonMetered(fixture, true);
}

function assertV3ReadbackFailure() {
  const fixture = fixtures['v3-readback-failure-v1.fixture.json'];
  assert(
    fixture.observed.c2paReadback.hardBindingValid &&
      fixture.observed.v3Readback.payloadAuthStatus === 'invalid',
    'V3 readback failure must remain independent of C2PA success',
  );
  assertFailureIsNonMetered(fixture, true);
}

function assertConfirmRollback() {
  const fixture = fixtures['confirm-rollback-v1.fixture.json'];
  assert(
    fixture.observed.c2paReadback.hardBindingValid &&
      fixture.observed.v3Readback.payloadAuthStatus === 'verified' &&
      fixture.observed.confirmStatus === 'rolled_back',
    'confirm rollback must occur after successful signer and dual readback',
  );
  assert(
    fixture.expected.artifactReturned === false &&
      fixture.expected.confirmWrites === 0 &&
      fixture.expected.customerMeteringQuantity === 0 &&
      fixture.expected.orphanSigningEventCreated === true &&
      fixture.expected.retryMustUseSameRequestDigest === true,
    'confirm rollback must withhold artifact, avoid metering, and record orphan signing',
  );
}

function assertDuplicateReplay() {
  const fixture = fixtures['duplicate-replay-v1.fixture.json'];
  assert(
    fixture.existingSuccessProjection.requestDigest === base.command.requestDigest &&
      fixture.observed.replayRequestDigest === base.command.requestDigest,
    'duplicate replay must use the same request digest',
  );
  assert(
    fixture.existingSuccessProjection.finalSignedPngSha256 ===
      base.signerReceipt.finalSignedPngSha256,
    'duplicate replay must return the existing final hash',
  );
  assert(
    fixture.observed.signerInvoked === false &&
      fixture.observed.newConfirmWrites === 0 &&
      fixture.observed.newLedgerWrites === 0 &&
      fixture.expected.replayedExistingProjection === true &&
      fixture.expected.customerMeteringQuantity === 0 &&
      fixture.expected.secondSignerReceiptCreated === false,
    'duplicate replay must not re-sign, rewrite, or re-meter',
  );
}

function assertConcurrentReservation() {
  const fixture = fixtures['concurrent-reservation-v1.fixture.json'];
  assert(
    fixture.observed.postgresConnections === 2 &&
      fixture.observed.sameIdempotencyKey === true &&
      fixture.observed.sameRequestDigest === true &&
      fixture.observed.signerInvocations === 1 &&
      fixture.observed.secondResult === 'replayed_existing_projection',
    'concurrent reservation must serialize two PostgreSQL connections to one signer invocation',
  );
  assert(
    fixture.expected.maxSignerInvocations === 1 &&
      fixture.expected.signingExecutionCount === 1 &&
      fixture.expected.committedConfirmedMarkedImageCount === 1 &&
      fixture.expected.secondSignerReceiptCreated === false,
    'concurrent reservation must create one projection, receipt, and metered result',
  );
}

function assertArtifactFinalizeRecovery() {
  const fixture = fixtures['artifact-finalize-recovery-v1.fixture.json'];
  assert(
    fixture.observed.firstFinalizeStatus === 'artifact_pending' &&
      fixture.observed.firstArtifactReturned === false &&
      fixture.observed.firstCustomerMeteringQuantity === 0 &&
      fixture.observed.recoveryUsesSameRequestDigest === true &&
      fixture.observed.recoverySignerInvoked === false &&
      fixture.observed.recoveredStatus === 'confirmed',
    'artifact recovery must resume the durable staged artifact without re-signing',
  );
  assert(
    fixture.expected.totalSignerInvocations === 1 &&
      fixture.expected.recoveryAttempts === 1 &&
      fixture.expected.newConfirmWritesDuringRecovery === 0 &&
      fixture.expected.newLedgerWritesDuringRecovery === 0 &&
      fixture.expected.artifactReturnedAfterRecovery === true &&
      fixture.expected.committedConfirmedMarkedImageCount === 1,
    'artifact recovery must finalize once without duplicate confirm or metering',
  );
}

function assertCrashRecoveryFixtures() {
  const cases = [
    ['crash-after-reservation-v1.fixture.json', 'reserved'],
    ['crash-after-signer-v1.fixture.json', 'reserved'],
    ['crash-after-artifact-stage-v1.fixture.json', 'reserved'],
    ['crash-after-confirm-v1.fixture.json', 'artifact_pending'],
  ];
  for (const [name, persistedStatus] of cases) {
    const fixture = fixtures[name];
    assert(
      fixture.observed.persistedStatusAfterCrash === persistedStatus &&
        fixture.observed.finalStatus === 'confirmed',
      `${fixture.fixtureType}: crash state must be durable and recover to confirmed`,
    );
    assert(
      fixture.expected.artifactReturned === true &&
        fixture.expected.replayedExistingProjection === true &&
        fixture.expected.committedConfirmedMarkedImageCount === 1 &&
        fixture.expected.customerMeteringQuantity === 1 &&
        fixture.expected.maxBillableSignerInvocations === 1 &&
        fixture.expected.maxUniqueArtifactStageWrites === 1,
      `${fixture.fixtureType}: recovery must remain single-cost and single-metered`,
    );
    assert(
      fixture.expected.signerReceiptPersisted === true &&
        fixture.expected.artifactStageReceiptPersisted === true &&
        fixture.expected.artifactFinalizeReceiptPersisted === true,
      `${fixture.fixtureType}: recovery must persist all production adapter receipts`,
    );
  }
}

function assertFailureIsNonMetered(fixture, signedBytesQuarantined) {
  assert(
    fixture.expected.artifactReturned === false &&
      fixture.expected.confirmWrites === 0 &&
      fixture.expected.customerMeteringQuantity === 0,
    `${fixture.fixtureType}: failure must withhold artifact and remain non-metered`,
  );
  if (signedBytesQuarantined) {
    assert(
      fixture.expected.signedBytesQuarantined === true,
      `${fixture.fixtureType}: signed bytes must be quarantined`,
    );
  }
}

function assertRequiredFields(schema, value, label) {
  assert(schema.type === 'object', `${label} schema must describe an object`);
  for (const field of schema.required ?? []) {
    assert(Object.hasOwn(value, field), `${label} is missing required field ${field}`);
  }
}

function assertWatermarkUid(value) {
  assert(
    /^HS-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}$/.test(value),
    'watermark UID must preserve the frozen opaque format',
  );
}

function assertDigest(value) {
  assert(
    typeof value === 'string' && /^[a-f0-9]{64}$/.test(value),
    'all bound digests must be lowercase SHA-256 hex',
  );
}

function stableSignerInvocationKey(command) {
  return createHash('sha256')
    .update(
      `hiddenshield-post-embed-signer-invocation-v1\0${command.idempotencyKey}\0${command.requestDigest}`,
    )
    .digest('hex');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(`AI Transparency post-embed signing contract failed: ${message}`);
  }
}
