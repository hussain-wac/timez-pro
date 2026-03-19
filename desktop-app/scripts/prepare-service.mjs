import { copyFileSync, chmodSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(__dirname, "..");
const srcTauriDir = join(rootDir, "src-tauri");
const resourcesDir = join(srcTauriDir, "resources");
const profile = process.argv.includes("--release") ? "release" : "debug";
const cargoArgs = [
  "build",
  "-p",
  "timez-service",
  "--bins",
  "--manifest-path",
  join(srcTauriDir, "Cargo.toml"),
  "--offline",
];

if (profile === "release") {
  cargoArgs.push("--release");
}

execFileSync("cargo", cargoArgs, {
  cwd: srcTauriDir,
  stdio: "inherit",
});

mkdirSync(resourcesDir, { recursive: true });
const binaries = [
  "timez-auth-service",
  "timez-task-service",
  "timez-tracker-service",
  "timez-idle-time-service",
  "timez-quit-service",
];

for (const baseName of binaries) {
  const binaryName =
    process.platform === "win32" ? `${baseName}.exe` : baseName;
  const builtBinary = join(srcTauriDir, "target", profile, binaryName);
  const bundledBinary = join(resourcesDir, binaryName);
  copyFileSync(builtBinary, bundledBinary);

  if (process.platform !== "win32") {
    chmodSync(bundledBinary, 0o755);
  }
}
