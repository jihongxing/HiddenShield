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

if (tauriConfig.plugins?.updater) {
  fail("updater plugin is still configured in tauri.conf.json");
}

if (packageJson.dependencies?.["@tauri-apps/plugin-updater"]) {
  fail("@tauri-apps/plugin-updater is still present in package.json");
}

const cargoToml = fs.readFileSync(path.join(root, "src-tauri", "Cargo.toml"), "utf8");
if (cargoToml.includes("tauri-plugin-updater")) {
  fail("tauri-plugin-updater is still present in src-tauri/Cargo.toml");
}

const csp = tauriConfig.app?.security?.csp;
if (!csp || typeof csp !== "string") {
  fail("app.security.csp must be explicitly configured for production");
}

if (csp.includes("http://")) {
  fail("app.security.csp must not allow plaintext http:// origins");
}

console.log(`release check passed for version ${version}`);
