import { createHash } from "node:crypto";
import { lstat, mkdir, readFile, realpath, writeFile } from "node:fs/promises";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const sourcePath = path.join(
  repoRoot,
  "docs",
  "contracts",
  "rights-evidence-pack-bundle-v1.source.json",
);
const source = JSON.parse(await readFile(sourcePath, "utf8"));
const bundleDir = path.join(repoRoot, source.bundleDir);
const baseCase = JSON.parse(
  await readFile(path.join(repoRoot, source.caseFixturePath), "utf8"),
);

const attachments = [];
for (const attachment of source.attachments) {
  const absolutePath = await safeBundlePath(bundleDir, attachment.relativePath);
  const bytes = await readFile(absolutePath);
  attachments.push({
    ...attachment,
    bytes: bytes.length,
    sha256: sha256(bytes),
  });
}

const capture = attachments.find((attachment) => attachment.role === "capture");
if (!capture) throw new Error("R4 bundle source must contain a capture attachment");

const collectionEvents = [
  {
    eventId: "collection-event-0001",
    sequence: 1,
    eventType: "original_received",
    occurredAt: "2026-07-14T11:20:00.000Z",
    timeStatus: "device_claimed",
    actor: "fixture_operator",
    attachmentIds: ["attachment-original-0001"],
    description: "接收合成原件 fixture，并按原始字节计算摘要。"
  },
  {
    eventId: "collection-event-0002",
    sequence: 2,
    eventType: "working_copy_derived",
    occurredAt: "2026-07-14T11:25:00.000Z",
    timeStatus: "device_claimed",
    actor: "hidden_shield_local",
    attachmentIds: ["attachment-original-0001", "attachment-working-copy-0002"],
    description: "从合成原件生成分析工作副本，原件保持不变。"
  },
  {
    eventId: "collection-event-0003",
    sequence: 3,
    eventType: "external_object_captured",
    occurredAt: "2026-07-14T11:30:00.000Z",
    timeStatus: "unverified",
    actor: "fixture_operator",
    attachmentIds: ["attachment-capture-0003"],
    description: "记录合成争议页面样本；该时间未经可信时间服务确认。"
  },
  {
    eventId: "collection-event-0004",
    sequence: 4,
    eventType: "external_receipt_imported",
    occurredAt: "2026-07-14T11:32:00.000Z",
    timeStatus: "unverified",
    actor: "fixture_operator",
    attachmentIds: ["attachment-external-receipt-0004"],
    description: "导入合成外部回执；未验证签名、签发主体或时间。"
  }
];

const caseDocument = {
  ...baseCase,
  collectionEvents,
  infringementSamples: baseCase.infringementSamples.map((sample) => ({
    ...sample,
    sha256: capture.sha256,
    bytes: capture.bytes,
    attachmentId: capture.attachmentId,
  })),
  automatedFindings: baseCase.automatedFindings.map((finding) => ({
    ...finding,
    inputAttachmentIds: [capture.attachmentId],
  })),
  attachments,
  bundleContract: {
    schemaVersion: 1,
    manifestPath: "case-manifest.json",
    attachmentRoot: "attachments/",
    attachmentRoles: source.attachmentRoles,
    eventChainAlgorithm: "sha256_append_chain_v1",
    attachmentChainAlgorithm: "sha256_append_chain_v1",
  },
};

const caseJson = `${JSON.stringify(caseDocument, null, 2)}\n`;
const caseBytes = Buffer.from(caseJson);
const eventChain = buildEventChain(collectionEvents);
const attachmentChain = buildAttachmentChain(attachments);
const manifestRootDigest = sha256Text(
  [
    "HiddenShield-Rights-Evidence-Pack-Root-v1",
    sha256(caseBytes),
    eventChain.rootDigest,
    attachmentChain.rootDigest,
  ].join("\n"),
);
const manifest = {
  schemaVersion: 1,
  manifestType: "rights_evidence_pack_manifest",
  packId: caseDocument.packId,
  caseId: caseDocument.case.caseId,
  generatedAt: caseDocument.generatedAt,
  directoryContract: {
    caseDocument: "case.json",
    manifest: "case-manifest.json",
    attachmentRoot: "attachments/",
    allowedTopLevelEntries: ["case.json", "case-manifest.json", "attachments"],
  },
  caseFile: {
    path: "case.json",
    bytes: caseBytes.length,
    sha256: sha256(caseBytes),
  },
  attachmentRoles: {
    original: {
      immutable: true,
      requiresDerivedFrom: false,
      meaning: "Claimed source material; role does not prove authorship or ownership.",
    },
    working_copy: {
      immutable: true,
      requiresDerivedFrom: true,
      meaning: "Derived analysis material that never replaces its source attachment.",
    },
    capture: {
      immutable: true,
      requiresDerivedFrom: false,
      meaning: "External disputed-object capture; role does not imply trusted collection.",
    },
    external_receipt: {
      immutable: true,
      requiresDerivedFrom: false,
      meaning: "Externally issued artifact; issuer, signature, and time remain independently evaluated.",
    },
  },
  files: attachments.map((attachment) => ({
    attachmentId: attachment.attachmentId,
    path: attachment.relativePath,
    role: attachment.role,
    bytes: attachment.bytes,
    sha256: attachment.sha256,
  })),
  eventChain,
  attachmentChain,
  integrity: {
    algorithm: "sha256_case_event_attachment_roots_v1",
    rootDigest: manifestRootDigest,
  },
  signature: {
    status: "not_signed",
  },
  trustedTime: {
    status: "not_timestamped",
  },
};

await mkdir(bundleDir, { recursive: true });
await writeFile(path.join(bundleDir, "case.json"), caseJson);
await writeFile(
  path.join(bundleDir, "case-manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
);

console.log(
  JSON.stringify(
    {
      status: "generated",
      bundleDir: source.bundleDir,
      attachmentCount: attachments.length,
      eventCount: collectionEvents.length,
      eventRootDigest: eventChain.rootDigest,
      attachmentRootDigest: attachmentChain.rootDigest,
      rootDigest: manifestRootDigest,
    },
    null,
    2,
  ),
);

function buildEventChain(events) {
  const genesis = "HiddenShield-Rights-Evidence-Pack-Event-Chain-v1";
  let previousChainDigest = sha256Text(genesis);
  const entries = events.map((event, index) => {
    const sequence = index + 1;
    if (event.sequence !== sequence) {
      throw new Error(`collection event sequence mismatch at ${event.eventId}`);
    }
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

function buildAttachmentChain(attachmentEntries) {
  const genesis = "HiddenShield-Rights-Evidence-Pack-Attachment-Chain-v1";
  let previousChainDigest = sha256Text(genesis);
  const entries = attachmentEntries.map((attachment, index) => {
    const sequence = index + 1;
    if (attachment.sequence !== sequence) {
      throw new Error(`attachment sequence mismatch at ${attachment.attachmentId}`);
    }
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
  if (
    typeof relativePath !== "string" ||
    !relativePath.startsWith("attachments/") ||
    relativePath.includes("\\")
  ) {
    throw new Error(`invalid attachment relative path: ${relativePath}`);
  }
  const resolved = path.resolve(bundleRoot, relativePath);
  if (!resolved.startsWith(`${path.resolve(bundleRoot)}${path.sep}`)) {
    throw new Error(`attachment path escapes bundle: ${relativePath}`);
  }
  const fileStat = await lstat(resolved);
  if (fileStat.isSymbolicLink() || !fileStat.isFile()) {
    throw new Error(`attachment must be a regular non-symlink file: ${relativePath}`);
  }
  const [realBundleRoot, realFilePath] = await Promise.all([
    realpath(bundleRoot),
    realpath(resolved),
  ]);
  if (!realFilePath.startsWith(`${realBundleRoot}${path.sep}`)) {
    throw new Error(`attachment real path escapes bundle: ${relativePath}`);
  }
  return realFilePath;
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function sha256Text(value) {
  return sha256(Buffer.from(value, "utf8"));
}
