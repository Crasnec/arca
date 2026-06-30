#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync, writeFileSync } from "node:fs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const writeIndex = args.indexOf("--write");
const checkIndex = args.indexOf("--check");
const writePath = writeIndex >= 0 ? args[writeIndex + 1] : null;
const checkPath = checkIndex >= 0 ? args[checkIndex + 1] : null;

const allowed = new Set([
  "0BSD",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "bzip2-1.0.6",
  "CC0-1.0",
  "GPL-3.0-or-later",
  "LGPL-2.1-or-later",
  "MIT",
  "MIT-0",
  "Unicode-3.0",
  "Unlicense",
  "Zlib",
]);

const metadata = spawnSync(
  "cargo",
  ["metadata", "--format-version=1", "--locked"],
  { cwd: root, encoding: "utf8" },
);
if (metadata.status !== 0) {
  process.stderr.write(metadata.stderr);
  process.stderr.write(metadata.stdout);
  process.exit(metadata.status ?? 1);
}

const packages = JSON.parse(metadata.stdout).packages
  .map((pkg) => ({
    name: pkg.name,
    version: pkg.version,
    license: pkg.license,
    licenseFile: pkg.license_file,
  }))
  .sort((a, b) => `${a.name} ${a.version}`.localeCompare(`${b.name} ${b.version}`));

const failures = [];
for (const pkg of packages) {
  if (!pkg.license) {
    failures.push(`${pkg.name} ${pkg.version}: missing license expression`);
    continue;
  }
  for (const atom of licenseAtoms(pkg.license)) {
    if (!allowed.has(atom)) {
      failures.push(`${pkg.name} ${pkg.version}: unreviewed license atom ${atom}`);
    }
  }
}

if (failures.length > 0) {
  process.stderr.write(`License check failed:\n${failures.join("\n")}\n`);
  process.exit(1);
}

const document = renderMarkdown(packages);
if (writePath) {
  writeFileSync(resolve(root, writePath), document);
}
if (checkPath) {
  const current = readFileSync(resolve(root, checkPath), "utf8");
  if (current !== document) {
    process.stderr.write(`${checkPath} is not up to date; run:\n`);
    process.stderr.write(`  node ./scripts/license-check.mjs --write ${checkPath}\n`);
    process.exit(1);
  }
}

console.log(`license check ok: ${packages.length} packages`);

function licenseAtoms(expression) {
  return expression
    .replaceAll("Apache-2.0 WITH LLVM-exception", "Apache-2.0_WITH_LLVM-exception")
    .replace(/[()]/g, " ")
    .split(/\s+(?:AND|OR)\s+|\/|\s+/)
    .filter(Boolean)
    .map((atom) => atom.replaceAll("Apache-2.0_WITH_LLVM-exception", "Apache-2.0 WITH LLVM-exception"));
}

function renderMarkdown(rows) {
  const lines = [
    "# Third-party Licenses",
    "",
    "This file is generated from `cargo metadata --locked` by `scripts/license-check.mjs`.",
    "Arca itself is licensed as GPL-3.0-or-later.",
    "",
    "| Package | Version | License |",
    "| --- | --- | --- |",
  ];
  for (const row of rows) {
    lines.push(
      `| ${escapeCell(row.name)} | ${escapeCell(row.version)} | ${escapeCell(row.license)} |`,
    );
  }
  lines.push("");
  return `${lines.join("\n")}`;
}

function escapeCell(value) {
  return String(value).replaceAll("|", "\\|");
}
