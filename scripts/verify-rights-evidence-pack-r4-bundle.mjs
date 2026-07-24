import { createHash } from "node:crypto";
import {
  appendFile,
  cp,
  lstat,
  mkdtemp,
  readdir,
  readFile,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const bundleDir = path.join(
  repoRoot,
  "docs",
  "fixtures",
  "rights-evidence-pack-r4",
  "case-fixture-r4-0001",
);
const result = await verifyBundle(bundleDir);
assert(result.matched, result.message);

const originalEventEntries = result.manifest.eventChain.entries;
const appendedEvent = {
  eventId: "collection-event-0005",
  sequence: 5,
  eventType: "review_note_added",
  occurredAt: "2026-07-14T11:40:00.000Z",
  timeStatus: "device_claimed",
  actor: "fixture_reviewer",
  attachmentIds: [],
  description: "追加合成复核说明。",
};
const appendedChain = buildEventChain([...result.caseDocument.collectionEvents, appendedEvent]);
assert(
  JSON.stringify(appendedChain.entries.slice(0, originalEventEntries.length)) ===
    JSON.stringify(originalEventEntries),
  "appending an event must preserve all previous event chain entries",
);

const tamperedAttachment = Buffer.concat([
  result.attachmentBytes.get("attachment-capture-0003"),
  Buffer.from("\ntampered"),
]);
assert(
  sha256(tamperedAttachment) !==
    result.attachments.get("attachment-capture-0003").sha256,
  "attachment tamper fixture must change SHA-256",
);

const tamperedEvents = structuredClone(result.caseDocument.collectionEvents);
tamperedEvents[1].description = "tampered event";
assert(
  buildEventChain(tamperedEvents).rootDigest !== result.manifest.eventChain.rootDigest,
  "event tampering must change the event root digest",
);

const tempRoot = await mkdtemp(path.join(tmpdir(), "hidden-shield-r4-bundle-"));
try {
  const attachmentTamperDir = path.join(tempRoot, "attachment-tamper");
  await cp(bundleDir, attachmentTamperDir, { recursive: true });
  await appendFile(
    path.join(
      attachmentTamperDir,
      "attachments",
      "capture",
      "ATT-03-disputed-page-capture.txt",
    ),
    "\nphysical tamper",
  );
  await expectRejected(
    () => verifyBundle(attachmentTamperDir),
    "physical attachment tampering must fail bundle verification",
  );

  const eventTamperDir = path.join(tempRoot, "event-tamper");
  await cp(bundleDir, eventTamperDir, { recursive: true });
  const tamperedCase = JSON.parse(
    await readFile(path.join(eventTamperDir, "case.json"), "utf8"),
  );
  tamperedCase.collectionEvents[0].description = "physical event tamper";
  await writeFile(
    path.join(eventTamperDir, "case.json"),
    `${JSON.stringify(tamperedCase, null, 2)}\n`,
  );
  await expectRejected(
    () => verifyBundle(eventTamperDir),
    "physical event tampering must fail bundle verification",
  );

  const unregisteredFileDir = path.join(tempRoot, "unregistered-file");
  await cp(bundleDir, unregisteredFileDir, { recursive: true });
  await writeFile(
    path.join(unregisteredFileDir, "attachments", "capture", "UNREGISTERED.txt"),
    "unregistered fixture",
  );
  await expectRejected(
    () => verifyBundle(unregisteredFileDir),
    "unregistered attachment files must fail bundle verification",
  );
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}

console.log(
  JSON.stringify(
    {
      status: "passed",
      bundleDir: path.relative(repoRoot, bundleDir).replaceAll("\\", "/"),
      attachmentCount: result.attachments.size,
      eventCount: result.caseDocument.collectionEvents.length,
      rootDigest: result.manifest.integrity.rootDigest,
      appendOnlyEventPrefixPreserved: true,
      attachmentTamperDetected: true,
      eventTamperDetected: true,
      physicalAttachmentTamperRejected: true,
      physicalEventTamperRejected: true,
      unregisteredFileRejected: true,
    },
    null,
    2,
  ),
);

async function verifyBundle(directory) {
  const caseBytes = await readFile(path.join(directory, "case.json"));
  const caseDocument = JSON.parse(caseBytes.toString("utf8"));
  const manifest = JSON.parse(
    await readFile(path.join(directory, "case-manifest.json"), "utf8"),
  );
  assert(manifest.schemaVersion === 1, "Manifest schema version mismatch");
  assert(
    manifest.manifestType === "rights_evidence_pack_manifest",
    "Manifest type mismatch",
  );
  assert(
    JSON.stringify(manifest.directoryContract.allowedTopLevelEntries) ===
      JSON.stringify(["case.json", "case-manifest.json", "attachments"]),
    "top-level directory contract mismatch",
  );
  const topLevelEntries = (await readdir(directory)).sort();
  assert(
    JSON.stringify(topLevelEntries) ===
      JSON.stringify(["attachments", "case-manifest.json", "case.json"]),
    "bundle contains unregistered top-level entries",
  );
  assert(manifest.caseFile.bytes === caseBytes.length, "case.json byte count mismatch");
  assert(manifest.caseFile.sha256 === sha256(caseBytes), "case.json SHA-256 mismatch");

  const attachments = new Map(
    caseDocument.attachments.map((attachment) => [
      attachment.attachmentId,
      attachment,
    ]),
  );
  const attachmentBytes = new Map();
  const allowedRoles = new Set([
    "original",
    "working_copy",
    "capture",
    "external_receipt",
  ]);
  assert(attachments.size === 4, "fixture must cover all four attachment roles");
  assert(
    new Set([...attachments.values()].map((attachment) => attachment.role)).size === 4,
    "fixture must contain one attachment per frozen role",
  );

  for (const attachment of attachments.values()) {
    assert(allowedRoles.has(attachment.role), `unsupported role ${attachment.role}`);
    const absolutePath = await safeBundlePath(directory, attachment.relativePath);
    const bytes = await readFile(absolutePath);
    attachmentBytes.set(attachment.attachmentId, bytes);
    assert(attachment.bytes === bytes.length, `${attachment.attachmentId} byte mismatch`);
    assert(
      attachment.sha256 === sha256(bytes),
      `${attachment.attachmentId} SHA-256 mismatch`,
    );
    if (attachment.role === "working_copy") {
      assert(
        attachments.has(attachment.derivedFromAttachmentId),
        "working copy must reference its source attachment",
      );
      assert(
        attachments.get(attachment.derivedFromAttachmentId).role === "original",
        "working copy source must be the original role in the v1 fixture",
      );
    } else {
      assert(
        attachment.derivedFromAttachmentId === null,
        `${attachment.role} must not declare a derivation source`,
      );
    }
  }
  assert(
    caseDocument.automatedFindings.every((finding) =>
      finding.inputAttachmentIds.every((attachmentId) =>
        attachments.has(attachmentId),
      ),
    ),
    "automated findings must reference registered attachments",
  );
  const physicalAttachmentPaths = await listAttachmentFiles(
    path.join(directory, "attachments"),
    directory,
  );
  const declaredAttachmentPaths = [...attachments.values()]
    .map((attachment) => attachment.relativePath)
    .sort();
  assert(
    JSON.stringify(physicalAttachmentPaths) ===
      JSON.stringify(declaredAttachmentPaths),
    "attachments directory contains missing or unregistered files",
  );

  const expectedEventChain = buildEventChain(caseDocument.collectionEvents);
  assert(
    JSON.stringify(expectedEventChain) === JSON.stringify(manifest.eventChain),
    "event append chain mismatch",
  );
  const expectedAttachmentChain = buildAttachmentChain(caseDocument.attachments);
  assert(
    JSON.stringify(expectedAttachmentChain) ===
      JSON.stringify(manifest.attachmentChain),
    "attachment append chain mismatch",
  );
  const expectedRootDigest = sha256Text(
    [
      "HiddenShield-Rights-Evidence-Pack-Root-v1",
      sha256(caseBytes),
      expectedEventChain.rootDigest,
      expectedAttachmentChain.rootDigest,
    ].join("\n"),
  );
  assert(
    manifest.integrity.algorithm ===
      "sha256_case_event_attachment_roots_v1" &&
      manifest.integrity.rootDigest === expectedRootDigest,
    "case bundle root digest mismatch",
  );
  assert(manifest.signature.status === "not_signed", "fixture must remain unsigned");
  assert(
    manifest.trustedTime.status === "not_timestamped",
    "fixture must remain without package trusted time",
  );
  return {
    matched: true,
    message: "matched",
    caseDocument,
    manifest,
    attachments,
    attachmentBytes,
  };
}

function buildEventChain(events) {
  const genesis = "HiddenShield-Rights-Evidence-Pack-Event-Chain-v1";
  let previousChainDigest = sha256Text(genesis);
  const entries = events.map((event, index) => {
    const sequence = index + 1;
    assert(event.sequence === sequence, `event sequence mismatch at ${event.eventId}`);
    const eventDigest = sha256Text(stableStringify(event));
    const chainDigest = sha256Text(
      `${sequence}\n${event.eventId}\n${eventDigest}\n${previousChainDigest}`,
    );
    const entry = {
      sequence,
      eventId: event.eventId,
      eventDigest,
      previousChainDigest,
      chainDigest,
    };
    previousChainDigest = chainDigest;
    return entry;
  });
  return {
    algorithm: "sha256_append_chain_v1",
    genesis,
    entries,
    rootDigest: previousChainDigest,
  };
}

function buildAttachmentChain(attachments) {
  const genesis = "HiddenShield-Rights-Evidence-Pack-Attachment-Chain-v1";
  let previousChainDigest = sha256Text(genesis);
  const entries = attachments.map((attachment, index) => {
    const sequence = index + 1;
    assert(
      attachment.sequence === sequence,
      `attachment sequence mismatch at ${attachment.attachmentId}`,
    );
    const chainDigest = sha256Text(
      [
        sequence,
        attachment.attachmentId,
        attachment.relativePath,
        attachment.role,
        attachment.bytes,
        attachment.sha256,
        previousChainDigest,
      ].join("\n"),
    );
    const entry = {
      sequence,
      attachmentId: attachment.attachmentId,
      path: attachment.relativePath,
      role: attachment.role,
      fileBytes: attachment.bytes,
      fileSha256: attachment.sha256,
      previousChainDigest,
      chainDigest,
    };
    previousChainDigest = chainDigest;
    return entry;
  });
  return {
    algorithm: "sha256_append_chain_v1",
    genesis,
    entries,
    rootDigest: previousChainDigest,
  };
}

function stableStringify(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

async function safeBundlePath(bundleRoot, relativePath) {
  assert(
    typeof relativePath === "string" &&
      relativePath.startsWith("attachments/") &&
      !relativePath.includes("\\"),
    `invalid attachment relative path ${relativePath}`,
  );
  const resolved = path.resolve(bundleRoot, relativePath);
  assert(
    resolved.startsWith(`${path.resolve(bundleRoot)}${path.sep}`),
    `attachment path escapes bundle: ${relativePath}`,
  );
  const fileStat = await lstat(resolved);
  assert(
    fileStat.isFile() && !fileStat.isSymbolicLink(),
    `attachment must be a regular non-symlink file: ${relativePath}`,
  );
  const [realBundleRoot, realFilePath] = await Promise.all([
    realpath(bundleRoot),
    realpath(resolved),
  ]);
  assert(
    realFilePath.startsWith(`${realBundleRoot}${path.sep}`),
    `attachment real path escapes bundle: ${relativePath}`,
  );
  return realFilePath;
}

async function listAttachmentFiles(directory, bundleRoot) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    assert(!entry.isSymbolicLink(), `symbolic links are forbidden: ${entryPath}`);
    if (entry.isDirectory()) {
      files.push(...(await listAttachmentFiles(entryPath, bundleRoot)));
    } else {
      assert(entry.isFile(), `unsupported attachment entry: ${entryPath}`);
      files.push(path.relative(bundleRoot, entryPath).replaceAll("\\", "/"));
    }
  }
  return files.sort();
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function sha256Text(value) {
  return sha256(Buffer.from(value, "utf8"));
}

async function expectRejected(action, message) {
  try {
    await action();
  } catch {
    return;
  }
  throw new Error(`Rights evidence pack R4 bundle verification failed: ${message}`);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(`Rights evidence pack R4 bundle verification failed: ${message}`);
  }
}
