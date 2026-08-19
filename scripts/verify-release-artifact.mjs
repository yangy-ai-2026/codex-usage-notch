import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const packageJson = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const version = packageJson.version;
const expectedFilename = `QuotaStrip_${version}_x64-setup.exe`;
const nsisDirectory = path.join(root, "src-tauri", "target", "release", "bundle", "nsis");

function fail(message) {
  console.error(`artifact-verification=FAIL\n${message}`);
  process.exit(1);
}

if (!fs.existsSync(nsisDirectory)) {
  fail(`missing-directory=${path.relative(root, nsisDirectory).replaceAll(path.sep, "/")}`);
}

const installerFilenames = fs.readdirSync(nsisDirectory)
  .filter((filename) => filename.endsWith("-setup.exe"))
  .sort();

if (installerFilenames.length !== 1 || installerFilenames[0] !== expectedFilename) {
  fail(`expected=${expectedFilename}\nfound=${installerFilenames.join(",") || "none"}`);
}

const installerPath = path.join(nsisDirectory, expectedFilename);
if (!fs.statSync(installerPath).isFile()) {
  fail(`not-a-file=${expectedFilename}`);
}

const relativeInstallerPath = path.relative(root, installerPath).replaceAll(path.sep, "/");
const manifest = [
  `version=${version}`,
  `filename=${expectedFilename}`,
  "format=nsis",
  `path=${relativeInstallerPath}`,
  "",
].join("\n");
fs.writeFileSync(path.join(nsisDirectory, "artifact-manifest.txt"), manifest, "utf8");

console.log(`artifact-verification=PASS\n${manifest.trimEnd()}`);
