import { readFileSync, readdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const fixtureDirectory = join(
  scriptDirectory,
  "..",
  "docs",
  "contracts",
  "ai-transparency-approval",
);
const schema = readJson(
  join(fixtureDirectory, "ai-transparency-approval-fixture-v1.schema.json"),
);
const desiredStateSchema = readJson(
  join(fixtureDirectory, "ai-transparency-desired-state-v1.schema.json"),
);
const fixtureFiles = readdirSync(fixtureDirectory)
  .filter((name) => name.endsWith(".fixture.json"))
  .sort();
const fixtures = new Map(
  fixtureFiles.map((name) => [name, readJson(join(fixtureDirectory, name))]),
);

assert(fixtureFiles.length === 8, "exactly eight approval fixtures must be frozen");
for (const [name, fixture] of fixtures) {
  assertEnvelope(schema, fixture, name);
}
assertUniqueFixtureTypes(fixtures);
assertActorRoleContract(fixtures);
assertVersionedEntitlementContract(fixtures);
assertChangeRequestContract(fixtures);
assertApprovalContract(fixtures);
assertExecutionContract(fixtures);
assertAuditStateMachineContract(fixtures);
assertPreMigrationGatesContract(fixtures, desiredStateSchema);
assertConcurrencyHarnessContract(fixtures);

console.log(
  `AI Transparency approval state-machine contract passed (${fixtureFiles.length} fixtures)`,
);

function assertEnvelope(candidateSchema, fixture, name) {
  assert(
    fixture.schemaVersion ===
      candidateSchema.properties.schemaVersion.const,
    `${name} must use the frozen schema version`,
  );
  assert(
    candidateSchema.properties.fixtureType.enum.includes(fixture.fixtureType),
    `${name} must use a supported fixture type`,
  );
  assert(
    typeof fixture.fixtureId === "string" && fixture.fixtureId.length > 0,
    `${name} must have a fixture ID`,
  );
}

function assertUniqueFixtureTypes(fixtureMap) {
  const fixtureTypes = [...fixtureMap.values()].map(
    (fixture) => fixture.fixtureType,
  );
  assert(
    new Set(fixtureTypes).size === fixtureTypes.length,
    "fixture types must be unique",
  );
}

function assertActorRoleContract(fixtureMap) {
  const fixture = fixtureMap.get("actor-role-source-v1.fixture.json");
  const actorMap = new Map(
    fixture.actors.map((actor) => [actor.actorId, actor]),
  );
  const requester = actorMap.get("actor_requester_001");
  const approver = actorMap.get("actor_compliance_001");
  const executor = actorMap.get("system_executor_001");

  assert(
    fixture.identitySource === "hiddenshield_internal_iam" &&
      fixture.invariants.singleAdminTokenIsIdentitySource === false,
    "Internal IAM must be the identity source rather than the shared admin token",
  );
  assert(
    requester.actorType === "human" &&
      approver.actorType === "human" &&
      requester.actorId !== approver.actorId,
    "requester and approver must be distinct humans",
  );
  assert(
    executor.actorType === "system" &&
      executor.roleBindings.some((binding) => binding.role === "system_executor"),
    "execution must use the system executor identity",
  );
  for (const actor of [requester, approver, executor]) {
    const binding = actor.roleBindings[0];
    assert(
      binding.status === "active" &&
        binding.tenantScope.includes("tenant_platform_001") &&
        binding.workspaceScope.includes("workspace_platform_001") &&
        binding.environmentScope.includes("production"),
      `${actor.actorId} role binding must cover the frozen production scope`,
    );
  }
  assert(
    fixture.invariants.requesterAndApproverMustBeHuman === true &&
      fixture.invariants.systemExecutorCannotApprove === true,
    "four-eyes actor-type invariants must be fail-closed",
  );
}

function assertVersionedEntitlementContract(fixtureMap) {
  const fixture = fixtureMap.get(
    "versioned-profile-entitlement-v1.fixture.json",
  );
  const versions = [...fixture.versions].sort(
    (left, right) => left.version - right.version,
  );

  assert(versions[0].version === 1, "entitlement versions must start at one");
  for (let index = 1; index < versions.length; index += 1) {
    assert(
      versions[index].version === versions[index - 1].version + 1 &&
        versions[index].previousVersionId ===
          versions[index - 1].profileEntitlementVersionId,
      "entitlement versions must form a strict immutable chain",
    );
  }
  assert(
    versions.filter((version) => version.status === "active").length === 1 &&
      fixture.invariants.activeVersionCount === 1,
    "each license/Profile pair must have one active version at most",
  );
  assert(
    versions[0].status === "superseded" &&
      versions[1].status === "active" &&
      versions[1].legalReviewReference,
    "production regulatory renewal must supersede history and preserve legal review evidence",
  );
  assert(
    fixture.invariants.historyIsImmutable === true &&
      fixture.invariants.renewCreatesNewVersion === true &&
      fixture.invariants.revokedOrExpiredVersionMayBeRevived === false,
    "version history must be immutable and terminal versions must not be revived",
  );
}

function assertChangeRequestContract(fixtureMap) {
  const fixture = fixtureMap.get("change-request-v1.fixture.json");
  const request = fixture.changeRequest;

  assert(
    request.operation === "renew_profile_entitlement" &&
      request.targetType === "profile_entitlement" &&
      request.environment === "production",
    "change request must freeze the production regulatory renewal operation",
  );
  assert(
    request.expectedCurrentVersion === 1 &&
      request.desiredNextVersion === 2,
    "change request must use optimistic version sequencing",
  );
  assertDigest(request.requestDigest, "change request digest");
  assert(
    request.legalReviewReference &&
      fixture.invariants.productionRegulatoryLegalReviewRequired === true,
    "production regulatory renewal must carry legal review evidence",
  );
  assertNoProductionSideEffects(fixture.invariants, "change request");
}

function assertApprovalContract(fixtureMap) {
  const actors = fixtureMap.get("actor-role-source-v1.fixture.json");
  const request = fixtureMap.get("change-request-v1.fixture.json").changeRequest;
  const fixture = fixtureMap.get("approval-v1.fixture.json");
  const approval = fixture.approval;
  const approver = actors.actors.find(
    (actor) => actor.actorId === approval.approverActorId,
  );

  assert(
    approval.changeRequestId === request.changeRequestId &&
      approval.requestDigest === request.requestDigest,
    "approval must bind to the exact immutable request",
  );
  assert(
    approval.approverActorId !== request.requesterActorId &&
      approver.actorType === "human",
    "approval must satisfy requester/approver separation",
  );
  assert(
    approval.approverRole === "ai_transparency_compliance_approver" &&
      approver.roleBindings.some(
        (binding) =>
          binding.role === approval.approverRole &&
          binding.environmentScope.includes(request.environment),
      ),
    "regulatory Profile renewal must use an in-scope compliance approver",
  );
  assert(
    fixture.selfApprovalRejection.approverActorId ===
      request.requesterActorId &&
      fixture.selfApprovalRejection.expectedReasonCode ===
        "requester_approver_not_separated",
    "self approval must have a stable rejection reason",
  );
  assert(
    fixture.invariants.approvalIsReusableAcrossRequests === false,
    "approval must never be reusable across requests",
  );
}

function assertExecutionContract(fixtureMap) {
  const request = fixtureMap.get("change-request-v1.fixture.json").changeRequest;
  const entitlement = fixtureMap.get(
    "versioned-profile-entitlement-v1.fixture.json",
  );
  const fixture = fixtureMap.get("execution-v1.fixture.json");
  const execution = fixture.execution;

  assert(
    execution.changeRequestId === request.changeRequestId &&
      execution.executorRole === "system_executor" &&
      execution.fromRequestStatus === "approved" &&
      execution.toRequestStatus === "succeeded",
    "only the system executor may move an approved request to succeeded",
  );
  assert(
    execution.targetVersionBefore === request.expectedCurrentVersion &&
      execution.targetVersionAfter === request.desiredNextVersion &&
      entitlement.versions.some(
        (version) =>
          version.profileEntitlementVersionId ===
            execution.resultingEntitlementVersionId &&
          version.version === execution.targetVersionAfter,
      ),
    "execution result must match the requested entitlement version",
  );
  assert(
    fixture.versionConflict.actualTargetVersion !==
      request.expectedCurrentVersion &&
      fixture.versionConflict.expectedReasonCode ===
        "target_version_conflict" &&
      fixture.versionConflict.targetStateWritten === false,
    "version conflicts must fail without writing target state",
  );
  assertNoProductionSideEffects(fixture.versionConflict, "version conflict");
  assert(
    fixture.invariants.executionRevalidatesApprovalDigest === true &&
      fixture.invariants.executionRevalidatesRoleBindings === true &&
      fixture.invariants.executionIsAtomic === true,
    "execution must revalidate approval, identity, and atomicity",
  );
}

function assertAuditStateMachineContract(fixtureMap) {
  const request = fixtureMap.get("change-request-v1.fixture.json").changeRequest;
  const fixture = fixtureMap.get("audit-state-machine-v1.fixture.json");
  const events = fixture.events;
  const expectedTransitions = [
    ["draft", "pending_review", "change_request_submitted"],
    ["pending_review", "approved", "approval_granted"],
    ["approved", "executing", "execution_started"],
    ["executing", "succeeded", "execution_succeeded"],
  ];

  assert(
    fixture.changeRequestId === request.changeRequestId,
    "audit stream must belong to the frozen change request",
  );
  for (const [fromState, toState, eventType] of expectedTransitions) {
    assert(
      events.filter(
        (event) =>
          event.fromState === fromState &&
          event.toState === toState &&
          event.eventType === eventType,
      ).length === 1,
      `state transition ${fromState} -> ${toState} must have one audit event`,
    );
  }
  assert(
    events.filter((event) => event.eventType === "target_state_changed")
      .length === 1,
    "successful execution must append one target_state_changed event",
  );
  assert(
    events[0].actorType === "human" &&
      events[1].actorType === "human" &&
      events.slice(2).every((event) => event.actorType === "system"),
    "audit actors must match requester, approver, and system execution phases",
  );
  assert(
    fixture.invariants.appendOnly === true &&
      fixture.invariants.eachStateTransitionHasOneAuditEvent === true &&
      fixture.invariants.auditFailureRollsBackStateChange === true,
    "audit stream must be append-only and fail-closed",
  );
  assertNoProductionSideEffects(fixture.invariants, "audit state machine");
}

function assertPreMigrationGatesContract(fixtureMap, schema) {
  const fixture = fixtureMap.get("pre-migration-gates-v1.fixture.json");
  const digest = fixture.digestVector;
  const operations = fixture.desiredStateExamples.map((example) => example.operation);
  const schemaBranches = schema.oneOf;
  const schemaOperations = schemaBranches.map(
    (branch) => branch.properties.operation.const,
  );
  const expectedOperations = [
    "create_license",
    "renew_license",
    "suspend_license",
    "revoke_license",
    "grant_profile_entitlement",
    "renew_profile_entitlement",
    "suspend_profile_entitlement",
    "revoke_profile_entitlement",
  ];

  assert(
    digest.version === "hs-ai-change-request-digest-v1" &&
      createHash("sha256").update(digest.canonicalJson, "utf8").digest("hex") ===
        digest.sha256,
    "request digest vector must freeze canonical UTF-8 SHA-256 output",
  );
  assert(
    JSON.parse(digest.canonicalJson)[0] === digest.version,
    "digest canonical array must begin with its algorithm version",
  );
  assert(
    schemaBranches.length === 8 &&
      sameStringSet(schemaOperations, expectedOperations) &&
      sameStringSet(operations, expectedOperations),
    "desiredState schema and fixtures must cover exactly eight frozen operations",
  );
  for (const example of fixture.desiredStateExamples) {
    const branch = schemaBranches.find(
      (candidate) =>
        candidate.properties.operation.const === example.operation,
    );
    const desiredState = example.desiredState;
    const definition = branch.properties.desiredState;
    assert(
      definition.additionalProperties === false &&
        definition.required.every((key) => key in desiredState) &&
        Object.keys(desiredState).every((key) => key in definition.properties),
      `${example.operation} desiredState must be closed and match its frozen fields`,
    );
  }
  assert(
    fixture.iamVerification.interface === "verify_actor_authorization" &&
      fixture.iamVerification.sourceIdentitySystem ===
        "hiddenshield_internal_iam" &&
      fixture.iamVerification.rawTokenPersisted === false &&
      fixture.iamVerification.unavailableFailsClosed === true,
    "Internal IAM verification must be authoritative, token-safe, and fail-closed",
  );
  assert(
    fixture.referenceVerification.interface === "verify_approval_reference" &&
      fixture.referenceVerification.fullDocumentPersisted === false &&
      fixture.referenceVerification.unavailableFailsClosed === true,
    "external reference verification must keep documents external and fail-closed",
  );
  assert(
    fixture.syntheticBackfill.evidenceQuality ===
      "migrated_legacy_without_four_eyes" &&
      fixture.syntheticBackfill.productionEligibility === false &&
      fixture.syntheticBackfill.historicalHumanApprovalAsserted === false &&
      fixture.syntheticBackfill.zhCN.includes("不声明该历史记录曾完成双人审批") &&
      fixture.syntheticBackfill.enUS.includes(
        "does not assert that historical maker-checker approval occurred",
      ),
    "synthetic backfill must never claim historical four-eyes approval",
  );
  assert(
    fixture.gate.allowsCreate0003 === true &&
      fixture.gate.allowsProductionIssuance === false &&
      fixture.gate.allowsWriteEndpoints === false,
    "completed pre-migration gates may permit 0003 design work but never issuance or writes",
  );
}

function assertConcurrencyHarnessContract(fixtureMap) {
  const fixture = fixtureMap.get("concurrency-harness-v1.fixture.json");
  const postgres = fixture.adapters.find(
    (adapter) => adapter.databaseKind === "postgres",
  );
  const expectedScenarios = [
    "duplicate_idempotency_request",
    "concurrent_profile_renew",
    "duplicate_execution",
    "grant_vs_revoke_same_target",
    "audit_failure_rollback",
    "projection_version_conflict",
  ];

  assert(
    postgres.independentConnections === 2 &&
      postgres.lockingMode === "row_and_target_lock" &&
      postgres.productionGate === true &&
      fixture.adapters.length === 1,
    "PostgreSQL must be the sole two-connection production concurrency gate",
  );
  assert(
    sameStringSet(
      fixture.scenarios.map((scenario) => scenario.scenarioId),
      expectedScenarios,
    ),
    "concurrency harness must cover all frozen races and audit failure",
  );
  assert(
    [
      "createAdapter",
      "applyMigrationsThrough0003",
      "seedScenario",
      "openConnection",
      "runBarrier",
      "executeCommand",
      "snapshotState",
      "assertNoProductionSideEffects",
      "dispose",
    ].every((method) => fixture.interfaceMethods.includes(method)),
    "concurrency harness interface must freeze setup, barriers, snapshots, and cleanup",
  );
  assert(
    fixture.invariants.postgresIsSoleProductionConcurrencyGate === true &&
      fixture.invariants.sqliteConcurrencyEvidenceAccepted === false &&
      fixture.invariants.credentialCount === 0 &&
      fixture.invariants.markingSessionCount === 0 &&
      fixture.invariants.manifestCount === 0 &&
      fixture.invariants.ledgerCount === 0 &&
      fixture.invariants.migrationAndRealConcurrencyTestsRequiredBeforeProduction ===
        true,
    "concurrency contract must preserve PostgreSQL-only production semantics and zero side effects",
  );
}

function assertNoProductionSideEffects(candidate, label) {
  assert(
    candidate.createsCredential === false &&
      candidate.createsMarkingSession === false &&
      candidate.createsLedger === false,
    `${label} must not create credentials, marking sessions, or ledger entries`,
  );
}

function sameStringSet(actual, expected) {
  return (
    actual.length === expected.length &&
    actual.every((value) => expected.includes(value)) &&
    new Set(actual).size === actual.length
  );
}

function assertDigest(value, label) {
  assert(
    typeof value === "string" && /^[a-f0-9]{64}$/.test(value),
    `${label} must be a lowercase SHA-256 hex digest`,
  );
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(
      `AI Transparency approval contract failed: ${message}`,
    );
  }
}
