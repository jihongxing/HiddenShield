import { existsSync, readFileSync } from 'node:fs';

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
  schema: readFileSync('src-tauri/src/db/schema.rs', 'utf8'),
  queries: readFileSync('src-tauri/src/db/queries.rs', 'utf8'),
  cloud: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  storage: readFileSync('src-tauri/src/sync/storage.rs', 'utf8'),
  releasePlan: readFileSync('docs/封版收口计划.md', 'utf8'),
  commercialRoadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
};

assert(
  sources.packageJson.includes('"vault:file-type-backfill-contract"') &&
    sources.packageJson.includes('verify-vault-file-type-backfill-contract.mjs'),
  'package.json must expose vault:file-type-backfill-contract',
);

includesAll(
  sources.schema,
  [
    'pub const CURRENT_VERSION: u32 = 18',
    'backfill_vault_record_file_types(conn)?',
    "SET file_type = 'image'",
    "SET file_type = 'audio'",
    "WHERE file_type = 'video'",
    'video_notary_id IS NULL',
    'video_fingerprint_root IS NULL',
    'video_visual_task_id IS NULL',
    'video_visual_media_hash IS NULL',
    "'%.png'",
    "'%.wav'",
    "'%.m4a'",
    'migration_18_backfills_legacy_media_file_types_without_touching_video_receipts',
  ],
  'SQLite migration v18',
);

includesAll(
  sources.queries,
  [
    'pub fn infer_vault_record_file_type(record: &VaultRecord)',
    'video_notary_id.is_some()',
    'video_fingerprint_root.is_some()',
    'video_visual_task_id.is_some()',
    'video_visual_media_hash.is_some()',
    'const IMAGE_EXTENSIONS',
    'const AUDIO_EXTENSIONS',
    'video_visual_output_content_type, file_type',
    'infer_vault_record_file_type(record)',
    'insert_record_persists_inferred_file_type',
  ],
  'vault record insert inference',
);

assert(
  (sources.queries.match(/infer_vault_record_file_type\(record\)/g) || []).length >= 2,
  'both insert_record and insert_record_tx must write inferred file_type',
);

assert(
  sources.cloud.includes('queries::infer_vault_record_file_type(record)'),
  'desktop cloud sync kind must use the shared vault file_type inference',
);
assert(
  sources.storage.includes('queries::infer_vault_record_file_type(record)'),
  'desktop/mobile changes response kind must use the shared vault file_type inference',
);

const indexPath = 'docs/RC1双端QA总索引.md';
assert(existsSync(indexPath), `${indexPath} must exist`);
const rc1Index = readFileSync(indexPath, 'utf8');
includesAll(
  rc1Index,
  [
    'vault:file-type-backfill-contract',
    'tmp-ui-qa/desktop-batch2-qa/desktop-batch2-final-evidence-check-20260704.json',
    'tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/android-batch2-page-qa-summary-1783106946906.json',
    'tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783067156075.json',
    'iOS',
    'BLOCKED',
  ],
  'RC1 dual-end QA total index',
);

includesAll(
  sources.releasePlan,
  [
    'vault:file-type-backfill-contract',
    'docs/RC1双端QA总索引.md',
    'file_type',
    '已解除',
  ],
  'release plan file_type closure',
);

includesAll(
  sources.commercialRoadmap,
  ['vault:file-type-backfill-contract', 'docs/RC1双端QA总索引.md'],
  'commercial roadmap evidence',
);
includesAll(
  sources.dualRoadmap,
  ['vault:file-type-backfill-contract', 'docs/RC1双端QA总索引.md'],
  'dual roadmap evidence',
);

console.log('vault:file-type-backfill-contract OK');
