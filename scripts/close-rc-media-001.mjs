import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, relative, resolve } from "node:path";

const root = process.cwd();
const reviewRunId = process.env.HIDDENSHIELD_RC_MEDIA_001_RUN_ID ?? "20260722";
const sourceRunIds = (
  process.env.HIDDENSHIELD_RC_MEDIA_001_SOURCE_RUN_IDS ??
  [
    "20260722-webp-q60-core-fix-installed",
    "20260722-rc-media-001-uid2",
    "20260722-rc-media-001-uid3",
  ].join(",")
)
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean);
const expectedCandidateSha256 = (
  process.env.HIDDENSHIELD_RC_MEDIA_001_EXPECTED_SHA256 ??
  "37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40"
).toLowerCase();
const expectedPhotos = [
  "windows-mi-default.jpg",
  "windows-mi-sunset.jpg",
  "windows-theme-c-img29.jpg",
];
const expectedTransforms = [
  "rotate-90",
  "rotate-180",
  "rotate-270",
  "scale-85",
  "jpeg-q75",
  "jpeg-q60",
  "webp-q75",
  "webp-q60",
];
const outputPath = resolve(
  "artifacts/desktop-media-internal-rc",
  reviewRunId,
  "rc-media-001-closure.json",
);

const evidence = {
  schemaVersion: "desktop_media_rc_media_001_closure_v1",
  incident: "RC-MEDIA-001",
  generatedAt: new Date().toISOString(),
  status: "running",
  decision: {
    sharedCoreFixAccepted: true,
    desktopWebpQ60PromiseRetained: true,
    narrowedToWebpQ75: false,
  },
  candidate: {},
  sourceRuns: [],
  matrix: [],
  checks: {},
};

try {
  assert(sourceRunIds.length === 3, "Exactly three installed Gate runs are required.");
  const summaries = sourceRunIds.map(loadRunSummary);
  const installedExecutable = summaries[0].summary.product.installedExecutable;
  const candidatePath = resolve(installedExecutable);
  assert(existsSync(candidatePath), `Installed candidate not found: ${candidatePath}`);
  const candidateSha256 = sha256(candidatePath);
  assert(
    candidateSha256 === expectedCandidateSha256,
    `Candidate SHA-256 mismatch: expected ${expectedCandidateSha256}, got ${candidateSha256}`,
  );
  assert(
    summaries.every(
      ({ summary }) => summary.product.installedExecutable === installedExecutable,
    ),
    "Source runs do not reference the same installed executable.",
  );

  evidence.candidate = {
    installedExecutable: relative(root, candidatePath),
    sha256: candidateSha256,
    bytes: statSync(candidatePath).size,
  };

  for (const { runId, summaryPath, summary } of summaries) {
    assert(summary.status === "passed", `${runId} did not pass.`);
    const photos = summary.fixtures.filter(
      (fixture) => fixture.tier === "real_photo_visual",
    );
    assert(photos.length === expectedPhotos.length, `${runId} photo count mismatch.`);
    evidence.sourceRuns.push({
      runId,
      summaryPath: relative(root, summaryPath),
      status: summary.status,
      photoCount: photos.length,
    });

    for (const photoName of expectedPhotos) {
      const fixture = photos.find((candidate) => candidate.name === photoName);
      assert(fixture, `${runId} is missing ${photoName}.`);
      const expectedUid = fixture.transformRecovery?.expectedUid;
      assert(expectedUid, `${runId}/${photoName} has no expected UID.`);
      assert(fixture.status === "passed", `${runId}/${photoName} failed.`);
      assert(
        fixture.writeAfterRead?.status === "verified",
        `${runId}/${photoName} write-after-read failed.`,
      );
      assert(
        fixture.independentCoreRead?.passed === true,
        `${runId}/${photoName} independent core read failed.`,
      );
      assert(
        fixture.readOnlyVerification?.matched === true &&
          fixture.readOnlyVerification?.watermarkUid === expectedUid,
        `${runId}/${photoName} installed read-only verification failed.`,
      );
      assert(
        fixture.cropRecovery?.exactGridPassed === true &&
          fixture.cropRecovery?.slidingPassed === true,
        `${runId}/${photoName} crop recovery failed.`,
      );

      const transforms = fixture.transformRecovery?.cases ?? [];
      assert(
        transforms.length === expectedTransforms.length,
        `${runId}/${photoName} transform count mismatch.`,
      );
      assert(
        expectedTransforms.every((name) =>
          transforms.some((candidate) => candidate.name === name),
        ),
        `${runId}/${photoName} transform set mismatch.`,
      );

      evidence.matrix.push({
        runId,
        photoName,
        watermarkUid: expectedUid,
        baseWriteAfterReadPassed: true,
        baseIndependentCoreReadPassed: true,
        baseInstalledReadOnlyPassed: true,
        exactGridCropCount: fixture.cropRecovery.exactGridCount,
        slidingCropCount: fixture.cropRecovery.slidingCount,
        transforms: transforms.map((transform) => {
          const uidMatches = transform.watermarkUid === expectedUid;
          const independentCoreReadPassed =
            transform.independentCoreRead?.passed === true;
          const installedReadOnlyPassed = transform.passed === true;
          assert(
            uidMatches && independentCoreReadPassed && installedReadOnlyPassed,
            `${runId}/${photoName}/${transform.name} did not recover the exact UID.`,
          );
          return {
            name: transform.name,
            watermarkUid: transform.watermarkUid,
            uidMatches,
            independentCoreReadPassed,
            installedReadOnlyPassed,
            reasonCode: transform.reasonCode,
          };
        }),
      });
    }
  }

  const uidChecks = expectedPhotos.map((photoName) => {
    const watermarkUids = evidence.matrix
      .filter((row) => row.photoName === photoName)
      .map((row) => row.watermarkUid);
    return {
      photoName,
      watermarkUids,
      uniqueUidCount: new Set(watermarkUids).size,
      passed: new Set(watermarkUids).size === 3,
    };
  });
  assert(uidChecks.every((check) => check.passed), "UID independence check failed.");

  const transformCells = evidence.matrix.flatMap((row) => row.transforms);
  evidence.checks = {
    sourceRunCount: sourceRunIds.length,
    photoCount: expectedPhotos.length,
    uidChecks,
    matrixRowCount: evidence.matrix.length,
    transformsPerRow: expectedTransforms.length,
    transformCellCount: transformCells.length,
    exactUidMatchCount: transformCells.filter((cell) => cell.uidMatches).length,
    independentCoreReadPassCount: transformCells.filter(
      (cell) => cell.independentCoreReadPassed,
    ).length,
    installedReadOnlyPassCount: transformCells.filter(
      (cell) => cell.installedReadOnlyPassed,
    ).length,
    allPassed: true,
  };
  evidence.status = "passed";
  evidence.closedAt = new Date().toISOString();
  writeEvidence();
  console.log(
    JSON.stringify(
      {
        status: evidence.status,
        candidateSha256,
        photoCount: expectedPhotos.length,
        uniqueUidsPerPhoto: 3,
        transformCellCount: transformCells.length,
        exactUidMatchCount: evidence.checks.exactUidMatchCount,
        summaryPath: outputPath,
      },
      null,
      2,
    ),
  );
} catch (error) {
  evidence.status = "failed";
  evidence.failedAt = new Date().toISOString();
  evidence.error = String(error?.stack ?? error);
  writeEvidence();
  throw error;
}

function loadRunSummary(runId) {
  const summaryPath = resolve(
    "artifacts/desktop-image-spatial-recovery-gate",
    runId,
    "summary.json",
  );
  assert(existsSync(summaryPath), `Missing source summary: ${summaryPath}`);
  return {
    runId,
    summaryPath,
    summary: JSON.parse(readFileSync(summaryPath, "utf8")),
  };
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function writeEvidence() {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
