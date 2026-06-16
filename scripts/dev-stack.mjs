import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const defaultCloudUrl = 'http://127.0.0.1:43188';
const defaultFlutterWebPort = '43189';
const children = new Set();

const options = parseArgs(process.argv.slice(2));

main().catch((error) => {
  console.error(`[dev-stack] ${error?.stack ?? error}`);
  shutdown(1);
});

async function main() {
  printBanner();

  if (options.cloud) {
    if (await isCloudHealthy(options.cloudUrl)) {
      console.log(`[dev-stack] Cloud backend already healthy: ${options.cloudUrl}`);
    } else {
      startProcess('cloud', command('cargo'), [
        'run',
        '--manifest-path',
        'feedback-backend/Cargo.toml',
        '--',
        '--bind-addr',
        '127.0.0.1:43188',
        '--db-path',
        'feedback-backend/cloud.sqlite',
      ]);
      await waitForCloud(options.cloudUrl);
    }
  }

  if (options.desktop) {
    startProcess('desktop', command('npx'), ['tauri', 'dev']);
  }

  if (options.mobile !== 'none') {
    const args = ['run'];
    if (options.mobile === 'chrome') {
      args.push(
        '-d',
        'chrome',
        '--web-hostname',
        '127.0.0.1',
        '--web-port',
        options.flutterWebPort,
      );
      console.log(
        `[dev-stack] Flutter Web preview will use http://127.0.0.1:${options.flutterWebPort}`,
      );
    } else {
      args.push('-d', options.mobile);
    }
    startProcess('mobile', command('flutter'), args, {
      cwd: resolve(rootDir, 'mobile_app'),
    });
  }

  console.log('\n[dev-stack] Stack is starting. Press Ctrl+C here to stop child processes.');
  console.log('[dev-stack] Desktop must be tested through Tauri, not Vite preview.');
  console.log('[dev-stack] Flutter Web is a UI preview; Android/iOS device runs are closer to production.\n');
}

function parseArgs(args) {
  const parsed = {
    cloud: true,
    desktop: true,
    mobile: 'chrome',
    cloudUrl: process.env.HIDDENSHIELD_CLOUD_URL?.trim() || defaultCloudUrl,
    flutterWebPort:
      process.env.HIDDENSHIELD_FLUTTER_WEB_PORT?.trim() || defaultFlutterWebPort,
  };

  for (const arg of args) {
    if (arg === '--help' || arg === '-h') {
      printHelp();
      process.exit(0);
    }
    if (arg === '--no-cloud') {
      parsed.cloud = false;
    } else if (arg === '--no-desktop') {
      parsed.desktop = false;
    } else if (arg === '--no-mobile') {
      parsed.mobile = 'none';
    } else if (arg.startsWith('--mobile=')) {
      parsed.mobile = valueAfterEquals(arg, '--mobile=');
    } else if (arg.startsWith('--mobile-device=')) {
      parsed.mobile = valueAfterEquals(arg, '--mobile-device=');
    } else if (arg.startsWith('--cloud-url=')) {
      parsed.cloudUrl = valueAfterEquals(arg, '--cloud-url=');
    } else if (arg.startsWith('--flutter-web-port=')) {
      parsed.flutterWebPort = valueAfterEquals(arg, '--flutter-web-port=');
    } else {
      throw new Error(`Unknown option: ${arg}`);
    }
  }

  if (!parsed.mobile) {
    parsed.mobile = 'chrome';
  }

  return parsed;
}

function valueAfterEquals(arg, prefix) {
  const value = arg.slice(prefix.length).trim();
  if (!value) {
    throw new Error(`Missing value for ${prefix.slice(0, -1)}`);
  }
  return value;
}

function printBanner() {
  console.log('HiddenShield dev stack');
  console.log(`- cloud: ${options.cloud ? options.cloudUrl : 'disabled'}`);
  console.log(`- desktop: ${options.desktop ? 'tauri dev' : 'disabled'}`);
  console.log(`- mobile: ${options.mobile === 'none' ? 'disabled' : options.mobile}`);
  console.log('');
}

function printHelp() {
  console.log(`HiddenShield dev stack

Usage:
  npm run dev:stack
  npm run dev:stack -- --mobile-device=<flutter-device-id>
  npm run dev:stack -- --no-mobile

Options:
  --no-cloud                 Do not start/check the cloud backend.
  --no-desktop               Do not start Tauri desktop.
  --no-mobile                Do not start Flutter.
  --mobile=chrome            Start Flutter Web preview in Chrome. This is the default.
  --mobile-device=<id>       Start Flutter on a real Android/iOS device id.
  --cloud-url=<url>          Cloud health URL base. Default: ${defaultCloudUrl}
  --flutter-web-port=<port>  Flutter Web preview port. Default: ${defaultFlutterWebPort}
`);
}

function command(name) {
  if (process.platform !== 'win32') {
    return name;
  }
  if (name === 'npx') {
    return 'npx.cmd';
  }
  if (name === 'flutter') {
    return 'flutter.bat';
  }
  if (name === 'cargo') {
    return 'cargo.exe';
  }
  return name;
}

function startProcess(label, executable, args, spawnOptions = {}) {
  console.log(`[dev-stack] Starting ${label}: ${executable} ${args.join(' ')}`);
  const child = spawn(executable, args, {
    cwd: rootDir,
    env: process.env,
    stdio: ['inherit', 'pipe', 'pipe'],
    ...spawnOptions,
  });

  children.add(child);
  child.stdout.on('data', (chunk) => writePrefixed(label, chunk));
  child.stderr.on('data', (chunk) => writePrefixed(label, chunk));
  child.on('exit', (code, signal) => {
    children.delete(child);
    console.log(`[dev-stack] ${label} exited${signal ? ` by ${signal}` : ` with ${code}`}`);
  });
  child.on('error', (error) => {
    children.delete(child);
    console.error(`[dev-stack] ${label} failed to start: ${error.message}`);
  });

  return child;
}

function writePrefixed(label, chunk) {
  for (const line of chunk.toString().split(/\r?\n/)) {
    if (line.length > 0) {
      console.log(`[${label}] ${line}`);
    }
  }
}

async function waitForCloud(baseUrl) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30_000) {
    if (await isCloudHealthy(baseUrl)) {
      console.log(`[dev-stack] Cloud backend healthy: ${baseUrl}`);
      return;
    }
    await delay(500);
  }
  throw new Error(`Cloud backend did not become healthy within 30s: ${baseUrl}`);
}

async function isCloudHealthy(baseUrl) {
  try {
    const response = await fetch(`${baseUrl.replace(/\/$/, '')}/v1/health`);
    return response.ok;
  } catch (_) {
    return false;
  }
}

function shutdown(code = 0) {
  for (const child of children) {
    if (!child.killed) {
      child.kill();
    }
  }
  process.exit(code);
}

process.on('SIGINT', () => shutdown(0));
process.on('SIGTERM', () => shutdown(0));
