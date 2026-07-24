#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    shell: process.platform === 'win32',
    windowsHide: true,
    ...options,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(' ')} failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
    );
  }
  return `${result.stdout}\n${result.stderr}`;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

run(npmCommand, ['--prefix', 'packages/public-rights-sdk', 'run', 'build']);
const output = run(npmCommand, ['pack', '--dry-run'], {
  cwd: 'packages/public-rights-sdk',
});

for (const token of [
  'package: @hiddenshield/public-rights-sdk@',
  'README.md',
  'package.json',
  'dist/index.js',
  'dist/index.d.ts',
]) {
  assert(output.includes(token), `SDK pack dry-run must include ${token}`);
}

for (const forbidden of [
  'tmp-ui-qa/',
  'feedback-backend/',
  'mobile_app/',
  'src-tauri/',
  'watermark-core/',
  'docs/',
  'hidden-shield@',
]) {
  assert(!output.includes(forbidden), `SDK pack dry-run must not include ${forbidden}`);
}

console.log('public rights SDK pack dry-run passed');
