#!/usr/bin/env node

/**
 * ZVault npm postinstall script.
 * Downloads the correct platform binary from GitHub Releases.
 */

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const os = require("os");

const REPO = "VanitasCaesar1/zvault";
const BIN_DIR = path.join(__dirname, "bin");
const VERSION = require("./package.json").version;

function getPlatform() {
  const platform = os.platform();
  const arch = os.arch();

  let osPart;
  switch (platform) {
    case "darwin":
      osPart = "darwin";
      break;
    case "linux":
      osPart = "linux";
      break;
    default:
      throw new Error(
        `Unsupported platform: ${platform}. ZVault supports macOS and Linux.\n` +
          `Install manually: https://zvault.cloud/install.sh`
      );
  }

  let archPart;
  switch (arch) {
    case "x64":
    case "amd64":
      archPart = "x86_64";
      break;
    case "arm64":
    case "aarch64":
      archPart = "aarch64";
      break;
    default:
      throw new Error(
        `Unsupported architecture: ${arch}. ZVault supports x86_64 and aarch64.\n` +
          `Install manually: https://zvault.cloud/install.sh`
      );
  }

  return `${osPart}-${archPart}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        // Follow redirects (GitHub releases redirect to S3)
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return download(res.headers.location).then(resolve).catch(reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`Download failed: HTTP ${res.statusCode} from ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function main() {
  const platform = getPlatform();
  const tag = `v${VERSION}`;
  const tarball = `zvault-${tag}-${platform}.tar.gz`;
  const url = `https://github.com/${REPO}/releases/download/${tag}/${tarball}`;

  console.log(`Downloading zvault ${tag} for ${platform}...`);

  try {
    const data = await download(url);

    // Write tarball to temp file
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "zvault-"));
    const tarPath = path.join(tmpDir, tarball);
    fs.writeFileSync(tarPath, data);

    // Extract
    fs.mkdirSync(BIN_DIR, { recursive: true });
    execSync(`tar -xzf "${tarPath}" -C "${tmpDir}"`, { stdio: "pipe" });

    // Find and copy binary
    const binaryName = "zvault";
    const extractedBin = path.join(tmpDir, binaryName);

    if (!fs.existsSync(extractedBin)) {
      throw new Error("Binary not found in archive");
    }

    const destBin = path.join(BIN_DIR, binaryName);
    fs.copyFileSync(extractedBin, destBin);
    fs.chmodSync(destBin, 0o755);

    // Cleanup
    fs.rmSync(tmpDir, { recursive: true, force: true });

    console.log(`✓ zvault ${tag} installed`);
  } catch (err) {
    console.error(`\nFailed to install zvault binary: ${err.message}`);
    console.error(`\nYou can install manually:`);
    console.error(`  curl -fsSL https://zvault.cloud/install.sh | sh`);
    console.error(`  cargo install --git https://github.com/${REPO} zvault-cli\n`);
    // Don't fail the npm install — the binary just won't be available
    process.exit(0);
  }
}

main();
