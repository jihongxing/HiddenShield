import { readFileSync } from 'node:fs';

const desktop = readFileSync('src/lib/tauri-api.ts', 'utf8');
const mobile = readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8');

const desktopFields = extractDesktopFields(desktop);
const mobileFields = extractMobileFields(mobile);

const expected = [
  'totalUnits',
  'totalEvents',
  'imageUnits',
  'videoUnits',
  'audioUnits',
  'lastUsedAt',
  'lastFeatureName',
];

for (const field of expected) {
  assert(desktopFields.has(field), `desktop UsageLedgerSummary missing ${field}`);
  assert(mobileFields.has(field), `mobile UsageLedgerSummary missing ${field}`);
}

console.log('Usage ledger contract OK');

function extractDesktopFields(source) {
  const match = source.match(/export interface UsageLedgerSummary\s*{([\s\S]*?)}/);
  assert(match, 'desktop UsageLedgerSummary interface not found');
  const body = match[1];
  return new Set(
    [...body.matchAll(/^\s*(\w+)\s*:/gm)].map((item) => item[1]),
  );
}

function extractMobileFields(source) {
  const match = source.match(/class UsageLedgerSummary\s*{([\s\S]*?)^\}/m);
  assert(match, 'mobile UsageLedgerSummary class not found');
  const body = match[1];
  const fields = new Set();
  for (const line of body.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('final ')) continue;
    const parts = trimmed.replace(/;$/, '').split(/\s+/);
    fields.add(parts[2]);
  }
  return fields;
}

function assert(condition, message) {
  if (!condition) {
    console.error(`Usage ledger contract failed: ${message}`);
    process.exit(1);
  }
}
