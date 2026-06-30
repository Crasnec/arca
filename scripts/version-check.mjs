#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const tagIndex = args.indexOf("--tag");
const printVersion = args.includes("--print-version");
const explicitTag = tagIndex >= 0 ? args[tagIndex + 1] : null;
const githubTag =
  process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : null;
const tag = explicitTag ?? githubTag;

if (tagIndex >= 0 && !explicitTag) {
  fail("--tag requires a value");
}

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

const document = JSON.parse(metadata.stdout);
const workspaceMembers = new Set(document.workspace_members);
const packages = document.packages
  .filter((pkg) => workspaceMembers.has(pkg.id))
  .map((pkg) => ({ name: pkg.name, version: pkg.version }))
  .sort((a, b) => a.name.localeCompare(b.name));

const arcaCli = packages.find((pkg) => pkg.name === "arca-cli");
if (!arcaCli) {
  fail("workspace package arca-cli not found");
}

const expected = arcaCli.version;
const mismatched = packages.filter((pkg) => pkg.version !== expected);
if (mismatched.length > 0) {
  fail(
    [
      `workspace package versions must match arca-cli ${expected}`,
      ...mismatched.map((pkg) => `  ${pkg.name}: ${pkg.version}`),
    ].join("\n"),
  );
}

if (tag) {
  if (!/^v[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$/.test(tag)) {
    fail(`release tag must look like v<semver>: ${tag}`);
  }
  const tagVersion = tag.slice(1);
  if (tagVersion !== expected) {
    fail(`release tag ${tag} does not match arca-cli version ${expected}`);
  }
}

if (printVersion) {
  console.log(expected);
} else {
  console.log(
    `version check ok: ${packages.map((pkg) => `${pkg.name} ${pkg.version}`).join(", ")}`,
  );
}

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
