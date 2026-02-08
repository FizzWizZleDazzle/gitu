#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const os = require("os");
const zlib = require("zlib");

const REPO = "FizzWizzleDazzle/gitu";

function getPlatformTarget() {
  const platform = os.platform();
  const arch = os.arch();

  const targets = {
    "linux-x64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "x86_64-unknown-linux-gnu",
    "darwin-x64": "x86_64-apple-darwin",
    "darwin-arm64": "aarch64-apple-darwin",
    "win32-x64": "x86_64-pc-windows-msvc",
  };

  // For linux arm64, use the aarch64 target
  if (platform === "linux" && arch === "arm64") {
    return "aarch64-unknown-linux-gnu";
  }

  const key = `${platform}-${arch}`;
  const target = targets[key];

  if (!target) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    process.exit(1);
  }

  return target;
}

function getDownloadUrl(target) {
  const ext = target.includes("windows") ? "zip" : "tar.gz";
  return `https://github.com/${REPO}/releases/latest/download/gitu-${target}.${ext}`;
}

function follow(url, callback) {
  https
    .get(url, { headers: { "User-Agent": "gitu-npm-installer" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        follow(res.headers.location, callback);
      } else {
        callback(res);
      }
    })
    .on("error", (err) => {
      console.error(`Download failed: ${err.message}`);
      process.exit(1);
    });
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    follow(url, (res) => {
      if (res.statusCode !== 200) {
        reject(new Error(`Download failed with status ${res.statusCode}`));
        return;
      }

      const file = fs.createWriteStream(dest);
      res.pipe(file);
      file.on("finish", () => {
        file.close(resolve);
      });
      file.on("error", reject);
    });
  });
}

async function extractTarGz(archivePath, destDir) {
  execSync(`tar xzf "${archivePath}" -C "${destDir}"`, { stdio: "inherit" });
}

async function extractZip(archivePath, destDir) {
  if (os.platform() === "win32") {
    execSync(
      `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`,
      { stdio: "inherit" }
    );
  } else {
    execSync(`unzip -o "${archivePath}" -d "${destDir}"`, { stdio: "inherit" });
  }
}

async function main() {
  const target = getPlatformTarget();
  const isWindows = target.includes("windows");
  const binaryName = isWindows ? "gitu.exe" : "gitu";
  const ext = isWindows ? "zip" : "tar.gz";
  const url = getDownloadUrl(target);

  const binDir = path.join(__dirname, "bin");
  const tmpDir = os.tmpdir();
  const archivePath = path.join(tmpDir, `gitu-${target}.${ext}`);

  console.log(`Downloading gitu for ${target}...`);
  console.log(`URL: ${url}`);

  try {
    await download(url, archivePath);

    console.log("Extracting...");
    if (isWindows) {
      await extractZip(archivePath, binDir);
    } else {
      await extractTarGz(archivePath, binDir);
    }

    // Ensure the binary is executable
    const binaryPath = path.join(binDir, binaryName);
    if (!isWindows) {
      fs.chmodSync(binaryPath, 0o755);
    }

    // Cleanup
    try {
      fs.unlinkSync(archivePath);
    } catch (_) {}

    console.log(`gitu installed successfully to ${binaryPath}`);
  } catch (err) {
    console.error(`Installation failed: ${err.message}`);
    console.error(
      "You can download the binary manually from: https://github.com/FizzWizzleDazzle/gitu/releases"
    );
    process.exit(1);
  }
}

main();
