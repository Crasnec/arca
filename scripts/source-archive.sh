#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
archive="${ARCA_SOURCE_ARCHIVE:-arca-source.tar.gz}"

case "$archive" in
  /*) archive_path="$archive" ;;
  *) archive_path="$root/$archive" ;;
esac

rm -rf "$root/dist/source-check"
mkdir -p "$root/dist"
rm -f "$archive_path" "$archive_path.sha256"

source_paths=(
  Cargo.toml
  Cargo.lock
  package.json
  package-lock.json
  LICENSE
  README.md
  .gitignore
  .gitattributes
  .github
  apps
  crates
  docs
  scripts
)

if [[ -d "$root/tests" ]]; then
  source_paths+=(tests)
fi

tar -C "$root" -czf "$archive_path" \
  --exclude=apps/arca-gui/dist \
  --exclude=apps/arca-gui/node_modules \
  --transform 's,^,arca-source/,' \
  "${source_paths[@]}"

mkdir -p "$root/dist/source-check"
tar -C "$root/dist/source-check" -xzf "$archive_path"
cargo metadata \
  --format-version=1 \
  --locked \
  --no-deps \
  --manifest-path "$root/dist/source-check/arca-source/Cargo.toml" \
  >/dev/null
cargo test \
  --workspace \
  --locked \
  --no-run \
  --manifest-path "$root/dist/source-check/arca-source/Cargo.toml" \
  >/dev/null
npm ci \
  --ignore-scripts \
  --prefix "$root/dist/source-check/arca-source" \
  >/dev/null
npm run \
  --prefix "$root/dist/source-check/arca-source" \
  gui:smoke \
  >/dev/null
npm run \
  --prefix "$root/dist/source-check/arca-source" \
  gui:web:build \
  >/dev/null

if command -v sha256sum >/dev/null 2>&1; then
  hash="$(sha256sum "$archive_path" | awk '{print $1}')"
else
  hash="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
fi
printf '%s  %s\n' "$hash" "$(basename "$archive_path")" > "$archive_path.sha256"

printf '%s\n' "$archive_path"
