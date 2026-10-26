#!/usr/bin/env node
// Validates that every `npm run <script>` referenced in Markdown docs
// exists in the root package.json scripts.
// Usage: npm run docs:check-scripts
const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..");
const pkg = JSON.parse(fs.readFileSync(path.join(repoRoot, "package.json"), "utf8"));
const defined = new Set(Object.keys(pkg.scripts || {}));

function* walk(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === "node_modules" || e.name === ".git") continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) yield* walk(p);
    else if (e.name.endsWith(".md")) yield p;
  }
}

const re = /npm run ([A-Za-z0-9:_-]+)/g;
let missing = [];
for (const file of walk(repoRoot)) {
  const text = fs.readFileSync(file, "utf8");
  let m;
  while ((m = re.exec(text))) {
    const name = m[1];
    if (!defined.has(name)) {
      missing.push(`${path.relative(repoRoot, file)}: npm run ${name}`);
    }
  }
}

if (missing.length) {
  console.error("Missing npm scripts referenced in docs:");
  for (const x of [...new Set(missing)].sort()) console.error(" - " + x);
  process.exit(1);
} else {
  console.log(`docs:check-scripts OK (${defined.size} scripts defined)`);
}
