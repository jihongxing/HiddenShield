import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { basename, join, relative, resolve } from "node:path";

const root = process.cwd();
const runId = process.env.HIDDENSHIELD_DESKTOP_MEDIA_RC_RUN_ID ?? "20260722";
const rcRelease002Suspended =
  process.argv.includes("--suspend-rc-release-002") ||
  process.env.HIDDENSHIELD_RC_RELEASE_002_SUSPENDED === "1";
const candidateSourceCommit =
  process.env.HIDDENSHIELD_DESKTOP_MEDIA_CANDIDATE_COMMIT ??
  "b7e3f4d69f7b5b6770cbd433beb557d8f03643e0";
const outputDir = resolve("artifacts/desktop-media-internal-rc", runId);
const sourceEvidenceDir = join(outputDir, "source-evidence");
const summaryPath = join(outputDir, "summary.json");
const blockerPath = join(outputDir, "release-blockers.md");

const paths = {
  frozenImage:
    "artifacts/desktop-image-spatial-recovery-gate/20260722-image-complete-final-installed/summary.json",
  finalCandidateImage:
    "artifacts/desktop-image-spatial-recovery-gate/20260722-media-rc-final-candidate/summary.json",
  rcMedia001Closure:
    "artifacts/desktop-media-internal-rc/20260722/rc-media-001-closure.json",
  imageFalsePositive:
    "artifacts/desktop-image-spatial-recovery-gate/20260722-image-complete-final/false-positive-summary.json",
  imageInstaller:
    "artifacts/desktop-installer-self-contained/20260722-image-complete-final/desktop-installer-self-contained-gate.json",
  audioResource:
    "artifacts/desktop-audio-resource-gate/20260722-final-v2/summary.json",
  audioHighBit:
    "artifacts/desktop-high-bit-depth-audio-gate/20260722-final/summary.json",
  finalInstaller:
    "artifacts/desktop-installer-self-contained/20260722-audio-resource-v2/desktop-installer-self-contained-gate.json",
  audioFormatBaseline:
    "tmp-ui-qa/watermark-real-file-matrix/20260721/baseline-30s-48k-format-channel-after-rate-fix.json",
  audioPerturbation:
    "tmp-ui-qa/watermark-real-file-matrix/20260721/perturbation-medium-audio-48k-preserve-spec-summary.json",
  audioFormatChannel:
    "artifacts/desktop-audio-format-channel-gate/20260722-final/summary.json",
  audioUpperEnvelope:
    "artifacts/desktop-audio-upper-envelope-gate/20260722-final/summary.json",
  authenticodeGate:
    "artifacts/authenticode-gate/20260722-rc-release-001/authenticode-gate.json",
  authenticodeSigningEvidence:
    "artifacts/authenticode-signing/20260722-rc-release-001/self-signed-authenticode-evidence.json",
  authenticodePreSignManifest:
    "artifacts/authenticode-signing/20260722-rc-release-001/pre-sign-manifest.json",
  authenticodeSigntoolVerification:
    "artifacts/authenticode-signing/20260722-rc-release-001/signtool-verification.json",
  postSignImageSmoke:
    "artifacts/desktop-image-resource-gate/20260722-post-sign-smoke/summary.json",
  postSignAudioSmoke:
    "artifacts/desktop-audio-format-channel-gate/20260722-post-sign-smoke/summary.json",
  localStandardNsisInstall:
    "artifacts/nsis-local-standard-install/20260723/summary.json",
  fullCoreTestLog: "tmp/rc-media-002-watermark-core-lib-green.log",
  failedProtectedImage:
    "tmp-ui-qa/desktop-image-spatial-recovery/20260722-media-rc-final-candidate/outputs/windows-theme-c-img29_watermarked.png",
  failedWebpQ60:
    "tmp-ui-qa/desktop-image-spatial-recovery/20260722-media-rc-final-candidate/rc-diagnostic-transforms/webp-q60.webp",
};

mkdirSync(sourceEvidenceDir, { recursive: true });
for (const path of Object.values(paths)) {
  assert(existsSync(path), `Required RC evidence is missing: ${path}`);
}

const evidence = Object.fromEntries(
  Object.entries(paths)
    .filter(([, path]) => path.endsWith(".json"))
    .map(([name, path]) => [name, readJson(path)]),
);

for (const name of ["audioFormatBaseline", "audioPerturbation"]) {
  const sourcePath = paths[name];
  copyFileSync(sourcePath, join(sourceEvidenceDir, basename(sourcePath)));
}

const coreTestLog = readText(paths.fullCoreTestLog);
const coreTestResult = parseCoreTestResult(coreTestLog);
const expectedImageRead = runImageRead(paths.failedProtectedImage);
const webpQ60Read = runImageRead(paths.failedWebpQ60);
const finalInstallerArtifacts = evidence.finalInstaller.artifacts ?? {};
const finalAuthenticodeStatuses = [
  finalInstallerArtifacts.nsis?.authenticodeStatus,
  finalInstallerArtifacts.msi?.authenticodeStatus,
  finalInstallerArtifacts.releaseExecutable?.authenticodeStatus,
  finalInstallerArtifacts.installedExecutable?.authenticodeStatus,
].filter(Boolean);
const rcMedia001Closed =
  evidence.rcMedia001Closure.status === "passed" &&
  evidence.rcMedia001Closure.checks?.transformCellCount === 72 &&
  evidence.rcMedia001Closure.checks?.exactUidMatchCount === 72 &&
  evidence.rcMedia001Closure.checks?.independentCoreReadPassCount === 72 &&
  evidence.rcMedia001Closure.checks?.installedReadOnlyPassCount === 72 &&
  evidence.rcMedia001Closure.checks?.uidChecks?.every(
    (check) => check.uniqueUidCount === 3 && check.passed === true,
  );
const rcMedia002Closed =
  coreTestResult.status === "passed" &&
  coreTestResult.failed === 0 &&
  coreTestResult.passed === 108;
const rcMedia003Closed =
  evidence.audioFormatChannel.status === "passed" &&
  evidence.audioFormatChannel.checks?.matrix?.total === 10 &&
  evidence.audioFormatChannel.checks?.matrix?.passedCount === 10 &&
  evidence.audioFormatChannel.checks?.matrix?.failedCount === 0 &&
  evidence.audioFormatChannel.fixtures?.every(
    (fixture) =>
      fixture.status === "passed" &&
      fixture.specification?.passed === true &&
      fixture.writeAfterRead?.status === "verified" &&
      fixture.independentCoreRead?.passed === true &&
      fixture.readOnlyVerification?.matched === true &&
      fixture.readOnlyVerification?.payloadProtocolVersion === 3,
  );
const rcMedia004Closed =
  evidence.audioUpperEnvelope.status === "passed" &&
  evidence.audioUpperEnvelope.fixture?.input?.durationSeconds === 1200 &&
  evidence.audioUpperEnvelope.fixture?.input?.sampleRate === 48_000 &&
  evidence.audioUpperEnvelope.fixture?.input?.channels === 2 &&
  evidence.audioUpperEnvelope.fixture?.input?.effectiveBitDepth === 24 &&
  evidence.audioUpperEnvelope.cancellation?.status === "passed" &&
  evidence.audioUpperEnvelope.cancellation?.vaultRecordCreated === false &&
  evidence.audioUpperEnvelope.cancellation?.workerQuiescenceMs <= 120_000 &&
  evidence.audioUpperEnvelope.completion?.status === "passed" &&
  evidence.audioUpperEnvelope.completion?.specification?.passed === true &&
  evidence.audioUpperEnvelope.completion?.writeAfterRead?.status === "verified" &&
  evidence.audioUpperEnvelope.completion?.independentCoreRead?.passed === true &&
  evidence.audioUpperEnvelope.completion?.readOnlyVerification?.matched === true &&
  evidence.audioUpperEnvelope.completion?.readOnlyVerification?.payloadProtocolVersion === 3 &&
  evidence.audioUpperEnvelope.completion?.resources?.peakRootWorkingSetBytes > 0 &&
  evidence.audioUpperEnvelope.completion?.resources?.peakProcessTreeWorkingSetBytes > 0;
const postSignImageInstalledPath = resolve(
  evidence.postSignImageSmoke.product?.installedExecutable ?? "",
);
const postSignAudioInstalledPath = resolve(
  evidence.postSignAudioSmoke.product?.installedExecutable ?? "",
);
const signedInstalledArtifact = evidence.authenticodeSigningEvidence.files?.find(
  (artifact) => resolve(artifact.path) === postSignAudioInstalledPath,
);
const postSignImageSmokePassed =
  evidence.postSignImageSmoke.status === "passed" &&
  evidence.postSignImageSmoke.fixtures?.length === 3 &&
  evidence.postSignImageSmoke.fixtures.every(
    (fixture) =>
      fixture.status === "passed" &&
      fixture.writeAfterRead?.status === "verified" &&
      fixture.independentCoreRead?.passed === true &&
      fixture.readOnlyVerification?.matched === true,
  );
const postSignAudioSmokePassed =
  evidence.postSignAudioSmoke.status === "passed" &&
  evidence.postSignAudioSmoke.checks?.matrix?.total === 10 &&
  evidence.postSignAudioSmoke.checks?.matrix?.passedCount === 10 &&
  evidence.postSignAudioSmoke.checks?.matrix?.failedCount === 0 &&
  evidence.postSignAudioSmoke.fixtures?.every(
    (fixture) =>
      fixture.status === "passed" &&
      fixture.specification?.passed === true &&
      fixture.writeAfterRead?.status === "verified" &&
      fixture.independentCoreRead?.passed === true &&
      fixture.readOnlyVerification?.matched === true &&
      fixture.readOnlyVerification?.payloadProtocolVersion === 3,
  );
const postSignCandidateMatchesSigningEvidence =
  postSignImageInstalledPath === postSignAudioInstalledPath &&
  signedInstalledArtifact?.status === "Valid" &&
  signedInstalledArtifact.sha256 === sha256File(postSignAudioInstalledPath) &&
  evidence.postSignAudioSmoke.product?.installedExecutableSha256 ===
    signedInstalledArtifact.sha256;
const localStandardNsisInstallFailed =
  evidence.localStandardNsisInstall.checks?.installerSigned === true &&
  evidence.localStandardNsisInstall.checks?.installedExecutableExists === true &&
  evidence.localStandardNsisInstall.checks?.startMenuShortcut === true &&
  evidence.localStandardNsisInstall.checks?.uninstallEntryExists === true &&
  evidence.localStandardNsisInstall.checks?.installedExecutableSigned === false;
const rcRelease001Closed =
  evidence.authenticodeGate.status === "passed" &&
  evidence.authenticodeGate.mode === "candidate" &&
  evidence.authenticodeGate.artifactGate?.status === "passed" &&
  evidence.authenticodeGate.artifactGate?.artifacts?.length === 5 &&
  evidence.authenticodeGate.artifactGate.artifacts.every(
    (artifact) =>
      artifact.originalStatus === "Valid" &&
      artifact.tamperedStatus !== "Valid",
  ) &&
  evidence.authenticodeSigningEvidence.status === "signed" &&
  evidence.authenticodeSigningEvidence.files?.length === 5 &&
  evidence.authenticodeSigningEvidence.files.every(
    (artifact) => artifact.status === "Valid",
  ) &&
  evidence.authenticodePreSignManifest.rebuildProhibited === true &&
  evidence.authenticodePreSignManifest.files?.length === 4 &&
  evidence.authenticodePreSignManifest.files.every(
    (artifact) => artifact.authenticodeStatus === "NotSigned",
  ) &&
  evidence.authenticodeSigntoolVerification.files?.length === 4 &&
  evidence.authenticodeSigntoolVerification.files.every(
    (artifact) => artifact.verified === true && artifact.timestampPresent === true,
  ) &&
  postSignImageSmokePassed &&
  postSignAudioSmokePassed &&
  postSignCandidateMatchesSigningEvidence;

const blockers = [
  ...(rcMedia001Closed
    ? []
    : [
        {
          id: "RC-MEDIA-001",
          severity: "critical",
          scope: "desktop-image",
          title:
            "Latest combined candidate can recover an incorrect UID after WebP quality 60",
          evidence: {
            gateStatus: evidence.finalCandidateImage.status,
            gateError: evidence.finalCandidateImage.error,
            sourceUid: expectedImageRead.watermarkUid,
            recoveredUid: webpQ60Read.watermarkUid,
            sameUid: expectedImageRead.watermarkUid === webpQ60Read.watermarkUid,
            transformedFile: relative(root, resolve(paths.failedWebpQ60)),
            transformedSha256: sha256File(paths.failedWebpQ60),
          },
          exitCriteria: [
            "Fix the shared-core recovery path or narrow the public recovery promise.",
            "Run three real photos with at least three independently issued UIDs each through all eight promised transforms.",
            "Require exact UID equality for every independent-core and installed read-only result.",
          ],
        },
      ]),
  ...(rcMedia002Closed
    ? []
    : [
        {
          id: "RC-MEDIA-002",
          severity: "high",
          scope: "watermark-core",
          title: "Default watermark-core library test suite is red",
          evidence: coreTestResult,
          exitCriteria: [
            "Make the default release suite green.",
            "Move intentional legacy or rollback-only expectations into an explicitly scoped suite with documented ownership.",
            "Keep the five formal V3 image service tests green.",
          ],
        },
      ]),
  ...(rcMedia003Closed
    ? []
    : [
        {
          id: "RC-MEDIA-003",
          severity: "high",
          scope: "desktop-audio",
          title: "The five-format mono/stereo baseline is not an installed-candidate Gate",
          evidence: {
            localBaselinePath: paths.audioFormatBaseline,
            localBaselineSummary: evidence.audioFormatBaseline.summary,
            installedGatePath: paths.audioFormatChannel,
            installedGateStatus: evidence.audioFormatChannel.status,
          },
          exitCriteria: [
            "Run WAV, MP3, FLAC, OGG and M4A mono/stereo baseline writes through the final installed candidate.",
            "Record write-after-read, independent-core read, installed read-only verification and output specification preservation.",
            "Store the final report under artifacts instead of relying only on tmp-ui-qa.",
          ],
        },
      ]),
  ...(rcMedia004Closed
    ? []
    : [
        {
          id: "RC-MEDIA-004",
          severity: "high",
          scope: "desktop-audio",
          title: "The valid upper-envelope audio combination is not covered",
          evidence: {
            durationBoundaryTestedSeparately: true,
            capacityBoundaryTestedWithTrailingJunkChunk: true,
            installedGatePath: paths.audioUpperEnvelope,
            installedGateStatus: evidence.audioUpperEnvelope.status,
            missingCombination: "20:00, 48 kHz, stereo, high-bit-depth decoded payload",
          },
          exitCriteria: [
            "Run a 20-minute 48 kHz stereo high-bit-depth fixture through the final installed candidate.",
            "Record peak memory, cancellation behavior, output specification, write-after-read and read-only verification.",
          ],
        },
      ]),
  ...(rcRelease001Closed
    ? []
    : [
        {
          id: "RC-RELEASE-001",
          severity: "critical",
          scope: "desktop-release",
          title:
            "The evidence-bound installed executable is not Authenticode signed",
          evidence: {
            priorStatuses: finalAuthenticodeStatuses,
            candidateGateStatus: evidence.authenticodeGate.status,
            signingEvidenceStatus: evidence.authenticodeSigningEvidence.status,
          },
          exitCriteria: [
            "Restore an evidence-bound four-artifact candidate set with a signed installed executable.",
            "Pass the four-artifact Authenticode candidate Gate, including tamper invalidation.",
            "Run image and audio write/read smoke tests against the same signed installed executable.",
          ],
        },
      ]),
  {
    id: "RC-RELEASE-002",
    status: localStandardNsisInstallFailed
      ? rcRelease002Suspended
        ? "failed_local_install_vm_suspended"
        : "failed_local_install"
      : rcRelease002Suspended
        ? "suspended_by_user"
        : "active",
    severity: localStandardNsisInstallFailed ? "critical" : "high",
    scope: "desktop-release",
    title: localStandardNsisInstallFailed
      ? "Signed NSIS installs an unsigned application executable"
      : "Clean offline Windows environment proof is still missing",
    evidence: {
      installerStatus: evidence.finalInstaller.status,
      limitations: evidence.finalInstaller.limitations,
      localStandardInstallPath: paths.localStandardNsisInstall,
      localStandardInstallDirectory:
        evidence.localStandardNsisInstall.installDirectory,
      signedInstaller:
        evidence.localStandardNsisInstall.checks?.installerSigned === true,
      installedExecutableSignature:
        evidence.localStandardNsisInstall.installedExecutable
          ?.authenticodeStatus ?? null,
      installedExecutableSha256:
        evidence.localStandardNsisInstall.installedExecutable?.sha256 ?? null,
    },
    exitCriteria: [
      "Reject the current NSIS candidate because its installed application executable is not Authenticode signed.",
      "For a future candidate, sign the release executable before packaging, then build and sign the NSIS/MSI wrappers.",
      "Install and launch the signed NSIS and MSI candidates in clean offline Windows snapshots without a pre-existing WebView2 runtime.",
      "Verify the newly installed executable is Authenticode Valid and matches the intended signed payload boundary.",
      "Record installation, launch, image verification and audio verification evidence without rebuilding.",
    ],
  },
];

const summary = {
  schemaVersion: "desktop_media_internal_rc_review_v1",
  runId,
  generatedAt: new Date().toISOString(),
  verdict: "blocked",
  scope: {
    desktopImage: true,
    desktopAudio: true,
    mobileFrozen: true,
    videoExcluded: true,
  },
  candidateSourceCommit,
  reviewGeneratorCommit: execFileSync("git", ["rev-parse", "HEAD"], {
    encoding: "utf8",
  }).trim(),
  finalCandidate: {
    installedExecutable: signedInstalledArtifact?.path ?? null,
    installedExecutableSha256: signedInstalledArtifact?.sha256 ?? null,
    version: "0.1.0",
  },
  evidenceIntegrity: Object.fromEntries(
    Object.entries(paths).map(([name, path]) => [
      name,
      {
        path,
        sha256: sha256File(path),
      },
    ]),
  ),
  passedEvidence: {
    rcMedia001Closure: {
      status: evidence.rcMedia001Closure.status,
      candidate: evidence.rcMedia001Closure.candidate,
      photoCount: evidence.rcMedia001Closure.checks.photoCount,
      uniqueUidsPerPhoto: evidence.rcMedia001Closure.checks.uidChecks.map(
        (check) => ({
          photoName: check.photoName,
          uniqueUidCount: check.uniqueUidCount,
        }),
      ),
      transformCellCount: evidence.rcMedia001Closure.checks.transformCellCount,
      exactUidMatchCount: evidence.rcMedia001Closure.checks.exactUidMatchCount,
      independentCoreReadPassCount:
        evidence.rcMedia001Closure.checks.independentCoreReadPassCount,
      installedReadOnlyPassCount:
        evidence.rcMedia001Closure.checks.installedReadOnlyPassCount,
    },
    frozenImageGate: {
      status: evidence.frozenImage.status,
      fixtures: evidence.frozenImage.fixtures.length,
      realPhotos: evidence.frozenImage.fixtures.filter(
        (fixture) => fixture.tier === "real_photo_visual",
      ).length,
    },
    imageFalsePositiveGate: {
      status: evidence.imageFalsePositive.status,
      samples: evidence.imageFalsePositive.samples?.length ?? null,
      falsePositives:
        evidence.imageFalsePositive.samples?.filter((sample) => sample.falsePositive).length ??
        null,
    },
    audioResourceGate: {
      status: evidence.audioResource.status,
      fixtures: evidence.audioResource.fixtures.map((fixture) => ({
        boundary: fixture.boundary,
        status: fixture.status,
        elapsedMs: fixture.elapsedMs,
      })),
    },
    audioHighBitGate: {
      status: evidence.audioHighBit.status,
      fixtures: evidence.audioHighBit.fixtures.length,
      allSpecificationChecksPassed: evidence.audioHighBit.fixtures.every(
        (fixture) => fixture.specification?.passed === true,
      ),
    },
    audioFormatChannelGate: {
      status: evidence.audioFormatChannel.status,
      total: evidence.audioFormatChannel.checks?.matrix?.total ?? null,
      passedCount: evidence.audioFormatChannel.checks?.matrix?.passedCount ?? null,
      failedCount: evidence.audioFormatChannel.checks?.matrix?.failedCount ?? null,
      installedExecutableSha256:
        evidence.audioFormatChannel.product?.installedExecutableSha256 ?? null,
      formats: evidence.audioFormatChannel.product?.inputFormats ?? [],
      channelModes: evidence.audioFormatChannel.product?.channelModes ?? [],
      elapsedMs: evidence.audioFormatChannel.elapsedMs ?? null,
    },
    audioUpperEnvelopeGate: {
      status: evidence.audioUpperEnvelope.status,
      installedExecutableSha256:
        evidence.audioUpperEnvelope.product?.installedExecutableSha256 ?? null,
      input: evidence.audioUpperEnvelope.fixture?.input ?? null,
      cancellation: {
        status: evidence.audioUpperEnvelope.cancellation?.status ?? null,
        cancelAcknowledgeMs:
          evidence.audioUpperEnvelope.cancellation?.cancelAcknowledgeMs ?? null,
        workerQuiescenceMs:
          evidence.audioUpperEnvelope.cancellation?.workerQuiescenceMs ?? null,
        vaultRecordCreated:
          evidence.audioUpperEnvelope.cancellation?.vaultRecordCreated ?? null,
        peakRootWorkingSetBytes:
          evidence.audioUpperEnvelope.cancellation?.resources
            ?.peakRootWorkingSetBytes ?? null,
        peakProcessTreeWorkingSetBytes:
          evidence.audioUpperEnvelope.cancellation?.resources
            ?.peakProcessTreeWorkingSetBytes ?? null,
      },
      completion: {
        status: evidence.audioUpperEnvelope.completion?.status ?? null,
        elapsedMs: evidence.audioUpperEnvelope.completion?.elapsedMs ?? null,
        peakRootWorkingSetBytes:
          evidence.audioUpperEnvelope.completion?.resources
            ?.peakRootWorkingSetBytes ?? null,
        peakProcessTreeWorkingSetBytes:
          evidence.audioUpperEnvelope.completion?.resources
            ?.peakProcessTreeWorkingSetBytes ?? null,
        specificationPassed:
          evidence.audioUpperEnvelope.completion?.specification?.passed ?? false,
        writeAfterReadStatus:
          evidence.audioUpperEnvelope.completion?.writeAfterRead?.status ?? null,
        independentCoreReadPassed:
          evidence.audioUpperEnvelope.completion?.independentCoreRead?.passed ?? false,
        installedReadOnlyMatched:
          evidence.audioUpperEnvelope.completion?.readOnlyVerification?.matched ?? false,
      },
    },
    authenticodeCandidateGate: {
      status: evidence.authenticodeGate.status,
      mode: evidence.authenticodeGate.mode,
      provider: evidence.authenticodeGate.artifactGate?.provider ?? null,
      signingEvidence:
        evidence.authenticodeGate.artifactGate?.signingEvidence ?? null,
      artifacts: evidence.authenticodeGate.artifactGate?.artifacts ?? [],
      preSignManifest: paths.authenticodePreSignManifest,
      rebuildProhibited:
        evidence.authenticodePreSignManifest.rebuildProhibited ?? null,
      signtoolVerification: paths.authenticodeSigntoolVerification,
      allTimestamped:
        evidence.authenticodeSigntoolVerification.files?.every(
          (artifact) =>
            artifact.verified === true && artifact.timestampPresent === true,
        ) ?? false,
    },
    postSignMediaSmoke: {
      installedExecutable: relative(root, postSignAudioInstalledPath),
      installedExecutableSha256: signedInstalledArtifact?.sha256 ?? null,
      signingEvidenceMatched: postSignCandidateMatchesSigningEvidence,
      image: {
        status: evidence.postSignImageSmoke.status,
        fixtureCount: evidence.postSignImageSmoke.fixtures?.length ?? 0,
        passedCount:
          evidence.postSignImageSmoke.fixtures?.filter(
            (fixture) => fixture.status === "passed",
          ).length ?? 0,
      },
      audio: {
        status: evidence.postSignAudioSmoke.status,
        total: evidence.postSignAudioSmoke.checks?.matrix?.total ?? 0,
        passedCount:
          evidence.postSignAudioSmoke.checks?.matrix?.passedCount ?? 0,
        failedCount:
          evidence.postSignAudioSmoke.checks?.matrix?.failedCount ?? 0,
      },
    },
    localStandardNsisInstall: {
      status: localStandardNsisInstallFailed ? "failed" : "passed",
      installDirectory: evidence.localStandardNsisInstall.installDirectory,
      installedExecutable:
        evidence.localStandardNsisInstall.installedExecutable,
      startMenuShortcut:
        evidence.localStandardNsisInstall.checks?.startMenuShortcut === true,
      desktopShortcut:
        evidence.localStandardNsisInstall.checks?.desktopShortcut === true,
      uninstallEntry:
        evidence.localStandardNsisInstall.checks?.uninstallEntryExists === true,
    },
    localAudioFormatBaseline: evidence.audioFormatBaseline.summary,
    focusedSourceValidation: {
      v3ImageServiceTests: "5 passed",
      audioBoundaryTests: "2 passed",
      frontendBuild: "passed",
      architectureContract: "passed after RC wording remediation",
      audioSupportContract: "passed",
    },
    fullCoreTest: coreTestResult,
  },
  failedEvidence: {
    finalCandidateImageGate: {
      status: evidence.finalCandidateImage.status,
      error: evidence.finalCandidateImage.error,
      firstTwoRealPhotosPassed: evidence.finalCandidateImage.fixtures.length === 2,
      failedPhoto: "windows-theme-c-img29.jpg",
      failedTransform: "webp-q60",
      expectedUid: expectedImageRead.watermarkUid,
      recoveredUid: webpQ60Read.watermarkUid,
    },
  },
  nonBlockingLimitations: [
    "Field-recording noise-floor perceptual quality remains outside the current public promise.",
    "Audio crop disturbances below the 30-second standalone input minimum remain outside the public recovery promise.",
    "Mobile remains frozen and does not inherit desktop image or audio release claims.",
    "At the 20-minute 48 kHz stereo 24-bit upper envelope, cancellation is acknowledged immediately but the current non-interruptible decode/embed stage can take tens of seconds to become CPU-quiescent; no vault record is created.",
    "The Authenticode provider is a self-signed release certificate trusted only in managed trust stores; general Windows clients do not trust this publisher by default.",
  ],
  resolvedBlockers: [
    ...(rcMedia001Closed
      ? [
        {
          id: "RC-MEDIA-001",
          status: "closed",
          resolution:
            "Shared-core exact recovery now prefers the 25-packet consensus. Three real photos, three independent UIDs per photo and all eight promised transforms passed with exact UID equality in independent-core and installed read-only verification.",
          evidencePath: paths.rcMedia001Closure,
        },
        ]
      : []),
    ...(rcMedia002Closed
      ? [
          {
            id: "RC-MEDIA-002",
            status: "closed",
            resolution:
              "The default watermark-core release suite is green at 108/108. Formal image tests and consumers now use V3 only; retired V2 image write, read and rollback paths are rejected with v2_image_rollback_retired. Legacy audio rollback remains isolated under npm run watermark:legacy-rollback-suite.",
            evidencePath: paths.fullCoreTestLog,
          },
        ]
      : []),
    ...(rcMedia003Closed
      ? [
          {
            id: "RC-MEDIA-003",
            status: "closed",
            resolution:
              "The final installed desktop candidate passed WAV, MP3, FLAC, OGG and M4A in mono and stereo, 10/10. Every cell preserved source sample rate and channel count, retained lossless bit depth for WAV/FLAC, passed write-after-read, matched the exact V3 UID in the independent core reader and passed installed read-only verification.",
            evidencePath: paths.audioFormatChannel,
          },
        ]
      : []),
    ...(rcMedia004Closed
      ? [
          {
            id: "RC-MEDIA-004",
            status: "closed",
            resolution:
              "The final installed desktop candidate completed a 20-minute, 48 kHz, stereo, 24-bit FLAC input to a 24-bit WAV protected copy with duration, sample rate, channels and effective bit depth preserved. Write-after-read, independent V3 core read and installed read-only verification passed with the exact UID. Cancellation acknowledged immediately, created no vault record and reached CPU quiescence within the 120-second Gate.",
            evidencePath: paths.audioUpperEnvelope,
          },
        ]
      : []),
    ...(rcRelease001Closed
      ? [
          {
            id: "RC-RELEASE-001",
            status: "closed",
            resolution:
              "The locked NSIS, MSI, release executable and current installed executable were signed in place without rebuilding. All four report Authenticode Valid under the HiddenShield Release Signing self-signed certificate and fail validation after Gate tampering. The same signed installed executable then passed PNG/JPEG/WebP image smoke 3/3 and WAV/MP3/FLAC/OGG/M4A mono/stereo audio smoke 10/10.",
            evidencePath: [
              paths.authenticodeGate,
              paths.postSignImageSmoke,
              paths.postSignAudioSmoke,
            ],
          },
        ]
      : []),
  ],
  blockers,
  releaseDecision:
    localStandardNsisInstallFailed
      ? "Reject the current desktop 0.1.0 release candidate. The signed NSIS wrapper installs an unsigned application executable. RC-RELEASE-002 remains blocking, and its clean offline Windows portion remains suspended by user decision."
      : rcRelease002Suspended
        ? "Do not approve the desktop media internal RC or external release. RC-MEDIA-001 through RC-MEDIA-004 and RC-RELEASE-001 are closed, while RC-RELEASE-002 is suspended by user decision and remains release-blocking."
      : "Do not approve the desktop media internal RC or external release. RC-MEDIA-001 through RC-MEDIA-004 and RC-RELEASE-001 are closed, but clean offline Windows installation proof remains blocking.",
  nextAction:
    localStandardNsisInstallFailed
      ? "Freeze and reject the current 0.1.0 installer candidate without rebuilding it. Prepare a signing-order remediation plan for the next candidate: sign the inner release executable before packaging, then sign the NSIS/MSI wrappers and repeat installed-executable verification."
      : rcRelease002Suspended
        ? "Keep RC-RELEASE-002 blocked and suspended. Proceed with a desktop 0.1.0 RC evidence-index integrity audit that verifies candidate hashes, evidence references and product-boundary wording without rebuilding or claiming offline installation proof."
      : "Prioritize RC-RELEASE-002: install the signed NSIS and MSI in clean offline Windows snapshots without rebuilding, verify the installed executable signature, launch with no pre-existing WebView2 runtime, and rerun image/audio verification smoke tests.",
};

writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
writeFileSync(blockerPath, renderBlockers(summary), "utf8");
console.log(`Desktop media internal RC review: ${summary.verdict}`);
console.log(summaryPath);
console.log(blockerPath);

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
}

function readText(path) {
  const bytes = readFileSync(path);
  if (bytes[0] === 0xff && bytes[1] === 0xfe) {
    return bytes.subarray(2).toString("utf16le");
  }
  if (bytes[0] === 0xfe && bytes[1] === 0xff) {
    return Buffer.from(bytes.subarray(2)).swap16().toString("utf16le");
  }
  return bytes.toString("utf8");
}

function runImageRead(path) {
  const reader = resolve("watermark-core/target/release/desktop_image_read_qa.exe");
  const result = spawnSync(reader, [path], {
    cwd: root,
    encoding: "utf8",
    timeout: 120_000,
  });
  assert(result.status === 0, `Independent image read failed: ${result.stderr}`);
  return JSON.parse(result.stdout.trim());
}

function parseCoreTestResult(log) {
  const match = log.match(
    /test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/,
  );
  assert(match, "Could not parse the full core test result.");
  const failureBlock = log.match(/failures:\s+([\s\S]*?)\s+test result: FAILED\./);
  const failures = [
    ...(failureBlock?.[1] ?? "").matchAll(
      /^\s{4}([a-zA-Z0-9_]+(?:::[a-zA-Z0-9_]+)+)\s*$/gm,
    ),
  ].map((match) => match[1]);
  return {
    status: match[1] === "ok" ? "passed" : "failed",
    passed: Number(match[2]),
    failed: Number(match[3]),
    ignored: Number(match[4]),
    failures,
  };
}

function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function renderBlockers(review) {
  const lines = [
    "# HiddenShield 桌面媒体内部 RC 发布阻断项",
    "",
    `- 评审批次：\`${review.runId}\``,
    `- 结论：\`${review.verdict}\``,
    "- 移动端：冻结，不纳入本轮评审。",
    "",
  ];
  if (review.resolvedBlockers.length > 0) {
    lines.push("## 已关闭阻断项", "");
    for (const blocker of review.resolvedBlockers) {
      lines.push(
        `- ${blocker.id}：\`${blocker.status}\`；${blocker.resolution}`,
        `- 证据：\`${blocker.evidencePath}\``,
        "",
      );
    }
  }
  for (const blocker of review.blockers) {
    lines.push(
      `## ${blocker.id} · ${blocker.severity.toUpperCase()} · ${blocker.title}`,
      "",
      `- 范围：\`${blocker.scope}\``,
      `- 状态：\`${blocker.status ?? "active"}\``,
      `- 证据：\`${JSON.stringify(blocker.evidence)}\``,
      "- 解除条件：",
      ...blocker.exitCriteria.map((criterion) => `  - ${criterion}`),
      "",
    );
  }
  lines.push("## 推荐处理顺序", "", `1. ${review.nextAction}`, "");
  return `${lines.join("\n")}\n`;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
