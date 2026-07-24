import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  design: readFileSync('docs/本地版权库与云版权库同步可靠性设计.md', 'utf8'),
  commercialRoadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
  releasePlan: readFileSync('docs/封版收口计划.md', 'utf8'),
  desktopSyncCommands: readFileSync('src-tauri/src/commands/sync.rs', 'utf8'),
  desktopSyncStorage: readFileSync('src-tauri/src/sync/storage.rs', 'utf8'),
  desktopCloud: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  desktopPipelineScheduler: readFileSync('src-tauri/src/pipeline/scheduler.rs', 'utf8'),
  tauriApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  vaultView: readFileSync('src/views/VaultView.vue', 'utf8'),
  settingsPanel: readFileSync('src/components/SettingsPanel.vue', 'utf8'),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  backendPostgresAuth: readFileSync('feedback-backend/src/postgres_auth.rs', 'utf8'),
  backendPostgresSync: readFileSync('feedback-backend/src/postgres_sync.rs', 'utf8'),
  postgresMigrationUp: readFileSync('feedback-backend/migrations/postgres/0001_auth_sync_registry.up.sql', 'utf8'),
  postgresSyncRuntimeQa: readFileSync('feedback-backend/src/bin/cloud_sync_postgres_runtime_qa.rs', 'utf8'),
  syncRuntimeQaGate: readFileSync('scripts/verify-cloud-sync-runtime-qa-readiness-gate.mjs', 'utf8'),
};

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertIncludes(source, needles, message) {
  for (const needle of needles) {
    assert(source.includes(needle), `${message}: missing ${needle}`);
  }
}

const forbiddenSyncPayloadKeys = [
  'originalPath',
  'original_path',
  'protectedCopyPath',
  'protected_copy_path',
  'localPath',
  'local_path',
  'objectRef',
  'object_ref',
  'signedUrl',
  'signed_url',
  'mediaBytes',
  'media_bytes',
];
const desktopSyncPayloadAllowlist =
  sources.desktopSyncStorage.match(/const VAULT_RECORD_SYNC_PAYLOAD_KEYS:[\s\S]*?\];/)?.[0] ?? '';

assertIncludes(
  sources.packageJson,
  [
    '"cloud:sync-reliability-contract"',
    'verify-cloud-sync-reliability-contract.mjs',
    '"cloud:sync-runtime-qa"',
    'verify-cloud-sync-runtime-qa-readiness-gate.mjs',
  ],
  'package.json must expose cloud:sync-reliability-contract',
);

assertIncludes(
  sources.design,
  [
    '后端权益快照覆盖本地缓存',
    'cloud:sync-reliability-contract',
    'PostgreSQL 只纳入合同令牌和后续 optional smoke',
  ],
  'sync reliability design must record the SQLite/Postgres audit boundary',
);

assertIncludes(
  sources.desktopSyncCommands,
  [
    'refresh_cloud_profile_snapshot_with_reauth',
    'client.get_me(&profile.access_token)',
    'client.refresh_auth_session(&profile.refresh_token, &profile.device_id)',
    'mark_uploadable_cloud_sync_queue_blocked_by_entitlement',
    'mark_cloud_sync_queue_blocked_by_entitlement',
    'mark_cloud_sync_queue_auth_required',
    'recover_stale_cloud_syncing_queue',
  ],
  'desktop flush must refresh backend snapshot, recover stale syncing, and classify 401/403',
);

assertIncludes(
  sources.desktopPipelineScheduler,
  [
    'enqueue_cloud_sync_event',
    'trigger_desktop_cloud_sync_after_local_enqueue',
  ],
  'desktop pipeline must trigger best-effort cloud auto sync after local record enqueue',
);

const flushIndex = sources.desktopSyncCommands.indexOf('pub async fn flush_desktop_cloud_sync_queue');
const refreshIndex = sources.desktopSyncCommands.indexOf('refresh_cloud_profile_snapshot_with_reauth', flushIndex);
const entitlementIndex = sources.desktopSyncCommands.indexOf('has_cloud_sync_entitlement(&profile)', flushIndex);
assert(
  flushIndex >= 0 && refreshIndex > flushIndex && entitlementIndex > refreshIndex,
  'manual flush must refresh remote entitlement snapshot before entitlement gating',
);

const preferenceIndex = sources.desktopSyncCommands.indexOf('pub async fn set_desktop_cloud_auto_sync_enabled');
const preferenceRefreshIndex = sources.desktopSyncCommands.indexOf(
  'refresh_cloud_profile_snapshot_with_reauth',
  preferenceIndex,
);
const preferenceUpdateIndex = sources.desktopSyncCommands.indexOf('update_sync_preferences', preferenceIndex);
assert(
  preferenceIndex >= 0 &&
    preferenceRefreshIndex > preferenceIndex &&
    preferenceUpdateIndex > preferenceRefreshIndex,
  'desktop auto-sync preference updates must refresh/re-auth the cloud profile before PATCH /v1/me/sync-preferences',
);

assertIncludes(
  sources.desktopSyncStorage,
  [
    'last_error_code TEXT',
    'last_http_status INTEGER',
    'blocked_reason TEXT',
    'lease_until TEXT',
    "status = 'blocked'",
    "'blocked_by_entitlement'",
    "'auth_required'",
    'cloud_queue_blocked_by_entitlement_is_not_retried_by_backoff_reset',
    'cloud_queue_recovers_stale_syncing_without_reenqueuing_synced',
  ],
  'desktop queue storage must expose structured diagnostics and stale-syncing recovery',
);

assertIncludes(
  sources.desktopCloud,
  [
    'pub struct CloudSyncEventDisposition',
    'pub event_results: Option<Vec<CloudSyncEventDisposition>>',
    'pub blocked: u64',
    'pub syncing: u64',
    'pub stale_recovered: u64',
    'pub last_error_code: Option<String>',
    'pub last_http_status: Option<u16>',
    'pub blocked_reason: Option<String>',
  ],
  'cloud queue status must expose structured diagnostic fields',
);

assertIncludes(
  sources.desktopSyncCommands,
  [
    'cloud_queue_batch_outcome',
    '"accepted" | "duplicate"',
    '"conflict_payload_changed" | "rejected_invalid_event"',
    'mark_cloud_sync_queue_failed_structured',
    'desktop_flush_event_results_keep_conflicts_failed',
  ],
  'desktop flush must consume per-event batch dispositions and keep conflict/rejected events failed',
);

assertIncludes(
  sources.tauriApi,
  ['blocked: number', 'syncing: number', 'lastErrorCode: string | null', 'blockedReason: string | null'],
  'frontend Tauri API types must expose structured queue diagnostics',
);

assertIncludes(
  sources.vaultView,
  ['云同步已被后端权益快照阻断', '最近错误码', '最近 HTTP 状态', '阻断原因'],
  'Vault UI must expose backend entitlement blocking diagnostics',
);

assertIncludes(
  sources.settingsPanel,
  [
    '后端权益快照已阻断',
    '阻断 {{ cloudQueueStatus?.blocked ?? 0 }}',
    'isCloudAuthExpiredError',
    'await signOutDesktopCloud();',
    '登录状态已失效，请重新登录后再调整自动云同步。',
  ],
  'Settings UI must expose blocked cloud sync count and guide expired sync preference sessions back to login',
);

for (const forbiddenKey of forbiddenSyncPayloadKeys) {
  assert(
    !desktopSyncPayloadAllowlist.includes(`"${forbiddenKey}"`),
    `desktop sync payload allowlist must not include recoverable media/local pointer key ${forbiddenKey}`,
  );
}

assertIncludes(
  sources.backendStorage,
  [
    'ensure_cloud_sync_entitled_with_conn',
    'device_cursor_with_conn',
    'device_cursor_with_conn(conn, &account.id, &device.id)',
    'device_cursor_with_conn(&conn, &session.account_id, &session.device_id)',
    'let client_since_sequence = sequence_from_cursor(cursor)',
    'client_since_sequence.min(stored_since_sequence)',
    'new_device_session_uses_device_cursor_before_first_pull',
    'blocked_by_entitlement',
    'payload_hash',
    'entity_revision',
    'CloudSyncEventDisposition',
    'conflict_payload_changed',
  ],
  'SQLite backend adapter must keep device cursor snapshots, Free cloud sync blocking, and S3 per-event dispositions',
);
assertIncludes(
  sources.backendPostgresAuth,
  ['device_cursor_pg', 'device_cursor_pg(pool, &account.id, &device.id)', 'cloud_device_cursors'],
  'Postgres auth adapter must expose device-level cloud vault cursor snapshots',
);
assertIncludes(
  sources.backendPostgresSync,
  [
    'ensure_cloud_sync_entitled_pg',
    'device_cursor_pg',
    'client_since_sequence.min(stored_since_sequence)',
    'StorageError::Forbidden',
    'payload_hash',
    'entity_revision',
    'CloudSyncEventDisposition',
    'conflict_payload_changed',
  ],
  'Postgres sync adapter must keep backend entitlement and S3 disposition semantics aligned',
);
assertIncludes(
  sources.backendSchema,
  ['pub struct CloudSyncEventDisposition', 'pub event_results: Vec<CloudSyncEventDisposition>'],
  'backend schema must expose per-event sync disposition results',
);
assertIncludes(
  sources.postgresMigrationUp,
  ['payload_hash TEXT', 'entity_revision BIGINT'],
  'Postgres migration must persist payload hash and entity revision',
);
assertIncludes(
  sources.postgresSyncRuntimeQa,
  ['duplicate_push.event_results', 'changed_duplicate_push', 'conflict_payload_changed'],
  'Postgres runtime QA must cover duplicate and changed-payload conflict semantics',
);
assertIncludes(
  sources.syncRuntimeQaGate,
  [
    'cloud_sync_runtime_qa_readiness_gate_v1',
    'desktop_installer_creator_sync_runtime_qa',
    'android_native_creator_sync_runtime_qa',
    'network_resume_and_startup_auto_flush_qa',
    'backend_event_disposition_runtime_qa',
    'blocked',
    'metadata_only_no_original_media_no_protected_media_no_local_path_no_object_ref_no_signed_url',
  ],
  'S4 runtime QA gate must machine-block missing desktop/mobile/external runtime evidence',
);

assertIncludes(
  sources.commercialRoadmap,
  ['cloud:sync-reliability-contract', '后端权益快照覆盖本地缓存'],
  'commercial roadmap must record S0 reliability gate',
);
assertIncludes(
  sources.dualRoadmap,
  ['cloud:sync-reliability-contract', '后端权益快照'],
  'dual-end roadmap must record sync reliability gate',
);
assertIncludes(
  sources.releasePlan,
  ['cloud:sync-reliability-contract', '同步可靠性'],
  'release plan must index the sync reliability evidence',
);

console.log('cloud sync reliability contract passed');
