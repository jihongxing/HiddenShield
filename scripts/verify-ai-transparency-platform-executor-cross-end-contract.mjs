import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const fixtureDirectory = 'docs/fixtures/ai-transparency-platform-executor-v1';
const manifestPath = `${fixtureDirectory}/manifest.json`;
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const mobileApi = readFileSync('mobile_app/rust/src/api.rs', 'utf8');
const desktopVerify = readFileSync('src-tauri/src/commands/verify.rs', 'utf8');
const packageJson = readFileSync('package.json', 'utf8');
const fixtureContract = readFileSync('docs/AI生成内容标识平台写入PNG跨端Fixture合同.md', 'utf8');

assert(
  manifest.schemaVersion === 'hs-ai-platform-executor-cross-end-fixture-v1',
  'fixture schema version is invalid',
);
assert(manifest.mediaType === 'image/png', 'fixture media type must be image/png');
assert(
  manifest.watermarkUid === 'HS-01234567-89ABCDEF-01234567-89ABCDEF',
  'fixture watermark UID is unexpected',
);
assert(manifest.payloadProtocolVersion === 3, 'fixture must use V3 payload');
assert(manifest.payloadBytesLength === 39, 'fixture must use 39-byte V3 anchor');
assert(manifest.payloadAuthStatus === 'verified', 'fixture auth status must be verified');
assert(manifest.legalConclusion === false, 'fixture must not make a legal conclusion');

for (const [name, entry] of Object.entries(manifest.files)) {
  const path = resolve(fixtureDirectory, entry.path);
  assert(existsSync(path), `${name} fixture is missing`);
  const bytes = readFileSync(path);
  assert(bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])), `${name} is not PNG`);
  const digest = createHash('sha256').update(bytes).digest('hex');
  assert(digest === entry.sha256, `${name} SHA-256 mismatch`);
}

const externalMetadata = readPngTextChunks(
  readFileSync(resolve(fixtureDirectory, manifest.files.withExternalMetadata.path)),
);
const externalMetadataStripped = readPngTextChunks(
  readFileSync(resolve(fixtureDirectory, manifest.files.externalMetadataStripped.path)),
);
assert(
  JSON.stringify(Object.keys(externalMetadata).sort()) ===
    JSON.stringify(manifest.files.withExternalMetadata.metadataKeys.sort()),
  'external metadata fixture keys are unexpected',
);
assert(
  externalMetadata.external_provenance_fixture === 'untrusted_test_metadata_v1' &&
    externalMetadata.external_metadata_namespace === 'example.invalid/ai-provenance',
  'external metadata fixture values are unexpected',
);
assert(
  Object.keys(externalMetadataStripped).length === 0,
  'metadata-stripped fixture must not retain external test metadata',
);

for (const source of [mobileApi, desktopVerify]) {
  assert(
    source.includes('platform_executor_png_fixtures_are_'),
    'desktop and mobile must each contain a platform executor fixture reader test',
  );
  assert(
    source.includes('platform-executor-v3-metadata-stripped.png'),
    'desktop and mobile must read the metadata-stripped fixture',
  );
  assert(
    source.includes('platform-executor-v3-with-external-metadata.png') &&
      source.includes('platform-executor-v3-external-metadata-stripped.png'),
    'desktop and mobile must read external metadata coexistence fixtures',
  );
  assert(source.includes('payload_bytes_length'), 'fixture reader must assert V3/39 bytes');
}

assert(
  packageJson.includes('ai-transparency:platform-executor-cross-end-contract'),
  'fixture contract must be exposed through package.json',
);
assert(
  fixtureContract.includes('iOS') && fixtureContract.includes('metadata 剥离'),
  'fixture contract must retain iOS and metadata stripping boundaries',
);

console.log('AI Transparency platform executor cross-end fixture contract passed');

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function readPngTextChunks(bytes) {
  const text = {};
  let offset = 8;
  while (offset < bytes.length) {
    const length = bytes.readUInt32BE(offset);
    const type = bytes.toString('ascii', offset + 4, offset + 8);
    const dataStart = offset + 8;
    const dataEnd = dataStart + length;
    assert(dataEnd + 4 <= bytes.length, 'PNG chunk is truncated');
    if (type === 'tEXt') {
      const separator = bytes.indexOf(0, dataStart);
      assert(separator >= dataStart && separator < dataEnd, 'PNG tEXt chunk is malformed');
      text[bytes.toString('latin1', dataStart, separator)] = bytes.toString(
        'latin1',
        separator + 1,
        dataEnd,
      );
    }
    offset = dataEnd + 4;
  }
  assert(offset === bytes.length, 'PNG chunk stream is malformed');
  return text;
}
