import { spawn, spawnSync } from 'node:child_process';
import { randomInt } from 'node:crypto';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

const rootDir = resolve('.');
const runId = `cloud-postgres-p5-podman-${Date.now()}`;
const artifactDir = resolve('tmp-ui-qa/postgres-p5-podman');
const databaseName = 'hiddenshield_http_gate_p5';
const password = process.env.HIDDENSHIELD_POSTGRES_TEST_PASSWORD || 'hiddenshield';
const runtime = detectPodman();
const image =
  process.env.HIDDENSHIELD_POSTGRES_TEST_IMAGE ||
  detectLocalPostgresImage() ||
  'localhost/postgres:16';
const sourceContainer = `${runId}-source`;
const restoreContainer = `${runId}-restore`;
const dataVolume = `${runId}-data`;
const backupVolume = `${runId}-backup`;
const restoreVolume = `${runId}-restore-data`;
const sourcePort = await findAvailablePort();
const restorePort = await findAvailablePort();
const databaseUrl = `postgres://postgres:${password}@127.0.0.1:${sourcePort}/${databaseName}`;
const backendUrl = `http://127.0.0.1:${await findAvailablePort()}`;
const backendExecutable = resolve(
  'feedback-backend',
  'target',
  'debug',
  process.platform === 'win32'
    ? 'hiddenshield-feedback-backend.exe'
    : 'hiddenshield-feedback-backend',
);

let backend;
const generated = {};
const cleanup = [];

try {
  mkdirSync(artifactDir, { recursive: true });
  await prepareVolumes();
  await startSourceDatabase();
  await waitForPostgres(sourceContainer);
  await resetSchema(databaseUrl);
  await enableObservability();
  await buildAndStartBackend();

  const load = await runCreatorSyncLoad();
  generated.load = writeArtifact('load', {
    schemaVersion: 'cloud_postgres_load_gate_artifact_v1',
    ok: load.thresholdsPassed,
    status: load.thresholdsPassed ? 'passed' : 'failed',
    environmentClass: 'local_podman_staging_equivalent',
    scope: 'cloud_copyright_core_auth_sync_registry',
    runtime: { kind: 'podman', version: runtime.version, image },
    workload: load.workload,
    latencyMs: load.latencyMs,
    checks: load.checks,
    limitations: [
      'Local single-node Podman is not an external staging network.',
      'Enterprise, payment, team workspace and cloud-video pressure are outside this core cloud-copyright rehearsal.',
    ],
  });

  const observability = await captureObservability(load);
  generated.observability = writeArtifact('observability', {
    schemaVersion: 'cloud_postgres_observability_artifact_v1',
    ok: observability.ok,
    status: observability.ok ? 'passed' : 'failed',
    environmentClass: 'local_podman_staging_equivalent',
    scope: 'cloud_copyright_core_auth_sync_registry',
    runtime: { kind: 'podman', version: runtime.version, image },
    metrics: observability.metrics,
    checks: observability.checks,
    limitations: [
      'Metrics are a local rehearsal snapshot, not a production dashboard or alert delivery proof.',
    ],
  });

  const restore = await runPitrRestoreDrill();
  generated.restore = writeArtifact('restore', {
    schemaVersion: 'cloud_postgres_restore_drill_artifact_v1',
    ok: restore.ok,
    status: restore.ok ? 'passed' : 'failed',
    environmentClass: 'local_podman_staging_equivalent',
    scope: 'cloud_copyright_core_auth_sync_registry',
    runtime: { kind: 'podman', version: runtime.version, image },
    method: 'pg_basebackup_plus_archive_wal_recovery_target_time',
    recoveryTargetTime: restore.recoveryTargetTime,
    recoveryTimeSeconds: restore.recoveryTimeSeconds,
    checks: restore.checks,
    limitations: [
      'Local named-volume recovery does not prove cloud-provider PITR retention, cross-zone restore or production RTO.',
    ],
  });

  generated.runbook = writeArtifact('cutover-runbook', {
    schemaVersion: 'cloud_postgres_cutover_runbook_artifact_v1',
    ok: false,
    status: 'blocked',
    environmentClass: 'local_podman_staging_equivalent',
    scope: 'cloud_copyright_core_auth_sync_registry',
    reviewStatus: 'pending_release_owner',
    releaseOwnerReviewed: false,
    steps: [
      'Freeze cloud sync writes and record queue depth.',
      'Verify latest formal HTTP, load, restore and observability artifacts.',
      'Take final production backup and record recovery point.',
      'Switch HIDDENSHIELD_DATABASE_BACKEND=postgres and inject production DATABASE_URL.',
      'Run health, auth, sync and registry canaries.',
      'Rollback within the approved window if canaries or queue drain fail.',
    ],
    rollbackTriggers: [
      'auth/session error rate exceeds approved threshold',
      'sync p95 exceeds 30 seconds',
      'registry reserve or confirm fails',
      'deadlock or pool saturation alert fires',
    ],
    reason: 'release_owner_review_required',
  });

  generated.signoff = writeArtifact('release-owner-signoff', {
    schemaVersion: 'cloud_postgres_release_owner_signoff_v1',
    ok: false,
    status: 'blocked',
    environmentClass: 'manual_release_approval_required',
    decision: 'pending',
    humanAttestation: false,
    signedBy: null,
    signedAt: null,
    reviewedArtifacts: Object.values(generated).map(normalizePath),
    reason: 'release_owner_must_review_and_sign',
  });

  const formalHttpArtifact = latestFormalHttpArtifact();
  const envScript = writeEnvironmentScript(formalHttpArtifact);
  console.log(`P5 Podman rehearsal environment script: ${envScript}`);
  console.log(
    `Release owner approval command: powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/approve-cloud-postgres-p5-release-owner.ps1 -Approve -SignedBy "<release-owner>" -FormalHttpArtifact "${formalHttpArtifact}" -LoadArtifact "${generated.load}" -RestoreArtifact "${generated.restore}" -ObservabilityArtifact "${generated.observability}" -DraftRunbookArtifact "${generated.runbook}" -OutputDirectory "${artifactDir}"`,
  );
  console.log('Technical rehearsal passed; production readiness remains blocked pending release-owner review/signoff.');
} finally {
  if (backend && backend.exitCode == null) {
    backend.kill();
    await waitForExit(backend);
  }
  for (const container of [restoreContainer, sourceContainer]) {
    cleanup.push(await removeContainer(container));
  }
  for (const volume of [restoreVolume, backupVolume, dataVolume]) {
    cleanup.push(await removeVolume(volume));
  }
  writeFileSync(
    resolve(artifactDir, `${runId}-cleanup.json`),
    `${JSON.stringify({ runId, generatedAt: new Date().toISOString(), cleanup }, null, 2)}\n`,
    'utf8',
  );
}

async function prepareVolumes() {
  for (const volume of [dataVolume, backupVolume, restoreVolume]) {
    await run('podman', ['volume', 'create', volume]);
  }
  await run('podman', [
    'run',
    '--rm',
    '-v',
    `${backupVolume}:/backup`,
    image,
    'sh',
    '-lc',
    'mkdir -p /backup/base /backup/wal && chown -R postgres:postgres /backup',
  ]);
}

async function startSourceDatabase() {
  await run('podman', [
    'run',
    '--detach',
    '--name',
    sourceContainer,
    '-e',
    `POSTGRES_PASSWORD=${password}`,
    '-e',
    `POSTGRES_DB=${databaseName}`,
    '-p',
    `${sourcePort}:5432`,
    '-v',
    `${dataVolume}:/var/lib/postgresql/data`,
    '-v',
    `${backupVolume}:/backup`,
    image,
    'postgres',
    '-c',
    'wal_level=replica',
    '-c',
    'archive_mode=on',
    '-c',
    'archive_timeout=5s',
    '-c',
    'archive_command=test ! -f /backup/wal/%f && cp %p /backup/wal/%f',
    '-c',
    'track_io_timing=on',
    '-c',
    'shared_preload_libraries=pg_stat_statements',
    '-c',
    'pg_stat_statements.track=all',
    '-c',
    'log_min_duration_statement=100',
    '-c',
    'deadlock_timeout=200ms',
  ]);
}

async function buildAndStartBackend() {
  await run('cargo', [
    'build',
    '--manifest-path',
    'feedback-backend/Cargo.toml',
    '--features',
    'postgres',
    '--bin',
    'hiddenshield-feedback-backend',
  ]);
  if (!existsSync(backendExecutable)) {
    throw new Error(`formal backend executable missing: ${backendExecutable}`);
  }
  backend = spawn(
    backendExecutable,
    [
      '--bind-addr',
      backendUrl.replace('http://', ''),
      '--database-backend',
      'postgres',
      '--database-url',
      databaseUrl,
      '--deployment-env',
      'staging',
    ],
    {
      cwd: rootDir,
      env: {
        ...process.env,
        HIDDENSHIELD_POSTGRES_HTTP_QA_ENTITLEMENT_GRANT: '1',
        HIDDENSHIELD_POSTGRES_HTTP_QA_INTERNAL_TOKEN: 'local-p5-rehearsal-internal-token',
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );
  backend.stdout.on('data', (chunk) => writePrefixed('p5-backend', chunk));
  backend.stderr.on('data', (chunk) => writePrefixed('p5-backend', chunk));
  const deadline = Date.now() + 120_000;
  while (Date.now() < deadline) {
    if (backend.exitCode != null) {
      throw new Error(`formal backend exited with code ${backend.exitCode}`);
    }
    try {
      const response = await fetch(`${backendUrl}/v1/health`);
      if (response.ok) {
        return;
      }
    } catch (_) {
      // Wait for repository pools.
    }
    await delay(500);
  }
  throw new Error('formal backend health timeout');
}

async function runCreatorSyncLoad() {
  const accountCount = Number(process.env.HIDDENSHIELD_P5_LOAD_ACCOUNTS || 8);
  const rounds = Number(process.env.HIDDENSHIELD_P5_LOAD_ROUNDS || 20);
  const sessions = [];
  for (let index = 0; index < accountCount; index += 1) {
    const identifier = `p5-load-${runId}-${index}@example.test`;
    const desktop = await createSession(identifier, `desktop-${index}`, 'desktop');
    const mobile = await createSession(identifier, `mobile-${index}`, 'android');
    await request('POST', '/internal/qa/entitlements/cloud-sync', {
      accountId: desktop.account.id,
      workspaceId: desktop.workspace.id,
    });
    sessions.push({ desktop, mobile });
  }

  const pushLatencies = [];
  const pullLatencies = [];
  let failures = 0;
  let peakConnections = 0;
  let sampling = true;
  const sampler = (async () => {
    while (sampling) {
      const active = Number(
        await psqlScalar(
          `SELECT COUNT(*) FROM pg_stat_activity WHERE datname='${databaseName}' AND state <> 'idle'`,
        ),
      );
      peakConnections = Math.max(peakConnections, active);
      await delay(50);
    }
  })();

  for (let round = 0; round < rounds; round += 1) {
    await Promise.all(
      sessions.map(async ({ desktop, mobile }, index) => {
        const eventId = `p5-${round}-${index}-${Date.now()}`;
        const pushStart = performance.now();
        const push = await request(
          'POST',
          '/v1/sync/events:batch',
          {
            deviceId: desktop.device.id,
            workspaceId: desktop.workspace.id,
            events: [
              {
                clientEventId: eventId,
                operation: 'upsertVaultRecord',
                entityType: 'vaultRecord',
                entityId: `record-${eventId}`,
                payload: {
                  id: `record-${eventId}`,
                  kind: 'image',
                  title: `p5-${round}-${index}.png`,
                  watermark_uid: `load-${eventId}`,
                  revision: 1,
                  created_at: new Date().toISOString(),
                },
              },
            ],
          },
          desktop.accessToken,
        );
        pushLatencies.push(performance.now() - pushStart);
        if (push.status !== 200 || !push.body.acceptedEventIds?.includes(eventId)) {
          failures += 1;
          return;
        }
        const pullStart = performance.now();
        const pull = await request(
          'GET',
          `/v1/sync/changes?workspaceId=${encodeURIComponent(mobile.workspace.id)}`,
          undefined,
          mobile.accessToken,
        );
        pullLatencies.push(performance.now() - pullStart);
        if (
          pull.status !== 200 ||
          !pull.body.changes?.some((change) => change.entity?.id === `record-${eventId}`)
        ) {
          failures += 1;
        }
      }),
    );
  }
  sampling = false;
  await sampler;

  const push = percentiles(pushLatencies);
  const pull = percentiles(pullLatencies);
  const thresholdsPassed =
    failures === 0 && push.p50 <= 5000 && push.p95 <= 30000 && pull.p50 <= 5000 && pull.p95 <= 30000;
  return {
    thresholdsPassed,
    workload: {
      accounts: accountCount,
      devices: accountCount * 2,
      rounds,
      pushOperations: pushLatencies.length,
      pullOperations: pullLatencies.length,
      concurrency: accountCount,
    },
    latencyMs: { push, pull },
    checks: {
      failures,
      successRate: (pushLatencies.length + pullLatencies.length - failures) /
        (pushLatencies.length + pullLatencies.length),
      peakDatabaseConnections: peakConnections,
      thresholds: {
        pushP50MaxMs: 5000,
        pushP95MaxMs: 30000,
        pullP50MaxMs: 5000,
        pullP95MaxMs: 30000,
      },
    },
  };
}

async function captureObservability(load) {
  await psql("CREATE TABLE IF NOT EXISTS p5_lock_probe (id INTEGER PRIMARY KEY, value INTEGER NOT NULL)");
  await psql("INSERT INTO p5_lock_probe(id, value) VALUES (1, 0) ON CONFLICT(id) DO NOTHING");
  const blocker = spawn(
    command('podman'),
    [
      'exec',
      sourceContainer,
      'psql',
      '-U',
      'postgres',
      '-d',
      databaseName,
      '-v',
      'ON_ERROR_STOP=1',
      '-c',
      'BEGIN; UPDATE p5_lock_probe SET value=value+1 WHERE id=1; SELECT pg_sleep(2); COMMIT;',
    ],
    { stdio: 'ignore' },
  );
  await delay(250);
  const waiter = spawn(
    command('podman'),
    [
      'exec',
      sourceContainer,
      'psql',
      '-U',
      'postgres',
      '-d',
      databaseName,
      '-v',
      'ON_ERROR_STOP=1',
      '-c',
      'UPDATE p5_lock_probe SET value=value+1 WHERE id=1;',
    ],
    { stdio: 'ignore' },
  );
  await delay(300);
  const lockWaitCount = Number(
    await psqlScalar(
      "SELECT COUNT(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock'",
    ),
  );
  await waitForExit(blocker);
  await waitForExit(waiter);
  await psql('SELECT pg_sleep(0.15)');

  const databaseStats = JSON.parse(
    await psqlScalar(
      `SELECT row_to_json(t)::text FROM (
         SELECT xact_commit, xact_rollback, blks_read, blks_hit, temp_files, temp_bytes,
                deadlocks, blk_read_time, blk_write_time
         FROM pg_stat_database WHERE datname=current_database()
       ) t`,
    ),
  );
  const statementStats = JSON.parse(
    await psqlScalar(
      `SELECT COALESCE(json_agg(row_to_json(t)), '[]'::json)::text FROM (
         SELECT calls, total_exec_time, mean_exec_time, rows,
                left(regexp_replace(query, '\\s+', ' ', 'g'), 180) AS query
         FROM pg_stat_statements
         WHERE dbid = (SELECT oid FROM pg_database WHERE datname=current_database())
         ORDER BY total_exec_time DESC
         LIMIT 10
       ) t`,
    ),
  );
  const indexUsage = JSON.parse(
    await psqlScalar(
      `SELECT COALESCE(json_agg(row_to_json(t)), '[]'::json)::text FROM (
         SELECT relname, seq_scan, idx_scan, n_live_tup
         FROM pg_stat_user_tables
         ORDER BY relname
       ) t`,
    ),
  );
  const checks = {
    pgStatStatementsAvailable: statementStats.length > 0,
    lockWaitObserved: lockWaitCount > 0,
    deadlocksZero: Number(databaseStats.deadlocks) === 0,
    peakConnectionsBelowPoolLimit: load.checks.peakDatabaseConnections <= 10,
  };
  return {
    ok: Object.values(checks).every(Boolean),
    metrics: {
      databaseStats,
      topStatements: statementStats,
      tableAccess: indexUsage,
      lockWaitCount,
      peakConnections: load.checks.peakDatabaseConnections,
      configuredPoolLimit: 10,
    },
    checks,
  };
}

async function runPitrRestoreDrill() {
  await run('podman', [
    'exec',
    '--user',
    'postgres',
    sourceContainer,
    'sh',
    '-lc',
    `rm -rf /backup/base/* && pg_basebackup -U postgres -D /backup/base -Fp -Xs -P`,
  ]);
  await psql(
    'CREATE TABLE IF NOT EXISTS p5_recovery_markers (marker TEXT PRIMARY KEY, created_at TIMESTAMPTZ NOT NULL DEFAULT now())',
  );
  await psql("INSERT INTO p5_recovery_markers(marker) VALUES ('before_target') ON CONFLICT DO NOTHING");
  const recoveryTargetTime = await psqlScalar(
    "SELECT to_char(clock_timestamp(), 'YYYY-MM-DD HH24:MI:SS.MS TZH:TZM')",
  );
  await delay(1500);
  await psql("INSERT INTO p5_recovery_markers(marker) VALUES ('after_target') ON CONFLICT DO NOTHING");
  await psql('SELECT pg_switch_wal()');
  await waitForArchivedWal();
  const archivedWalSegments = Number(await archivedWalCount());
  await run('podman', ['stop', '--time', '30', sourceContainer]);

  const escapedTarget = recoveryTargetTime.replaceAll("'", "''");
  const recoveryConfig = Buffer.from(
    `\nrestore_command = 'cp /backup/wal/%f %p'\nrecovery_target_time = '${escapedTarget}'\nrecovery_target_action = 'promote'\n`,
    'utf8',
  ).toString('base64');
  await run('podman', [
    'run',
    '--rm',
    '-v',
    `${backupVolume}:/backup`,
    '-v',
    `${restoreVolume}:/restore`,
    image,
    'sh',
    '-lc',
    `rm -rf /restore/* &&
     cp -a /backup/base/. /restore/ &&
     touch /restore/recovery.signal &&
     printf '%s' '${recoveryConfig}' | base64 -d >> /restore/postgresql.auto.conf &&
     chown -R postgres:postgres /restore`,
  ]);

  const startedAt = performance.now();
  await run('podman', [
    'run',
    '--detach',
    '--name',
    restoreContainer,
    '-p',
    `${restorePort}:5432`,
    '-v',
    `${restoreVolume}:/var/lib/postgresql/data`,
    '-v',
    `${backupVolume}:/backup`,
    image,
    'postgres',
  ]);
  await waitForPostgres(restoreContainer);
  const recoveryTimeSeconds = (performance.now() - startedAt) / 1000;
  const beforeCount = Number(
    await psqlScalarInContainer(
      restoreContainer,
      "SELECT COUNT(*) FROM p5_recovery_markers WHERE marker='before_target'",
    ),
  );
  const afterCount = Number(
    await psqlScalarInContainer(
      restoreContainer,
      "SELECT COUNT(*) FROM p5_recovery_markers WHERE marker='after_target'",
    ),
  );
  const recoveryEnded =
    (await psqlScalarInContainer(restoreContainer, 'SELECT pg_is_in_recovery()')) === 'f';
  const checks = {
    baseBackupCreated: true,
    archivedWalAvailable: archivedWalSegments > 0,
    beforeTargetPresent: beforeCount === 1,
    afterTargetExcluded: afterCount === 0,
    promotedAfterRecovery: recoveryEnded,
    recoveryWithinLocalRtoSeconds: recoveryTimeSeconds <= 120,
  };
  return {
    ok: Object.values(checks).every(Boolean),
    recoveryTargetTime,
    recoveryTimeSeconds,
    checks,
  };
}

async function enableObservability() {
  await psql('CREATE EXTENSION IF NOT EXISTS pg_stat_statements');
  await psql('SELECT pg_stat_statements_reset()');
}

async function createSession(identifier, deviceId, platform) {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier,
    password: `P5-${runId}-Password`,
    verificationCode: '000000',
    device: {
      clientDeviceId: deviceId,
      name: `P5 ${platform}`,
      platform,
      appVersion: 'p5-podman-rehearsal',
    },
    localCreatorProfile: {
      displayName: 'P5 Podman Rehearsal',
      creatorSeedRef: `p5-seed-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  if (response.status !== 200) {
    throw new Error(`session creation failed: ${response.status} ${JSON.stringify(response.body)}`);
  }
  return response.body;
}

async function request(method, path, body, token) {
  const headers = {};
  if (body !== undefined) {
    headers['content-type'] = 'application/json';
  }
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }
  if (path === '/internal/qa/entitlements/cloud-sync') {
    headers['x-hiddenshield-internal-token'] =
      process.env.HIDDENSHIELD_POSTGRES_HTTP_QA_INTERNAL_TOKEN ?? '';
  }
  const response = await fetch(`${backendUrl}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  return {
    status: response.status,
    body: text ? JSON.parse(text) : {},
  };
}

async function resetSchema(url) {
  await run(
    'cargo',
    [
      'run',
      '--quiet',
      '--manifest-path',
      'feedback-backend/Cargo.toml',
      '--features',
      'postgres',
      '--bin',
      'postgres_http_schema',
      '--',
      'reset',
    ],
    {
      HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: url,
    },
  );
}

async function waitForPostgres(containerName) {
  const deadline = Date.now() + 90_000;
  let lastError = '';
  while (Date.now() < deadline) {
    const result = spawnSync(
      command('podman'),
      [
        'exec',
        containerName,
        'pg_isready',
        '-U',
        'postgres',
        '-d',
        databaseName,
      ],
      { encoding: 'utf8', shell: false },
    );
    if (result.status === 0) {
      return;
    }
    lastError = `${result.stdout ?? ''}${result.stderr ?? ''}`;
    await delay(750);
  }
  throw new Error(`PostgreSQL readiness timeout for ${containerName}: ${lastError}`);
}

async function waitForArchivedWal() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (Number(await archivedWalCount()) >= 1) {
      return;
    }
    await delay(500);
  }
  throw new Error('WAL archive did not receive a segment');
}

async function archivedWalCount() {
  const result = await run('podman', [
    'exec',
    sourceContainer,
    'sh',
    '-lc',
    "find /backup/wal -type f | wc -l",
  ]);
  return result.stdout.trim();
}

function psql(sql) {
  return run('podman', [
    'exec',
    sourceContainer,
    'psql',
    '-U',
    'postgres',
    '-d',
    databaseName,
    '-v',
    'ON_ERROR_STOP=1',
    '-c',
    sql,
  ]);
}

async function psqlScalar(sql) {
  const result = await run('podman', [
    'exec',
    sourceContainer,
    'psql',
    '-U',
    'postgres',
    '-d',
    databaseName,
    '-At',
    '-v',
    'ON_ERROR_STOP=1',
    '-c',
    sql,
  ]);
  return result.stdout.trim();
}

async function psqlScalarInContainer(containerName, sql) {
  const result = await run('podman', [
    'exec',
    containerName,
    'psql',
    '-U',
    'postgres',
    '-d',
    databaseName,
    '-At',
    '-v',
    'ON_ERROR_STOP=1',
    '-c',
    sql,
  ]);
  return result.stdout.trim();
}

function writeArtifact(kind, payload) {
  const path = resolve(artifactDir, `${runId}-${kind}.json`);
  writeFileSync(
    path,
    `${JSON.stringify(
      { runId, generatedAt: new Date().toISOString(), productionDatabaseAllowed: false, ...payload },
      null,
      2,
    )}\n`,
    'utf8',
  );
  console.log(`${payload.schemaVersion} artifact: ${path}`);
  return path;
}

function writeEnvironmentScript(formalHttpArtifact) {
  const path = resolve(artifactDir, `${runId}-invoke-readiness.ps1`);
  const content = [
    `$env:HIDDENSHIELD_POSTGRES_FORMAL_HTTP_GATE_ARTIFACT='${escapePowerShell(formalHttpArtifact)}'`,
    `$env:HIDDENSHIELD_POSTGRES_STAGING_LOAD_ARTIFACT='${escapePowerShell(generated.load)}'`,
    `$env:HIDDENSHIELD_POSTGRES_BACKUP_RESTORE_ARTIFACT='${escapePowerShell(generated.restore)}'`,
    `$env:HIDDENSHIELD_POSTGRES_OBSERVABILITY_ARTIFACT='${escapePowerShell(generated.observability)}'`,
    `$env:HIDDENSHIELD_POSTGRES_CUTOVER_RUNBOOK_ARTIFACT='${escapePowerShell(generated.runbook)}'`,
    `$env:HIDDENSHIELD_POSTGRES_RELEASE_OWNER_SIGNOFF_ARTIFACT='${escapePowerShell(generated.signoff)}'`,
    "$env:HIDDENSHIELD_POSTGRES_REQUIRE_PRODUCTION_READY='1'",
    'npm run cloud:postgres-production-readiness-gate',
    '',
  ].join('\r\n');
  writeFileSync(path, content, 'utf8');
  return path;
}

function latestFormalHttpArtifact() {
  const result = spawnSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-Command',
      "(Get-ChildItem 'tmp-ui-qa/postgres-http-gate/*.json' | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName",
    ],
    { cwd: rootDir, encoding: 'utf8' },
  );
  const path = result.stdout.trim();
  if (!path || !existsSync(path)) {
    throw new Error('missing formal PostgreSQL HTTP Gate artifact');
  }
  return path;
}

function percentiles(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return {
    count: sorted.length,
    min: sorted[0] ?? null,
    p50: percentile(sorted, 0.5),
    p95: percentile(sorted, 0.95),
    p99: percentile(sorted, 0.99),
    max: sorted.at(-1) ?? null,
  };
}

function percentile(sorted, ratio) {
  if (sorted.length === 0) {
    return null;
  }
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * ratio) - 1)];
}

function detectPodman() {
  const result = spawnSync(command('podman'), ['--version'], {
    encoding: 'utf8',
    shell: false,
  });
  if (result.status !== 0) {
    throw new Error('Podman is required for the P5 local rehearsal');
  }
  return { version: result.stdout.trim() };
}

function detectLocalPostgresImage() {
  const result = spawnSync(command('podman'), ['images', '--format', '{{.Repository}}:{{.Tag}}'], {
    encoding: 'utf8',
    shell: false,
  });
  if (result.status !== 0) {
    return null;
  }
  const images = result.stdout.split(/\r?\n/).map((value) => value.trim()).filter(Boolean);
  return (
    images.find((value) => value === 'localhost/postgres:16') ||
    images.find((value) => /(?:^|\/)postgres:16(?:-|$)/.test(value)) ||
    null
  );
}

function run(bin, args, extraEnvironment = {}, options = {}) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command(bin), args, {
      cwd: rootDir,
      env: { ...process.env, ...extraEnvironment },
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
      if (options.echo) {
        process.stdout.write(chunk);
      }
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
      if (options.echo) {
        process.stderr.write(chunk);
      }
    });
    child.on('exit', (code) => {
      if (code === 0 || options.allowFailure) {
        resolvePromise({ code, stdout, stderr });
      } else {
        reject(new Error(`${bin} ${args.join(' ')} failed (${code})\n${stdout}${stderr}`));
      }
    });
    child.on('error', reject);
  });
}

async function removeContainer(name) {
  const result = await run('podman', ['rm', '--force', name], {}, { allowFailure: true });
  return { kind: 'container', name, status: result.code === 0 ? 'removed' : 'not_present' };
}

async function removeVolume(name) {
  const result = await run('podman', ['volume', 'rm', '--force', name], {}, { allowFailure: true });
  return { kind: 'volume', name, status: result.code === 0 ? 'removed' : 'remove_failed' };
}

function waitForExit(child) {
  if (child.exitCode != null) {
    return Promise.resolve();
  }
  return new Promise((resolvePromise, reject) => {
    child.once('exit', (code) => {
      if (code === 0 || code == null) {
        resolvePromise();
      } else {
        reject(new Error(`child process exited with code ${code}`));
      }
    });
    child.once('error', reject);
  });
}

async function findAvailablePort() {
  return await new Promise((resolvePromise, reject) => {
    const server = createServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => {
        if (!address || typeof address === 'string') {
          reject(new Error('failed to allocate port'));
          return;
        }
        resolvePromise(address.port);
      });
    });
  });
}

function command(name) {
  if (process.platform !== 'win32') {
    return name;
  }
  if (name === 'cargo') {
    return 'cargo.exe';
  }
  if (name === 'node') {
    return 'node.exe';
  }
  return name;
}

function writePrefixed(label, chunk) {
  for (const line of chunk.toString().split(/\r?\n/)) {
    if (line) {
      console.log(`[${label}] ${line}`);
    }
  }
}

function escapePowerShell(value) {
  return value.replaceAll("'", "''");
}

function normalizePath(path) {
  return path.replaceAll('\\', '/');
}

function delay(milliseconds) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, milliseconds));
}
