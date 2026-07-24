import { readFile } from "node:fs/promises";

const reportPaths = process.argv.slice(2);

if (reportPaths.length === 0) {
  console.error(
    "Usage: node scripts/verify-watermark-release-gate.mjs <report.json> [more-report.json...]",
  );
  process.exit(2);
}

const audioProductionTransforms = new Set([
  "baseline_wav",
  "wav_reencode",
  "volume_80",
  "volume_120",
  "resample_22050",
  "mp3_192_roundtrip",
]);

const audioObservationTransforms = new Set(["clip_10s_middle"]);

let checkedReports = 0;
let failed = false;

for (const reportPath of reportPaths) {
  const report = JSON.parse(await readFile(reportPath, "utf8"));
  const results = Array.isArray(report.results) ? report.results : [];
  checkedReports += 1;

  const imageResults = results.filter((result) => result.media === "image");
  const audioProductionResults = results.filter(
    (result) =>
      result.media === "audio" && audioProductionTransforms.has(result.transform),
  );
  const audioObservationResults = results.filter(
    (result) =>
      result.media === "audio" &&
      (audioObservationTransforms.has(result.transform) ||
        result.transform.startsWith("matrix_")),
  );
  const unknownAudioResults = results.filter(
    (result) =>
      result.media === "audio" &&
      !audioProductionTransforms.has(result.transform) &&
      !audioObservationTransforms.has(result.transform) &&
      !result.transform.startsWith("matrix_"),
  );

  const imageFailures = imageResults.filter((result) => !result.success);
  const audioProductionFailures = audioProductionResults.filter(
    (result) => !result.success,
  );

  console.log(`Watermark release gate: ${reportPath}`);
  printGroup("image production", imageResults);
  printGroup("audio production", audioProductionResults);
  printGroup("audio observation", audioObservationResults, { observation: true });

  if (unknownAudioResults.length > 0) {
    failed = true;
    console.error(
      `Unknown audio transforms must be classified before release: ${summarizeTransforms(
        unknownAudioResults,
      )}`,
    );
  }

  if (imageFailures.length > 0) {
    failed = true;
    console.error(`Image production failures: ${formatFailures(imageFailures)}`);
  }

  if (audioProductionFailures.length > 0) {
    failed = true;
    console.error(
      `Audio production failures: ${formatFailures(audioProductionFailures)}`,
    );
  }
}

if (failed) {
  process.exit(1);
}

console.log(`Watermark release gate passed (${checkedReports} report(s))`);

function printGroup(label, results, options = {}) {
  if (results.length === 0) {
    console.log(`  ${label}: skipped`);
    return;
  }

  const passed = results.filter((result) => result.success).length;
  const suffix = options.observation ? " (not release-blocking)" : "";
  console.log(`  ${label}: ${passed}/${results.length} passed${suffix}`);
}

function formatFailures(results) {
  return results
    .slice(0, 10)
    .map(
      (result) =>
        `${result.media}:${result.source}:${result.transform} (${result.error ?? "failed"})`,
    )
    .join("; ");
}

function summarizeTransforms(results) {
  const counts = new Map();
  for (const result of results) {
    counts.set(result.transform, (counts.get(result.transform) ?? 0) + 1);
  }
  return Array.from(counts.entries())
    .map(([name, count]) => `${name} x${count}`)
    .join(", ");
}
