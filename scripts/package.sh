#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
archive="${ARCA_ARCHIVE:-arca-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m).tar.gz}"
if [[ -n "${ARCA_BIN:-}" ]]; then
  binary="$ARCA_BIN"
else
  binary="$root/target/release/arca"
  cargo build --release --locked --bin arca --manifest-path "$root/Cargo.toml"
fi

case "$archive" in
  /*) archive_path="$archive" ;;
  *) archive_path="$root/$archive" ;;
esac

if [[ ! -x "$binary" ]]; then
  echo "release binary not found or not executable: $binary" >&2
  exit 1
fi
if ! command -v node >/dev/null 2>&1; then
  echo "node is required to verify package binary version" >&2
  exit 1
fi
expected_version="$(node "$root/scripts/version-check.mjs" --print-version)"

rm -rf "$root/dist/arca"
mkdir -p "$root/dist/arca"
cp "$binary" "$root/dist/arca/arca"
chmod 755 "$root/dist/arca/arca"
cp "$root/docs/package-readme.md" "$root/dist/arca/README.md"
cp "$root/LICENSE" "$root/dist/arca/"
node "$root/scripts/license-check.mjs" --write "$root/dist/arca/THIRD_PARTY_LICENSES.md"

rm -f "$archive_path" "$archive_path.sha256"
tar -C "$root/dist" -czf "$archive_path" arca
rm -rf "$root/dist/package-check"
mkdir -p "$root/dist/package-check"
tar -C "$root/dist/package-check" -xzf "$archive_path"
test -x "$root/dist/package-check/arca/arca"
test -f "$root/dist/package-check/arca/README.md"
test -f "$root/dist/package-check/arca/LICENSE"
test -f "$root/dist/package-check/arca/THIRD_PARTY_LICENSES.md"
actual_version="$("$root/dist/package-check/arca/arca" --version)"
if [[ "$actual_version" != "arca $expected_version" ]]; then
  echo "packaged binary version mismatch: expected arca $expected_version, got $actual_version" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  hash="$(sha256sum "$archive_path" | awk '{print $1}')"
else
  hash="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
fi
printf '%s  %s\n' "$hash" "$(basename "$archive_path")" > "$archive_path.sha256"

printf '%s\n' "$archive_path"
