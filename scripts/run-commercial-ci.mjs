import { spawn } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const steps = [
  ['Commercial readiness contract', 'npm', ['run', 'commercial:contract']],
  ['Enterprise gateway contract', 'npm', ['run', 'enterprise:gateway-contract']],
  ['Enterprise gateway dry-run runtime QA', 'npm', ['run', 'enterprise:gateway-dry-run-runtime-qa']],
  ['Enterprise key issuance runtime QA', 'npm', ['run', 'enterprise:key-issuance-runtime-qa']],
  ['Enterprise public rights runtime QA', 'npm', ['run', 'enterprise:public-rights-runtime-qa']],
  ['Cloud DB portability contract', 'npm', ['run', 'cloud:db-portability-contract']],
  ['Cloud PostgreSQL migration contract', 'npm', ['run', 'cloud:postgres-migration-contract']],
  // Requires HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or Docker, so it is intentionally not part of default commercial CI yet.
  ['Public rights SDK pack dry-run', 'npm', ['run', 'rights:sdk-pack-dry-run']],
  ['Public rights production readiness contract', 'npm', ['run', 'public-rights:production-readiness-contract']],
  ['Commercial metrics contract', 'npm', ['run', 'commercial:metrics']],
  ['Dual consistency contract', 'npm', ['run', 'dual:contract']],
  ['Vault file_type backfill contract', 'npm', ['run', 'vault:file-type-backfill-contract']],
  ['Process first-principles contract', 'npm', ['run', 'process:first-principles-contract']],
  ['Billing contract', 'npm', ['run', 'billing:contract']],
  ['Usage ledger contract', 'npm', ['run', 'usage:contract']],
  ['Report export contract', 'npm', ['run', 'report:contract']],
  ['Team workspace contract', 'npm', ['run', 'team:contract']],
  ['Watermark architecture contract', 'npm', ['run', 'watermark:architecture-contract']],
  ['Watermark UID format contract', 'npm', ['run', 'watermark:uid-format-contract']],
  ['Watermark video phase contract', 'npm', ['run', 'watermark:video-phase-contract']],
  ['Watermark cross-end release gate', 'npm', ['run', 'watermark:cross-end-contract']],
  ['Desktop web build', 'npm', ['run', 'build']],
  ['Backend tests', 'cargo', ['test', '--manifest-path', 'feedback-backend/Cargo.toml', '--lib']],
  [
    'Tauri desktop release-scope tests',
    'cargo',
    ['test', '--manifest-path', 'src-tauri/Cargo.toml', '--lib', '--', '--skip', 'l3'],
  ],
  ['Flutter analyze', 'flutter', ['analyze'], { cwd: 'mobile_app' }],
  ['Flutter tests', 'flutter', ['test'], { cwd: 'mobile_app' }],
  ['Cloud sync CI', 'npm', ['run', 'cloud:ci']],
  ['Cloud video CI', 'npm', ['run', 'cloud-video:ci']],
];

console.log('HiddenShield commercial CI starting');
console.log('Cloud sync CI and cloud video CI run serially to avoid port conflicts.');

for (const [label, bin, args, options = {}] of steps) {
  await runStep(label, bin, args, options);
}

console.log('HiddenShield commercial CI OK');

function runStep(label, bin, args, options) {
  console.log(`\n=== ${label} ===`);
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command(bin), args, {
      cwd: options.cwd ? resolve(rootDir, options.cwd) : rootDir,
      env: process.env,
      stdio: 'inherit',
      shell: process.platform === 'win32',
    });
    child.on('exit', (code) => {
      if (code === 0) {
        resolvePromise();
      } else {
        reject(new Error(`${label} failed with exit code ${code}`));
      }
    });
    child.on('error', reject);
  });
}

function command(name) {
  if (process.platform !== 'win32') {
    return name;
  }
  if (name === 'npm') {
    return 'npm.cmd';
  }
  if (name === 'cargo') {
    return 'cargo.exe';
  }
  if (name === 'flutter') {
    return 'flutter.bat';
  }
  return name;
}
