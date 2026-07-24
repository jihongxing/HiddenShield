import { readFileSync, statSync } from 'node:fs';
import { extname, join, relative } from 'node:path';
import { execFileSync } from 'node:child_process';

const root = process.cwd();

const sources = {
  agents: readFileSync('AGENTS.md', 'utf8'),
  plan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
  audit: readFileSync('docs/共享水印核心算法审计.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
  commercialRoadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  packageJson: readFileSync('package.json', 'utf8'),
  corePayload: readFileSync('watermark-core/src/payload.rs', 'utf8'),
  coreError: readFileSync('watermark-core/src/error.rs', 'utf8'),
  desktopScheduler: readFileSync('src-tauri/src/pipeline/scheduler.rs', 'utf8'),
  desktopIdentity: readFileSync('src-tauri/src/identity.rs', 'utf8'),
  desktopSyncCommand: readFileSync('src-tauri/src/commands/sync.rs', 'utf8'),
  mobileRustApi: readFileSync('mobile_app/rust/src/api.rs', 'utf8'),
  mobilePayloadSeed: readFileSync('mobile_app/lib/features/workspace/watermark_payload_seed.dart', 'utf8'),
  mobilePreviewBridge: readFileSync('mobile_app/lib/bridge/local_preview_watermark_bridge.dart', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileStateTest: readFileSync('mobile_app/test/mobile_app_state_test.dart', 'utf8'),
  mobileRewritePreflightTest: readFileSync('mobile_app/test/rewrite_preflight_test.dart', 'utf8'),
  mobileImageEmbedPage: readFileSync('mobile_app/lib/features/workspace/image_embed_page.dart', 'utf8'),
  mobileAudioEmbedPage: readFileSync('mobile_app/lib/features/workspace/audio_embed_page.dart', 'utf8'),
  mobileVerifyPage: readFileSync('mobile_app/lib/features/verify/verify_page.dart', 'utf8'),
  backendCargoToml: readFileSync('feedback-backend/Cargo.toml', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  releasePlan: readFileSync('docs/双端现有能力发布计划.md', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
};

assert(
  sources.agents.includes('single source of truth for all current and future blind-watermark') &&
    sources.agents.includes('Image, audio, video-audio-track, and future video-visual blind-watermark algorithms must live in `watermark-core`') &&
    sources.agents.includes('cloud service is allowed to handle scheduling, entitlement, key custody, strategy delivery, and self-check orchestration, but not to become a second algorithm source'),
  'AGENTS.md must make watermark-core the only algorithm source for all current and future blind-watermark capabilities',
);

assert(
  sources.plan.includes('正式盲水印能力必须只有一套算法核心') &&
    sources.plan.includes('云端视频画面盲水印如果需要服务端能力，服务端也只能部署或调用 `watermark-core` 产物') &&
    sources.plan.includes('禁止在 `watermark-core` 之外新增 DCT/DWT/SVD/QIM/LSB') &&
    sources.audit.includes('核心外算法分叉') &&
    sources.dualRoadmap.includes('只能在 `watermark-core` 实现') &&
    sources.commercialRoadmap.includes('`watermark-core` 视频画面算法和云端执行包装设计'),
  'Phase I docs and roadmaps must preserve the all-blind-watermark algorithms live in watermark-core boundary',
);

assert(
  sources.packageJson.includes('"watermark:architecture-contract"'),
  'package.json must expose watermark:architecture-contract',
);

assert(
    sources.corePayload.includes('PayloadBuildInput') &&
    sources.corePayload.includes('PayloadDigestBuildInput') &&
    sources.corePayload.includes('WatermarkIdentity') &&
    sources.corePayload.includes('IdentityBuildInput') &&
    sources.corePayload.includes('from_identity_and_media') &&
    sources.corePayload.includes('from_identity_and_media_sha256') &&
    !sources.corePayload.includes('PrecomputedPayloadBuildInput') &&
    !sources.corePayload.includes('from_precomputed') &&
    sources.desktopIdentity.includes('WatermarkIdentity') &&
    sources.desktopIdentity.includes('IdentityBuildInput') &&
    sources.desktopIdentity.includes('legacy_seed_identity_is_not_loaded') &&
    !sources.desktopIdentity.includes('pub user_seed_hex') &&
    !sources.desktopIdentity.includes('pub device_id_hex') &&
    !sources.desktopIdentity.includes('compute_user_seed') &&
    !sources.desktopIdentity.includes('compute_device_id') &&
    !sources.desktopSyncCommand.includes('user_seed_hex') &&
    !sources.desktopSyncCommand.includes('device_id_hex') &&
    sources.desktopScheduler.includes('WatermarkPayload::from_identity_and_media_sha256') &&
    !sources.desktopScheduler.includes('PrecomputedPayloadBuildInput') &&
    !sources.desktopScheduler.includes('compute_file_hash_prefix') &&
    sources.mobileRustApi.includes('WatermarkPayload::from_identity_and_media') &&
    sources.mobilePayloadSeed.includes('creatorIdentity:') &&
    sources.mobilePayloadSeed.includes('deviceIdentity:') &&
    sources.mobilePayloadSeed.includes('mediaBytes: bytes') &&
    !sources.mobilePayloadSeed.includes('sha256.convert') &&
    !sources.mobilePayloadSeed.includes('creatorDigest'),
  'Phase I-1 payload construction must be centralized in watermark-core builders, not Dart or platform wrappers',
);

assert(
  sources.coreError.includes('pub enum WatermarkErrorCode') &&
    sources.coreError.includes('AlreadyWatermarked') &&
    sources.coreError.includes('MissingCreatorIdentity') &&
    sources.coreError.includes('MissingDeviceIdentity') &&
    sources.coreError.includes('MissingMediaBytes') &&
    sources.coreError.includes('pub fn code(&self) -> WatermarkErrorCode') &&
    sources.coreError.includes('pub fn code_str(&self)') &&
    sources.mobileRustApi.includes('code: String') &&
    sources.mobileRustApi.includes('existing_uid: Option<String>') &&
    sources.desktopScheduler.includes('core_watermark_error_to_pipeline') &&
    sources.desktopScheduler.includes('error.code_str()'),
  'Phase I-1 must expose structured core watermark error codes through desktop and mobile wrappers',
);

assert(
  !outsideCoreText().includes('WatermarkPayload::new(') &&
    !outsideCoreText().includes('WatermarkPayload::from_precomputed('),
  'formal wrappers must not construct payloads from precomputed seed/device/file-hash values',
);

const backendText = [
  sources.backendCargoToml,
  sources.backendLib,
  sources.backendSchema,
  sources.backendStorage,
].join('\n');

const backendAlgorithmScanText = sanitizeAllowedWrapperMetadata(backendText);

const backendForbiddenPatterns = [
  {
    pattern: /watermark[-_]core|watermark_core/,
    message: 'backend must not add a direct watermark-core dependency unless the architecture contract is updated for an execution-wrapper service',
  },
  {
    pattern: /\b(?:WatermarkPayload|PayloadBuildInput|PayloadDigestBuildInput|IdentityBuildInput|WatermarkIdentity|WatermarkService|MediaInput|MediaOutput)\b/,
    message: 'backend must not construct formal watermark payloads or media IO types',
  },
  {
    pattern: /\b(?:embed_watermark|extract_watermark|embed_image_watermark|extract_image_watermark|embed_audio_watermark|extract_audio_watermark|embed_video_visual|extract_video_visual)\b/,
    message: 'backend must not implement or call formal blind-watermark write/read algorithms',
  },
  {
    pattern: /\b(?:encode_payload|decode_payload|PAYLOAD_BYTES|sync marker|payload bitstream|frequency modulation|频域调制|同步标记|载荷比特)\b/,
    message: 'backend must not implement payload encoding or watermark bitstream rules',
  },
  {
    pattern: /\b(?:DCT|DWT|SVD|QIM|LSB)\b/,
    message: 'backend must not contain blind-watermark algorithm primitives',
  },
];

const backendViolations = backendForbiddenPatterns
  .filter((rule) => rule.pattern.test(backendAlgorithmScanText))
  .map((rule) => rule.message);

assert(
  backendViolations.length === 0,
  `backend must remain metadata/notary/sync only and must not grow non-core watermark algorithms:\n${backendViolations.join('\n')}`,
);

assert(
  sources.backendSchema.includes('watermark_uid') &&
    sources.backendStorage.includes('watermark_uid') &&
    sources.backendLib.includes('create_video_fingerprint_notary'),
  'backend may keep watermark_uid metadata for cloud sync and L2 notary, but not formal blind-watermark algorithms',
);

assert(
  sources.backendStorage.includes('l3_video_visual_declared_capacity_is_supported') &&
    sources.backendStorage.includes('L3_VIDEO_VISUAL_CAPACITY_BYTES') === false &&
    sources.backendStorage.includes('L3_VIDEO_VISUAL_PAYLOAD_BYTES') &&
    sources.backendStorage.includes('L3_VIDEO_VISUAL_DCT_COEFF_PAIRS') &&
    sources.backendLib.includes('"algorithmSource": "watermark-core"') &&
    sources.backendStorage.includes('"algorithmSource": "watermark-core"'),
  'backend may keep L3 wrapper capacity preflight and worker receipt source metadata, but must not implement embed/extract algorithms',
);

assert(
  sources.mobilePreviewBridge.includes('supportsProductionWatermark => false') &&
    sources.mobilePreviewBridge.includes('isProductionWatermark: false') &&
    countOccurrences(sources.mobilePreviewBridge, 'isProductionWatermark: false') >= 2 &&
    sources.mobilePreviewBridge.includes('不生成可被桌面端验证的正式盲水印') &&
    !sources.mobilePreviewBridge.includes("return 'HS-") &&
    !sources.mobilePreviewBridge.includes('RegExp(r\'^HS-') &&
    sources.mobilePreviewBridge.includes("return 'PREVIEW-") &&
    sources.mobileRewritePreflightTest.includes("startsWith('HS-'), isFalse"),
  'Web preview must be visibly preview-only and must not generate HS-looking formal copyright IDs',
);

assert(
  sources.mobileState.includes("throw StateError('Web 预览结果不能写入正式版权库或云同步队列。')") &&
    sources.mobileState.includes("throw StateError('Web 预览验证结果不能写入正式版权库或云同步队列。')") &&
    countOccurrences(sources.mobileState, 'if (!result.isProductionWatermark)') >= 2 &&
    sources.mobileStateTest.includes('rejects web preview write results from formal vault and sync') &&
    sources.mobileStateTest.includes('rejects web preview read results from formal vault and sync'),
  'Web preview write/read results must be rejected before formal vault or cloud sync queue insertion',
);

assert(
  sources.mobileState.includes('Map<String, Object?> sanitizeVaultRecordSyncPayload') &&
    sources.mobileState.includes('Map<String, Object?> toSyncPayload()') &&
    sources.mobileState.includes('return sanitizeVaultRecordSyncPayload({') &&
    !sources.mobileState.includes("'local_path'") &&
    !sources.mobileState.includes("'protected_copy_path'") &&
    !sources.mobileState.includes("'media_file_path'") &&
    !sources.mobileState.includes("'preview_marker'"),
  'mobile sync payload must stay whitelisted and must not include local paths, media paths, protected-copy paths, or preview markers',
);

assert(
  sources.mobileState.includes('Future<FormalReportDraft> buildFormalReportDraft(VaultRecord record)') &&
    sources.mobileState.includes('String buildCopyrightSummary(VaultRecord record)') &&
    sources.mobileState.includes('factory FormalReportDraft.fromRecord({') &&
    sources.mobileState.includes('required VaultRecord record') &&
    !sources.mobileState.includes('FormalReportDraft.fromWriteResult') &&
    !sources.mobileState.includes('FormalReportDraft.fromReadResult') &&
    !sources.mobileState.includes('buildCopyrightSummary(WatermarkWriteResult') &&
    !sources.mobileState.includes('buildCopyrightSummary(WatermarkReadResult'),
  'formal reports and copyright summaries must be built from VaultRecord only, never directly from preview write/read results',
);

assert(
  sources.releasePlan.includes('状态：发布主线，L3 冻结为内部储备') &&
    sources.releasePlan.includes('短期不继续推进 L3 视频画面盲水印算法') &&
    sources.releasePlan.includes('L3 视频画面盲水印继续属于“只能内部测试”') &&
    sources.releasePlan.includes('不得进入 UI、订阅权益、销售材料、正式报告、云任务或账本扣费') &&
    sources.capabilityBoundary.includes('L3 视频画面盲水印') &&
    sources.capabilityBoundary.includes('只能内部测试'),
  'release preparation must keep L3 video visual watermark suspended as internal-only and out of shipped product promises',
);

const scanRoots = ['src', 'src-tauri/src', 'mobile_app/lib', 'mobile_app/rust/src', 'feedback-backend/src', 'scripts'];
const sourceFiles = listSourceFiles(scanRoots).filter(
  (file) => !isAllowedCoreWrapper(file) && !isContractRuleScript(file),
);

const forbiddenAlgorithmPatterns = [
  {
    pattern: /\bfn\s+(?:embed|extract)_(?:image|audio|video)?_?watermark\b/,
    message: 'core-side Rust watermark embed/extract function outside watermark-core',
  },
  {
    pattern: /\b(?:QIM|DCT|DWT|SVD|LSB|payload bitstream|sync marker|frequency modulation|频域调制|同步标记|载荷比特)\b/i,
    message: 'non-core source mentions blind-watermark algorithm primitives',
  },
  {
    pattern: /\b(?:encode_payload|decode_payload|PAYLOAD_BYTES)\b/,
    message: 'payload encoding/decoding must not be reimplemented outside watermark-core',
  },
];

const violations = [];
for (const file of sourceFiles) {
  const text = sanitizeAllowedWrapperMetadata(readFileSync(file, 'utf8'));
  if (isDocumentedPreviewOnly(file, text)) {
    continue;
  }
  for (const rule of forbiddenAlgorithmPatterns) {
    if (rule.pattern.test(text)) {
      violations.push(`${toPosix(file)}: ${rule.message}`);
    }
  }
}

assert(
  violations.length === 0,
  `blind-watermark algorithm code must stay in watermark-core:\n${violations.join('\n')}`,
);

console.log('Watermark architecture contract OK');

function listSourceFiles(roots) {
  const output = execFileSync('git', ['ls-files', ...roots], {
    cwd: root,
    encoding: 'utf8',
  });
  return output
    .split(/\r?\n/)
    .filter(Boolean)
    .filter((file) => {
      const extension = extname(file);
      return ['.rs', '.dart', '.ts', '.tsx', '.js', '.mjs', '.vue'].includes(extension);
    })
    .filter((file) => {
      try {
        return statSync(join(root, file)).isFile();
      } catch {
        return false;
      }
    });
}

function isAllowedCoreWrapper(file) {
  const normalized = toPosix(file);
  return [
    'src-tauri/src/pipeline/watermark.rs',
    'src-tauri/src/pipeline/image_watermark.rs',
    'src-tauri/src/pipeline/scheduler.rs',
    'src-tauri/src/commands/verify.rs',
    'src-tauri/src/commands/vault.rs',
    'src-tauri/src/sync/cloud.rs',
    'mobile_app/rust/src/api.rs',
    'mobile_app/lib/bridge/rust_watermark_bridge.dart',
    'mobile_app/lib/bridge/watermark_bridge.dart',
    'scripts/verify-dual-consistency-contract.mjs',
  ].includes(normalized);
}

function isContractRuleScript(file) {
  const normalized = toPosix(file);
  return normalized.startsWith('scripts/verify-') && normalized.endsWith('.mjs');
}

function isDocumentedPreviewOnly(file, text) {
  const normalized = toPosix(file);
  return (
    normalized === 'mobile_app/lib/bridge/local_preview_watermark_bridge.dart' &&
    text.includes('supportsProductionWatermark => false') &&
    text.includes('不生成可被桌面端验证的正式盲水印')
  );
}

function toPosix(path) {
  return relative(root, join(root, path)).replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    console.error(`Watermark architecture contract failed: ${message}`);
    process.exit(1);
  }
}

function countOccurrences(text, needle) {
  return text.split(needle).length - 1;
}

function sanitizeAllowedWrapperMetadata(text) {
  return text
    .replaceAll('"algorithmSource": "watermark-core"', '"algorithmSource": "shared-core-wrapper-receipt"')
    .replaceAll('L3_VIDEO_VISUAL_PAYLOAD_BYTES', 'L3_VIDEO_VISUAL_CAPACITY_BYTES')
    .replaceAll('L3_VIDEO_VISUAL_DCT_COEFF_PAIRS', 'L3_VIDEO_VISUAL_CAPACITY_COEFF_PAIRS')
    .replaceAll('const PAYLOAD_BYTES: u32 = 119;', 'const CAPACITY_BYTES: u32 = 119;')
    .replaceAll('const DCT_COEFF_PAIRS: u32 = 3;', 'const CAPACITY_COEFF_PAIRS: u32 = 3;')
    .replaceAll(' * DCT_COEFF_PAIRS', ' * CAPACITY_COEFF_PAIRS')
    .replaceAll(' + PAYLOAD_BYTES * 8 * ECC_REPEAT', ' + CAPACITY_BYTES * 8 * ECC_REPEAT');
}

function outsideCoreText() {
  return [
    sources.desktopScheduler,
    sources.desktopIdentity,
    sources.desktopSyncCommand,
    sources.mobileRustApi,
    sources.mobilePayloadSeed,
  ].join('\n');
}
