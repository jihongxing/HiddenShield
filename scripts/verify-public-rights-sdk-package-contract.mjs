import { readFileSync } from 'node:fs';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  sdkPackageJson: readFileSync('packages/public-rights-sdk/package.json', 'utf8'),
  sdkSource: readFileSync('packages/public-rights-sdk/src/index.ts', 'utf8'),
  sdkReadme: readFileSync('packages/public-rights-sdk/README.md', 'utf8'),
  desktopSdk: readFileSync('src/lib/public-rights-sdk.ts', 'utf8'),
  mobileSdk: readFileSync('mobile_app/lib/features/public_rights/public_rights_scanner.dart', 'utf8'),
};

assert(
  sources.sdkPackageJson.includes('@hiddenshield/public-rights-sdk') &&
    sources.sdkPackageJson.includes('"types"') &&
    sources.sdkPackageJson.includes('"exports"'),
  'public rights SDK package must define an external package name, types, and exports',
);

for (const token of [
  'createPublicRightsScanner',
  'scanOne',
  'scanBatch',
  'resolvePolicy',
  'formatUserMessage',
  'legalConclusion: false',
  'canTreatAsTrainingAllowed: false',
  '/v1/enterprise/public-rights/batch',
]) {
  assert(sources.sdkSource.includes(token), `SDK source must include ${token}`);
}

for (const token of [
  'scanOne',
  'resolvePolicy',
  'formatUserMessage',
  '不是法律授权结论',
]) {
  assert(sources.desktopSdk.includes(token), `desktop SDK must keep shared semantic token ${token}`);
}

for (const token of ['scanOne', 'resolvePublicRightsPolicy', 'formatPublicRightsUserMessage']) {
  assert(sources.mobileSdk.includes(token), `mobile SDK must keep shared semantic token ${token}`);
}

assert(
  sources.sdkReadme.includes('legalConclusion') &&
    sources.sdkReadme.includes('always `false`') &&
    sources.sdkReadme.includes('not published'),
  'SDK README must document non-legal-conclusion boundary and distribution status',
);

assert(
  sources.packageJson.includes('rights:sdk-package-contract'),
  'root package.json must expose rights:sdk-package-contract',
);

console.log('public rights SDK package contract passed');
