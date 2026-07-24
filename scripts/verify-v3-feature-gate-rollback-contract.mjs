import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const runId = Date.now().toString();
const outputDir = resolve('tmp-ui-qa', 'v3-feature-gate-rollback', runId);
const qaJsonPath = join(outputDir, `v3-feature-gate-rollback-contract-${runId}.json`);
const qaMdPath = join(outputDir, `v3-feature-gate-rollback-contract-${runId}.md`);
mkdirSync(outputDir, { recursive: true });

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  gateDoc: readFileSync('docs/V3 feature gate写入与回滚验证方案.md', 'utf8'),
  migrationContract: readFileSync('docs/V3跨端fixture与迁移桥接报告字段冻结合同.md', 'utf8'),
  payload: readFileSync('watermark-core/src/payload.rs', 'utf8'),
  service: readFileSync('watermark-core/src/service.rs', 'utf8'),
  coreLib: readFileSync('watermark-core/src/lib.rs', 'utf8'),
  coreV3InternalQa: readFileSync('watermark-core/src/v3_internal_qa.rs', 'utf8'),
  coreQaBin: readFileSync('watermark-core/src/bin/v3_feature_gate_rollback_qa.rs', 'utf8'),
  coreImage: readFileSync('watermark-core/src/image.rs', 'utf8'),
};

assert(
  sources.packageJson.includes('"rights:v3-feature-gate-rollback-contract"') &&
    sources.packageJson.includes('verify-v3-feature-gate-rollback-contract.mjs'),
  'package.json must expose rights:v3-feature-gate-rollback-contract',
);

assert(
  sources.payload.includes('pub const PAYLOAD_BYTES: usize = 119;') &&
    sources.payload.includes('PAYLOAD_V3_MINIMAL_ANCHOR_BYTES') &&
    sources.payload.includes('PAYLOAD_V3_MINIMAL_ANCHOR_BYTES, 39'),
  'payload constants must keep V2/119 and V3/39 frozen',
);

assert(
    sources.service.includes('PayloadWriteMode') &&
    sources.service.includes('DefaultV3') &&
    sources.service.includes('ForceV2Rollback') &&
    sources.service.includes('v2_image_rollback_retired') &&
    sources.service.includes('WatermarkPayloadV3MinimalAnchor') &&
    sources.service.includes('pub fn embed_v2') &&
    !sources.service.includes('embed_v3_internal_qa_media') &&
    !sources.service.includes('build_v3_readonly_candidate_image_fixture_png_bytes') &&
    !sources.service.includes('build_v3_readonly_candidate_audio_fixture_wav_bytes') &&
    sources.service.includes('extract_image_v3_bytes') &&
    sources.service.includes('extract_watermark_wav_readonly_candidate_bytes_with_delta'),
  'default WatermarkService must route V3/39 while keeping internal_qa helpers out of formal paths and retaining explicit V2 rollback',
);

for (const required of [
  'off',
  'internal_qa',
  'force_v2_rollback',
  'V3 默认写入已开启',
  'payloadProtocolVersion=2',
  'payloadBytesLength=119',
  'off -> internal_qa -> force_v2_rollback',
  'rights:v3-feature-gate-rollback-contract',
]) {
  assert(sources.gateDoc.includes(required), `gate doc must include ${required}`);
}

assert(
  sources.coreLib.includes('embed_v3_internal_qa_media') &&
    sources.coreLib.includes('V3InternalQaWriteGate') &&
    sources.coreLib.includes('V3InternalQaWriteInput') &&
    sources.coreV3InternalQa.includes('V3InternalQaWriteGate::Off') &&
    sources.coreV3InternalQa.includes('V3InternalQaWriteGate::InternalQa') &&
    sources.coreV3InternalQa.includes('V3InternalQaWriteGate::ForceV2Rollback') &&
    sources.coreV3InternalQa.includes('v3_internal_qa_write_gate_off') &&
    sources.coreV3InternalQa.includes('v3_internal_qa_force_v2_rollback'),
  'watermark-core must expose explicit internal QA V3 gate APIs without changing default service',
);

assert(
  sources.coreQaBin.includes('V3InternalQaWriteGate::InternalQa') &&
    sources.coreQaBin.includes('"off"') &&
    sources.coreQaBin.includes('"force_v2_rollback"') &&
    sources.coreQaBin.includes('"v2_full_record"') &&
    sources.coreQaBin.includes('retired_v2_image_row_json') &&
    sources.coreQaBin.includes('v2_image_rollback_retired') &&
    sources.coreQaBin.includes('output.payload_bytes_length') &&
    sources.coreQaBin.includes('WatermarkService::embed'),
  'feature gate QA bin must cover off/internal_qa/force_v2_rollback matrix',
);

const defaultImageTestModule = sources.coreImage.split('#[cfg(test)]')[1] ?? '';
for (const forbidden of [
  'embed_image_watermark_bytes(',
  'extract_image_watermark_bytes(',
  'detect_existing_image_watermark_bytes(',
  'extract_image_watermark_readonly_candidate_bytes(',
  'image_bytes_report_v2_',
  'image_sync_packet_corrects_two_bit_errors',
]) {
  assert(
    !defaultImageTestModule.includes(forbidden),
    `default watermark-core image tests must not exercise legacy/rollback-only API: ${forbidden}`,
  );
}

assert(
  sources.migrationContract.includes('v3_feature_gate_rollback') &&
    sources.migrationContract.includes('R2 feature gate 写入') &&
    sources.migrationContract.includes('默认正式路径写读 V3/39') &&
    sources.migrationContract.includes('force_v2_rollback'),
  'migration contract must retain feature gate rollback requirements',
);

for (const retiredExport of [
  'detect_existing_image_watermark_bytes',
  'embed_image_watermark_bytes',
  'extract_image_watermark_bytes',
]) {
  assert(
    !sources.coreLib.includes(retiredExport),
    `watermark-core crate root must not expose retired V2 image API: ${retiredExport}`,
  );
}

run('cargo', [
  'run',
  '--release',
  '--manifest-path',
  'watermark-core/Cargo.toml',
  '--bin',
  'v3_feature_gate_rollback_qa',
  '--',
  '--run-id',
  runId,
  '--out-dir',
  outputDir,
]);

const matrix = JSON.parse(readFileSync(join(outputDir, 'v3-feature-gate-rollback-matrix.json'), 'utf8'));
const rows = matrix.rows;
assert(rows.length === 6, 'matrix must contain image/audio rows for off, internal_qa, and force_v2_rollback');
for (const gate of ['off', 'internal_qa', 'force_v2_rollback']) {
  for (const kind of ['image', 'audio']) {
    const row = rows.find((candidate) => candidate.gate === gate && candidate.kind === kind);
    assert(row, `missing ${gate} ${kind} row`);
    assert(row.pass === true, `${gate} ${kind} row must pass`);
    if (gate === 'off' || gate === 'internal_qa') {
      assert(row.watermarkUid?.startsWith('HS-'), `${gate} ${kind} row must include watermarkUid`);
      assert(row.payloadProtocolVersion === 3, `${gate} ${kind} must write V3`);
      assert(row.payloadBytesLength === 39, `${gate} ${kind} must be V3/39`);
      assert(row.mediaPayloadRole === 'v3_minimal_anchor', `${gate} ${kind} must be V3 minimal anchor`);
    } else if (kind === 'image') {
      assert(row.watermarkUid === null, 'retired V2 image row must not expose a UID');
      assert(row.expectedOutcome === 'rejected', 'V2 image rollback must be rejected');
      assert(row.reasonCode === 'v2_image_rollback_retired', 'V2 image rollback reason must be stable');
      assert(
        row.mediaPayloadRole === 'retired_v2_image_rollback',
        'V2 image rollback row must be marked retired',
      );
    } else {
      assert(row.watermarkUid?.startsWith('HS-'), `${gate} ${kind} row must include watermarkUid`);
      assert(row.payloadProtocolVersion === 2, `${gate} ${kind} must write V2 fallback`);
      assert(row.payloadBytesLength === 119, `${gate} ${kind} must be V2/119`);
      assert(row.mediaPayloadRole === 'v2_full_record', `${gate} ${kind} must be V2 full record`);
    }
  }
}

const result = {
  runId,
  outputDir,
  defaultV3WriteEnabled: true,
  v3InternalQaWriteImplemented: true,
  matrix,
  boundary:
    'Default WatermarkService writes and reads V3/39. V2 image rollback is retired and rejected with v2_image_rollback_retired. The isolated matrix keeps legacy audio rollback evidence separate from the current V3 image product boundary.',
  pass: true,
};

writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
console.log(`V3 feature gate rollback contract JSON: ${qaJsonPath}`);
console.log(`V3 feature gate rollback contract Markdown: ${qaMdPath}`);

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: resolve('.'),
    env: process.env,
    encoding: 'utf8',
    maxBuffer: 128 * 1024 * 1024,
  });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    if (result.error) console.error(result.error);
    throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function renderMarkdown(result) {
  return `# HiddenShield V3 feature gate rollback contract

- Run ID: \`${result.runId}\`
- Evidence dir: \`${result.outputDir}\`
- Boundary: ${result.boundary}

| Gate | Kind | Watermark UID | Payload | Role | Path | Result |
| --- | --- | --- | --- | --- | --- | --- |
${result.matrix.rows
  .map(
    (row) =>
      `| ${row.gate} | ${row.kind} | ${row.watermarkUid} | V${row.payloadProtocolVersion}/${row.payloadBytesLength} | ${row.mediaPayloadRole} | ${row.path} | PASS |`,
  )
  .join('\n')}

## Conclusion

Formal image writing and reading support V3/39 only. The \`force_v2_rollback\` image row is an expected rejection with \`v2_image_rollback_retired\`; it does not produce a V2 image. Legacy audio rollback remains isolated in this suite. Default user writes produce V3/39, and the explicit \`internal_qa\` state continues to produce controlled V3/39 samples for QA evidence.
`;
}
