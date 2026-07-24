import { readFileSync } from 'node:fs';

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function includesAll(source, values, label) {
  for (const value of values) {
    assert(source.includes(value), `${label} must include ${value}`);
  }
}

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  featureGateDoc: readFileSync('docs/V3 feature gate写入与回滚验证方案.md', 'utf8'),
  migrationContract: readFileSync('docs/V3跨端fixture与迁移桥接报告字段冻结合同.md', 'utf8'),
  protocolDoc: readFileSync('docs/公开权利信号与训练许可扫描协议设计.md', 'utf8'),
  vaultFieldContract: readFileSync('docs/双端版权记录字段一致性契约.md', 'utf8'),
  watermarkPlan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  payload: readFileSync('watermark-core/src/payload.rs', 'utf8'),
  coreLib: readFileSync('watermark-core/src/lib.rs', 'utf8'),
  coreV3InternalQa: readFileSync('watermark-core/src/v3_internal_qa.rs', 'utf8'),
  coreV3FeatureGateRollbackQaBin: readFileSync(
    'watermark-core/src/bin/v3_feature_gate_rollback_qa.rs',
    'utf8',
  ),
  coreV3ReadonlyFixture: readFileSync('watermark-core/src/v3_readonly_fixture.rs', 'utf8'),
  service: readFileSync('watermark-core/src/service.rs', 'utf8'),
  desktopCargo: readFileSync('src-tauri/Cargo.toml', 'utf8'),
  desktopVerify: readFileSync('src-tauri/src/commands/verify.rs', 'utf8'),
  desktopReport: readFileSync('src-tauri/src/commands/report.rs', 'utf8'),
  desktopCloud: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  desktopStorage: readFileSync('src-tauri/src/sync/storage.rs', 'utf8'),
  desktopLib: readFileSync('src-tauri/src/lib.rs', 'utf8'),
  desktopApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  desktopV3ReadonlyFixture: readFileSync('src-tauri/src/commands/v3_readonly_fixture.rs', 'utf8'),
  desktopV3ReadonlyQaBin: readFileSync('src-tauri/examples/v3_readonly_fixture_qa.rs', 'utf8'),
  desktopV3ReadonlyCandidateQaBin: readFileSync(
    'src-tauri/examples/v3_readonly_candidate_runtime_qa.rs',
    'utf8',
  ),
  desktopV3InternalQaWriteQaBin: readFileSync(
    'src-tauri/examples/v3_internal_qa_write_runtime_qa.rs',
    'utf8',
  ),
  desktopScheduler: readFileSync('src-tauri/src/pipeline/scheduler.rs', 'utf8'),
  mobileRustApi: readFileSync('mobile_app/rust/src/api.rs', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileDartBridge: readFileSync('mobile_app/lib/bridge/rust_watermark_bridge.dart', 'utf8'),
  mobileBridgeContract: readFileSync('mobile_app/lib/bridge/watermark_bridge.dart', 'utf8'),
  mobileGeneratedApi: readFileSync('mobile_app/lib/src/rust/api.dart', 'utf8'),
  mobileV3ReadonlyCandidateQaTool: readFileSync(
    'mobile_app/tool/v3_readonly_candidate_runtime_qa.dart',
    'utf8',
  ),
  mobileV3InternalQaWriteQaTool: readFileSync(
    'mobile_app/tool/v3_internal_qa_write_runtime_qa.dart',
    'utf8',
  ),
  v3ReadonlyQa: readFileSync('scripts/verify-v3-readonly-fixture-qa.mjs', 'utf8'),
  v3ReadonlyCandidateRuntimeQa: readFileSync(
    'scripts/verify-v3-readonly-candidate-runtime-qa.mjs',
    'utf8',
  ),
  v3FeatureGateRollbackContract: readFileSync(
    'scripts/verify-v3-feature-gate-rollback-contract.mjs',
    'utf8',
  ),
  v3InternalQaWriteRuntimeQa: readFileSync(
    'scripts/verify-v3-internal-qa-write-runtime-qa.mjs',
    'utf8',
  ),
  v3ReportSyncQa: readFileSync('scripts/verify-dual-vault-field-runtime-qa.mjs', 'utf8'),
  image: readFileSync('watermark-core/src/image.rs', 'utf8'),
  audio: readFileSync('watermark-core/src/audio.rs', 'utf8'),
  videoVisual: readFileSync('watermark-core/src/video_visual.rs', 'utf8'),
};

assert(
  sources.packageJson.includes('"rights:v3-migration-contract"') &&
    sources.packageJson.includes('verify-v3-migration-contract.mjs') &&
    sources.packageJson.includes('"rights:v3-readonly-fixture-qa"') &&
    sources.packageJson.includes('verify-v3-readonly-fixture-qa.mjs') &&
    sources.packageJson.includes('"rights:v3-readonly-candidate-runtime-qa"') &&
    sources.packageJson.includes('verify-v3-readonly-candidate-runtime-qa.mjs') &&
    sources.packageJson.includes('"rights:v3-report-sync-migration-qa"') &&
    sources.packageJson.includes('"rights:v3-feature-gate-rollback-contract"') &&
    sources.packageJson.includes('verify-v3-feature-gate-rollback-contract.mjs') &&
    sources.packageJson.includes('"rights:v3-internal-qa-write-runtime-qa"') &&
    sources.packageJson.includes('verify-v3-internal-qa-write-runtime-qa.mjs'),
  'package.json must expose rights:v3-migration-contract',
);

includesAll(
  sources.migrationContract,
  [
    'v3_image_desktop_write_mobile_read',
    'v3_image_mobile_write_desktop_read',
    'v3_audio_desktop_write_mobile_read',
    'v3_audio_mobile_write_desktop_read',
    'v2_legacy_read_bridge_image',
    'v2_legacy_read_bridge_audio',
    'registry_overrides_v3_payload',
    'registry_conflict_marks_conflict',
    'v3_feature_gate_rollback',
  ],
  'V3 migration fixture matrix',
);

includesAll(
  sources.migrationContract,
  [
    '`watermark_uid`',
    '`payload_protocol_version`',
    '`payload_bytes_length`',
    '`payload_auth_status`',
    '`revision`',
    '`parent_watermark_uid`',
    '`watermark_id_issue_mode`',
    '`watermark_id_registry_status`',
    '`watermark_id_registry_receipt`',
    '`training_permission_declaration`',
  ],
  'V3 migration sync field contract',
);

includesAll(
  sources.migrationContract,
  [
    '版权库详情',
    '验证页',
    '正式报告 / 摘要',
    '公开权利卡',
    'anchorProtocol=v2_migration_anchor',
    'anchorProtocol=v3_minimal_anchor',
    'Payload 认证状态',
    'legalConclusion=false',
  ],
  'V3 migration report and UI display contract',
);

includesAll(
  sources.migrationContract,
  [
    '当前 `PAYLOAD_BYTES = 119` 不得被直接修改',
    'V3 媒体内只保留可验证锚点',
    '禁止把 `auth_tag` 原值',
    '禁止把 V3 媒体 payload 当作完整授权来源同步',
    'Android 或 Web QA 不能替代 iOS QA',
    'R0 codec 准备',
    'R1 只读解析',
    'R2 feature gate 写入',
    'R3 运行态 QA',
    'R4 默认切换',
  ],
  'V3 migration forbidden behaviors and rollback gates',
);

includesAll(
  [sources.featureGateDoc, sources.v3FeatureGateRollbackContract].join('\n'),
  [
    'V3 默认写入已开启',
    'internal_qa',
    'force_v2_rollback',
    'payloadProtocolVersion=2',
    'payloadBytesLength=119',
    'rights:v3-feature-gate-rollback-contract',
    'defaultV3WriteEnabled: true',
    'v3InternalQaWriteImplemented: true',
    'off -> internal_qa -> force_v2_rollback',
    'default WatermarkService must route V3/39',
  ],
  'V3 feature gate rollback design and contract',
);

includesAll(
  [sources.coreLib, sources.coreV3InternalQa, sources.coreV3FeatureGateRollbackQaBin].join('\n'),
  [
    'embed_v3_internal_qa_media',
    'V3InternalQaWriteGate',
    'V3InternalQaWriteGate::InternalQa',
    'V3InternalQaWriteGate::ForceV2Rollback',
    'v3_internal_qa_write_gate_off',
    'v3_internal_qa_force_v2_rollback',
    '"internal_qa"',
    '"force_v2_rollback"',
    '"v2_full_record"',
    '"v3_minimal_anchor"',
  ],
  'V3 internal QA write API and rollback QA bin',
);

includesAll(
  [
    sources.desktopCargo,
    sources.desktopV3InternalQaWriteQaBin,
    sources.mobileRustApi,
    sources.mobileGeneratedApi,
    sources.mobileV3InternalQaWriteQaTool,
    sources.v3InternalQaWriteRuntimeQa,
  ].join('\n'),
  [
    'v3_internal_qa_write_runtime_qa',
    'embed_v3_internal_qa_media',
    'embed_v3_internal_qa_for_mobile',
    'embedV3InternalQaForMobile',
    'default_write',
    'v3_minimal_anchor_verified',
    'defaultMobileWriteV3Enabled',
    'rights:v3-internal-qa-write-runtime-qa',
  ],
  'V3 internal QA write desktop and Android runtime QA',
);

assert(
  !sources.desktopScheduler.includes('embed_v3_internal_qa_media') &&
    !sources.mobileDartBridge.includes('embedV3InternalQaForMobile'),
  'formal desktop scheduler and mobile default bridge.write must not call internal QA V3 writing',
);

assert(
  sources.payload.includes('pub const PAYLOAD_BYTES: usize = 119;') &&
    sources.payload.includes('PAYLOAD_V3_MINIMAL_ANCHOR_BYTES') &&
    sources.payload.includes('WatermarkDecodedPayload') &&
    sources.payload.includes('decode_watermark_payload_readonly') &&
    sources.payload.includes('payload_bytes_length(&self) -> usize') &&
    sources.payload.includes("payload_auth_status(&self) -> &'static str") &&
    sources.payload.includes('readonly_decoder_accepts_v2_payload') &&
    sources.payload.includes('readonly_decoder_accepts_v3_minimal_anchor') &&
    sources.payload.includes('readonly_decoder_rejects_unknown_length'),
  'watermark-core payload layer must expose a read-only V2/V3 decoder without changing V2/119',
);

assert(
  sources.coreLib.includes('WatermarkDecodedPayload') &&
    sources.coreLib.includes('decode_watermark_payload_readonly') &&
    sources.coreLib.includes('embed_v3_readonly_anchor_png_bytes') &&
    sources.coreLib.includes('extract_v3_readonly_anchor_wav_bytes'),
  'watermark-core lib exports must expose the read-only V2/V3 payload decoder',
);

assert(
  sources.coreV3ReadonlyFixture.includes('embed_v3_readonly_anchor_png_bytes') &&
    sources.coreV3ReadonlyFixture.includes('extract_v3_readonly_anchor_png_bytes') &&
    sources.coreV3ReadonlyFixture.includes('embed_v3_readonly_anchor_wav_bytes') &&
    sources.coreV3ReadonlyFixture.includes('extract_v3_readonly_anchor_wav_bytes') &&
    sources.coreV3ReadonlyFixture.includes('v3_readonly_png_container_fixture_roundtrips_anchor') &&
    sources.coreV3ReadonlyFixture.includes('v3_readonly_wav_container_fixture_roundtrips_anchor') &&
    sources.coreV3ReadonlyFixture.includes('tEXt') &&
    sources.coreV3ReadonlyFixture.includes('hsV3'),
  'watermark-core must expose staged PNG/WAV V3 readonly fixture container helpers',
);

assert(
  sources.image.includes('IMAGE_SYNC_V3_READONLY_PACKET_BYTES') &&
    sources.image.includes('encode_image_sync_packet_v3_readonly') &&
    sources.image.includes('decode_image_sync_packet_v3_readonly_bytes') &&
    sources.image.includes('extract_image_watermark_readonly_candidate_bytes') &&
    sources.image.includes('extract_image_sync_packet_readonly_candidate_from_prepared') &&
    sources.image.includes(
      'image_sync_packet_v3_readonly_roundtrips_minimal_anchor_without_v2_decode',
    ) &&
    sources.image.includes(
      'image_readonly_candidate_extracts_v3_sync_packet_for_migration_bridge',
    ) &&
    sources.audio.includes('AUDIO_RECOVERY_V3_READONLY_PACKET_BYTES') &&
    sources.audio.includes('encode_audio_recovery_packet_v3_readonly') &&
    sources.audio.includes('decode_audio_recovery_packet_v3_readonly') &&
    sources.audio.includes('extract_watermark_wav_readonly_candidate_bytes') &&
    sources.audio.includes('extract_watermark_samples_recovery_readonly_candidate') &&
    sources.audio.includes(
      'audio_recovery_packet_v3_readonly_roundtrips_minimal_anchor_without_v2_decode',
    ) &&
    sources.audio.includes(
      'audio_readonly_candidate_extracts_v3_recovery_packet_for_migration_bridge',
    ),
  'formal image sync packet and audio recovery packet modules must have V3 readonly candidate readers and fixtures',
);

assert(
  sources.coreLib.includes('extract_image_watermark_readonly_candidate_bytes') &&
    sources.coreLib.includes('extract_watermark_wav_readonly_candidate_bytes') &&
    sources.coreLib.includes('extract_watermark_samples_readonly_candidate') &&
    sources.coreLib.includes('build_v3_readonly_candidate_image_fixture_png_bytes') &&
    sources.coreLib.includes('build_v3_readonly_candidate_audio_fixture_wav_bytes'),
  'watermark-core must export explicit V3 readonly candidate readers while default service remains V3-only',
);

assert(
  sources.desktopVerify.includes('verify_suspect_readonly_candidate') &&
    sources.desktopVerify.includes('WatermarkService::extract') &&
    sources.desktopVerify.includes('MediaInput::ImageBytes') &&
    sources.desktopVerify.includes('extract_watermark_wav_readonly_candidate_bytes') &&
    sources.desktopVerify.includes('media_payload_role') &&
    sources.desktopVerify.includes('v3_minimal_anchor') &&
    sources.desktopVerify.includes('v2_full_record') &&
    sources.desktopLib.includes('commands::verify::verify_suspect_readonly_candidate') &&
    sources.desktopApi.includes('verifySuspectReadonlyCandidate') &&
    sources.desktopApi.includes('mediaPayloadRole'),
  'desktop must expose a controlled readonly candidate verification entry and report bridge fields',
);

assert(
  sources.mobileRustApi.includes('extract_image_readonly_candidate_for_mobile') &&
    sources.mobileRustApi.includes('extract_audio_wav_readonly_candidate_for_mobile') &&
    sources.mobileRustApi.includes('mobile_readonly_candidate_reads_default_v3_image_report_bridge_fields') &&
    sources.mobileRustApi.includes('mobile_readonly_candidate_reads_default_v3_audio_report_bridge_fields') &&
    sources.mobileBridgeContract.includes('readReadonlyCandidate') &&
    sources.mobileDartBridge.includes('extractImageReadonlyCandidateForMobile') &&
    sources.mobileDartBridge.includes('extractAudioWavReadonlyCandidateForMobile') &&
    sources.mobileGeneratedApi.includes('extractImageReadonlyCandidateForMobile') &&
    sources.mobileGeneratedApi.includes('extractAudioWavReadonlyCandidateForMobile'),
  'Android native bridge must expose a controlled readonly candidate verification entry while default read is V3',
);

assert(
  sources.desktopCargo.includes('v3_readonly_fixture_qa') &&
    sources.desktopV3ReadonlyFixture.includes('decode_v3_readonly_fixture_for_desktop') &&
    sources.desktopV3ReadonlyFixture.includes('decode_v3_readonly_media_fixture_for_desktop') &&
    sources.desktopV3ReadonlyFixture.includes('build_v3_readonly_fixture_bytes') &&
    sources.desktopV3ReadonlyFixture.includes('build_v3_readonly_fixture_media_bytes') &&
    sources.desktopV3ReadonlyFixture.includes('payload_protocol_version: anchor.protocol_version as u32') &&
    sources.desktopV3ReadonlyFixture.includes(
      'payload_bytes_length: PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32',
    ) &&
    sources.desktopV3ReadonlyFixture.includes('payload_auth_status: "verified".to_string()') &&
    sources.desktopV3ReadonlyQaBin.includes('defaultV3WriteEnabled') &&
    sources.desktopV3ReadonlyQaBin.includes('true'),
  'desktop bridge must have controlled V3 readonly fixture generation and default V3 writes enabled',
);

assert(
    sources.mobileRustApi.includes('decode_v3_readonly_fixture_for_mobile') &&
    sources.mobileRustApi.includes('decode_v3_readonly_media_fixture_for_mobile') &&
    sources.mobileRustApi.includes('WatermarkDecodedPayload::V3MinimalAnchor') &&
    sources.mobileRustApi.includes('payload_protocol_version: anchor.protocol_version as u32') &&
    sources.mobileRustApi.includes(
      'payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32',
    ) &&
    sources.mobileRustApi.includes('payload_auth_status: "verified".to_string()') &&
    sources.mobileRustApi.includes('mobile_v3_readonly_fixture_preserves_anchor_fields') &&
    sources.mobileRustApi.includes('mobile_v3_readonly_media_fixture_preserves_anchor_fields'),
  'mobile bridge must preserve V3 readonly fixture payload fields',
);

assert(
  sources.desktopReport.includes('media_payload_role: String') &&
    sources.desktopReport.includes('media_payload_role_for_protocol') &&
    sources.desktopReport.includes('formal_report_marks_v2_and_v3_payload_roles') &&
    sources.desktopReport.includes('媒体载荷角色: {}') &&
    sources.desktopCloud.includes('"media_payload_role".to_string()') &&
    sources.desktopCloud.includes('vault_record_to_cloud_event_marks_v3_minimal_anchor_role') &&
    sources.desktopStorage.includes('"media_payload_role"') &&
    sources.mobileState.includes("'media_payload_role'") &&
    sources.mobileState.includes('_mediaPayloadRoleForProtocol') &&
    sources.mobileState.includes('媒体载荷角色: ${_mediaPayloadRoleLabel') &&
    sources.v3ReportSyncQa.includes("'media_payload_role'") &&
    sources.v3ReportSyncQa.includes('mediaPayloadRoleForProtocol') &&
    sources.v3ReportSyncQa.includes('payloadProtocolVersion: 3') &&
    sources.v3ReportSyncQa.includes('payloadBytesLength: 39') &&
    sources.v3ReportSyncQa.includes('媒体载荷角色'),
  'formal reports and sync backfill QA must carry V2/V3 media payload role bridge fields without enabling V3 writes',
);

includesAll(
  sources.v3ReadonlyQa,
  [
    'payloadProtocolVersion === 3',
    'payloadBytesLength === 39',
    "payloadAuthStatus === 'verified'",
    'defaultV3WriteEnabled === true',
    'mobile_v3_readonly_fixture_preserves_anchor_fields',
    'mobile_v3_readonly_media_fixture_preserves_anchor_fields',
    'desktop bridge readonly media container',
  ],
  'V3 readonly fixture QA',
);

includesAll(
  [
    sources.desktopV3ReadonlyCandidateQaBin,
    sources.mobileV3ReadonlyCandidateQaTool,
    sources.v3ReadonlyCandidateRuntimeQa,
  ].join('\n'),
  [
    'v3_readonly_candidate_runtime_qa',
    'build_v3_readonly_candidate_image_fixture_png_bytes',
    'build_v3_readonly_candidate_audio_fixture_wav_bytes',
    'readReadonlyCandidate',
    'payloadProtocolVersion === 3',
    'payloadBytesLength === 39',
    "payloadAuthStatus === 'verified'",
    "watermarkIdIssueMode === 'registry_resolved'",
    "mediaPayloadRole === 'v3_minimal_anchor'",
    "defaultReadStatus === 'default_v3_contract_guarded'",
    '"default_v3_contract_guarded"',
    'defaultV3WriteEnabled === true',
    'defaultWatermarkServiceExtractV3Enabled === true',
    '正式图片 sync packet 与音频 recovery packet',
  ],
  'V3 readonly candidate real-media runtime QA',
);

assert(
  sources.image.includes('fn encode_image_sync_packet(payload: &WatermarkPayload)') &&
    sources.audio.includes('fn encode_audio_recovery_packet(') &&
    sources.image.includes('decode_image_sync_packet_bytes(&packet).is_err()') &&
    sources.audio.includes('decode_audio_recovery_packet(&packet).is_err()') &&
    sources.service.includes('DefaultV3') &&
    sources.service.includes('ForceV2Rollback') &&
    sources.service.includes('extract_image_v3_bytes') &&
    sources.service.includes('extract_watermark_wav_readonly_candidate_bytes_with_delta') &&
    !sources.videoVisual.includes('decode_watermark_payload_readonly') &&
    !sources.image.includes('extract_v3_readonly_anchor_png_bytes') &&
    !sources.audio.includes('extract_v3_readonly_anchor_wav_bytes') &&
    !sources.videoVisual.includes('encode_payload_v3_minimal_anchor'),
  'default image/audio service paths must consume V3 while video visual paths remain outside this migration',
);

includesAll(
  [
    sources.protocolDoc,
    sources.vaultFieldContract,
    sources.watermarkPlan,
    sources.dualRoadmap,
    sources.capabilityBoundary,
  ].join('\n'),
  [
    'docs/V3跨端fixture与迁移桥接报告字段冻结合同.md',
    'rights:v3-migration-contract',
    '只读解析',
    '不修改 V2/119 payload',
  ],
  'V3 migration docs',
);

assert(
  !sources.protocolDoc.includes('/v1/enterprise/public-rights') ||
    (sources.protocolDoc.includes('POST /v1/enterprise/public-rights/batch') &&
      sources.protocolDoc.includes('registry') &&
      sources.protocolDoc.includes('legalConclusion=false') &&
      sources.protocolDoc.includes('仍未开放外部客户 key 管理 / quota 管理路由')),
  'V3 migration work may mention the read-only Enterprise batch route only as registry/query layer, not as payload migration',
);

console.log('V3 migration contract passed');
