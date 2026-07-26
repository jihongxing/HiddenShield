import { existsSync, readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  ci: readFileSync('.github/workflows/ci.yml', 'utf8'),
  agents: readFileSync('AGENTS.md', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  watermarkPlan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
  watermarkArchitectureContract: readFileSync('scripts/verify-watermark-architecture-contract.mjs', 'utf8'),
  vaultFieldContract: readFileSync('docs/双端版权记录字段一致性契约.md', 'utf8'),
  roadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
  commercialRoadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  enterprisePublicRightsApiKeyDesign: readFileSync('docs/Enterprise公开扫描API Key与额度账本模型草案.md', 'utf8'),
  qaChecklist: readFileSync('docs/双端能力一致性QA清单.md', 'utf8'),
  desktopWorkbench: readFileSync('src/views/WorkbenchView.vue', 'utf8'),
  desktopDropZone: readFileSync('src/components/DropZone.vue', 'utf8'),
  desktopBatch: readFileSync('src/views/LocalBatchView.vue', 'utf8'),
  desktopSettings: readFileSync('src/components/SettingsPanel.vue', 'utf8'),
  desktopVault: readFileSync('src/views/VaultView.vue', 'utf8'),
  desktopVerify: readFileSync('src/views/VerifyView.vue', 'utf8'),
  desktopApp: readFileSync('src/App.vue', 'utf8'),
  publicRightsSdk: readFileSync('src/lib/public-rights-sdk.ts', 'utf8'),
  desktopLegal: readFileSync('src/content/legal.ts', 'utf8'),
  desktopCloud: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  desktopStorage: readFileSync('src-tauri/src/sync/storage.rs', 'utf8'),
  desktopSchema: readFileSync('src-tauri/src/db/schema.rs', 'utf8'),
  desktopQueries: readFileSync('src-tauri/src/db/queries.rs', 'utf8'),
  desktopVaultCommand: readFileSync('src-tauri/src/commands/vault.rs', 'utf8'),
  desktopReport: readFileSync('src-tauri/src/commands/report.rs', 'utf8'),
  mobileHandoffRuntimeQa: readFileSync(
    'scripts/run-report-mobile-handoff-runtime-qa.mjs',
    'utf8',
  ),
  mobileHandoffRuntimeQaBin: readFileSync(
    'src-tauri/examples/report_mobile_handoff_runtime_qa.rs',
    'utf8',
  ),
  desktopPublicMetadata: readFileSync('src-tauri/src/commands/public_metadata.rs', 'utf8'),
  backend: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  desktopApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  desktopUserFacingErrors: readFileSync('src/lib/user-facing-errors.ts', 'utf8'),
  desktopTranscode: readFileSync('src-tauri/src/commands/transcode.rs', 'utf8'),
  desktopCopyrightCard: readFileSync('src/components/CopyrightCard.vue', 'utf8'),
  desktopSubscription: readFileSync('src/components/SubscriptionPanel.vue', 'utf8'),
  mobileWorkspace: readFileSync('mobile_app/lib/features/workspace/workspace_page.dart', 'utf8'),
  mobileAdaptiveEmbed: readFileSync('mobile_app/lib/features/workspace/adaptive_embed_page.dart', 'utf8'),
  mobileMediaKind: readFileSync('mobile_app/lib/features/workspace/media_file_kind.dart', 'utf8'),
  mobileWriteModels: readFileSync('mobile_app/lib/bridge/watermark_models.dart', 'utf8'),
  mobilePreviewBridge: readFileSync('mobile_app/lib/bridge/local_preview_watermark_bridge.dart', 'utf8'),
  mobileBridge: readFileSync('mobile_app/lib/bridge/watermark_bridge.dart', 'utf8'),
  mobileRustApi: readFileSync('mobile_app/rust/src/api.rs', 'utf8'),
  mobileImageEmbed: readFileSync('mobile_app/lib/features/workspace/image_embed_page.dart', 'utf8'),
  mobileAudioEmbed: readFileSync('mobile_app/lib/features/workspace/audio_embed_page.dart', 'utf8'),
  mobileProtectedCopyShare: readFileSync('mobile_app/lib/features/workspace/protected_copy_share.dart', 'utf8'),
  mobileRewritePreflight: readFileSync('mobile_app/lib/features/workspace/rewrite_preflight.dart', 'utf8'),
  mobileBatch: readFileSync('mobile_app/lib/features/workspace/local_batch_page.dart', 'utf8'),
  mobileSettings: readFileSync('mobile_app/lib/features/settings/settings_page.dart', 'utf8'),
  mobileShell: readFileSync('mobile_app/lib/app/mobile_shell.dart', 'utf8'),
  mobileVault: readFileSync('mobile_app/lib/features/vault/vault_page.dart', 'utf8'),
  mobileReportVerifier: readFileSync(
    'mobile_app/lib/features/vault/report_bundle_verifier.dart',
    'utf8',
  ),
  mobileReportHandoff: readFileSync(
    'mobile_app/lib/features/vault/report_handoff_bundle.dart',
    'utf8',
  ),
  mobileReportVerifierTest: readFileSync(
    'mobile_app/test/report_bundle_verifier_test.dart',
    'utf8',
  ),
  mobileReportAndroidTest: readFileSync(
    'mobile_app/integration_test/report_bundle_android_test.dart',
    'utf8',
  ),
  mobileVerify: readFileSync('mobile_app/lib/features/verify/verify_page.dart', 'utf8'),
  mobilePublicRightsScanner: readFileSync('mobile_app/lib/features/public_rights/public_rights_scanner.dart', 'utf8'),
  mobilePublicMetadataEmbedder: readFileSync(
    'mobile_app/lib/features/public_rights/public_metadata_embedder.dart',
    'utf8',
  ),
  mobilePublicMetadataEmbedderTest: readFileSync(
    'mobile_app/test/public_metadata_embedder_test.dart',
    'utf8',
  ),
  androidPublicMetadataEmbedQa: readFileSync(
    'scripts/verify-android-public-metadata-embed-runtime-qa.mjs',
    'utf8',
  ),
  androidPublicMetadataEmbedClickQa: readFileSync(
    'scripts/verify-android-public-metadata-embed-click-qa.mjs',
    'utf8',
  ),
  mobilePublicRightsScannerTest: readFileSync('mobile_app/test/public_rights_scanner_test.dart', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileCloudClient: readFileSync('mobile_app/lib/sync/cloud_account_client.dart', 'utf8'),
  mobileMain: readFileSync('mobile_app/lib/main.dart', 'utf8'),
  mobileTimeAttestation: readFileSync('mobile_app/lib/app/mobile_time_attestation.dart', 'utf8'),
  mobileAnonymousFeedback: readFileSync('mobile_app/lib/app/mobile_anonymous_feedback.dart', 'utf8'),
  mobileStorage: readFileSync('mobile_app/lib/storage/vault_store.dart', 'utf8'),
  mobileStorageFactory: readFileSync('mobile_app/lib/storage/vault_store_factory.dart', 'utf8'),
  mobileStorageFactoryWeb: readFileSync('mobile_app/lib/storage/vault_store_factory_web.dart', 'utf8'),
  mobileWebProfileStore: readFileSync('mobile_app/lib/storage/web_profile_vault_store.dart', 'utf8'),
  mobileSyncTransport: readFileSync('mobile_app/lib/sync/sync_transport.dart', 'utf8'),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  mobileWidgetTest: readFileSync('mobile_app/test/widget_test.dart', 'utf8'),
  mobileStateTest: readFileSync('mobile_app/test/mobile_app_state_test.dart', 'utf8'),
  mobileRewritePreflightTest: readFileSync('mobile_app/test/rewrite_preflight_test.dart', 'utf8'),
  mobileSyncTest: readFileSync('mobile_app/test/sync_transport_test.dart', 'utf8'),
};
const desktopEnterpriseAuditExists = existsSync('src/views/EnterpriseAuditView.vue');

const expectedVaultRecordSyncPayloadKeys = [
  'id',
  'kind',
  'title',
  'watermark_uid',
  'revision',
  'creator_display_name',
  'trusted_time_status',
  'trusted_time_source',
  'trusted_time_at',
  'third_party_verification_status',
  'third_party_verification_provider',
  'third_party_verification_path',
  'sha256',
  'parent_watermark_uid',
  'rewrite_reason',
  'extracted_timestamp',
  'extracted_device_id_hex',
  'extracted_file_hash_hex',
  'write_verification_status',
  'write_verification_message',
  'write_verification_at',
  'protected_copy_name',
  'protected_copy_hash',
  'payload_protocol_version',
  'payload_bytes_length',
  'media_payload_role',
  'watermark_id_issue_mode',
  'watermark_id_registry_status',
  'watermark_id_registry_receipt',
  'payload_auth_status',
  'output_strategy',
  'work_source_declaration',
  'training_permission_declaration',
  'creation_method_declaration',
  'human_edit_level_declaration',
  'authenticity_claim_declaration',
  'custom_rights_statement',
  'video_notary_id',
  'video_notary_at',
  'video_notary_receipt_signature',
  'video_notary_usage_ledger_id',
  'video_fingerprint_root',
  'video_bundle_sha256',
  'video_bundle_bytes',
  'video_bundle_scene_count',
  'video_bundle_elapsed_ms',
  'video_frame_sample_policy',
  'video_visual_task_id',
  'video_visual_completed_at',
  'video_visual_strategy_digest',
  'video_visual_self_check_confidence',
  'video_visual_self_check_threshold',
  'video_visual_checked_frames',
  'video_visual_media_hash',
  'video_visual_receipt_hash',
  'video_visual_output_bytes',
  'video_visual_output_content_type',
  'source',
  'sync_status',
  'created_at',
];

const vaultFieldChains = [
  ['版权编号', 'watermark_uid', 'watermark_uid', 'watermark_uid', 'watermarkUid', 'watermark_uid', '版权编号'],
  ['版本次数', 'revision', 'revision', 'revision', 'revision', 'revision', '版本次数'],
  ['上一版编号', 'parent_watermark_uid', 'parent_watermark_uid', 'parent_watermark_uid', 'parentWatermarkUid', 'parent_watermark_uid', '上一版编号'],
  ['更新说明', 'rewrite_reason', 'rewrite_reason', 'rewrite_reason', 'rewriteReason', 'rewrite_reason', '更新说明'],
  ['创作者身份', 'creator_display_name', 'creator_display_name', 'creator_display_name', 'creatorDisplayName', 'creator_display_name', '创作者身份'],
  ['作品指纹', 'sha256', 'original_hash', 'original_hash', 'sha256', 'sha256', '作品指纹'],
  ['保护副本名称', 'protected_copy_name', 'protected_copy_name', 'protected_copy_name', 'protectedCopyName', 'protected_copy_name', '保护副本名称'],
  ['保护副本摘要', 'protected_copy_hash', 'protected_copy_hash', 'protected_copy_hash', 'protectedCopyHash', 'protected_copy_hash', '保护副本摘要'],
  ['输出策略', 'output_strategy', 'output_strategy', 'output_strategy', 'outputStrategy', 'output_strategy', '输出策略'],
  ['写入后验证状态', 'write_verification_status', 'write_verification_status', 'write_verification_status', 'writeVerificationStatus', 'write_verification_status', '完成后验证'],
  ['写入后验证说明', 'write_verification_message', 'write_verification_message', 'write_verification_message', 'writeVerificationMessage', 'write_verification_message', '验证说明'],
  ['写入后验证时间', 'write_verification_at', 'write_verification_at', 'write_verification_at', 'writeVerificationAt', 'write_verification_at', '验证时间'],
  ['Payload 协议版本', 'payload_protocol_version', 'payload_protocol_version', 'payload_protocol_version', 'payloadProtocolVersion', 'payload_protocol_version', 'Payload 协议'],
  ['Payload 字节长度', 'payload_bytes_length', 'payload_bytes_length', 'payload_bytes_length', 'payloadBytesLength', 'payload_bytes_length', 'Payload 协议'],
  ['编号签发模式', 'watermark_id_issue_mode', 'watermark_id_issue_mode', 'watermark_id_issue_mode', 'watermarkIdIssueMode', 'watermark_id_issue_mode', '编号签发模式'],
  ['登记状态', 'watermark_id_registry_status', 'watermark_id_registry_status', 'watermark_id_registry_status', 'watermarkIdRegistryStatus', 'watermark_id_registry_status', '登记状态'],
  ['登记收据', 'watermark_id_registry_receipt', 'watermark_id_registry_receipt', 'watermark_id_registry_receipt', 'watermarkIdRegistryReceipt', 'watermark_id_registry_receipt', '登记收据'],
  ['Payload 认证状态', 'payload_auth_status', 'payload_auth_status', 'payload_auth_status', 'payloadAuthStatus', 'payload_auth_status', 'Payload 认证状态'],
  ['作品来源声明', 'work_source_declaration', 'work_source_declaration', 'work_source_declaration', 'workSourceDeclaration', 'work_source_declaration', '作品来源声明'],
  ['训练许可声明', 'training_permission_declaration', 'training_permission_declaration', 'training_permission_declaration', 'trainingPermissionDeclaration', 'training_permission_declaration', '训练许可声明'],
  ['创作方式声明', 'creation_method_declaration', 'creation_method_declaration', 'creation_method_declaration', 'creationMethodDeclaration', 'creation_method_declaration', '创作方式声明'],
  ['人工编辑声明', 'human_edit_level_declaration', 'human_edit_level_declaration', 'human_edit_level_declaration', 'humanEditLevelDeclaration', 'human_edit_level_declaration', '人工编辑声明'],
  ['真实性声明', 'authenticity_claim_declaration', 'authenticity_claim_declaration', 'authenticity_claim_declaration', 'authenticityClaimDeclaration', 'authenticity_claim_declaration', '真实性声明'],
  ['自定义版权声明', 'custom_rights_statement', 'custom_rights_statement', 'custom_rights_statement', 'customRightsStatement', 'custom_rights_statement', '自定义版权声明'],
  ['L2 存证编号', 'video_notary_id', 'video_notary_id', 'video_notary_id', 'videoNotaryId', 'video_notary_id', '存证编号'],
  ['L2 收据签名', 'video_notary_receipt_signature', 'video_notary_receipt_signature', 'video_notary_receipt_signature', 'videoNotaryReceiptSignature', 'video_notary_receipt_signature', '收据签名'],
  ['L2 指纹根', 'video_fingerprint_root', 'video_fingerprint_root', 'video_fingerprint_root', 'videoFingerprintRoot', 'video_fingerprint_root', '指纹根'],
  ['L2 指纹包摘要', 'video_bundle_sha256', 'video_bundle_sha256', 'video_bundle_sha256', 'videoBundleSha256', 'video_bundle_sha256', '指纹包摘要'],
  ['L2 抽帧策略', 'video_frame_sample_policy', 'video_frame_sample_policy', 'video_frame_sample_policy', 'videoFrameSamplePolicy', 'video_frame_sample_policy', '采样策略'],
  ['L3 任务编号', 'video_visual_task_id', 'video_visual_task_id', 'video_visual_task_id', 'videoVisualTaskId', 'video_visual_task_id', '任务编号'],
  ['L3 完成时间', 'video_visual_completed_at', 'video_visual_completed_at', 'video_visual_completed_at', 'videoVisualCompletedAt', 'video_visual_completed_at', '完成时间'],
  ['L3 策略摘要', 'video_visual_strategy_digest', 'video_visual_strategy_digest', 'video_visual_strategy_digest', 'videoVisualStrategyDigest', 'video_visual_strategy_digest', '策略摘要'],
  ['L3 自检置信度', 'video_visual_self_check_confidence', 'video_visual_self_check_confidence', 'video_visual_self_check_confidence', 'videoVisualSelfCheckConfidence', 'video_visual_self_check_confidence', '自检置信度'],
  ['L3 自检阈值', 'video_visual_self_check_threshold', 'video_visual_self_check_threshold', 'video_visual_self_check_threshold', 'videoVisualSelfCheckThreshold', 'video_visual_self_check_threshold', '自检阈值'],
  ['L3 检查帧数', 'video_visual_checked_frames', 'video_visual_checked_frames', 'video_visual_checked_frames', 'videoVisualCheckedFrames', 'video_visual_checked_frames', '检查帧数'],
  ['L3 成品媒体摘要', 'video_visual_media_hash', 'video_visual_media_hash', 'video_visual_media_hash', 'videoVisualMediaHash', 'video_visual_media_hash', '成品媒体摘要'],
  ['L3 Worker 收据摘要', 'video_visual_receipt_hash', 'video_visual_receipt_hash', 'video_visual_receipt_hash', 'videoVisualReceiptHash', 'video_visual_receipt_hash', 'Worker 收据摘要'],
  ['L3 成品字节数', 'video_visual_output_bytes', 'video_visual_output_bytes', 'video_visual_output_bytes', 'videoVisualOutputBytes', 'video_visual_output_bytes', '成品字节数'],
  ['L3 成品内容类型', 'video_visual_output_content_type', 'video_visual_output_content_type', 'video_visual_output_content_type', 'videoVisualOutputContentType', 'video_visual_output_content_type', '成品内容类型'],
];

assert(
  sources.packageJson.includes('"dual:contract"') &&
    sources.packageJson.includes('verify-dual-consistency-contract.mjs') &&
    sources.ci.includes('Run dual consistency contract') &&
    sources.ci.includes('npm run dual:contract'),
  'dual consistency contract must be exposed in package.json and run by CI',
);

assert(
  sources.capabilityBoundary.includes('## 2. 可对用户承诺') &&
    sources.capabilityBoundary.includes('## 3. 只能内部测试') &&
    sources.capabilityBoundary.includes('## 4. 明确不能承诺') &&
    sources.capabilityBoundary.includes('图片盲水印写入与验证') &&
    sources.capabilityBoundary.includes('音频盲水印写入与验证') &&
    sources.capabilityBoundary.includes('移动端保护副本出口') &&
    sources.capabilityBoundary.includes('L1 视频音轨水印') &&
    sources.capabilityBoundary.includes('L2 视频指纹存证') &&
    sources.capabilityBoundary.includes('L3 视频画面盲水印 release 候选') &&
    sources.capabilityBoundary.includes('watermark:l3-video-visual-release-gate') &&
    sources.capabilityBoundary.includes('POST /internal/video-tasks/:task_id/completion') &&
    sources.capabilityBoundary.includes('trusted completion 通过后才允许扣 `video_minutes`') &&
    sources.capabilityBoundary.includes('明确不能承诺') &&
    sources.agents.includes('Current capability boundary statements must follow `docs/当前真实能力边界说明.md`') &&
    sources.watermarkPlan.includes('docs/当前真实能力边界说明.md') &&
    sources.roadmap.includes('docs/当前真实能力边界说明.md') &&
    sources.commercialRoadmap.includes('docs/当前真实能力边界说明.md'),
  'dual-end capability claims must be governed by the current real capability boundary document',
);

assert(
  sources.packageJson.includes('"watermark:architecture-contract"') &&
    sources.ci.includes('Run watermark architecture contract') &&
    sources.ci.includes('npm run watermark:architecture-contract') &&
    sources.agents.includes('single source of truth for all current and future blind-watermark') &&
    sources.agents.includes('not to become a second algorithm source') &&
    sources.watermarkPlan.includes('禁止在 `watermark-core` 之外新增 DCT/DWT/SVD/QIM/LSB') &&
    sources.watermarkArchitectureContract.includes('Phase I-1 payload construction must be centralized in watermark-core builders') &&
    sources.watermarkArchitectureContract.includes('blind-watermark algorithm code must stay in watermark-core'),
  'dual consistency must preserve the all-blind-watermark algorithms live in watermark-core architecture gate',
);

assert(
  sources.roadmap.includes('| Phase H | 双端一致性合同与 QA | 已完成 |') &&
    sources.roadmap.includes('dual:contract') &&
    sources.roadmap.includes('docs/双端能力一致性QA清单.md') &&
    sources.roadmap.includes('双端一致性合同 OK'),
  'roadmap must record Phase H completion, QA checklist, and validation result',
);

assert(
  sources.qaChecklist.includes('图片写入与验证') &&
    sources.qaChecklist.includes('音频写入与验证') &&
    sources.qaChecklist.includes('版权库与报告') &&
    sources.qaChecklist.includes('云同步一致性') &&
    sources.qaChecklist.includes('L2 视频存证') &&
    sources.qaChecklist.includes('不上传原始媒体、加水印媒体或本地路径') &&
    sources.qaChecklist.includes('报告不是法律意见或司法鉴定'),
  'QA checklist must cover dual image/audio/vault/report/sync/L2/manual boundary checks',
);

assert(
    sources.backend.includes('/v1/public/rights/:watermark_uid/metadata') &&
    sources.backend.includes('PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS') &&
    sources.backendSchema.includes('PUBLIC_RIGHTS_STABLE_ERROR_CODES') &&
    sources.roadmap.includes('匿名批量查询最大 100 条') &&
    sources.roadmap.includes('API key、额度账本、调用审计和网关限流') &&
    sources.roadmap.includes('不新增桌面端或移动端入口') &&
    sources.commercialRoadmap.includes('不是 Studio / Enterprise 可售额度') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('POST /v1/enterprise/public-rights/batch') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('内部后台入口') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('quota balance 初始化') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('不上传原始媒体、保护副本、本地路径或可还原媒体内容') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayAuthContext') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayRateLimitPolicy') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayQuotaChargePlan') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayAuditContract') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayReadOnlyScanContract') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayDryRunRequest') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayDryRunDecision') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('dry_run_enterprise_gateway_readonly_scan') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('ENTERPRISE_GATEWAY_REQUIRED_STEPS') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('ENTERPRISE_GATEWAY_STABLE_ERROR_CODES') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('authenticate_api_key') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('authorize_scope') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('check_entitlement_api_access') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('apply_rate_limit') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('resolve_readonly_public_rights') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('record_quota_ledger') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('record_api_audit_event') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('enterprise_api_closed') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('quota_contract_missing') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('api_access_disabled') &&
    sources.backendStorage.includes('enterprise_public_rights_external_batch_charges_quota_and_audits') &&
    sources.backendStorage.includes('enterprise_quota_balance_initialization_is_idempotent_without_resetting_usage') &&
    sources.backendStorage.includes('enterprise_gateway_readonly_contract_freezes_auth_rate_limit_quota_and_audit') &&
    sources.backendStorage.includes('enterprise_gateway_dry_run_helper_outputs_auth_rate_limit_quota_and_audit_decisions') &&
    sources.backendStorage.includes('enterprise_gateway_dry_run_helper_denies_without_charging_or_legal_conclusion') &&
    sources.backend.includes('/internal/enterprise/quota-balances') &&
    sources.backend.includes('/internal/enterprise/api-keys/:api_key_id/pause') &&
    sources.backend.includes('/internal/enterprise/api-keys/:api_key_id/revoke') &&
    sources.backend.includes('/internal/enterprise/admin-audit-events') &&
    sources.backend.includes('record_enterprise_admin_operation') &&
    sources.backend.includes('list_enterprise_admin_audit_events_internal') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS enterprise_admin_audit_events') &&
    sources.backendStorage.includes('record_enterprise_admin_audit_event_internal') &&
    sources.backendStorage.includes('list_enterprise_admin_audit_events_internal') &&
    sources.backendStorage.includes('enterprise_admin_audit_events_can_be_filtered_read_only') &&
    sources.backend.includes('/v1/enterprise/public-rights/batch') &&
    sources.backend.includes('enterprise_public_rights_batch') &&
    !sources.desktopApp.includes('enterpriseAudit') &&
    !sources.desktopApp.includes('Enterprise 内部') &&
    !desktopEnterpriseAuditExists &&
    sources.desktopApi.includes('fetchPublicRightsBatch') &&
    sources.publicRightsSdk.includes('scanOne(watermarkUid') &&
    sources.publicRightsSdk.includes('scanBatch(watermarkUids') &&
    sources.publicRightsSdk.includes('resolvePolicy(scanResult') &&
    sources.publicRightsSdk.includes('formatUserMessage(result') &&
    sources.publicRightsSdk.includes('legalConclusion: false') &&
    sources.mobilePublicRightsScanner.includes('Future<PublicRightsSdkResult> scanOne') &&
    sources.mobilePublicRightsScanner.includes('resolvePublicRightsPolicy') &&
    sources.mobilePublicRightsScanner.includes('formatPublicRightsUserMessage') &&
    sources.mobilePublicRightsScanner.includes('canTreatAsTrainingAllowed: false') &&
    sources.mobilePublicRightsScannerTest.includes('resolvePolicy never treats registry declaration as legal conclusion') &&
    sources.desktopVault.includes('createPublicRightsScanner') &&
    sources.desktopVerify.includes('createPublicRightsScanner') &&
    sources.mobileVault.includes('PublicRightsScanner(') &&
    sources.mobileVault.includes('appState: widget.appState') &&
    sources.mobileVault.includes(').scanOne(widget.watermarkUid)') &&
    sources.mobileVerify.includes('PublicRightsScanner(appState: appState).scanOne') &&
    !sources.mobileVault.includes('String _publicRightsScanStatusLabel') &&
    !sources.mobileVerify.includes('String _publicRightsScanStatusLabel') &&
    sources.desktopApi.includes('fetchPublicRightsMetadata') &&
    sources.desktopVault.includes('exportSelectedPublicRightsMetadata') &&
    sources.desktopVault.includes('导出公开元数据 JSON') &&
    sources.desktopVault.includes('导出嵌入元数据图片副本') &&
    sources.desktopApi.includes('exportPublicRightsEmbeddedImage') &&
    sources.desktopPublicMetadata.includes('export_public_rights_embedded_image') &&
    sources.desktopPublicMetadata.includes('embed_png_xmp') &&
    sources.desktopPublicMetadata.includes('embed_jpeg_xmp') &&
    sources.mobileCloudClient.includes('getPublicRightsMetadata') &&
    sources.mobileState.includes('fetchPublicRightsMetadata') &&
    sources.mobilePublicRightsScanner.includes("publicRightsMetadataJsonExportLabel = '导出公开元数据 JSON'") &&
    sources.mobileVault.includes('publicRightsEmbeddedImageExportUnavailableMessage') &&
    sources.mobilePublicMetadataEmbedder.includes('publicRightsEmbeddedImageExportRequiresFileMessage') &&
    sources.mobilePublicMetadataEmbedder.includes('embedPublicRightsMetadataInImage') &&
    sources.mobilePublicMetadataEmbedder.includes('PublicMetadataImageFormat.png') &&
    sources.mobilePublicMetadataEmbedder.includes('PublicMetadataImageFormat.jpeg') &&
    sources.mobilePublicMetadataEmbedder.includes('checkEmbeddedPublicMetadataBytes') &&
    sources.mobilePublicMetadataEmbedderTest.includes('PNG embedding writes iTXt XMP') &&
    sources.mobilePublicMetadataEmbedderTest.includes('JPEG embedding writes APP1 XMP') &&
    sources.mobileVault.includes('FilePicker.pickFiles') &&
    sources.mobileVault.includes("allowedExtensions: const ['png', 'jpg', 'jpeg']") &&
    sources.mobileVault.includes('embedPublicRightsMetadataInImage') &&
    sources.mobileVault.includes('checkEmbeddedPublicMetadataBytes') &&
    sources.mobileVault.includes('publicRightsEmbeddedImageExportLabel') &&
    sources.androidPublicMetadataEmbedQa.includes('tool/public_metadata_embed_runtime_qa.dart') &&
    sources.androidPublicMetadataEmbedQa.includes('hasLegalConclusionFalse') &&
    sources.androidPublicMetadataEmbedClickQa.includes('tool/public_metadata_embed_click_qa.dart') &&
    sources.androidPublicMetadataEmbedClickQa.includes('uiautomator') &&
    sources.androidPublicMetadataEmbedClickQa.includes('导出嵌入元数据图片副本') &&
    sources.androidPublicMetadataEmbedClickQa.includes('byteContains') &&
    sources.mobileVault.includes('SharePlus.instance.share') &&
    sources.mobileVault.includes('fileNameOverrides: [fileName]') &&
    sources.mobileVault.includes('sharePositionOrigin: sharePositionOrigin') &&
    sources.capabilityBoundary.includes('sidecar 导出') &&
    sources.capabilityBoundary.includes('桌面端 PNG / JPEG 图片嵌入副本') &&
    sources.capabilityBoundary.includes('Android PNG / JPEG 嵌入器') &&
    sources.capabilityBoundary.includes('WAV 使用 RIFF `hsPM` chunk') &&
    sources.capabilityBoundary.includes('MP4 / M4A / MOV 使用 `uuid` box') &&
    sources.capabilityBoundary.includes('官方 `c2pa` Rust SDK 写入音视频容器级 signed manifest') &&
    sources.capabilityBoundary.includes('音频 / 视频嵌入 QA 已确认 WAV / MP4 C2PA active manifest') &&
    sources.capabilityBoundary.includes('所有输出固定 `legalConclusion=false`'),
  'public rights metadata export must wire sidecar on both ends, desktop/image C2PA embedding, Android file-byte-gated embedding, and AV C2PA propagation boundaries',
);

assert(
  sources.desktopWorkbench.includes("isImage ? '图片写入' : '音频写入'") &&
    sources.mobileWorkspace.includes("title: '作品写入'") &&
    sources.mobileWorkspace.includes('系统自动识别类型') &&
    sources.mobileAdaptiveEmbed.includes('mediaKindForFileName(file.name)') &&
    sources.mobileAdaptiveEmbed.includes('ImageEmbedPage(') &&
    sources.mobileAdaptiveEmbed.includes('AudioEmbedPage(') &&
    sources.mobileMediaKind.includes('supportedImageExtensions') &&
    sources.mobileMediaKind.includes('supportedAudioExtensions') &&
    sources.mobileVerify.includes('allowedExtensions: supportedMediaExtensions') &&
    sources.mobileVerify.includes('mediaKindForFileName(file.name)') &&
    !sources.mobileVerify.includes('SegmentedButton<WatermarkAssetKind>') &&
    sources.mobileShell.includes("_NavTab(label: '验证'") &&
    sources.mobileVerify.includes("title: '验证记录'") &&
    sources.desktopSettings.includes('正式报告') &&
    sources.mobileSettings.includes('正式报告') &&
    !sources.desktopSettings.includes('报告导出') &&
    !sources.mobileSettings.includes('报告导出') &&
    !sources.mobileWorkspace.includes("title: '图片保护'") &&
    !sources.mobileWorkspace.includes("title: '音频保护'"),
  'dual product language must keep desktop adaptive media handling and mobile unified 作品写入 / 验证 / 正式报告 consistently in current UI',
);

assert(
  sources.desktopWorkbench.includes('音频时长不足 30 秒') &&
    sources.desktopWorkbench.includes('无法确认音频时长') &&
    sources.desktopWorkbench.includes('未生成保护副本') &&
    sources.desktopBatch.includes('音频时长不足 30 秒，未生成保护副本') &&
    sources.desktopBatch.includes('无法确认音频时长，未生成保护副本'),
  'desktop must keep friendly 30s audio preflight and local batch failure copy',
);

assert(
  sources.mobileAudioEmbed.includes('当前音频短于 30 秒，暂不生成保护副本') &&
    sources.mobileAudioEmbed.includes('无法确认音频时长') &&
    sources.mobileBatch.includes('音频时长不足 30 秒，未生成保护副本') &&
    sources.mobileBatch.includes('无法确认音频时长，未生成保护副本') &&
    sources.mobileWidgetTest.includes('local batch shows friendly audio duration failures'),
  'mobile must keep friendly 30s audio preflight and local batch failure copy',
);

assert(
  sources.mobileWriteModels.includes('outputFileName') &&
    sources.mobileWriteModels.includes('outputLocationLabel') &&
    sources.mobileImageEmbed.includes('保护副本名称') &&
    sources.mobileImageEmbed.includes('复制存证摘要') &&
    sources.mobileImageEmbed.includes('buildCopyrightSummary') &&
    sources.mobileImageEmbed.includes('已生成保护副本，可通过系统分享面板保存到相册、文件或其他应用。') &&
    sources.mobileImageEmbed.includes('保存或分享保护副本') &&
    sources.mobileImageEmbed.includes('shareProtectedCopy') &&
    sources.mobileImageEmbed.includes("mimeType: 'image/png'") &&
    sources.mobileAudioEmbed.includes('保护副本名称') &&
    sources.mobileAudioEmbed.includes('复制存证摘要') &&
    sources.mobileAudioEmbed.includes('buildCopyrightSummary') &&
    sources.mobileAudioEmbed.includes('已生成保护副本，可通过系统分享面板保存到文件或其他应用。') &&
    sources.mobileAudioEmbed.includes('保存或分享保护副本') &&
    sources.mobileAudioEmbed.includes('shareProtectedCopy') &&
    sources.mobileAudioEmbed.includes("mimeType: 'audio/wav'") &&
    sources.mobileProtectedCopyShare.includes('SharePlus.instance.share') &&
    sources.mobileProtectedCopyShare.includes('XFile.fromData') &&
    sources.mobileProtectedCopyShare.includes('fileNameOverrides') &&
    sources.mobileProtectedCopyShare.includes('Uint8List.fromList(result.bytes)') &&
    sources.mobileProtectedCopyShare.includes('当前设备暂时无法打开系统分享面板'),
  'mobile write result cards must expose protected copy name, real system save/share entry, summary copy, and save/share wording without faking local paths',
);

assert(
    sources.mobileState.includes('buildCopyrightSummary') &&
    sources.mobileState.includes('VaultRecord record') &&
    sources.mobileVault.includes('appState.buildCopyrightSummary(record)') &&
    sources.mobilePreviewBridge.includes('_previewWatermarkUidFromSeed') &&
    sources.mobilePreviewBridge.includes("'PREVIEW-") &&
    !sources.mobilePreviewBridge.includes("'HS-") &&
    !sources.mobilePreviewBridge.includes('RegExp(r\'^HS-') &&
    !sources.mobilePreviewBridge.includes("'preview-$uidPrefix") &&
    !sources.mobileWriteModels.includes('preview-img-') &&
    !sources.mobileWriteModels.includes('preview-aud-'),
  'mobile copyright summary must be shared by write result and vault paths, public models must not encode legacy preview IDs, and Web preview must not mimic formal HS copyright IDs',
);

assert(
  sources.desktopApi.includes('creatorDisplayName: string | null') &&
    sources.desktopApi.includes('networkTime: string | null') &&
    sources.desktopApi.includes('tsaSource: string | null') &&
    sources.desktopCopyrightCard.includes('创作者显示名称') &&
    sources.desktopCopyrightCard.includes('第三方时间证明') &&
    sources.desktopCopyrightCard.includes('可信时间') &&
    sources.mobileState.includes('creator_display_name') &&
    sources.mobileState.includes('third_party_verification_status') &&
    sources.mobileState.includes('trusted_time_status') &&
    sources.mobileState.includes('requestTrustedTimeAttestation') &&
    sources.mobileTimeAttestation.includes('MobileTrustedTimeClient') &&
    sources.mobileTimeAttestation.includes('/v1/trusted-time') &&
    sources.backend.includes('/v1/trusted-time') &&
    sources.mobileTimeAttestation.includes('已记录网络授时') &&
    sources.mobileTimeAttestation.includes('HTTP Date 响应头') &&
    sources.mobileVault.includes('创作者身份') &&
    sources.mobileVault.includes('第三方验证 / 可信时间') &&
    sources.mobileVault.includes('可信时间'),
  'desktop and mobile vault records must expose creator identity plus real third-party verification / trusted-time capability fields',
);

assert(
  sources.desktopApi.includes('function formatLocalDateTime') &&
    sources.desktopApi.includes('function formatEvidenceTime') &&
    sources.desktopApi.includes('`可信时间: ${formatCopyrightDateTime(record.networkTime || record.createdAt)}`') &&
    sources.desktopApi.includes('`处理完成时间: ${formatCopyrightDateTime(record.createdAt)}`') &&
    sources.desktopApi.includes('`验证完成时间: ${formatCopyrightDateTime(record.writeVerificationAt)}`') &&
    sources.desktopCopyrightCard.includes('formatCopyrightDateTime(record.createdAt)') &&
    sources.desktopCopyrightCard.includes('formatCopyrightDateTime(record.networkTime)') &&
    sources.desktopVault.includes('formatCopyrightDateTime(selectedLineageRecord.networkTime)') &&
    sources.desktopVault.includes('formatCopyrightDateTime(selectedLineageRecord.createdAt)') &&
    sources.mobileState.includes('_summaryEvidenceValue(record.trustedTimeAt, record.trustedTimeStatus)') &&
    sources.mobileState.includes('_summaryLocalDateTime(record.createdAt)') &&
    sources.mobileState.includes("'处理时间: ${_summaryLocalDateTime(record.createdAt)}'") &&
    sources.mobileState.includes("'验证时间: ${_summaryLocalOptionalDate(record.writeVerificationAt)}'") &&
    sources.mobileState.includes("'- 导出时间: ${_summaryLocalDateTime(exportedAt)}'") &&
    sources.mobileState.includes("'- 入库时间: ${_summaryLocalDateTime(record.createdAt)}'") &&
    sources.mobileState.includes("'- 记录时间: ${_summaryEvidenceDateTime(record.trustedTimeAt, '未记录')}'") &&
    sources.roadmap.includes('用户可见摘要不再直接暴露 UTC ISO 字符串'),
  'desktop and mobile user-visible vault summaries and reports must render local-readable times while preserving raw trusted-time receipts',
);

assert(
  sources.mobileState.includes('const Set<String> vaultRecordSyncPayloadKeys') &&
    sources.mobileState.includes('sanitizeVaultRecordSyncPayload') &&
    sources.mobileState.includes('return sanitizeVaultRecordSyncPayload({') &&
    sources.mobileSyncTransport.includes('return sanitizeVaultRecordSyncPayload(decoded)') &&
    sources.desktopStorage.includes('VAULT_RECORD_SYNC_PAYLOAD_KEYS') &&
    sources.desktopStorage.includes('sanitize_mobile_sync_payload') &&
    sources.desktopStorage.includes('record_sync_event_sanitizes_local_paths_from_payload_json'),
  'desktop and mobile must keep vault record sync payload allowlists and storage sanitizers',
);

const desktopVaultRecordSyncPayloadKeys = extractRustStringArray(
  sources.desktopStorage,
  'VAULT_RECORD_SYNC_PAYLOAD_KEYS',
);
const mobileVaultRecordSyncPayloadKeys = extractDartStringSet(
  sources.mobileState,
  'vaultRecordSyncPayloadKeys',
);

assert(
  sameStringSet(desktopVaultRecordSyncPayloadKeys, expectedVaultRecordSyncPayloadKeys) &&
    sameStringSet(mobileVaultRecordSyncPayloadKeys, expectedVaultRecordSyncPayloadKeys) &&
    sameStringSet(desktopVaultRecordSyncPayloadKeys, mobileVaultRecordSyncPayloadKeys),
  'desktop and mobile vault record sync payload allowlists must stay identical; update docs/双端版权记录字段一致性契约.md and both receivers when adding a shared field',
);

assert(
  sources.vaultFieldContract.includes('桌面端字段 -> 移动端字段 -> 同步 payload -> 报告字段') &&
    sources.vaultFieldContract.includes('protectedCopyPath') &&
    sources.vaultFieldContract.includes('禁止进入同步 payload 和正式报告') &&
    sources.vaultFieldContract.includes('L3 视频画面盲水印收据元数据') &&
    sources.vaultFieldContract.includes('不包含对象存储引用、签名下载 URL、本地路径或媒体字节') &&
    sources.roadmap.includes('docs/双端版权记录字段一致性契约.md'),
  'dual vault field contract document must exist, define the field chain, exclude local paths, and be linked from the dual-end roadmap',
);

for (const [
  label,
  syncKey,
  desktopField,
  desktopDbColumn,
  mobileField,
  mobileDbColumn,
  reportLabel,
] of vaultFieldChains) {
  assert(
    sources.vaultFieldContract.includes(`\`${syncKey}\``) &&
      sources.vaultFieldContract.includes(`\`${desktopField}\``) &&
      sources.vaultFieldContract.includes(`\`${mobileField}\``) &&
      sources.vaultFieldContract.includes(reportLabel),
    `vault field contract document must map ${label} across desktop/mobile/sync/report`,
  );
  assert(
    sources.desktopVaultCommand.includes(`pub ${desktopField}:`) &&
      sources.desktopSchema.includes(desktopDbColumn) &&
      sources.desktopQueries.includes(desktopDbColumn) &&
      sources.desktopStorage.includes(`"${syncKey}"`) &&
      sources.desktopCloud.includes(`"${syncKey}".to_string()`) &&
      sources.desktopReport.includes(reportLabel),
    `desktop vault field chain must persist, sync, receive, and report ${label}`,
  );
  assert(
    sources.mobileState.includes(mobileField) &&
      sources.mobileStorage.includes(`'${mobileDbColumn}'`) &&
      sources.mobileState.includes(`'${syncKey}':`) &&
      sources.mobileSyncTransport.includes(`json['${syncKey}']`) &&
      sources.mobileState.includes(reportLabel),
    `mobile vault field chain must persist, sync, receive, and report ${label}`,
  );
}

assert(
  sources.desktopReport.includes('formal_report_includes_l3_video_visual_receipt_without_paths_or_urls') &&
    sources.desktopReport.includes('videoVisualWatermark') &&
    sources.desktopReport.includes('!json.contains("object://")') &&
    sources.desktopReport.includes('!json.contains("output-download")') &&
    sources.desktopReport.includes('!markdown.contains("object://")') &&
    sources.mobileState.includes('video_visual_watermark_receipt') &&
    sources.mobileState.includes('## L3 视频画面盲水印') &&
    !sources.mobileState.includes('video_visual_output_storage_ref') &&
    !sources.mobileState.includes('videoVisualOutputStorageRef'),
  'L3 vault/report fields must persist only receipt metadata and exclude object refs, signed URLs, local paths, or media bytes from reports and sync',
);

assert(
  sources.mobileStateTest.includes("payload.containsKey('output_ref'), isFalse") &&
    sources.mobileStateTest.includes("payload.containsKey('local_path'), isFalse") &&
    sources.mobileStateTest.includes("payload.containsKey('input_ref'), isFalse") &&
    sources.mobileStateTest.includes("payload.containsKey('protected_media_path'), isFalse") &&
    sources.mobileSyncTest.includes("payload.containsKey('output_ref'), isFalse") &&
    sources.mobileSyncTest.includes("payload.containsKey('local_path'), isFalse") &&
    sources.mobileSyncTest.includes("payload.containsKey('output_douyin'), isFalse") &&
    sources.desktopStorage.includes('payload.get("output_ref").is_none()') &&
    sources.desktopStorage.includes('payload.get("local_path").is_none()') &&
    sources.desktopStorage.includes('payload.get("input_ref").is_none()') &&
    sources.desktopCloud.includes('event.payload.get("output_douyin").is_none()') &&
    sources.desktopCloud.includes('event.payload.get("local_path").is_none()') &&
    sources.desktopCloud.includes('event.payload.get("bundle_path").is_none()'),
  'sync tests must prove original/protected media references and local paths stay out of cloud payloads',
);

assert(
  sources.mobileState.includes('MobileSyncResolutionType.duplicateIgnored') &&
    sources.mobileState.includes('MobileSyncResolutionType.pendingRegistryReconcile') &&
    sources.mobileState.includes('MobileSyncResolutionType.revisionUpgraded') &&
    sources.mobileState.includes('MobileSyncResolutionType.staleRevisionIgnored') &&
    sources.mobileStateTest.includes('does not overwrite same id local record while awaiting registry arbitration') &&
    sources.desktopStorage.includes('pending_registry_reconcile') &&
    sources.desktopVaultCommand.includes('repair_watermark_record_reissue') &&
    sources.desktopVault.includes('重新签发版权编号并修复保护副本') &&
    sources.mobileVault.includes('申请重新签发'),
  'desktop/mobile sync conflict resolution must keep duplicate, registry arbitration, upgraded, stale revision, and reissue repair outcomes',
);

assert(
  sources.mobileStateTest.includes('sign out keeps local vault records and sync queue for local use') &&
    sources.desktopSettings.includes('已退出云同步账户，本地版权库仍保留') &&
    sources.desktopLegal.includes('默认在本机完成图片、音频水印写入与验证'),
  'sign out must preserve local vault and queue behavior on both ends',
);

assert(
  sources.desktopSettings.includes('同一账户下同步版权库、验证记录、创作者身份和权益状态') &&
    sources.mobileSettings.includes('云同步') &&
    sources.mobileSettings.includes('创作者身份') &&
    sources.mobileSettings.includes('默认不同步原始媒体、加水印媒体和本地文件路径'),
  'settings must keep account sync, creator identity, and privacy boundary copy aligned',
);

assert(
  sources.desktopUserFacingErrors.includes('userFacingErrorMessage') &&
    sources.desktopUserFacingErrors.includes('暂时无法连接服务') &&
    sources.desktopUserFacingErrors.includes('登录状态已失效') &&
    sources.desktopUserFacingErrors.includes('[already_watermarked]') &&
    sources.desktopUserFacingErrors.includes('[missing_creator_identity]') &&
    sources.desktopUserFacingErrors.includes('[embed_failed]') &&
    sources.desktopUserFacingErrors.includes('watermark already exists in source media') &&
    sources.desktopWorkbench.includes('userFacingErrorMessage(err, "启动写入任务")') &&
    sources.desktopWorkbench.includes('handleSourceSelectError') &&
    sources.desktopDropZone.includes('error: [message: string]') &&
    sources.desktopDropZone.includes('emit("error"') &&
    sources.desktopDropZone.includes('file picker failed') &&
    sources.desktopWorkbench.includes('ensureRewritePreflightBeforeStart') &&
    sources.desktopWorkbench.includes('rewritePreflightBlocksStart') &&
    sources.desktopWorkbench.includes('canStartCurrentTask') &&
    sources.desktopWorkbench.includes('普通作品无需提前检查；正式写入时仍会阻止覆盖已有水印。') &&
    sources.desktopWorkbench.includes('这是已有作品的新版') &&
    sources.desktopWorkbench.includes('existingWatermarkBlockedMessage') &&
    sources.desktopWorkbench.includes('[already_watermarked]') &&
    sources.desktopWorkbench.includes('[missing_creator_identity]') &&
    sources.desktopWorkbench.includes('[embed_failed]') &&
    sources.desktopWorkbench.includes('v-if="false && isVideo" class="cloud-video-card"') &&
    sources.desktopWorkbench.includes('cloud-video-card__status">L1 视频音轨写入仍可直接使用，当前只是不开放 L2 提交。') &&
    sources.desktopWorkbench.includes('@click="emit(\'openSubscription\')"') &&
    sources.desktopTranscode.includes('pipeline_failure_stage') &&
    sources.desktopTranscode.includes('watermark_code() == Some("already_watermarked")') &&
    sources.desktopTranscode.includes('watermark_code() == Some("embed_failed")') &&
    sources.desktopWorkbench.includes('userFacingErrorMessage(err, "生成指纹包")') &&
    sources.desktopVault.includes('userFacingErrorMessage(error, "导出正式报告")') &&
    sources.desktopSettings.includes('userFacingErrorMessage(e, "继续使用 HiddenShield 账户")') &&
    sources.desktopSubscription.includes('userFacingErrorMessage(error, "创建支付会话")') &&
    sources.mobileState.includes('mobileUserFacingErrorMessage(error, action: \'登录\')') &&
    sources.mobileState.includes('暂时无法连接服务') &&
    sources.mobileImageEmbed.includes('_ensureRewritePreflightBeforeWrite') &&
    sources.mobileImageEmbed.includes('blocksInitialWrite') &&
    sources.mobileImageEmbed.includes('existingWatermarkRewriteBlockedMessage') &&
    sources.mobileImageEmbed.includes('mobileWatermarkWriteErrorMessage(error)') &&
    sources.mobileAudioEmbed.includes('_ensureRewritePreflightBeforeWrite') &&
    sources.mobileAudioEmbed.includes('blocksInitialWrite') &&
    sources.mobileAudioEmbed.includes('existingWatermarkRewriteBlockedMessage') &&
    sources.mobileAudioEmbed.includes('mobileWatermarkWriteErrorMessage(error)') &&
    sources.mobileRewritePreflight.includes('shouldBlockInitialWrite') &&
    sources.mobileRewritePreflight.includes("error.code == 'already_watermarked'") &&
    sources.mobileRewritePreflight.includes("error.code == 'missing_creator_identity'") &&
    sources.mobileRewritePreflight.includes("audio_decode_failed") &&
    sources.mobileRewritePreflight.includes('watermark already exists in source media') &&
    sources.mobileRustApi.includes('code: String') &&
    sources.mobileRustApi.includes('existing_uid: Option<String>') &&
    sources.mobileRustApi.includes('MobileWatermarkError::from_core') &&
    sources.mobileBridge.includes('supportsProductionWatermark') &&
    sources.mobilePreviewBridge.includes('bool get supportsProductionWatermark => false') &&
    sources.mobilePreviewBridge.includes('不生成可被桌面端验证的正式盲水印') &&
    sources.mobileImageEmbed.includes('!productionReady') &&
    sources.mobileAudioEmbed.includes('!productionReady') &&
    sources.mobileVerify.includes('!productionReady') &&
    sources.mobileRustApi.includes('mobile_image_output_is_desktop_core_extractable') &&
    sources.mobileRustApi.includes('mobile_audio_output_is_desktop_core_extractable') &&
    sources.mobileWriteModels.includes('isProductionWatermark') &&
    sources.mobilePreviewBridge.includes('isProductionWatermark: false') &&
    sources.mobileState.includes('Web 预览结果不能写入正式版权库或云同步队列') &&
    sources.mobileState.includes('Web 预览验证结果不能写入正式版权库或云同步队列') &&
    sources.mobileStateTest.includes('rejects web preview write results from formal vault and sync') &&
    sources.mobileStateTest.includes('rejects web preview read results from formal vault and sync') &&
    sources.mobilePreviewBridge.includes('watermark already exists in source media') &&
    sources.mobileRewritePreflightTest.includes('watermarked preview bytes are rejected before second write') &&
    sources.mobileSyncTransport.includes('无法连接云服务，请检查网络或系统配置中的云服务地址后重试。') &&
    !sources.mobileSyncTransport.includes("_shortBody('$error')") &&
    sources.roadmap.includes('技术性错误不再直接进入主提示') &&
    sources.roadmap.includes('已有水印写入前硬阻断'),
  'desktop and mobile user-visible errors and rewrite preflight must translate technical failures into product copy before write starts',
);

assert(
  sources.mobileMain.includes('openDefaultVaultStore()') &&
    !sources.mobileMain.includes('kIsWeb') &&
    !sources.mobileMain.includes('MemoryVaultStore()') &&
    sources.mobileStorageFactory.includes("if (dart.library.html) 'vault_store_factory_web.dart'") &&
    sources.mobileStorageFactoryWeb.includes('WebProfileVaultStore.open()') &&
    sources.mobileWebProfileStore.includes('web.window.localStorage') &&
    sources.mobileWebProfileStore.includes('hiddenshield.mobile.sync_profile.v1') &&
    sources.mobileWebProfileStore.includes('onboardingCompleted') &&
    sources.mobileWebProfileStore.includes('creatorDisplayName') &&
    sources.mobileWebProfileStore.includes('authToken') &&
    sources.mobileWebProfileStore.includes('anonymousFeedbackQueueJson') &&
    sources.roadmap.includes('Web 预览不再使用纯内存库保存首登资料'),
  'mobile web preview must persist sync profile instead of recreating MemoryVaultStore on every restart',
);

assert(
  sources.desktopSettings.includes('匿名反馈') &&
    sources.desktopSettings.includes('体验改进') &&
    sources.desktopSettings.includes('占用') &&
    sources.desktopSettings.includes('问题反馈') &&
    sources.desktopSettings.includes('导出日志') &&
    sources.mobileSettings.includes("title: '匿名反馈'") &&
    sources.mobileSettings.includes("title: '体验改进'") &&
    sources.mobileSettings.includes("title: '占用'") &&
    sources.mobileSettings.includes("title: '问题反馈'") &&
    sources.mobileSettings.includes("label: const Text('导出日志')") &&
    sources.mobileState.includes('flushAnonymousFeedbackQueue') &&
    sources.mobileState.includes('exportSafeDiagnosticLog') &&
    sources.mobileState.includes('MobileDataUsageSnapshot') &&
    sources.mobileAnonymousFeedback.includes('/v1/anonymous-feedback/batches') &&
    sources.mobileSettings.includes('不包含媒体文件、本地路径、文件名或完整作品指纹') &&
    sources.mobileStorage.includes('anonymous_feedback_queue_json'),
  'mobile settings must provide real anonymous feedback, experience improvement, usage, support feedback, and safe log export parity',
);

assert(
  sources.desktopDropZone.includes('当前发布版本仅开放图片和音频，视频能力已暂停。') &&
    sources.desktopWorkbench.includes('视频能力已暂停') &&
    sources.desktopWorkbench.includes('v-if="false && isVideo" class="video-track-card"') &&
    sources.desktopWorkbench.includes('v-if="false && isVideo" class="cloud-video-card"') &&
    !sources.desktopSettings.includes('当前是视频指纹存证，不是视频画面盲水印') &&
    sources.roadmap.includes('移动端继续冻结') &&
    sources.roadmap.includes('桌面端全部视频入口必须隐藏或屏蔽') &&
    sources.mobileState.includes('mobile_video_fingerprint_notary') &&
    sources.mobileState.includes('metadata_hash_only_no_raw_video_no_local_path') &&
    !sources.mobileState.includes('originalVideoPath') &&
    !sources.mobileState.includes('bundlePath') &&
    sources.commercialRoadmap.includes('屏蔽桌面端全部视频能力入口和用户可见承诺'),
  'current release contract must hide desktop video while retaining frozen metadata-only compatibility without making mobile a release dependency',
);

assert(
  !sources.desktopSettings.includes('临时直连') &&
    !sources.desktopSettings.includes('配对码') &&
    !sources.mobileSettings.includes('临时直连') &&
    !sources.mobileSettings.includes('调试配对码') &&
    sources.mobileWidgetTest.includes("find.text('临时直连'), findsNothing") &&
    sources.mobileWidgetTest.includes("find.text('桥接层已接入'), findsNothing"),
  'product UI must not expose bridge, pairing, or temporary direct connection language',
);

assert(
  sources.desktopVault.includes('导出正式报告') &&
    sources.mobileVault.includes('buildFormalReportDraft') &&
    sources.mobileVault.includes('Clipboard.setData') &&
    sources.mobileVault.includes('复制存证摘要') &&
    sources.mobileVault.includes('写入后验证信息') &&
    sources.mobileState.includes("WriteVerificationStatus.verified => '已通过'") &&
    sources.mobileVault.includes('桌面签发交接包已打开分享面板') &&
    sources.desktopLegal.includes('不构成法律意见') &&
    sources.mobileSettings.includes('报告是技术辅助材料') &&
    sources.mobileState.includes("VaultRecordSource.verify => '验证'"),
  'formal report entry and legal boundary copy must stay aligned across desktop and mobile',
);

assert(
  sources.desktopReport.includes('sha256_chain_v1') &&
    sources.mobileReportVerifier.includes('sha256_chain_v1') &&
    sources.mobileReportVerifier.includes('if (schemaVersion != 2)') &&
    sources.mobileReportVerifier.includes(
      'Manifest v2 只允许 report.pdf 和 report.json 两个受校验文件',
    ) &&
    sources.mobileReportVerifier.includes('documentContractStatus') &&
    sources.mobileReportVerifier.includes("'not_signed'") &&
    sources.mobileReportVerifier.includes("'not_timestamped'") &&
    !sources.mobileReportVerifier.includes('watermark') &&
    sources.mobileVault.includes('校验桌面报告包') &&
    sources.mobileReportVerifierTest.includes("'image'") &&
    sources.mobileReportVerifierTest.includes("'audio'") &&
    sources.mobileReportVerifierTest.includes("'l2-video'") &&
    sources.mobileReportAndroidTest.includes(
      'Android verifies desktop image audio and L2 report bundles',
    ),
  'desktop and mobile must share Manifest schema v2 chain verification without turning bundle integrity into watermark or signature trust',
);

assert(
  sources.mobileReportHandoff.includes('formal_report_handoff') &&
    sources.mobileReportHandoff.includes('awaiting_desktop_render') &&
    sources.mobileReportHandoff.includes('not_signed') &&
    sources.mobileReportHandoff.includes('packageTimestampPresent') &&
    sources.mobileVault.includes('生成桌面签发交接包') &&
    sources.desktopVerify.includes('跨端报告包校验') &&
    sources.desktopVerify.includes('生成最终 PDF') &&
    sources.desktopReport.includes('import_mobile_report_handoff') &&
    sources.desktopReport.includes('source_handoff_root_digest') &&
    sources.desktopReport.includes(
      'desktop_verifies_mobile_generated_report_handoff_fixture',
    ),
  'mobile-to-desktop report handoff must preserve Manifest v2 facts through final desktop rendering while keeping signature and trusted-time boundaries explicit',
);
assert(
  sources.packageJson.includes('"report:mobile-handoff-runtime-qa"') &&
    sources.mobileHandoffRuntimeQa.includes('import_mobile_report_handoff') &&
    sources.mobileHandoffRuntimeQa.includes('sourceHandoffRootDigest') &&
    sources.mobileHandoffRuntimeQa.includes('sha256_chain_v1') &&
    sources.mobileHandoffRuntimeQa.includes('report.pdf') &&
    sources.mobileHandoffRuntimeQa.includes('report.json') &&
    sources.mobileHandoffRuntimeQa.includes('manifest.json') &&
    sources.mobileHandoffRuntimeQaBin.includes('tauri::test::mock_app()') &&
    sources.mobileHandoffRuntimeQaBin.includes('run_mobile_report_handoff_runtime_qa'),
  'dual report handoff must keep a Tauri runtime QA over the mobile fixture and final desktop three-file bundle',
);

console.log('Dual consistency contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Dual consistency contract failed: ${message}`);
    process.exit(1);
  }
}

function extractRustStringArray(source, constName) {
  const pattern = new RegExp(`const\\s+${constName}:[\\s\\S]*?=\\s*&\\[([\\s\\S]*?)\\];`);
  const match = source.match(pattern);
  if (!match) {
    console.error(`Dual consistency contract failed: missing Rust array ${constName}`);
    process.exit(1);
  }
  return [...match[1].matchAll(/"([^"]+)"/g)].map((item) => item[1]);
}

function extractDartStringSet(source, constName) {
  const pattern = new RegExp(`const\\s+Set<String>\\s+${constName}\\s*=\\s*\\{([\\s\\S]*?)\\};`);
  const match = source.match(pattern);
  if (!match) {
    console.error(`Dual consistency contract failed: missing Dart set ${constName}`);
    process.exit(1);
  }
  return [...match[1].matchAll(/'([^']+)'/g)].map((item) => item[1]);
}

function sameStringSet(left, right) {
  const normalize = (values) => [...new Set(values)].sort();
  return JSON.stringify(normalize(left)) === JSON.stringify(normalize(right));
}
