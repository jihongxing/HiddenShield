import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';

const searchRoot = resolve(
  process.env.HIDDENSHIELD_VIDEO_FINGERPRINT_BUNDLE_DIR ??
    'src-tauri/target/video-fingerprint-spike',
);
const tempDir = await mkdtemp(join(tmpdir(), 'hiddenshield-video-bundle-'));

try {
  let bundlePaths = await findBundleJson(searchRoot);
  if (bundlePaths.length === 0) {
    const fixturePath = join(tempDir, 'bundle.json');
    await writeFile(fixturePath, `${JSON.stringify(sampleBundle(), null, 2)}\n`, 'utf8');
    bundlePaths = [fixturePath];
    console.log(`No spike bundle found under ${searchRoot}; using built-in fixture`);
  }

  const results = [];
  for (const bundlePath of bundlePaths) {
    results.push(await verifyBundle(bundlePath));
  }

  console.log(
    `Video fingerprint bundle verification OK: ${results.length} bundle(s), ${results.reduce(
      (sum, result) => sum + result.globalFrameCount,
      0,
    )} global frames`,
  );
} finally {
  await rm(tempDir, { recursive: true, force: true });
}

async function verifyBundle(bundlePath) {
  const bytes = await readFile(bundlePath);
  const bundle = JSON.parse(bytes.toString('utf8'));
  assert(bundle.schemaVersion === 'video_fingerprint_v1', `${bundlePath}: schemaVersion mismatch`);
  assertNonEmpty(bundle.watermarkUid, `${bundlePath}: watermarkUid is required`);
  assertSha256Like(bundle.sourceHash, `${bundlePath}: sourceHash must be sha256-prefixed`);
  assert(Number.isSafeInteger(bundle.durationMs) && bundle.durationMs > 0, `${bundlePath}: durationMs is invalid`);
  assertNonEmpty(bundle.frameSamplePolicy, `${bundlePath}: frameSamplePolicy is required`);
  assert(
    Number.isSafeInteger(bundle.sceneCount) && bundle.sceneCount > 0,
    `${bundlePath}: sceneCount is invalid`,
  );
  assert(Array.isArray(bundle.fingerprints) && bundle.fingerprints.length > 0, `${bundlePath}: fingerprints required`);
  assert(bundle.sceneCount === bundle.fingerprints.length, `${bundlePath}: sceneCount must match fingerprints length`);
  assertNonEmpty(bundle.clientSignature, `${bundlePath}: clientSignature is required`);

  let localBlockCount = 0;
  let cropWindowCount = 0;
  const globalFrameFingerprints = [];
  for (const [index, frame] of bundle.fingerprints.entries()) {
    verifyFrame(bundlePath, frame, index);
    localBlockCount += frame.localBlocks.length;
    cropWindowCount += frame.cropWindows.length;
    globalFrameFingerprints.push({
      sceneIndex: frame.sceneIndex,
      timestampMs: frame.timestampMs,
      phash: frame.phash,
      colorHash: frame.colorHash,
      edgeHash: frame.edgeHash,
      motionSummary: frame.motionSummary,
    });
  }
  assert(localBlockCount > 0, `${bundlePath}: localBlocks required`);
  assert(cropWindowCount > 0, `${bundlePath}: cropWindows required`);

  const localBlockFingerprintRoot = sha256Root([
    'video-local-block-root-v1',
    ...bundle.fingerprints.flatMap((frame) => [
      frame.sceneIndex,
      frame.timestampMs,
      ...frame.localBlocks.flatMap((block) => [block.grid, block.row, block.col, block.phash, block.edgeHash]),
    ]),
  ]);
  const cropWindowFingerprintRoot = sha256Root([
    'video-crop-window-root-v1',
    ...bundle.fingerprints.flatMap((frame) => [
      frame.sceneIndex,
      frame.timestampMs,
      ...frame.cropWindows.flatMap((window) => [window.region, window.phash, window.edgeHash]),
    ]),
  ]);
  const fingerprintRoot = sha256Root([
    'video-fingerprint-notary-root-v1',
    bundle.schemaVersion,
    bundle.watermarkUid,
    bundle.sourceHash,
    bundle.durationMs,
    bundle.frameSamplePolicy,
    bundle.sceneCount,
    ...globalFrameFingerprints.flatMap((frame) => [
      frame.sceneIndex,
      frame.timestampMs,
      frame.phash,
      frame.colorHash,
      frame.edgeHash,
      frame.motionSummary,
    ]),
    localBlockFingerprintRoot,
    cropWindowFingerprintRoot,
    bundle.clientSignature,
  ]);
  const bundleSha256 = `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
  const request = {
    schemaVersion: 'video_fingerprint_notary_request_v1',
    workspaceId: 'bundle-smoke-workspace',
    creatorProfileId: 'bundle-smoke-creator',
    watermarkUid: bundle.watermarkUid,
    sourceHash: bundle.sourceHash,
    durationMs: bundle.durationMs,
    frameSamplePolicy: bundle.frameSamplePolicy,
    sceneCount: bundle.sceneCount,
    fingerprintSchemaVersion: bundle.schemaVersion,
    globalFrameFingerprints,
    localBlockFingerprintRoot,
    localBlockCount,
    cropWindowFingerprintRoot,
    cropWindowCount,
    fingerprintRoot,
    clientSignature: bundle.clientSignature,
    uploadManifest: {
      schemaVersion: 'video_upload_manifest_v1',
      containsOriginalVideo: false,
      containsWatermarkedVideo: false,
      containsLocalPaths: false,
      containsProxy: false,
      items: [
        {
          kind: 'video_fingerprint_bundle',
          sha256: bundleSha256,
          bytes: bytes.length,
        },
      ],
    },
  };

  verifyNotaryRequest(bundlePath, request, bundleSha256, bytes.length);
  return {
    path: bundlePath,
    globalFrameCount: globalFrameFingerprints.length,
  };
}

function verifyFrame(bundlePath, frame, index) {
  assert(Number.isSafeInteger(frame.sceneIndex), `${bundlePath}: frame ${index} sceneIndex invalid`);
  assert(Number.isSafeInteger(frame.timestampMs), `${bundlePath}: frame ${index} timestampMs invalid`);
  assertHexString(frame.phash, `${bundlePath}: frame ${index} phash invalid`);
  assertHexString(frame.colorHash, `${bundlePath}: frame ${index} colorHash invalid`);
  assertHexString(frame.edgeHash, `${bundlePath}: frame ${index} edgeHash invalid`);
  assert(Array.isArray(frame.localBlocks) && frame.localBlocks.length > 0, `${bundlePath}: frame ${index} localBlocks required`);
  assert(Array.isArray(frame.cropWindows) && frame.cropWindows.length > 0, `${bundlePath}: frame ${index} cropWindows required`);
  assertNonEmpty(frame.motionSummary, `${bundlePath}: frame ${index} motionSummary required`);

  for (const [blockIndex, block] of frame.localBlocks.entries()) {
    assertNonEmpty(block.grid, `${bundlePath}: frame ${index} block ${blockIndex} grid required`);
    assert(Number.isSafeInteger(block.row), `${bundlePath}: frame ${index} block ${blockIndex} row invalid`);
    assert(Number.isSafeInteger(block.col), `${bundlePath}: frame ${index} block ${blockIndex} col invalid`);
    assertHexString(block.phash, `${bundlePath}: frame ${index} block ${blockIndex} phash invalid`);
    assertHexString(block.edgeHash, `${bundlePath}: frame ${index} block ${blockIndex} edgeHash invalid`);
  }

  for (const [windowIndex, window] of frame.cropWindows.entries()) {
    assertNonEmpty(window.region, `${bundlePath}: frame ${index} crop ${windowIndex} region required`);
    assertHexString(window.phash, `${bundlePath}: frame ${index} crop ${windowIndex} phash invalid`);
    assertHexString(window.edgeHash, `${bundlePath}: frame ${index} crop ${windowIndex} edgeHash invalid`);
  }
}

function verifyNotaryRequest(bundlePath, request, expectedSha256, expectedBytes) {
  assert(request.localBlockCount > 0, `${bundlePath}: localBlockCount required`);
  assert(request.cropWindowCount > 0, `${bundlePath}: cropWindowCount required`);
  assertSha256Like(request.localBlockFingerprintRoot, `${bundlePath}: localBlockFingerprintRoot invalid`);
  assertSha256Like(request.cropWindowFingerprintRoot, `${bundlePath}: cropWindowFingerprintRoot invalid`);
  assertSha256Like(request.fingerprintRoot, `${bundlePath}: fingerprintRoot invalid`);
  assert(request.uploadManifest.containsOriginalVideo === false, `${bundlePath}: original video must not be uploaded`);
  assert(request.uploadManifest.containsWatermarkedVideo === false, `${bundlePath}: watermarked video must not be uploaded`);
  assert(request.uploadManifest.containsLocalPaths === false, `${bundlePath}: local paths must not be uploaded`);
  assert(request.uploadManifest.items.length === 1, `${bundlePath}: exactly one bundle manifest item required`);
  assert(request.uploadManifest.items[0].kind === 'video_fingerprint_bundle', `${bundlePath}: manifest item kind mismatch`);
  assert(request.uploadManifest.items[0].sha256 === expectedSha256, `${bundlePath}: manifest sha256 mismatch`);
  assert(request.uploadManifest.items[0].bytes === expectedBytes, `${bundlePath}: manifest bytes mismatch`);
  assertNoForbiddenMediaFields(request, bundlePath);
}

async function findBundleJson(root) {
  if (!existsSync(root)) {
    return [];
  }
  const found = [];
  await walk(root, found);
  return found.sort((a, b) => a.localeCompare(b));
}

async function walk(dir, found) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const child = join(dir, entry.name);
    if (entry.isDirectory()) {
      await walk(child, found);
    } else if (entry.isFile() && basename(child) === 'bundle.json') {
      found.push(child);
    }
  }
}

function sha256Root(parts) {
  const hash = createHash('sha256');
  for (const part of parts) {
    if (typeof part === 'number') {
      const bytes = Buffer.alloc(8);
      bytes.writeBigUInt64LE(BigInt(part));
      hash.update(bytes);
    } else {
      const value = String(part);
      const length = Buffer.alloc(8);
      length.writeBigUInt64LE(BigInt(Buffer.byteLength(value)));
      hash.update(length);
      hash.update(value);
    }
  }
  return `sha256:${hash.digest('hex')}`;
}

function assertNoForbiddenMediaFields(value, label) {
  const forbiddenKeys = new Set([
    'path',
    'filePath',
    'file_path',
    'localPath',
    'local_path',
    'sourcePath',
    'source_path',
    'originalVideo',
    'original_video',
    'watermarkedVideo',
    'watermarked_video',
    'videoBytes',
    'video_bytes',
  ]);
  const visit = (node, trace) => {
    if (Array.isArray(node)) {
      node.forEach((item, index) => visit(item, `${trace}[${index}]`));
      return;
    }
    if (node && typeof node === 'object') {
      for (const [key, child] of Object.entries(node)) {
        assert(!forbiddenKeys.has(key), `${label}: forbidden media/local field ${trace}.${key}`);
        visit(child, `${trace}.${key}`);
      }
    }
  };
  visit(value, '$');
}

function assertHexString(value, message) {
  assert(typeof value === 'string' && /^[0-9a-f]{16,}$/i.test(value), message);
}

function assertSha256Like(value, message) {
  assert(typeof value === 'string' && /^sha256:[0-9a-f]{16,}$/i.test(value), message);
}

function assertNonEmpty(value, message) {
  assert(typeof value === 'string' && value.trim().length > 0, message);
}

function assert(condition, message) {
  if (!condition) {
    console.error(`Video fingerprint bundle verification failed: ${message}`);
    process.exit(1);
  }
}

function sampleBundle() {
  return {
    schemaVersion: 'video_fingerprint_v1',
    watermarkUid: 'wm-video-bundle-smoke',
    sourceHash: 'sha256:0123456789abcdef0123456789abcdef',
    durationMs: 125000,
    frameSamplePolicy: 'uniform_2_frames_v1',
    sceneCount: 2,
    fingerprints: [
      {
        sceneIndex: 0,
        timestampMs: 1000,
        phash: '0000000000000001',
        colorHash: '0000000000000002',
        edgeHash: '0000000000000003',
        localBlocks: [
          {
            grid: '4x4',
            row: 0,
            col: 0,
            phash: '0000000000000011',
            edgeHash: '0000000000000012',
          },
        ],
        cropWindows: [
          {
            region: 'center_80',
            phash: '0000000000000021',
            edgeHash: '0000000000000022',
          },
        ],
        motionSummary: 'static-frame-v1',
      },
      {
        sceneIndex: 1,
        timestampMs: 9000,
        phash: '0000000000000101',
        colorHash: '0000000000000102',
        edgeHash: '0000000000000103',
        localBlocks: [
          {
            grid: 'dense_64x36',
            row: 1,
            col: 2,
            phash: '0000000000000111',
            edgeHash: '0000000000000112',
          },
        ],
        cropWindows: [
          {
            region: 'right_80',
            phash: '0000000000000121',
            edgeHash: '0000000000000122',
          },
        ],
        motionSummary: 'motion-low-v1',
      },
    ],
    clientSignature: 'sha256:abcdefabcdefabcdefabcdefabcdef12',
  };
}
