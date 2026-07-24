import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const backendUrl = process.env.HIDDENSHIELD_QA_BACKEND_URL ?? 'http://127.0.0.1:43188';
const deviceId = process.env.HIDDENSHIELD_IOS_DEVICE_ID ?? discoverIosDeviceId();
const outputDir = resolve('tmp-ui-qa', 'ios-real-runtime-status');
mkdirSync(outputDir, { recursive: true });

const command = [
  'flutter',
  'run',
  '-d',
  deviceId,
  '-t',
  'mobile_app/tool/ios_real_runtime_qa.dart',
  `--dart-define=HIDDENSHIELD_QA_BACKEND_URL=${backendUrl}`,
];

console.log(`iOS runtime QA device: ${deviceId}`);
console.log(`iOS runtime QA command: ${command.join(' ')}`);
writeFileSync(
  resolve(outputDir, 'ios-real-runtime-status-qa-command.txt'),
  `${command.join(' ')}\n`,
  'utf8',
);

const result = spawnSync(command[0], command.slice(1), {
  cwd: process.cwd(),
  stdio: 'inherit',
  shell: process.platform === 'win32',
  windowsHide: true,
});

if (result.status !== 0) {
  process.exitCode = result.status ?? 1;
  throw new Error(`iOS runtime QA failed with status ${result.status}`);
}

function discoverIosDeviceId() {
  const result = spawnSync('flutter', ['devices'], {
    cwd: process.cwd(),
    encoding: 'utf8',
    shell: process.platform === 'win32',
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(`flutter devices failed with status ${result.status}`);
  }
  const lines = String(result.stdout ?? '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const candidates = lines.filter(
    (line) =>
      /iPhone|iPad|Simulator|iOS/i.test(line) &&
      !/No devices|Found \d+ connected/i.test(line),
  );
  const line = candidates[0];
  if (!line) {
    throw new Error(
      `No iOS device found. Run on macOS with Xcode/iOS Simulator, or set HIDDENSHIELD_IOS_DEVICE_ID. flutter devices output:\n${lines.join('\n')}`,
    );
  }
  const match = line.match(/\(([^)]+)\)$/);
  if (match?.[1]) {
    return match[1];
  }
  const parts = line.split('•').map((part) => part.trim()).filter(Boolean);
  return parts[0] ?? line;
}
