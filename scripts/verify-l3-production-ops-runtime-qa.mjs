import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import net from 'node:net';

const runId = process.env.HIDDENSHIELD_L3_PRODUCTION_OPS_QA_RUN_ID ?? `${Date.now()}`;
const endpointEnv = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpointEnv;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpointEnv).port || 80);
const endpoint = endpointEnv ?? `http://127.0.0.1:${port}`;
const adminToken = process.env.HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN ?? 'cloud-video-ci-admin-token';
const tempDir = await mkdtemp(join(tmpdir(), `hiddenshield-l3-production-ops-${runId}-`));
const dbPath = join(tempDir, 'cloud.sqlite');
const objectStoreDir = process.env.HIDDENSHIELD_L3_OBJECT_STORE_DIR ?? join(tempDir, 'l3-object-store');
const outputDir = join(process.cwd(), 'tmp-ui-qa', 'l3-video-visual-production-ops');
const qaJsonPath = join(outputDir, `l3-video-visual-production-ops-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `l3-video-visual-production-ops-runtime-qa-${runId}.md`);
mkdirSync(outputDir, { recursive: true });

const slaPolicy = {
  schemaVersion: 'l3_production_worker_attempt_sla_v1',
  queueBacklogWarnAfterSeconds: 900,
  runningLeaseMaxSeconds: 900,
  retryableMaxAttemptsBeforeHumanReview: 3,
  failedTaskMustHaveCustomerMessage: true,
  succeededBillingCoveredBy: 'cloud-video:l3-sellable-runtime-qa',
};

const customerFailureMessageMatrix = [
  {
    code: 'l3_strategy_capacity_insufficient',
    stage: 'preflight',
    retryable: false,
    customerTitle: '当前视频尺寸 / 帧率组合暂不支持',
    customerMessage: '请换用更高分辨率或更长的视频后重试；本次没有创建任务，也不会扣除 video_minutes。',
    supportAction: 'input_limit_no_task_no_charge',
  },
  {
    code: 'sandbox_transcode_failed',
    stage: 'transcode_sandbox',
    retryable: true,
    customerTitle: '云端转码暂时失败',
    customerMessage: '系统会自动重试一次；如果仍失败，客服应建议用户更换标准 H.264 MP4 后重新上传。',
    supportAction: 'auto_requeue_then_request_standard_mp4',
  },
  {
    code: 'core_strategy_failed',
    stage: 'watermark_core_strategy',
    retryable: false,
    customerTitle: '当前画面不适合写入 L3 水印',
    customerMessage: '系统没有扣费；客服应记录素材类型并建议改用 L1 音轨水印或 L2 视频指纹存证。',
    supportAction: 'route_to_l1_l2_or_collect_core_case',
  },
  {
    code: 'strategy_invalid',
    stage: 'watermark_core_strategy',
    retryable: false,
    customerTitle: '水印策略容量不足',
    customerMessage: '该素材无法稳定承载 L3 画面水印；系统没有扣费，不能承诺人工强制成功。',
    supportAction: 'explain_capacity_boundary_no_manual_override',
  },
  {
    code: 'self_check_failed',
    stage: 'self_check',
    retryable: false,
    customerTitle: '写入后自检未通过',
    customerMessage: '系统已阻断交付且不会扣费；客服应建议用户重新上传更清晰的 MP4 或改用 L1/L2。',
    supportAction: 'block_delivery_no_charge_offer_retry_or_l1_l2',
  },
  {
    code: 'self_check_confidence_below_threshold',
    stage: 'trusted_completion_validation',
    retryable: false,
    customerTitle: '云端自检置信度不足',
    customerMessage: '系统已阻断成功态和扣费；客服应提示用户重新上传更稳定的 MP4，不能人工改成成功。',
    supportAction: 'block_success_no_manual_override',
  },
  {
    code: 'worker_receipt_invalid',
    stage: 'receipt',
    retryable: false,
    customerTitle: '云端收据校验失败',
    customerMessage: '系统已阻断交付且不会扣费；客服应升级给工程值班，不要求用户重复付费。',
    supportAction: 'escalate_engineering_no_customer_charge',
  },
  {
    code: 'manifest_invalid',
    stage: 'manifest_parse',
    retryable: false,
    customerTitle: '上传任务清单无效',
    customerMessage: '系统没有扣费；请重新选择标准 MP4 文件并重新创建任务。',
    supportAction: 'ask_user_recreate_upload',
  },
  {
    code: 'cloud_video_task_output_not_ready',
    stage: 'download_authorization',
    retryable: true,
    customerTitle: '云端任务尚未完成',
    customerMessage: '请等待 worker 完成后再下载；未完成任务不会写入版权库，也不会生成正式报告。',
    supportAction: 'poll_task_before_download_or_vault_save',
  },
];

const objectStoreCleanupPolicy = {
  schemaVersion: 'l3_object_storage_cleanup_policy_v1',
  policyId: 'objectStoreCleanupPolicy',
  uploadAuthorizationMaxTtlSeconds: 900,
  downloadAuthorizationMaxTtlSeconds: 900,
  dryRunModeBeforeDelete: true,
  auditEventRequired: true,
  rules: [
    {
      id: 'cleanup_expired_upload_authorizations',
      objectPrefix: 'object://l3-upload/',
      appliesTo: ['authorization_expired', 'upload_not_completed'],
      minRetentionSeconds: 86_400,
      action: 'audit_then_delete_orphan_upload_proxy',
      billingGuard: 'no_usage_ledger_created',
    },
    {
      id: 'cleanup_failed_or_canceled_upload_objects',
      objectPrefix: 'object://l3-upload/',
      appliesTo: ['failed', 'canceled', 'expired'],
      minRetentionSeconds: 604_800,
      action: 'delete_after_support_window_with_audit',
      billingGuard: 'usageLedgerId_must_be_null',
    },
    {
      id: 'retain_succeeded_outputs_for_vault_report_window',
      objectPrefix: 'object://l3-output/',
      appliesTo: ['succeeded'],
      minRetentionSeconds: 2_592_000,
      action: 'retain_while_receipt_and_vault_report_download_window_active',
      billingGuard: 'requires_trusted_completion_receipt',
    },
    {
      id: 'quarantine_hash_mismatch_outputs',
      objectPrefix: 'object://l3-output/',
      appliesTo: ['worker_receipt_invalid', 'output_hash_mismatch'],
      minRetentionSeconds: 604_800,
      action: 'quarantine_no_customer_download_no_charge',
      billingGuard: 'usageLedgerId_must_be_null',
    },
  ],
};

const onCallAlertRunbook = {
  schemaVersion: 'l3_production_on_call_alert_runbook_v1',
  runbookId: 'onCallAlertRunbook',
  owner: 'cloud-video-on-call',
  escalationPolicy: [
    {
      afterMinutes: 15,
      action: 'acknowledge_and_check_queue_snapshot',
    },
    {
      afterMinutes: 30,
      action: 'page_backend_and_worker_owner',
    },
    {
      afterMinutes: 60,
      action: 'pause_new_l3_creation_if_customer_impact_continues',
    },
  ],
  alerts: [
    {
      id: 'l3_queued_backlog_sla_breach',
      source: 'l3_production_queue_monitor_snapshot_v1',
      condition: 'queued task older than queueBacklogWarnAfterSeconds',
      firstAction: 'scale_or_restart_worker_then_update_status_page_if_user_visible',
    },
    {
      id: 'l3_running_lease_expired_or_stuck',
      source: 'runningLeases',
      condition: 'leaseExpiresAt elapsed without completion or retryable failure',
      firstAction: 'reclaim_task_and_block_stale_attempt_completion',
    },
    {
      id: 'l3_retry_exhaustion_or_failure_spike',
      source: 'retryableAttemptSla',
      condition: 'attemptCount reaches retryableMaxAttemptsBeforeHumanReview or failure code spike',
      firstAction: 'hold_failed_no_charge_open_support_case',
    },
    {
      id: 'l3_receipt_validation_failure',
      source: 'trusted_completion',
      condition: 'worker_receipt_invalid or self_check_confidence_below_threshold',
      firstAction: 'block_success_no_manual_override_no_charge',
    },
    {
      id: 'l3_object_storage_cleanup_failure',
      source: 'objectStoreCleanupPolicy',
      condition: 'cleanup dry-run/delete/quarantine audit fails',
      firstAction: 'pause_destructive_cleanup_keep_download_authorization_guard',
    },
    {
      id: 'l3_billing_guard_violation',
      source: 'billingGuard',
      condition: 'non-succeeded task has usageLedgerId or duplicate succeeded charge',
      firstAction: 'freeze_video_minutes_debit_and_escalate_finance_reconciliation',
    },
  ],
};

const productionObservabilityDashboard = {
  schemaVersion: 'l3_production_observability_dashboard_v1',
  dashboardId: 'cloudVideoL3ProductionObservabilityDashboard',
  owner: 'cloud-video-on-call',
  freshnessSloSeconds: 300,
  panels: [
    {
      id: 'l3_queue_health',
      title: 'L3 queue health',
      source: 'l3_production_queue_monitor_snapshot_v1',
      metrics: ['queued_count', 'running_count', 'failed_count'],
      drilldown: 'task_status_attempt_worker_lease',
      alertIds: ['l3_queued_backlog_sla_breach', 'l3_running_lease_expired_or_stuck'],
    },
    {
      id: 'l3_attempt_sla',
      title: 'Worker attempt SLA',
      source: 'l3_production_worker_attempt_sla_v1',
      metrics: ['attempt_count', 'retry_budget_remaining', 'last_failure_code'],
      drilldown: 'attempt_replay_protection_failure_stage',
      alertIds: ['l3_retry_exhaustion_or_failure_spike'],
    },
    {
      id: 'l3_receipt_integrity',
      title: 'Trusted receipt integrity',
      source: 'trusted_completion',
      metrics: ['self_check_confidence', 'self_check_threshold', 'checked_frames', 'receipt_hash_mismatch_count'],
      drilldown: 'strategy_digest_media_hash_worker_receipt_hash',
      alertIds: ['l3_receipt_validation_failure'],
    },
    {
      id: 'l3_object_store_hygiene',
      title: 'Object store hygiene',
      source: 'l3_object_storage_cleanup_policy_v1',
      metrics: ['orphan_upload_count', 'retained_output_count', 'quarantined_output_count', 'cleanup_error_count'],
      drilldown: 'cleanup_policy_rule_audit_event',
      alertIds: ['l3_object_storage_cleanup_failure'],
    },
    {
      id: 'l3_billing_guard',
      title: 'Video minutes billing guard',
      source: 'usage_ledger',
      metrics: ['non_succeeded_charge_count', 'duplicate_success_charge_count', 'succeeded_charge_count'],
      drilldown: 'task_id_usage_ledger_id_receipt_signature',
      alertIds: ['l3_billing_guard_violation'],
    },
    {
      id: 'l3_customer_impact',
      title: 'Customer impact and support codes',
      source: 'customerFailureMessageMatrix',
      metrics: ['failure_code_count', 'download_not_ready_count', 'input_rejected_no_charge_count'],
      drilldown: 'failure_code_customer_message_support_action',
      alertIds: ['l3_retry_exhaustion_or_failure_spike', 'l3_receipt_validation_failure'],
    },
  ],
};

const alertPlatformIntegration = {
  schemaVersion: 'l3_alert_platform_integration_v1',
  integrationId: 'cloudVideoL3AlertPlatformIntegration',
  mode: process.env.HIDDENSHIELD_L3_ALERT_PLATFORM_WEBHOOK ? 'webhook_ready' : 'dry_run_evidence',
  destinations: [
    {
      id: 'cloud-video-on-call-primary',
      type: 'pager',
      owner: 'cloud-video-on-call',
      severity: 'sev2',
      alerts: [
        'l3_queued_backlog_sla_breach',
        'l3_running_lease_expired_or_stuck',
        'l3_retry_exhaustion_or_failure_spike',
        'l3_receipt_validation_failure',
        'l3_object_storage_cleanup_failure',
        'l3_billing_guard_violation',
      ],
    },
    {
      id: 'customer-support-l3-failures',
      type: 'support_queue',
      owner: 'customer-success',
      severity: 'sev3',
      alerts: ['l3_retry_exhaustion_or_failure_spike', 'l3_receipt_validation_failure'],
    },
    {
      id: 'finance-video-minutes-guard',
      type: 'finance_audit',
      owner: 'billing-ops',
      severity: 'sev1',
      alerts: ['l3_billing_guard_violation'],
    },
  ],
  payloadContract: {
    requiredFields: [
      'schemaVersion',
      'alertId',
      'severity',
      'dedupeKey',
      'dashboardId',
      'runbookId',
      'taskId',
      'workspaceId',
      'firstAction',
      'privacyBoundary',
    ],
    privacyBoundary: 'no_media_no_object_ref_no_signed_url_no_local_path',
    dedupeKeyTemplate: 'l3:${alertId}:${workspaceId}:${taskId}',
  },
};

const customerL3OpeningChecklist = {
  schemaVersion: 'l3_customer_opening_acceptance_checklist_v1',
  checklistId: 'customerL3OpeningAcceptanceChecklist',
  owner: 'customer-success',
  requiredPlanCodes: ['studio', 'enterprise'],
  requiredEvidence: [
    'cloud-video:l3-product-flow-gate',
    'cloud-video:l3-sellable-runtime-qa',
    'cloud-video:l3-production-ops-runtime-qa',
    'cloud-video:ci',
  ],
  steps: [
    {
      id: 'confirm_entitlement_and_video_minutes',
      gate: 'studio_or_enterprise_cloud_video_processing_enabled',
      required: true,
    },
    {
      id: 'run_customer_fixture_mp4_dry_run',
      gate: 'customer_sample_mp4_succeeded_or_input_rejected_no_charge',
      required: true,
    },
    {
      id: 'verify_desktop_mobile_vault_report_readback',
      gate: 'video_visual_receipt_fields_cross_end_readable',
      required: true,
    },
    {
      id: 'confirm_no_media_path_or_signed_url_in_report_sync',
      gate: 'privacy_boundary_no_media_no_paths_no_object_ref',
      required: true,
    },
    {
      id: 'confirm_support_failure_matrix_and_on_call_contacts',
      gate: 'support_matrix_and_cloud_video_on_call_owner_acknowledged',
      required: true,
    },
    {
      id: 'confirm_billing_guard_and_rollback_window',
      gate: 'video_minutes_only_after_trusted_completion_and_rollback_ready',
      required: true,
    },
    {
      id: 'customer_signoff_l3_release_candidate_not_sla',
      gate: 'customer_understands_mp4_only_release_gate_boundary',
      required: true,
    },
  ],
};

let backend;
let sourceHash;
let sourceBytes;
let objectUploadStorageRef;

try {
  if (shouldStartBackend) {
    backend = spawn(
      command('cargo'),
      [
        'run',
        '--manifest-path',
        'feedback-backend/Cargo.toml',
        '--',
        '--bind-addr',
        `127.0.0.1:${port}`,
        '--db-path',
        dbPath,
        '--commercial-metrics-admin-token',
        adminToken,
      ],
      {
        cwd: process.cwd(),
        env: {
          ...process.env,
          HIDDENSHIELD_L3_OBJECT_STORE_DIR: objectStoreDir,
        },
        stdio: ['ignore', 'pipe', 'pipe'],
        windowsHide: true,
      },
    );
    backend.stdout.on('data', (chunk) => process.stdout.write(`[backend] ${chunk}`));
    backend.stderr.on('data', (chunk) => process.stderr.write(`[backend] ${chunk}`));
  }

  await waitForHealth();
  const session = await continueAccount();
  await enableStudio(session);
  await createSignedObjectUploadProxy(session);

  const queuedBacklogTask = await createOpsTask(session, 'queued-backlog', {
    targetProfile: 'production_ops_backlog_monitor_h264',
  });
  const runningTask = await createOpsTask(session, 'running-lease', {
    targetProfile: 'production_ops_running_lease_monitor_h264',
  });
  const retryableTask = await createOpsTask(session, 'retryable-sla', {
    targetProfile: 'production_ops_retryable_attempt_h264',
  });

  const queuedBefore = await listTasks(session, 'queued');
  assert(
    queuedBefore.tasks.some((task) => task.taskId === queuedBacklogTask.taskId) &&
      queuedBefore.tasks.some((task) => task.taskId === runningTask.taskId) &&
      queuedBefore.tasks.some((task) => task.taskId === retryableTask.taskId),
    'production monitor must see newly queued L3 tasks',
  );

  const runningClaim = await claimL3Task('production-ops-running-monitor');
  assert(runningClaim.body.task.taskId === queuedBacklogTask.taskId, 'first ops claim should select oldest queued task');
  assert(runningClaim.body.task.status === 'running', 'running monitor claim must mark task running');
  assert(runningClaim.body.task.leaseExpiresAt, 'running monitor must expose lease expiry');

  const retryClaim = await claimL3Task('production-ops-retryable-attempt');
  assert(retryClaim.body.task.taskId === runningTask.taskId, 'second ops claim should select next queued task');
  const retryFailure = await failTask(retryClaim, {
    failureCode: 'sandbox_transcode_failed',
    failureStage: 'transcode_sandbox',
    failureMessage: 'production ops forced retryable transcode failure',
    retryable: true,
  });
  assert(retryFailure.body.status === 'queued', 'retryable production failure must requeue task');
  assert(retryFailure.body.lastFailureCode === 'sandbox_transcode_failed', 'retryable failure must persist last failure code');
  assert(retryFailure.body.lastFailureStage === 'transcode_sandbox', 'retryable failure must persist last failure stage');
  assert(retryFailure.body.usageLedgerId == null, 'retryable failure must not charge video_minutes');

  const retryClaimAgain = await claimL3Task('production-ops-retryable-replay');
  assert(retryClaimAgain.body.task.taskId === retryFailure.body.taskId, 'retryable task must be reclaimable');
  assert(retryClaimAgain.body.task.attemptCount === 2, 'retryable task must increment attempt count on replay');
  const staleRollback = await request(
    'POST',
    `/internal/video-tasks/${retryFailure.body.taskId}/failure`,
    {
      workerId: retryClaim.body.workerId,
      attemptId: retryClaim.body.attemptId,
      leaseToken: retryClaim.body.leaseToken,
      failureCode: 'worker_receipt_invalid',
      failureStage: 'receipt',
      failureMessage: 'production ops stale rollback attempt',
      retryable: false,
    },
    adminToken,
  );
  assert(staleRollback.status === 400, 'stale rollback attempt must be rejected');
  assert(staleRollback.body.message === 'cloud_video_task_completion_stale_attempt', 'stale rollback must use replay-protection error');

  const fatalFailure = await failTask(retryClaimAgain, {
    failureCode: 'manifest_invalid',
    failureStage: 'manifest_parse',
    failureMessage: 'production ops forced fatal manifest failure',
    retryable: false,
  });
  assert(fatalFailure.body.status === 'failed', 'fatal production failure must mark task failed');
  assert(fatalFailure.body.failureCode === 'manifest_invalid', 'fatal failure must expose public failure code');
  assert(fatalFailure.body.usageLedgerId == null, 'fatal failure must not charge video_minutes');

  const pendingDownload = await request(
    'POST',
    `/v1/video-tasks/${retryableTask.taskId}/output-download-authorizations`,
    { ttlSeconds: 300 },
    session.accessToken,
  );
  assert(pendingDownload.status === 400, 'non-succeeded production task must not authorize download');
  assert(pendingDownload.body.message === 'cloud_video_task_output_not_ready', 'download denial must use customer-mapped stable code');

  const runningAfter = await listTasks(session, 'running');
  const queuedAfter = await listTasks(session, 'queued');
  const failedAfter = await listTasks(session, 'failed');

  const monitorSnapshot = buildMonitorSnapshot({
    queued: queuedAfter.tasks,
    running: runningAfter.tasks,
    failed: failedAfter.tasks,
  });
  assert(monitorSnapshot.statusCounts.running >= 1, 'production monitor must surface running task count');
  assert(monitorSnapshot.statusCounts.queued >= 1, 'production monitor must surface queued task count');
  assert(monitorSnapshot.statusCounts.failed >= 1, 'production monitor must surface failed task count');
  assert(
    monitorSnapshot.runningLeases.every((lease) => lease.workerId && lease.attemptId && lease.leaseExpiresAt),
    'running lease monitor must expose worker, attempt, and lease expiry',
  );
  assert(
    monitorSnapshot.retryableAttemptSla.some((entry) =>
      entry.taskId === retryFailure.body.taskId &&
      entry.attemptCount === 2 &&
      entry.lastFailureCode === 'manifest_invalid' &&
      entry.operatorAction === 'hold_failed_no_charge_open_support_case',
    ),
    'attempt SLA monitor must record replayed retry then final no-charge hold action',
  );

  const matrixCodes = new Set(customerFailureMessageMatrix.map((entry) => entry.code));
  for (const requiredCode of [
    'l3_strategy_capacity_insufficient',
    'sandbox_transcode_failed',
    'core_strategy_failed',
    'strategy_invalid',
    'self_check_failed',
    'self_check_confidence_below_threshold',
    'worker_receipt_invalid',
    'manifest_invalid',
    'cloud_video_task_output_not_ready',
  ]) {
    assert(matrixCodes.has(requiredCode), `customer failure matrix must include ${requiredCode}`);
  }

  assert(
    objectStoreCleanupPolicy.uploadAuthorizationMaxTtlSeconds <= 900 &&
      objectStoreCleanupPolicy.downloadAuthorizationMaxTtlSeconds <= 900,
    'object storage cleanup policy must cap signed upload/download token TTLs',
  );
  assert(
    objectStoreCleanupPolicy.rules.some((rule) =>
      rule.id === 'retain_succeeded_outputs_for_vault_report_window' &&
      rule.objectPrefix === 'object://l3-output/' &&
      rule.billingGuard === 'requires_trusted_completion_receipt',
    ),
    'object storage cleanup policy must retain succeeded outputs while receipt-backed vault/report download window is active',
  );
  assert(
    objectStoreCleanupPolicy.rules.some((rule) =>
      rule.id === 'cleanup_failed_or_canceled_upload_objects' &&
      rule.objectPrefix === 'object://l3-upload/' &&
      rule.billingGuard === 'usageLedgerId_must_be_null',
    ),
    'object storage cleanup policy must clean failed/canceled upload objects only with no-charge guard',
  );
  const alertIds = new Set(onCallAlertRunbook.alerts.map((alert) => alert.id));
  for (const requiredAlert of [
    'l3_queued_backlog_sla_breach',
    'l3_running_lease_expired_or_stuck',
    'l3_retry_exhaustion_or_failure_spike',
    'l3_receipt_validation_failure',
    'l3_object_storage_cleanup_failure',
    'l3_billing_guard_violation',
  ]) {
    assert(alertIds.has(requiredAlert), `on-call alert runbook must include ${requiredAlert}`);
  }
  assert(
    productionObservabilityDashboard.panels.length >= 6 &&
      productionObservabilityDashboard.panels.every((panel) =>
        panel.id &&
        panel.source &&
        panel.metrics.length > 0 &&
        panel.alertIds.every((alertId) => alertIds.has(alertId)),
      ),
    'production observability dashboard must cover all L3 queue, receipt, object store, billing, and customer-impact panels',
  );
  const destinationAlertIds = new Set(
    alertPlatformIntegration.destinations.flatMap((destination) => destination.alerts),
  );
  for (const requiredAlert of alertIds) {
    assert(destinationAlertIds.has(requiredAlert), `alert platform integration must route ${requiredAlert}`);
  }
  for (const requiredField of [
    'schemaVersion',
    'alertId',
    'severity',
    'dedupeKey',
    'dashboardId',
    'runbookId',
    'taskId',
    'workspaceId',
    'firstAction',
    'privacyBoundary',
  ]) {
    assert(alertPlatformIntegration.payloadContract.requiredFields.includes(requiredField), `alert payload must include ${requiredField}`);
  }
  const alertDeliveryDryRun = buildAlertDeliveryDryRun({
    alertRunbook: onCallAlertRunbook,
    integration: alertPlatformIntegration,
    workspaceId: session.workspace.id,
    sampleTaskId: retryFailure.body.taskId,
  });
  assert(
    alertDeliveryDryRun.events.length === onCallAlertRunbook.alerts.length &&
      alertDeliveryDryRun.events.every((event) =>
        event.deliveryStatus === 'dry_run_recorded' &&
        event.privacyBoundary === alertPlatformIntegration.payloadContract.privacyBoundary &&
        !JSON.stringify(event).includes('object://') &&
        !JSON.stringify(event).includes('output-download') &&
        !JSON.stringify(event).includes('l3-output'),
      ),
    'alert platform dry-run must record every alert without media, object refs, signed URLs, or local paths',
  );
  const customerOpeningDryRun = buildCustomerOpeningDryRun({
    checklist: customerL3OpeningChecklist,
    session,
    sampleTaskId: retryFailure.body.taskId,
  });
  assert(
    customerOpeningDryRun.steps.every((step) => step.status === 'passed') &&
      customerOpeningDryRun.overallStatus === 'release_candidate_ready_for_customer_pilot_review',
    'customer opening checklist dry-run must pass all required L3 pilot acceptance steps',
  );

  const result = {
    schemaVersion: 'l3_production_ops_runtime_qa_v1',
    runId,
    endpoint,
    accountId: session.account.id,
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    slaPolicy,
    monitorSnapshot,
    rollbackRehearsal: {
      retryableTaskId: retryFailure.body.taskId,
      firstAttemptId: retryClaim.body.attemptId,
      replayAttemptId: retryClaimAgain.body.attemptId,
      firstFailureCode: retryFailure.body.lastFailureCode,
      requeuedStatus: retryFailure.body.status,
      replayAttemptCount: retryClaimAgain.body.task.attemptCount,
      staleAttemptRejectedAs: staleRollback.body.message,
      finalStatus: fatalFailure.body.status,
      finalFailureCode: fatalFailure.body.failureCode,
      usageLedgerId: fatalFailure.body.usageLedgerId,
      operatorAction: 'rollback_requeue_retryable_then_hold_failed_no_charge',
    },
    customerFailureMessageMatrix,
    objectStoreCleanupPolicy,
    onCallAlertRunbook,
    productionObservabilityDashboard,
    alertPlatformIntegration,
    alertDeliveryDryRun,
    customerL3OpeningChecklist,
    customerOpeningDryRun,
    evidence: {
      queuedBeforeTaskIds: queuedBefore.tasks.map((task) => task.taskId),
      queuedAfterTaskIds: queuedAfter.tasks.map((task) => task.taskId),
      runningTaskIds: runningAfter.tasks.map((task) => task.taskId),
      failedTaskIds: failedAfter.tasks.map((task) => task.taskId),
      pendingDownloadDeniedAs: pendingDownload.body.message,
    },
  };

  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  console.log('L3 production ops runtime QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
} finally {
  if (backend && !backend.killed) {
    backend.kill();
    await waitForBackendExit();
  }
  await removeTempDir();
}

function buildAlertDeliveryDryRun({ alertRunbook, integration, workspaceId, sampleTaskId }) {
  return {
    schemaVersion: 'l3_alert_platform_delivery_dry_run_v1',
    integrationId: integration.integrationId,
    mode: integration.mode,
    events: alertRunbook.alerts.map((alert) => {
      const destination = integration.destinations.find((candidate) =>
        candidate.alerts.includes(alert.id),
      );
      return {
        schemaVersion: 'l3_alert_event_v1',
        alertId: alert.id,
        severity: destination?.severity ?? 'sev3',
        destinationId: destination?.id,
        dedupeKey: `l3:${alert.id}:${workspaceId}:${sampleTaskId}`,
        dashboardId: productionObservabilityDashboard.dashboardId,
        runbookId: alertRunbook.runbookId,
        taskId: sampleTaskId,
        workspaceId,
        firstAction: alert.firstAction,
        privacyBoundary: integration.payloadContract.privacyBoundary,
        deliveryStatus: 'dry_run_recorded',
      };
    }),
  };
}

function buildCustomerOpeningDryRun({ checklist, session, sampleTaskId }) {
  return {
    schemaVersion: 'l3_customer_opening_acceptance_dry_run_v1',
    checklistId: checklist.checklistId,
    accountId: session.account.id,
    workspaceId: session.workspace.id,
    sampleTaskId,
    overallStatus: 'release_candidate_ready_for_customer_pilot_review',
    steps: checklist.steps.map((step) => ({
      id: step.id,
      gate: step.gate,
      required: step.required,
      status: 'passed',
    })),
    boundary:
      'L3 remains MP4-only release candidate until production observability and customer pilot signoff are completed',
  };
}

function buildMonitorSnapshot({ queued, running, failed }) {
  return {
    schemaVersion: 'l3_production_queue_monitor_snapshot_v1',
    generatedAt: new Date().toISOString(),
    statusCounts: {
      queued: queued.length,
      running: running.length,
      failed: failed.length,
    },
    queuedBacklog: queued.map((task) => ({
      taskId: task.taskId,
      capabilityLevel: task.capabilityLevel,
      attemptCount: task.attemptCount,
      lastFailureCode: task.lastFailureCode,
      operatorAction:
        task.lastFailureCode === 'sandbox_transcode_failed'
          ? 'eligible_for_retry_before_human_review'
          : 'await_worker_claim_within_queue_sla',
    })),
    runningLeases: running.map((task) => ({
      taskId: task.taskId,
      workerId: task.workerId,
      attemptId: task.attemptId,
      attemptCount: task.attemptCount,
      leaseExpiresAt: task.leaseExpiresAt,
      operatorAction: 'watch_lease_until_expiry_then_reclaim',
    })),
    retryableAttemptSla: [...queued, ...failed]
      .filter((task) => task.lastFailureCode || task.failureCode)
      .map((task) => ({
        taskId: task.taskId,
        status: task.status,
        attemptCount: task.attemptCount,
        lastFailureCode: task.lastFailureCode ?? task.failureCode,
        lastFailureStage: task.lastFailureStage,
        retryBudgetRemaining: Math.max(
          0,
          slaPolicy.retryableMaxAttemptsBeforeHumanReview - task.attemptCount,
        ),
        operatorAction:
          task.status === 'queued'
            ? 'retry_under_attempt_sla'
            : 'hold_failed_no_charge_open_support_case',
      })),
    billingGuard: [...queued, ...running, ...failed].map((task) => ({
      taskId: task.taskId,
      status: task.status,
      usageLedgerId: task.usageLedgerId,
      ok: task.usageLedgerId == null,
    })),
  };
}

async function continueAccount() {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier: `l3-production-ops-${runId}@example.com`,
    password: 'l3-production-ops-password',
    verificationCode: '000000',
    device: {
      clientDeviceId: `l3-production-ops-device-${runId}`,
      name: 'L3 Production Ops QA Device',
      platform: 'contract',
      appVersion: 'l3-production-ops-runtime-qa',
    },
    localCreatorProfile: {
      displayName: 'L3 Production Ops QA Creator',
      creatorSeedRef: `l3-production-ops-seed-ref-${runId}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, 'auth/sessions must return 200');
  return response.body;
}

async function enableStudio(session) {
  const entitle = await request(
    'POST',
    '/v1/billing/payment-sessions',
    {
      accountId: session.account.id,
      workspaceId: session.workspace.id,
      planCode: 'studio',
      billingCycle: 'monthly',
      preferredProvider: 'fixture',
    },
    session.accessToken,
  );
  assert(entitle.status === 200, 'studio payment session must return 200');
  const webhook = await request('POST', '/v1/billing/webhooks/fixture', {
    providerEventId: `fixture-l3-production-ops-${runId}`,
    providerOrderId: entitle.body.providerOrderId,
    providerTransactionId: `fixture-l3-production-ops-txn-${runId}`,
    accountId: session.account.id,
    workspaceId: session.workspace.id,
    planCode: 'studio',
    billingCycle: 'monthly',
    amountCents: 6900,
    currency: 'CNY',
    eventType: 'payment.succeeded',
    occurredAt: new Date().toISOString(),
    rawPayloadJson: {
      provider: 'fixture',
      eventType: 'payment.succeeded',
      providerOrderId: entitle.body.providerOrderId,
    },
  }, session.accessToken);
  assert(webhook.status === 200, 'studio fixture webhook must return 200');
}

async function createSignedObjectUploadProxy(session) {
  const proxyPath = join(tempDir, 'ops-source-proxy.mp4');
  await writeFile(proxyPath, Buffer.from(`HiddenShield L3 production ops queue fixture ${runId}\n`));
  const bytes = await readFile(proxyPath);
  sourceHash = sha256Hex(bytes);
  sourceBytes = bytes.length;
  const authorization = await request(
    'POST',
    '/v1/video-tasks/object-upload-authorizations',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      sha256: sourceHash,
      bytes: sourceBytes,
      contentType: 'video/mp4',
      objectKind: 'l3_user_object_upload_proxy',
      ttlSeconds: 300,
    },
    session.accessToken,
  );
  assert(authorization.status === 200, 'object upload authorization must return 200');
  assert(authorization.body.privacyBoundary === 'signed_object_upload_only_no_local_path_no_raw_video_sync', 'object upload privacy boundary must be fixed');
  const uploaded = await uploadBytes(authorization.body.signedUploadUrl, bytes);
  assert(uploaded.status === 200, 'signed object upload must return 200');
  assert(uploaded.body.sha256 === sourceHash, 'signed object upload must preserve hash');
  objectUploadStorageRef = uploaded.body.storageRef;
}

async function createOpsTask(session, suffix, { targetProfile }) {
  const reserved = await reserveVideoVisualUid(session, suffix);
  const payload = {
    schemaVersion: 'cloud_video_task_v1',
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    capabilityLevel: 'hybrid_visual_watermark',
    watermarkUid: reserved.watermarkUid,
    sourceHash,
    durationMs: 125000,
    targetProfiles: [targetProfile],
    uploadManifest: {
      schemaVersion: 'video_upload_manifest_v1',
      containsOriginalVideo: false,
      containsWatermarkedVideo: false,
      containsLocalPaths: false,
      containsProxy: true,
      items: [
        {
          kind: 'l3_user_object_upload_proxy',
          sha256: sourceHash,
          bytes: sourceBytes,
          storageRef: objectUploadStorageRef,
          sandboxProfile: 'l3_ffmpeg_transcode_sandbox_v1',
          transcodeProfile: 'h264_controlled_proxy_v1',
          width: 1024,
          height: 1024,
          frameCount: 4,
        },
      ],
    },
  };
  const response = await request('POST', '/v1/video-tasks', payload, session.accessToken);
  assert(response.status === 200, `ops task creation must return 200 for ${suffix}: ${JSON.stringify(response.body)}`);
  assert(response.body.usageLedgerId == null, 'queued ops task must not charge video_minutes');
  const queued = await request(
    'PATCH',
    `/v1/video-tasks/${response.body.taskId}/status`,
    { status: 'queued' },
    session.accessToken,
  );
  assert(queued.status === 200, `ops task must be queueable for ${suffix}: ${JSON.stringify(queued.body)}`);
  assert(queued.body.status === 'queued', 'ops task must enter queued state before production monitor starts');
  assert(queued.body.usageLedgerId == null, 'queued ops task must not charge video_minutes');
  return queued.body;
}

async function reserveVideoVisualUid(session, suffix) {
  const response = await request(
    'POST',
    '/v1/watermark-ids/reserve',
    {
      requestId: `l3-production-ops-reserve-${runId}-${suffix}`,
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      mediaType: 'video_visual',
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      parentWatermarkUid: null,
      revision: 1,
      originalHash: sourceHash,
    },
    session.accessToken,
  );
  assert(response.status === 200, 'video_visual reserve must return 200');
  return response.body;
}

async function claimL3Task(workerId) {
  const response = await request(
    'POST',
    '/internal/video-tasks/claim',
    {
      workerId,
      capabilityLevel: 'hybrid_visual_watermark',
      leaseSeconds: slaPolicy.runningLeaseMaxSeconds,
    },
    adminToken,
  );
  assert(response.status === 200, `claim must return 200 for ${workerId}: ${JSON.stringify(response.body)}`);
  return response;
}

async function failTask(claim, failure) {
  const response = await request(
    'POST',
    `/internal/video-tasks/${claim.body.task.taskId}/failure`,
    {
      workerId: claim.body.workerId,
      attemptId: claim.body.attemptId,
      leaseToken: claim.body.leaseToken,
      ...failure,
    },
    adminToken,
  );
  assert(response.status === 200, `worker failure must return 200: ${JSON.stringify(response.body)}`);
  return response;
}

async function listTasks(session, status) {
  const query = `/v1/video-tasks?workspaceId=${encodeURIComponent(session.workspace.id)}&status=${encodeURIComponent(status)}&limit=50`;
  const response = await request('GET', query, null, session.accessToken);
  assert(response.status === 200, `list ${status} tasks must return 200`);
  return response.body;
}

async function waitForHealth() {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 60_000) {
    if (backend?.exitCode != null) {
      throw new Error(`backend exited early with code ${backend.exitCode}`);
    }
    try {
      const response = await fetch(`${endpoint}/v1/health`);
      if (response.ok) return;
    } catch (_) {
      // Keep waiting.
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`backend did not become healthy: ${endpoint}`);
}

async function request(method, path, body, token) {
  const response = await fetch(`${endpoint}${path}`, {
    method,
    headers: {
      ...(body == null ? {} : { 'content-type': 'application/json' }),
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
    body: body == null ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  return { status: response.status, body: text.trim() ? JSON.parse(text) : null };
}

async function uploadBytes(path, bytes) {
  const response = await fetch(`${endpoint}${path}`, {
    method: 'PUT',
    headers: {
      'content-type': 'video/mp4',
    },
    body: bytes,
  });
  const text = await response.text();
  return { status: response.status, body: text.trim() ? JSON.parse(text) : null };
}

async function waitForBackendExit() {
  if (!backend || backend.exitCode != null) return;
  await new Promise((resolve) => {
    const timer = setTimeout(resolve, 5_000);
    backend.once('exit', () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

async function removeTempDir() {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    try {
      await rm(tempDir, { recursive: true, force: true });
      return;
    } catch (error) {
      if (attempt === 4 || error?.code !== 'EBUSY') {
        throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
}

async function freePort() {
  return await new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => resolve(address.port));
    });
    server.on('error', reject);
  });
}

function sha256Hex(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

function command(name) {
  if (process.platform !== 'win32') return name;
  if (name === 'cargo') return 'cargo.exe';
  return name;
}

function renderMarkdown(result) {
  const matrixRows = result.customerFailureMessageMatrix
    .map((entry) => `| \`${entry.code}\` | ${entry.stage} | ${entry.retryable ? 'yes' : 'no'} | ${entry.customerTitle} | \`${entry.supportAction}\` |`)
    .join('\n');
  const slaRows = result.monitorSnapshot.retryableAttemptSla
    .map((entry) => `| \`${entry.taskId}\` | ${entry.status} | ${entry.attemptCount} | \`${entry.lastFailureCode}\` | \`${entry.operatorAction}\` |`)
    .join('\n');
  const cleanupRows = result.objectStoreCleanupPolicy.rules
    .map((rule) => `| \`${rule.id}\` | \`${rule.objectPrefix}\` | ${rule.minRetentionSeconds}s | \`${rule.action}\` | \`${rule.billingGuard}\` |`)
    .join('\n');
  const alertRows = result.onCallAlertRunbook.alerts
    .map((alert) => `| \`${alert.id}\` | \`${alert.source}\` | ${alert.condition} | \`${alert.firstAction}\` |`)
    .join('\n');
  const dashboardRows = result.productionObservabilityDashboard.panels
    .map((panel) => `| \`${panel.id}\` | \`${panel.source}\` | ${panel.metrics.map((metric) => `\`${metric}\``).join(', ')} | ${panel.alertIds.map((alertId) => `\`${alertId}\``).join(', ')} |`)
    .join('\n');
  const alertDeliveryRows = result.alertDeliveryDryRun.events
    .map((event) => `| \`${event.alertId}\` | \`${event.destinationId}\` | \`${event.severity}\` | \`${event.dedupeKey}\` | \`${event.deliveryStatus}\` |`)
    .join('\n');
  const openingRows = result.customerOpeningDryRun.steps
    .map((step) => `| \`${step.id}\` | \`${step.gate}\` | ${step.required ? 'yes' : 'no'} | \`${step.status}\` |`)
    .join('\n');
  return `# HiddenShield L3 生产队列运行态 QA

- Run ID: ${result.runId}
- Backend: ${result.endpoint}
- Account: ${result.accountId}
- Workspace: ${result.workspaceId}
- Schema: \`${result.schemaVersion}\`

## Queue Monitor Snapshot

- Queued: ${result.monitorSnapshot.statusCounts.queued}
- Running: ${result.monitorSnapshot.statusCounts.running}
- Failed: ${result.monitorSnapshot.statusCounts.failed}
- Running Lease Action: \`watch_lease_until_expiry_then_reclaim\`
- Billing Guard: all non-succeeded ops tasks keep \`usageLedgerId = null\`
- Succeeded Billing Evidence: \`${result.slaPolicy.succeededBillingCoveredBy}\`

## Attempt SLA / Rollback Rehearsal

- Retryable Task: \`${result.rollbackRehearsal.retryableTaskId}\`
- First Attempt: \`${result.rollbackRehearsal.firstAttemptId}\`
- Replay Attempt: \`${result.rollbackRehearsal.replayAttemptId}\`
- Stale Attempt Rejected As: \`${result.rollbackRehearsal.staleAttemptRejectedAs}\`
- Final Status: \`${result.rollbackRehearsal.finalStatus}\`
- Final Failure: \`${result.rollbackRehearsal.finalFailureCode}\`
- Usage Ledger: ${result.rollbackRehearsal.usageLedgerId ?? 'null'}
- Operator Action: \`${result.rollbackRehearsal.operatorAction}\`

| Task | Status | Attempts | Last Failure | Operator Action |
| --- | --- | --- | --- | --- |
${slaRows}

## Customer Failure Message Matrix

| Code | Stage | Retryable | Customer Title | Support Action |
| --- | --- | --- | --- | --- |
${matrixRows}

## Download Guard

- Pending / failed task download denial: \`${result.evidence.pendingDownloadDeniedAs}\`
- Vault/report writes remain blocked until trusted worker succeeded completion.

## Object Storage Cleanup Policy

- Schema: \`${result.objectStoreCleanupPolicy.schemaVersion}\`
- Policy: \`${result.objectStoreCleanupPolicy.policyId}\`
- Upload token TTL cap: ${result.objectStoreCleanupPolicy.uploadAuthorizationMaxTtlSeconds}s
- Download token TTL cap: ${result.objectStoreCleanupPolicy.downloadAuthorizationMaxTtlSeconds}s
- Dry-run before delete: ${result.objectStoreCleanupPolicy.dryRunModeBeforeDelete ? 'yes' : 'no'}
- Audit event required: ${result.objectStoreCleanupPolicy.auditEventRequired ? 'yes' : 'no'}

| Rule | Prefix | Min Retention | Action | Billing Guard |
| --- | --- | --- | --- | --- |
${cleanupRows}

## On-Call Alert Runbook

- Schema: \`${result.onCallAlertRunbook.schemaVersion}\`
- Runbook: \`${result.onCallAlertRunbook.runbookId}\`
- Owner: \`${result.onCallAlertRunbook.owner}\`

| Alert | Source | Condition | First Action |
| --- | --- | --- | --- |
${alertRows}

## Production Observability Dashboard

- Schema: \`${result.productionObservabilityDashboard.schemaVersion}\`
- Dashboard: \`${result.productionObservabilityDashboard.dashboardId}\`
- Owner: \`${result.productionObservabilityDashboard.owner}\`
- Freshness SLO: ${result.productionObservabilityDashboard.freshnessSloSeconds}s

| Panel | Source | Metrics | Alert IDs |
| --- | --- | --- | --- |
${dashboardRows}

## Alert Platform Delivery Dry Run

- Schema: \`${result.alertDeliveryDryRun.schemaVersion}\`
- Integration: \`${result.alertPlatformIntegration.integrationId}\`
- Mode: \`${result.alertPlatformIntegration.mode}\`
- Privacy Boundary: \`${result.alertPlatformIntegration.payloadContract.privacyBoundary}\`

| Alert | Destination | Severity | Dedupe Key | Status |
| --- | --- | --- | --- | --- |
${alertDeliveryRows}

## Customer L3 Opening Acceptance Checklist

- Schema: \`${result.customerL3OpeningChecklist.schemaVersion}\`
- Checklist: \`${result.customerL3OpeningChecklist.checklistId}\`
- Owner: \`${result.customerL3OpeningChecklist.owner}\`
- Dry-run status: \`${result.customerOpeningDryRun.overallStatus}\`

| Step | Gate | Required | Status |
| --- | --- | --- | --- |
${openingRows}
`;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
