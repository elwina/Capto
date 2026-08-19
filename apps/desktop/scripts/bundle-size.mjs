#!/usr/bin/env node
// Bundle size budget gate for the Capto frontend.
//
// Reads the `rollup-plugin-visualizer` raw-data report produced by
// `vite build --mode analyze` (dist/stats.json), enumerates the emitted
// assets from its module tree, measures their on-disk size, and fails the
// build when the total exceeds `MAX_BUDGET_BYTES`. Off-the-shelf bundle
// analyzers (and the visualizer HTML in dist/analyze.html) let a reviewer see
// which dependency dominates (heavy_dependency_detection):
//   npm run build:analyze   # generates dist/analyze.html + dist/stats.json
//   npm run bundle-size     # prints the breakdown, exits 1 over budget
import { readFileSync, statSync } from "node:fs";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";

const STATS_PATH = resolve("dist/stats.json");
// Deliberately generous baseline (a Tauri shell app embeds React + i18next).
// Lower it as the bundle is trimmed: the shell ships inside the installer, so
// ~1.5 MB of fetched JS is fine — this gate exists to catch regressions, not
// to micro-optimize.
const MAX_BUDGET_BYTES = 5_000_000;

function fmt(bytes) {
  return `${(bytes / 1024).toFixed(1)} KB`;
}

let stats;
try {
  stats = JSON.parse(readFileSync(STATS_PATH, "utf8"));
} catch (err) {
  console.error(`No bundle stats at ${STATS_PATH}. Run \`npm run build:analyze\` first.`);
  process.exit(2);
}

// Collect emitted asset paths (e.g. "assets/index-abc.js") from the tree.
function collectAssets(node, out) {
  if (!node) return;
  const isAsset =
    node.name && typeof node.name === "string" && /^assets\/.+\.(js|css|mjs|cjs)$/.test(node.name);
  if (isAsset) out.push(node.name);
  if (Array.isArray(node.children)) {
    for (const child of node.children) collectAssets(child, out);
  }
}
const assets = [];
collectAssets(stats?.tree, assets);

if (assets.length === 0) {
  console.error("No emitted assets found in dist/stats.json; run `npm run build:analyze` first.");
  process.exit(2);
}

const rows = [];
let fetchedBytes = 0;
for (const asset of assets) {
  const file = join(resolve("dist"), asset);
  if (!existsSync(file)) continue;
  const size = statSync(file).size;
  fetchedBytes += size;
  rows.push(`${fmt(size).padStart(9)}  ${asset}`);
}
rows.sort((a, b) => Number.parseFloat(b) - Number.parseFloat(a));

console.log("Emitted bundle assets (largest first):");
for (const row of rows) {
  console.log(`  ${row}`);
}

console.log(`\nTotal emitted JS/CSS: ${fmt(fetchedBytes)}`);
console.log(`Budget: ${fmt(MAX_BUDGET_BYTES)}`);

if (fetchedBytes > MAX_BUDGET_BYTES) {
  console.error(
    `\nFAIL: frontend bundle is over budget (${fmt(fetchedBytes)} > ${fmt(MAX_BUDGET_BYTES)}).`,
  );
  console.error("Trim heavy dependencies (see dist/analyze.html) and lower the budget.");
  process.exit(1);
}

console.log("\nOK: frontend bundle within budget.");
