import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  processDoc: readFileSync('docs/处理页第一性原则功能取舍清单.md', 'utf8'),
  desktopWorkbench: readFileSync('src/views/WorkbenchView.vue', 'utf8'),
  desktopDeclarationPanel: readFileSync('src/components/AIContentMarker.vue', 'utf8'),
  desktopApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  desktopVault: readFileSync('src/views/VaultView.vue', 'utf8'),
  desktopCopyrightCard: readFileSync('src/components/CopyrightCard.vue', 'utf8'),
  desktopReport: readFileSync('src-tauri/src/commands/report.rs', 'utf8'),
  desktopSchema: readFileSync('src-tauri/src/db/schema.rs', 'utf8'),
  desktopQueries: readFileSync('src-tauri/src/db/queries.rs', 'utf8'),
  desktopScheduler: readFileSync('src-tauri/src/pipeline/scheduler.rs', 'utf8'),
  desktopCloud: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  desktopSyncStorage: readFileSync('src-tauri/src/sync/storage.rs', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileStorage: readFileSync('mobile_app/lib/storage/vault_store.dart', 'utf8'),
  mobileSyncTransport: readFileSync('mobile_app/lib/sync/sync_transport.dart', 'utf8'),
  mobileVault: readFileSync('mobile_app/lib/features/vault/vault_page.dart', 'utf8'),
  mobileDeclarationPanel: readFileSync('mobile_app/lib/features/workspace/work_declaration_panel.dart', 'utf8'),
  mobileImageWrite: readFileSync('mobile_app/lib/features/workspace/image_embed_page.dart', 'utf8'),
  mobileAudioWrite: readFileSync('mobile_app/lib/features/workspace/audio_embed_page.dart', 'utf8'),
};

assert(
  sources.packageJson.includes('"process:first-principles-contract"') &&
    sources.processDoc.includes('处理页不是素材剪辑、平台分发、画幅适配或 AI 内容检测工具'),
  'package.json and process document must expose the first-principles process contract',
);

for (const forbidden of [
  'PlatformSelector',
  'showProMultiPlatform',
  'showRecommendHint',
  '加黑边保画面',
  '智能裁剪填满',
  '优先加速',
  '优先质量',
  '请至少勾选一个目标平台',
  '多平台需订阅',
]) {
  assert(
    !sources.desktopWorkbench.includes(forbidden),
    `desktop process page must not expose legacy distribution control: ${forbidden}`,
  );
}

assert(
  sources.desktopWorkbench.includes('L1 视频音轨水印') &&
    sources.desktopWorkbench.includes('最小必要变更策略') &&
    sources.desktopDeclarationPanel.includes('作品声明与授权策略'),
  'desktop process page must expose L1 audio-track protection, minimal-change strategy, and rights declarations',
);

for (const sourceName of ['desktopApi', 'desktopReport', 'desktopCloud', 'desktopSyncStorage', 'mobileState', 'mobileStorage', 'mobileSyncTransport', 'mobileVault']) {
  const source = sources[sourceName];
  for (const key of [
    'protected_copy_name',
    'protected_copy_hash',
    'output_strategy',
    'work_source_declaration',
    'training_permission_declaration',
    'creation_method_declaration',
    'human_edit_level_declaration',
    'authenticity_claim_declaration',
    'custom_rights_statement',
  ]) {
    assert(source.includes(key) || source.includes(camelCase(key)), `${sourceName} must carry ${key}`);
  }
}

assert(
  sources.desktopSchema.includes('protected_copy_path') &&
    sources.desktopSyncStorage.includes('protected_copy_path: None') &&
    !sources.mobileState.includes("'protected_copy_path'"),
  'protected_copy_path must be local-only and must not enter mobile/cloud sync payloads',
);

assert(
  sources.desktopReport.includes('protected_copy: FormalReportProtectedCopy') &&
    sources.desktopReport.includes('rights_declaration: FormalReportRightsDeclaration') &&
    sources.mobileState.includes('## 作品声明与授权策略') &&
    sources.mobileState.includes('保护副本摘要'),
  'formal reports and summaries must include protected-copy metadata and rights declarations from VaultRecord',
);

assert(
  sources.mobileDeclarationPanel.includes('作品声明与授权策略') &&
    sources.mobileDeclarationPanel.includes('HiddenShield 只记录声明，不检测 AI') &&
    sources.mobileImageWrite.includes('WorkDeclarationPanel') &&
    sources.mobileImageWrite.includes('declaration: _workDeclaration') &&
    sources.mobileAudioWrite.includes('WorkDeclarationPanel') &&
    sources.mobileAudioWrite.includes('declaration: _workDeclaration'),
  'mobile write pages must let users fill rights declarations and persist them through addWriteResult',
);

assert(
    sources.desktopScheduler.includes('output_douyin: None') &&
    sources.desktopScheduler.includes('protected_copy_path: Some') &&
    sources.desktopScheduler.includes('declaration_work_source') &&
    sources.desktopSchema.includes('历史保护副本'),
  'new desktop writes must use protected-copy fields; legacy output fields may only feed migration/backfill',
);

console.log('Process first-principles contract OK');

function camelCase(value) {
  return value.replace(/_([a-z])/g, (_, char) => char.toUpperCase());
}

function assert(condition, message) {
  if (!condition) {
    console.error(`Process first-principles contract failed: ${message}`);
    process.exit(1);
  }
}
