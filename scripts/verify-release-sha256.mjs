import fs from "node:fs";
import crypto from "node:crypto";
import path from "node:path";

const root = process.cwd();
const expectedFilename = "QuotaStrip_0.1.0_x64-setup.exe";
const nsisDirectory = path.join(root, "src-tauri", "target", "release", "bundle", "nsis");
const installerPath = path.join(nsisDirectory, expectedFilename);
const checksumPath = path.join(nsisDirectory, "SHA256SUMS.txt");

function fail(message) {
  console.error(`sha256-verification=FAIL\n${message}`);
  process.exit(1);
}

function hashBuffer(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function hashFileFresh(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash("sha256");
    const stream = fs.createReadStream(filePath);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("end", () => resolve(hash.digest("hex")));
    stream.on("error", reject);
  });
}

if (!fs.existsSync(installerPath) || !fs.statSync(installerPath).isFile()) {
  fail(`missing-installer=${expectedFilename}`);
}

const generatedHash = hashBuffer(fs.readFileSync(installerPath));
const checksumLine = `${generatedHash}  ${expectedFilename}\n`;
fs.writeFileSync(checksumPath, checksumLine, "utf8");

const checksumContents = fs.readFileSync(checksumPath, "utf8");
const checksumMatch = checksumContents.match(
  /^([0-9a-f]{64})  (QuotaStrip_0\.1\.0_x64-setup\.exe)\n$/,
);
if (!checksumMatch) {
  fail("checksum-file-format-invalid");
}

const verifiedHash = await hashFileFresh(installerPath);
if (verifiedHash !== checksumMatch[1] || verifiedHash !== generatedHash) {
  fail(`checksum-mismatch=${expectedFilename}`);
}

console.log(`sha256-verification=PASS\n${checksumLine.trimEnd()}`);
