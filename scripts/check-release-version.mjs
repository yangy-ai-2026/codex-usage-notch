import fs from "node:fs";
import path from "node:path";

const root = process.cwd();

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(root, relativePath), "utf8"));
}

function fail(message) {
  console.error(`version-consistency=FAIL\n${message}`);
  process.exit(1);
}

const packageJson = readJson("package.json");
const packageLock = readJson("package-lock.json");
const tauriConfig = readJson("src-tauri/tauri.conf.json");
const cargoToml = fs.readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8");
const cargoLock = fs.readFileSync(path.join(root, "src-tauri/Cargo.lock"), "utf8");

const cargoVersion = cargoToml.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
const cargoLockVersion = cargoLock.match(/\[\[package\]\]\s+name\s*=\s*"quotastrip"\s+version\s*=\s*"([^"]+)"/s)?.[1];
const versions = {
  npm: packageJson.version,
  "npm-lock": packageLock.packages?.[""].version,
  cargo: cargoVersion,
  "cargo-lock": cargoLockVersion,
  tauri: tauriConfig.version,
};

const missing = Object.entries(versions).filter(([, version]) => !version).map(([name]) => name);
if (missing.length > 0) {
  fail(`missing=${missing.join(",")}`);
}

const uniqueVersions = new Set(Object.values(versions));
if (uniqueVersions.size !== 1) {
  fail(Object.entries(versions).map(([name, version]) => `${name}=${version}`).join("\n"));
}

const version = versions.npm;
console.log([
  "version-consistency=PASS",
  `version=${version}`,
  ...Object.entries(versions).map(([name, value]) => `${name}=${value}`),
].join("\n"));
