#!/usr/bin/env node
// bump-version.cjs — Synchronize version across all desktop config files.
//
// Usage:
//   node scripts/bump-version.cjs 0.3.0
//
// This updates:
//   desktop/package.json          → "version"
//   desktop/src-tauri/Cargo.toml  → [package] version
//   desktop/src-tauri/tauri.conf.json → "version"

const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "..");
const newVersion = process.argv[2];

if (!newVersion || !/^\d+\.\d+\.\d+/.test(newVersion)) {
  console.error("Usage: node bump-version.cjs <version>  (e.g. 0.3.0)");
  process.exit(1);
}

// ── package.json ─────────────────────────────────────────────────────
const pkgPath = path.join(root, "package.json");
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
pkg.version = newVersion;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 4) + "\n");
console.log(`✅ package.json → ${newVersion}`);

// ── Cargo.toml ───────────────────────────────────────────────────────
const cargoPath = path.join(root, "src-tauri", "Cargo.toml");
let cargo = fs.readFileSync(cargoPath, "utf8");
cargo = cargo.replace(
  /^(version\s*=\s*)"[^"]*"/m,
  `$1"${newVersion}"`
);
fs.writeFileSync(cargoPath, cargo);
console.log(`✅ Cargo.toml → ${newVersion}`);

// ── tauri.conf.json ──────────────────────────────────────────────────
const tauriPath = path.join(root, "src-tauri", "tauri.conf.json");
const tauri = JSON.parse(fs.readFileSync(tauriPath, "utf8"));
tauri.version = newVersion;
fs.writeFileSync(tauriPath, JSON.stringify(tauri, null, 4) + "\n");
console.log(`✅ tauri.conf.json → ${newVersion}`);

console.log(`\n🎉 All versions synchronized to ${newVersion}`);
console.log(`   Next: git commit -am "release: desktop v${newVersion}" && git tag desktop-v${newVersion}`);
