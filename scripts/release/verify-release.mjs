import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

function readJson(relPath) {
  const fullPath = path.join(root, relPath);
  return JSON.parse(fs.readFileSync(fullPath, "utf8"));
}

function readCargoVersion(relPath) {
  const fullPath = path.join(root, relPath);
  const content = fs.readFileSync(fullPath, "utf8");
  const match = content.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`Unable to find Cargo version in ${relPath}`);
  }
  return match[1];
}

function fail(message) {
  console.error(`release check failed: ${message}`);
  process.exit(1);
}

const packageJson = readJson("package.json");
const tauriConfig = readJson(path.join("src-tauri", "tauri.conf.json"));
const cargoVersion = readCargoVersion(path.join("src-tauri", "Cargo.toml"));

const versions = [
  { name: "package.json", value: packageJson.version },
  { name: "src-tauri/Cargo.toml", value: cargoVersion },
  { name: "src-tauri/tauri.conf.json", value: tauriConfig.version },
];

const distinctVersions = [...new Set(versions.map((item) => item.value))];
if (distinctVersions.length !== 1) {
  fail(
    `version mismatch: ${versions.map((item) => `${item.name}=${item.value}`).join(", ")}`,
  );
}

const version = distinctVersions[0];
const releaseTag = process.env.RELEASE_TAG || process.env.GITHUB_REF_NAME || "";
if (releaseTag && releaseTag !== `v${version}`) {
  fail(`tag ${releaseTag} does not match application version v${version}`);
}

const updater = tauriConfig.plugins?.updater;
if (!updater) {
  fail("updater plugin must be configured in tauri.conf.json");
}

if (tauriConfig.bundle?.createUpdaterArtifacts !== true) {
  fail("bundle.createUpdaterArtifacts must be enabled for updater releases");
}

if (
  typeof updater.pubkey !== "string" ||
  updater.pubkey.length < 32 ||
  updater.pubkey.includes("PLACEHOLDER")
) {
  fail("updater pubkey must be a non-placeholder public key");
}

if (
  !Array.isArray(updater.endpoints) ||
  updater.endpoints.length === 0 ||
  updater.endpoints.some((endpoint) => typeof endpoint !== "string" || !endpoint.startsWith("https://"))
) {
  fail("updater endpoints must contain HTTPS URLs");
}

if (!packageJson.dependencies?.["@tauri-apps/plugin-updater"]) {
  fail("@tauri-apps/plugin-updater is required in package.json");
}

if (!packageJson.dependencies?.["@tauri-apps/plugin-process"]) {
  fail("@tauri-apps/plugin-process is required in package.json");
}

const cargoToml = fs.readFileSync(path.join(root, "src-tauri", "Cargo.toml"), "utf8");
if (!cargoToml.includes("tauri-plugin-updater")) {
  fail("tauri-plugin-updater is required in src-tauri/Cargo.toml");
}

if (!cargoToml.includes("tauri-plugin-process")) {
  fail("tauri-plugin-process is required in src-tauri/Cargo.toml");
}

const capabilities = readJson(path.join("src-tauri", "capabilities", "default.json"));
if (!capabilities.permissions?.includes("updater:default")) {
  fail("default Tauri capability must include updater:default");
}

if (!capabilities.permissions?.includes("process:allow-restart")) {
  fail("default Tauri capability must include process:allow-restart");
}

const releaseWorkflow = fs.readFileSync(path.join(root, ".github", "workflows", "release.yml"), "utf8");
for (const requiredFragment of [
  "environment: production",
  "TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}",
  "TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}",
  "uploadUpdaterJson: true",
  "uploadUpdaterSignatures: true",
]) {
  if (!releaseWorkflow.includes(requiredFragment)) {
    fail(`release workflow must include ${requiredFragment}`);
  }
}

if (releaseWorkflow.includes("WINDOWS_SELF_SIGNED_CERTIFICATE")) {
  fail("release workflow must not use the self-signed Windows certificate for public updater assets");
}

const csp = tauriConfig.app?.security?.csp;
if (!csp || typeof csp !== "string") {
  fail("app.security.csp must be explicitly configured for production");
}

const insecureOrigins = [...csp.matchAll(/http:\/\/[^;\s]+/g)]
  .map((match) => match[0])
  .filter((origin) => !/^http:\/\/127\.0\.0\.1(?::\d+)?$/.test(origin));
if (insecureOrigins.length > 0) {
  fail(`app.security.csp must not allow external plaintext origins: ${insecureOrigins.join(", ")}`);
}

console.log(`release check passed for version ${version}`);
