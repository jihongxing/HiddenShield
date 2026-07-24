import { mkdirSync, writeFileSync, existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const runId = `cloud-sync-runtime-qa-readiness-${Date.now()}`;
const artifactDir = resolve('tmp-ui-qa/cloud-sync-runtime-readiness');
mkdirSync(artifactDir, { recursive: true });

const requiredArtifacts = [
  {
    key: 'desktop_installer_creator_sync_runtime_qa',
    env: 'HIDDENSHIELD_CLOUD_SYNC_DESKTOP_INSTALLER_QA_ARTIFACT',
    description:
      'Desktop installed app QA proving Creator auto pull/flush/pull, Free blocked_by_entitlement, and no media/path/object-ref sync.',
    expectedSchemaVersion: 'cloud_sync_desktop_installer_runtime_qa_v1',
  },
  {
    key: 'android_native_creator_sync_runtime_qa',
    env: 'HIDDENSHIELD_CLOUD_SYNC_ANDROID_QA_ARTIFACT',
    description:
      'Android native QA proving the same Creator/Free sync semantics and queue diagnostics as desktop.',
    expectedSchemaVersion: 'cloud_sync_android_native_runtime_qa_v1',
  },
  {
    key: 'network_resume_and_startup_auto_flush_qa',
    env: 'HIDDENSHIELD_CLOUD_SYNC_NETWORK_RESUME_QA_ARTIFACT',
    description:
      'Runtime QA proving stale syncing recovery, startup/foreground/network-resume auto flush, and synced items not retransmitted.',
    expectedSchemaVersion: 'cloud_sync_network_resume_runtime_qa_v1',
  },
  {
    key: 'backend_event_disposition_runtime_qa',
    env: 'HIDDENSHIELD_CLOUD_SYNC_EVENT_DISPOSITION_QA_ARTIFACT',
    description:
      'Runtime QA proving accepted/duplicate/conflict_payload_changed eventResults under SQLite dev adapter and disposable Postgres.',
    expectedSchemaVersion: 'cloud_sync_event_disposition_runtime_qa_v1',
  },
];

function readArtifact(entry) {
  const file = process.env[entry.env];
  if (!file) {
    return {
      key: entry.key,
      status: 'missing',
      env: entry.env,
      description: entry.description,
      expectedSchemaVersion: entry.expectedSchemaVersion,
    };
  }
  if (!existsSync(file)) {
    return {
      key: entry.key,
      status: 'missing_file',
      env: entry.env,
      file,
      description: entry.description,
      expectedSchemaVersion: entry.expectedSchemaVersion,
    };
  }
  try {
    const parsed = JSON.parse(readFileSync(file, 'utf8'));
    const ok = parsed.schemaVersion === entry.expectedSchemaVersion && parsed.ok === true;
    return {
      key: entry.key,
      status: ok ? 'ready' : 'invalid',
      env: entry.env,
      file,
      expectedSchemaVersion: entry.expectedSchemaVersion,
      actualSchemaVersion: parsed.schemaVersion ?? null,
      ok: parsed.ok === true,
    };
  } catch (error) {
    return {
      key: entry.key,
      status: 'invalid_json',
      env: entry.env,
      file,
      expectedSchemaVersion: entry.expectedSchemaVersion,
      error: String(error),
    };
  }
}

const checks = requiredArtifacts.map(readArtifact);
const ready = checks.every((check) => check.status === 'ready');
const requireReady = process.env.HIDDENSHIELD_CLOUD_SYNC_REQUIRE_RUNTIME_QA_READY === '1';
const artifact = {
  schemaVersion: 'cloud_sync_runtime_qa_readiness_gate_v1',
  runId,
  generatedAt: new Date().toISOString(),
  ok: ready,
  status: ready ? 'ready' : 'blocked',
  productionDatabaseAllowed: false,
  formalUiMockReleaseDefaultPath: 'not_switched',
  doesNotChangeWatermarkPayload: true,
  privacyBoundary:
    'metadata_only_no_original_media_no_protected_media_no_local_path_no_object_ref_no_signed_url',
  requiredArtifacts: checks,
  nextAction: ready
    ? 'Run RC1 desktop installer and Android sync QA evidence review.'
    : 'Collect the missing desktop installer, Android, network-resume, and event-disposition runtime QA artifacts, then rerun with HIDDENSHIELD_CLOUD_SYNC_REQUIRE_RUNTIME_QA_READY=1.',
};

const artifactPath = resolve(artifactDir, `${runId}.json`);
writeFileSync(artifactPath, `${JSON.stringify(artifact, null, 2)}\n`);
console.log(`Cloud sync runtime QA readiness artifact: ${artifactPath}`);

if (requireReady && !ready) {
  throw new Error('cloud sync runtime QA readiness is blocked; missing required runtime artifacts');
}
