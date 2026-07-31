import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const outputDir = resolve(
  process.argv[2] ?? "watermark-core/target/promo-performance-fixtures-v1",
);
const imageDir = join(outputDir, "images");
const audioDir = join(outputDir, "audio");

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(imageDir, { recursive: true });
mkdirSync(audioDir, { recursive: true });

const imageRows = [];
const audioRows = [];

for (let index = 0; index < 5; index += 1) {
  const seed = 1001 + index * 97;
  const outputPath = join(imageDir, `photo-${index + 1}.jpg`);
  const filter = [
    "nullsrc=s=4000x3000",
    `geq=random(${seed})*255:128+64*sin(X/${23 + index * 2})+random(${seed + 1})*48:128+64*cos(Y/${29 + index * 2})+random(${seed + 2})*48`,
  ].join(",");
  execFileSync("ffmpeg", [
    "-hide_banner",
    "-loglevel",
    "error",
    "-y",
    "-f",
    "lavfi",
    "-i",
    filter,
    "-frames:v",
    "1",
    "-q:v",
    "2",
    outputPath,
  ]);
  const bytes = statSync(outputPath).size;
  assertRange(bytes, 8 * 1024 * 1024, 12 * 1024 * 1024, outputPath);
  imageRows.push({
    file: outputPath.replaceAll("\\", "/"),
    bytes,
    mib: round(bytes / 1024 / 1024),
    width: 4000,
    height: 3000,
    megapixels: 12,
    format: "jpeg",
  });
}

const amplitudes = [0.04, 0.045, 0.05, 0.055, 0.06];
for (let index = 0; index < 5; index += 1) {
  const amplitude = amplitudes[index].toFixed(3);
  const outputPath = join(audioDir, `track-${index + 1}.flac`);
  execFileSync("ffmpeg", [
    "-hide_banner",
    "-loglevel",
    "error",
    "-y",
    "-f",
    "lavfi",
    "-i",
    "sine=frequency=220:sample_rate=44100:duration=180",
    "-f",
    "lavfi",
    "-i",
    `anoisesrc=color=white:amplitude=${amplitude}:sample_rate=44100:duration=180:seed=${2001 + index * 31}`,
    "-f",
    "lavfi",
    "-i",
    `anoisesrc=color=white:amplitude=${amplitude}:sample_rate=44100:duration=180:seed=${3001 + index * 37}`,
    "-filter_complex",
    "[0:a][1:a]amix=inputs=2:weights='0.8 0.2'[l];[0:a][2:a]amix=inputs=2:weights='0.75 0.25'[r];[l][r]join=inputs=2:channel_layout=stereo",
    "-c:a",
    "flac",
    "-sample_fmt",
    "s16",
    outputPath,
  ]);
  const bytes = statSync(outputPath).size;
  assertRange(bytes, 18 * 1024 * 1024, 22 * 1024 * 1024, outputPath);
  audioRows.push({
    file: outputPath.replaceAll("\\", "/"),
    bytes,
    mib: round(bytes / 1024 / 1024),
    durationSeconds: 180,
    sampleRate: 44100,
    channels: 2,
    bitsPerSample: 16,
    format: "flac",
  });
}

const manifest = {
  schemaVersion: "hiddenshield-promo-performance-fixtures-v1",
  generatedAt: new Date().toISOString(),
  deterministic: true,
  imageBucket: {
    requested: "8–12 MiB, approximately 12 MP JPEG",
    count: imageRows.length,
    fixtures: imageRows,
  },
  audioBucket: {
    requested: "18–22 MiB, approximately 3 minute FLAC",
    count: audioRows.length,
    fixtures: audioRows,
  },
};

writeFileSync(
  join(outputDir, "manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
  "utf8",
);
console.log(`Generated promo performance fixtures: ${outputDir}`);

function assertRange(value, minimum, maximum, file) {
  if (value < minimum || value > maximum) {
    throw new Error(
      `${file} is outside the required byte range: ${value} not in ${minimum}..${maximum}`,
    );
  }
}

function round(value) {
  return Math.round(value * 100) / 100;
}
