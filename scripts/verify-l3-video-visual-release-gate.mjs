import { spawn } from 'node:child_process';
import { appendFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const runId = Date.now().toString();
const evidenceDir = resolve(rootDir, 'tmp-ui-qa', 'l3-video-visual-release-gate', runId);
const rawLogPath = resolve(evidenceDir, 'raw-output.log');
mkdirSync(evidenceDir, { recursive: true });
writeFileSync(rawLogPath, '');

const env = {
  ...process.env,
  HIDDENSHIELD_L3_FULL_RELEASE_POOL: '1',
};

const timeoutMs = Number(process.env.HIDDENSHIELD_L3_RELEASE_GATE_TIMEOUT_MS ?? 3_600_000);
const result = await runCargoL3ReleasePool(env, timeoutMs);
const parsed = parseL3ReleasePoolOutput(`${result.stdout}\n${result.stderr}`);
if (result.timedOut) {
  parsed.blockers.unshift(`Gate timed out after ${timeoutMs}ms before the full 24-sample pool completed.`);
  parsed.pass = false;
}
const pass = result.code === 0 && !result.timedOut && parsed.pass;
const evidence = {
  runId,
  gate: 'watermark:l3-video-visual-release-gate',
  command:
    'cargo test --release --manifest-path src-tauri/Cargo.toml l3_2k_high_bitrate_release_sample_pool_records_thresholds --lib -- --nocapture --test-threads=1',
  fullReleasePool: env.HIDDENSHIELD_L3_FULL_RELEASE_POOL === '1',
  exitCode: result.code,
  timedOut: result.timedOut,
  timeoutMs,
  pass,
  thresholds: {
    h264HdPerSampleMin: 0.95,
    h264HdGroupMeanMin: 0.97,
    h264LtPerSampleMin: 0.95,
    h264LtGroupMeanMin: 0.98,
    h264MtPerSampleMin: 0.95,
    h264MtGroupMeanMin: 0.98,
    hevcHdPerSampleMin: 0.97,
    hevcHdGroupMeanMin: 0.99,
    hevcMixPerSampleMin: 0.97,
    hevcMixGroupMeanMin: 0.99,
  },
  parsed,
};

writeFileSync(
  resolve(evidenceDir, 'l3-video-visual-release-gate.json'),
  `${JSON.stringify(evidence, null, 2)}\n`,
);
writeFileSync(
  resolve(evidenceDir, 'l3-video-visual-release-gate.md'),
  renderMarkdown(evidence),
);

console.log(`L3 video visual release gate evidence: ${evidenceDir}`);
if (!pass) {
  console.error('L3 video visual release gate failed.');
  console.error(parsed.blockers.join('\n'));
  process.exit(1);
}
console.log('L3 video visual release gate passed.');

function runCargoL3ReleasePool(extraEnv, timeout) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(
      command('cargo'),
      [
        'test',
        '--release',
        '--manifest-path',
        'src-tauri/Cargo.toml',
        'l3_2k_high_bitrate_release_sample_pool_records_thresholds',
        '--lib',
        '--',
        '--nocapture',
        '--test-threads=1',
      ],
      {
        cwd: rootDir,
        env: extraEnv,
        shell: process.platform === 'win32',
      },
    );
    let stdout = '';
    let stderr = '';
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      child.kill('SIGKILL');
      resolvePromise({ code: null, stdout, stderr, timedOut: true });
    }, timeout);
    child.stdout.on('data', (chunk) => {
      const text = chunk.toString();
      stdout += text;
      appendFileSync(rawLogPath, text);
      process.stdout.write(text);
    });
    child.stderr.on('data', (chunk) => {
      const text = chunk.toString();
      stderr += text;
      appendFileSync(rawLogPath, text);
      process.stderr.write(text);
    });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolvePromise({ code, stdout, stderr, timedOut: false });
    });
  });
}

function parseL3ReleasePoolOutput(output) {
  const samplePattern =
    /l3_2k_high_bitrate_release_sample_pool_case=(?<caseName>\S+) group=(?<group>\S+) risk_profile=(?<riskProfile>\S+) expected_attribution=(?<expectedAttribution>\S+) observed_attribution=(?<observedAttribution>\S+) source_duration_s=(?<duration>\d+) sampled_frames=(?<sampledFrames>\d+) max_regions=(?<maxRegions>\d+) region_selection=(?<regionSelection>\S+) resolution=(?<width>\d+)x(?<height>\d+) codec=(?<codec>\S+) first_pass_bitrate=(?<firstPassBitrate>\S+) second_pass_bitrate=(?<secondPassBitrate>\S+) min_confidence=(?<minConfidence>[0-9.]+) checked_frames=(?<checkedFrames>\d+) confidence=(?<confidence>[0-9.]+) self_check_status=(?<selfCheckStatus>\S+) ffmpeg_source_and_sample_ms=(?<ffmpegSourceAndSampleMs>\d+) core_embed_ms=(?<coreEmbedMs>\d+) ffmpeg_first_pass_ms=(?<ffmpegFirstPassMs>\d+) ffmpeg_second_pass_ms=(?<ffmpegSecondPassMs>\d+) ffmpeg_decode_second_pass_ms=(?<ffmpegDecodeSecondPassMs>\d+) core_self_check_ms=(?<coreSelfCheckMs>\d+) total_ms=(?<totalMs>\d+)/g;
  const skipPattern =
    /l3_2k_high_bitrate_release_sample_pool_case=(?<caseName>\S+) group=(?<group>\S+) failure_attribution=encoder_unavailable skipped=libx265_unavailable/g;
  const summaryPattern =
    /l3_2k_high_bitrate_release_sample_pool_summary h264_hd_min_confidence=(?<min>[0-9.]+) h264_hd_avg_confidence=(?<avg>[0-9.]+) release_status=(?<status>\S+)/;

  const samples = [];
  for (const match of output.matchAll(samplePattern)) {
    const groups = match.groups;
    samples.push({
      caseName: groups.caseName,
      group: groups.group,
      riskProfile: groups.riskProfile,
      expectedAttribution: groups.expectedAttribution,
      observedAttribution: groups.observedAttribution,
      sampledFrames: Number(groups.sampledFrames),
      maxRegions: Number(groups.maxRegions),
      regionSelection: groups.regionSelection,
      resolution: `${groups.width}x${groups.height}`,
      codec: groups.codec,
      firstPassBitrate: groups.firstPassBitrate,
      secondPassBitrate: groups.secondPassBitrate,
      minConfidence: Number(groups.minConfidence),
      checkedFrames: Number(groups.checkedFrames),
      confidence: Number(groups.confidence),
      selfCheckStatus: groups.selfCheckStatus,
      timingsMs: {
        ffmpegSourceAndSample: Number(groups.ffmpegSourceAndSampleMs),
        coreEmbed: Number(groups.coreEmbedMs),
        ffmpegFirstPass: Number(groups.ffmpegFirstPassMs),
        ffmpegSecondPass: Number(groups.ffmpegSecondPassMs),
        ffmpegDecodeSecondPass: Number(groups.ffmpegDecodeSecondPassMs),
        coreSelfCheck: Number(groups.coreSelfCheckMs),
        total: Number(groups.totalMs),
      },
    });
  }

  const skipped = Array.from(output.matchAll(skipPattern), (match) => ({
    caseName: match.groups.caseName,
    group: match.groups.group,
    reason: 'encoder_unavailable',
  }));
  const summaryMatch = output.match(summaryPattern);
  const summary = summaryMatch
    ? {
        h264HdMinConfidence: Number(summaryMatch.groups.min),
        h264HdAvgConfidence: Number(summaryMatch.groups.avg),
        releaseStatus: summaryMatch.groups.status,
      }
    : null;

  const groups = summarizeGroups(samples);
  const blockers = [];
  requireCount(groups, 'H264-HD', 6, blockers);
  requireCount(groups, 'H264-LT', 4, blockers);
  requireCount(groups, 'H264-MT', 4, blockers);
  requireCount(groups, 'H264-RISK', 2, blockers);
  requireCount(groups, 'HEVC-HD', 4, blockers);
  requireCount(groups, 'HEVC-MIX', 4, blockers);

  if (skipped.length > 0) {
    blockers.push(`HEVC/full release samples skipped: ${skipped.map((item) => item.caseName).join(', ')}`);
  }

  requireGroup(samples, 'H264-HD', { min: 0.95, avg: 0.97 }, blockers);
  requireGroup(samples, 'H264-LT', { min: 0.95, avg: 0.98 }, blockers);
  requireGroup(samples, 'H264-MT', { min: 0.95, avg: 0.98 }, blockers);
  requireGroup(samples, 'HEVC-HD', { min: 0.97, avg: 0.99 }, blockers);
  requireGroup(samples, 'HEVC-MIX', { min: 0.97, avg: 0.99 }, blockers);

  for (const sample of samples.filter((item) => item.group === 'H264-RISK')) {
    if (sample.observedAttribution !== 'risk_boundary_expected') {
      blockers.push(`${sample.caseName} must remain risk_boundary_expected, got ${sample.observedAttribution}`);
    }
  }
  for (const sample of samples.filter((item) => item.group !== 'H264-RISK')) {
    if (sample.observedAttribution !== 'pass') {
      blockers.push(`${sample.caseName} expected pass, got ${sample.observedAttribution}`);
    }
  }
  if (!summary) {
    blockers.push('Missing H264-HD release summary line.');
  } else if (summary.releaseStatus !== 'release_thresholds_met') {
    blockers.push(`H264-HD release summary status is ${summary.releaseStatus}.`);
  }

  return {
    pass: blockers.length === 0,
    samples,
    skipped,
    groups,
    summary,
    blockers,
  };
}

function summarizeGroups(samples) {
  const groups = {};
  for (const sample of samples) {
    const item = groups[sample.group] ?? {
      count: 0,
      minConfidence: Number.POSITIVE_INFINITY,
      avgConfidence: 0,
      totalMs: 0,
    };
    item.count += 1;
    item.minConfidence = Math.min(item.minConfidence, sample.confidence);
    item.avgConfidence += sample.confidence;
    item.totalMs += sample.timingsMs.total;
    groups[sample.group] = item;
  }
  for (const item of Object.values(groups)) {
    item.avgConfidence = item.count === 0 ? 0 : item.avgConfidence / item.count;
  }
  return groups;
}

function requireCount(groups, group, expected, blockers) {
  const actual = groups[group]?.count ?? 0;
  if (actual !== expected) {
    blockers.push(`${group} expected ${expected} samples, got ${actual}.`);
  }
}

function requireGroup(samples, group, thresholds, blockers) {
  const groupSamples = samples.filter((sample) => sample.group === group);
  if (groupSamples.length === 0) return;
  const min = Math.min(...groupSamples.map((sample) => sample.confidence));
  const avg =
    groupSamples.reduce((sum, sample) => sum + sample.confidence, 0) /
    groupSamples.length;
  if (min < thresholds.min) {
    blockers.push(`${group} min confidence ${min.toFixed(3)} < ${thresholds.min.toFixed(3)}.`);
  }
  if (avg < thresholds.avg) {
    blockers.push(`${group} avg confidence ${avg.toFixed(3)} < ${thresholds.avg.toFixed(3)}.`);
  }
}

function renderMarkdown(evidence) {
  const lines = [
    '# HiddenShield L3 Video Visual Release Gate',
    '',
    `- runId: \`${evidence.runId}\``,
    `- pass: \`${evidence.pass}\``,
    `- fullReleasePool: \`${evidence.fullReleasePool}\``,
    `- exitCode: \`${evidence.exitCode}\``,
    `- timedOut: \`${evidence.timedOut}\``,
    `- timeoutMs: \`${evidence.timeoutMs}\``,
    `- rawLog: \`raw-output.log\``,
    '',
    '| group | count | min confidence | avg confidence | total ms |',
    '| --- | ---: | ---: | ---: | ---: |',
  ];
  for (const [group, item] of Object.entries(evidence.parsed.groups)) {
    lines.push(
      `| ${group} | ${item.count} | ${item.minConfidence.toFixed(3)} | ${item.avgConfidence.toFixed(3)} | ${item.totalMs} |`,
    );
  }
  lines.push('', '## Blockers', '');
  if (evidence.parsed.blockers.length === 0) {
    lines.push('- none');
  } else {
    for (const blocker of evidence.parsed.blockers) {
      lines.push(`- ${blocker}`);
    }
  }
  lines.push('', '## Samples', '');
  lines.push('| group | sample | attribution | confidence | checked frames | total ms |');
  lines.push('| --- | --- | --- | ---: | ---: | ---: |');
  for (const sample of evidence.parsed.samples) {
    lines.push(
      `| ${sample.group} | ${sample.caseName} | ${sample.observedAttribution} | ${sample.confidence.toFixed(3)} | ${sample.checkedFrames} | ${sample.timingsMs.total} |`,
    );
  }
  if (evidence.parsed.skipped.length > 0) {
    lines.push('', '## Skipped', '');
    for (const skipped of evidence.parsed.skipped) {
      lines.push(`- ${skipped.group}: ${skipped.caseName} (${skipped.reason})`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function command(name) {
  if (process.platform !== 'win32') return name;
  if (name === 'cargo') return 'cargo.exe';
  return name;
}
