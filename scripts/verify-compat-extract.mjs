#!/usr/bin/env node
import { readdirSync, readFileSync, statSync } from "node:fs";
import { relative, resolve, sep } from "node:path";

const [actualArg, expectedArg] = process.argv.slice(2);
if (!actualArg || process.argv.includes("--help") || process.argv.includes("-h")) {
  fail("usage: node verify-compat-extract.mjs <actual-extracted-dir> [expected-dir]");
}

const actualRoot = resolve(actualArg);
const expectedRoot = resolve(expectedArg ?? "expected");
const failures = [];
const expectedFiles = listFiles(expectedRoot);
const actualFiles = listFiles(actualRoot);

const expectedSet = new Set(expectedFiles);
const actualSet = new Set(actualFiles);

for (const file of expectedFiles) {
  if (!actualSet.has(file)) {
    failures.push(`missing file: ${file}`);
    continue;
  }
  const expected = readFileSync(resolve(expectedRoot, file));
  const actual = readFileSync(resolve(actualRoot, file));
  if (!expected.equals(actual)) {
    failures.push(`content differs: ${file}`);
  }
}

for (const file of actualFiles) {
  if (!expectedSet.has(file)) {
    failures.push(`unexpected file: ${file}`);
  }
}

if (failures.length > 0) {
  fail(`compat extraction mismatch:\n${failures.join("\n")}`);
}

console.log(`compat extraction ok: ${expectedFiles.length} files`);

function listFiles(root) {
  const rootStat = statSync(root, { throwIfNoEntry: false });
  if (!rootStat?.isDirectory()) {
    fail(`directory not found: ${root}`);
  }

  const files = [];
  walk(root);
  return files.sort();

  function walk(dir) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = resolve(dir, entry.name);
      if (entry.isDirectory()) {
        walk(path);
      } else if (entry.isFile()) {
        files.push(relative(root, path).split(sep).join("/"));
      } else {
        failures.push(`unsupported file type: ${relative(root, path).split(sep).join("/")}`);
      }
    }
  }
}

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
