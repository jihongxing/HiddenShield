import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = process.cwd();

const scanRoots = [
  'src',
  'src-tauri/src',
  'mobile_app/lib',
  'mobile_app/test',
  'mobile_app/rust/src',
  'feedback-backend/src',
  'scripts',
  'packages',
];

const allowedExtensions = new Set([
  '.dart',
  '.json',
  '.mjs',
  '.rs',
  '.ts',
  '.vue',
]);

const oldLiteralUidPattern = /\bHS-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}\b/g;
const oldRegexUidPattern =
  /HS-\[[^\]]+\]\{4\}-\[[^\]]+\]\{4\}-\[[^\]]+\]\{4\}/g;
const formalUidPattern =
  /HS-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}/;

const requiredLongFormatFiles = {
  'src/lib/user-facing-errors.ts': 'formalWatermarkUidPattern',
  'src/views/WorkbenchView.vue': 'HS-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}',
  'mobile_app/lib/features/workspace/rewrite_preflight.dart':
    'HS-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}',
  'src/lib/tauri-api.ts': 'HS-26A47D91-CA8F13B4-A9C0D2E1-F3456789',
};

const findings = [];

for (const scanRoot of scanRoots) {
  for (const file of listFiles(join(root, scanRoot))) {
    if (relative(root, file).replaceAll('\\', '/') === 'scripts/verify-watermark-uid-format-contract.mjs') {
      continue;
    }
    const text = readFileSync(file, 'utf8');
    collectMatches(file, text, oldLiteralUidPattern, 'old literal UID');
    collectMatches(file, text, oldRegexUidPattern, 'old UID regex');
  }
}

for (const [file, token] of Object.entries(requiredLongFormatFiles)) {
  const text = readFileSync(join(root, file), 'utf8');
  if (!text.includes(token)) {
    findings.push(`${file}: missing required long UID format token ${JSON.stringify(token)}`);
  }
}

const packageJson = readFileSync(join(root, 'package.json'), 'utf8');
if (
  !packageJson.includes('"watermark:uid-format-contract"') ||
  !packageJson.includes('verify-watermark-uid-format-contract.mjs')
) {
  findings.push('package.json must expose watermark:uid-format-contract');
}

if (findings.length > 0) {
  console.error('Watermark UID format contract failed:');
  for (const finding of findings) {
    console.error(`- ${finding}`);
  }
  process.exit(1);
}

console.log('Watermark UID format contract OK');

function collectMatches(file, text, pattern, label) {
  pattern.lastIndex = 0;
  for (const match of text.matchAll(pattern)) {
    const line = lineNumber(text, match.index ?? 0);
    findings.push(`${relative(root, file)}:${line}: ${label} ${JSON.stringify(match[0])}`);
  }
}

function* listFiles(dir) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch (error) {
    if (error.code === 'ENOENT') {
      return;
    }
    throw error;
  }

  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (shouldSkipDirectory(entry.name)) {
        continue;
      }
      yield* listFiles(path);
      continue;
    }
    if (!entry.isFile()) {
      continue;
    }
    if (allowedExtensions.has(extension(path)) && statSync(path).size < 2_000_000) {
      yield path;
    }
  }
}

function shouldSkipDirectory(name) {
  return ['.dart_tool', 'build', 'dist', 'node_modules', 'target'].includes(name);
}

function extension(path) {
  const dot = path.lastIndexOf('.');
  return dot === -1 ? '' : path.slice(dot);
}

function lineNumber(text, index) {
  let line = 1;
  for (let i = 0; i < index; i += 1) {
    if (text.charCodeAt(i) === 10) {
      line += 1;
    }
  }
  return line;
}
