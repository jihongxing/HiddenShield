import { execFileSync } from 'node:child_process';

const syncUrl = process.env.HIDDENSHIELD_SYNC_URL?.trim();
const pairingCode = process.env.HIDDENSHIELD_PAIRING_CODE?.trim();
const requiredTargets = [
  'aarch64-linux-android',
  'armv7-linux-androideabi',
  'i686-linux-android',
  'x86_64-linux-android',
];

function run(command, args, options = {}) {
  try {
    return execFileSync(command, args, {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: process.platform === 'win32',
      ...options,
    }).trim();
  } catch (error) {
    return null;
  }
}

function line(ok, label, detail) {
  const mark = ok ? '[OK]' : '[WARN]';
  console.log(`${mark} ${label}${detail ? ` - ${detail}` : ''}`);
}

console.log('HiddenShield mobile doctor\n');

const flutterVersion = run('flutter', ['--version']);
line(Boolean(flutterVersion), 'Flutter SDK', flutterVersion?.split('\n')[0] ?? 'flutter not found');

const devices = run('flutter', ['devices']);
line(Boolean(devices), 'Flutter devices', devices ? devices.split('\n')[0] : 'cannot list devices');
if (devices) {
  const deviceLines = devices
    .split('\n')
    .filter((item) => item.includes('•'))
    .filter((item) => {
      const lower = item.toLowerCase();
      return lower.includes('android') || lower.includes('ios');
    });
  line(deviceLines.length > 0, 'Android/iOS device', deviceLines[0] ?? 'no mobile device detected');
}

const installedTargets = run('rustup', ['target', 'list', '--installed']);
line(Boolean(installedTargets), 'rustup targets', installedTargets ? 'installed list available' : 'rustup not found');
if (installedTargets) {
  for (const target of requiredTargets) {
    line(installedTargets.split('\n').includes(target), `Rust target ${target}`);
  }
}

line(Boolean(syncUrl), 'HIDDENSHIELD_SYNC_URL', syncUrl ?? 'set to http://<desktop-lan-ip>:47219');
line(Boolean(pairingCode), 'HIDDENSHIELD_PAIRING_CODE', pairingCode ? 'provided' : 'set to desktop pairing code');

if (syncUrl) {
  const healthUrl = `${syncUrl.replace(/\/$/, '')}/api/mobile-sync/v1/health`;
  try {
    const response = await fetch(healthUrl);
    const body = await response.text();
    line(response.ok, 'Desktop sync health', `HTTP ${response.status} ${body.slice(0, 120)}`);
  } catch (error) {
    line(false, 'Desktop sync health', `${error}`);
  }
}

console.log('\nNext commands:');
console.log('  cd mobile_app');
console.log('  flutter analyze');
console.log('  flutter test');
console.log('  flutter run');
console.log('\nFor push verification:');
console.log('  HIDDENSHIELD_SYNC_URL=http://<desktop-lan-ip>:47219 HIDDENSHIELD_PAIRING_CODE=<code> npm run sync:verify');
