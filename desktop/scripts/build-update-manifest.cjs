// Walks an artifacts/ directory containing the bundles + .sig files produced
// by all platform matrix jobs, then emits a Tauri-compatible latest.json on
// stdout.
//
// Usage (inside the CI release job):
//   node desktop/scripts/build-update-manifest.cjs \
//       <artifacts-dir> <version> [pub-date] [notes-file] > latest.json
//
// Required env: GITHUB_REPOSITORY (owner/repo), GITHUB_REF_NAME (tag)

const fs = require("fs");
const path = require("path");

const [, , artifactsDir, version, pubDate, notesFile] = process.argv;

if (!artifactsDir || !version) {
    console.error("usage: build-update-manifest.cjs <dir> <version> [pubDate] [notesFile]");
    process.exit(2);
}

const repo = process.env.GITHUB_REPOSITORY;
const tag = process.env.GITHUB_REF_NAME;
if (!repo || !tag) {
    console.error("GITHUB_REPOSITORY and GITHUB_REF_NAME must be set");
    process.exit(2);
}

let notes = "";
if (notesFile && fs.existsSync(notesFile)) {
    notes = fs.readFileSync(notesFile, "utf8");
}

function walk(dir) {
    const out = [];
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const p = path.join(dir, entry.name);
        if (entry.isDirectory()) out.push(...walk(p));
        else out.push(p);
    }
    return out;
}

const files = walk(artifactsDir);

// Bundle file patterns matched per (platform, arch). The first regex that hits
// a real file wins; we expect a sibling <filename>.sig for the signature.
//
// Tauri 2 with createUpdaterArtifacts:true produces:
//   macOS:   <Product>.app.tar.gz  (+ .sig)  — NO arch suffix in v2!
//   Windows: <Product>_<ver>_x64-setup.exe  (+ .sig)
//   Linux:   <Product>_<ver>_amd64.AppImage  (+ .sig)
//
// NOTE: When both darwin-aarch64 and darwin-x86_64 are present (e.g. universal
// or dual builds), the arch-specific patterns must come BEFORE the fallback
// pattern to ensure correct matching.
const PATTERNS = {
    "darwin-x86_64": [/_(x64|x86_64)\.app\.tar\.gz$/i],
    "darwin-aarch64": [/_(aarch64|arm64)\.app\.tar\.gz$/i, /\.app\.tar\.gz$/i],
    "linux-x86_64": [/_(amd64|x86_64|x64)\.AppImage$/i, /\.AppImage$/i],
    "windows-x86_64": [/_(x64|x86_64)[^.]*-setup\.exe$/i, /-setup\.exe$/i],
    "windows-aarch64": [/_(arm64|aarch64)[^.]*-setup\.exe$/i],
    "linux-aarch64": [/_(arm64|aarch64)\.AppImage$/i],
};

function findPair(patterns) {
    for (const re of patterns) {
        const bundle = files.find((f) => re.test(f) && !f.endsWith(".sig"));
        if (!bundle) continue;
        const sigPath = bundle + ".sig";
        if (!fs.existsSync(sigPath)) {
            console.error(`warn: bundle ${path.basename(bundle)} has no .sig sibling — skipping`);
            continue;
        }
        const signature = fs.readFileSync(sigPath, "utf8").trim();
        return { name: path.basename(bundle), signature };
    }
    return null;
}

const platforms = {};
for (const [key, patterns] of Object.entries(PATTERNS)) {
    const m = findPair(patterns);
    if (!m) {
        console.error(`info: no bundle for platform ${key}`);
        continue;
    }
    platforms[key] = {
        signature: m.signature,
        url: `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(m.name)}`,
    };
}

if (Object.keys(platforms).length === 0) {
    console.error("error: no platform bundles matched");
    process.exit(1);
}

const manifest = {
    version,
    notes,
    pub_date: pubDate || new Date().toISOString(),
    platforms,
};

process.stdout.write(JSON.stringify(manifest, null, 2));
