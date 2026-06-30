#!/usr/bin/env bash
set -euo pipefail

asset_dir="${1:-release-assets}"
if [[ ! -d "$asset_dir" ]]; then
  echo "release asset directory not found: $asset_dir" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  verify_checksum() {
    sha256sum -c "$1"
  }
elif command -v shasum >/dev/null 2>&1; then
  verify_checksum() {
    shasum -a 256 -c "$1"
  }
else
  echo "sha256sum or shasum is required to verify release assets" >&2
  exit 1
fi

default_expected=(
  arca-linux-x86_64.tar.gz
  arca-linux-x86_64.tar.gz.sha256
  arca-macos-native.tar.gz
  arca-macos-native.tar.gz.sha256
  arca-windows-x86_64.zip
  arca-windows-x86_64.zip.sha256
  arca-compat-fixtures.tar.gz
  arca-compat-fixtures.tar.gz.sha256
  arca-source.tar.gz
  arca-source.tar.gz.sha256
)

if [[ -n "${ARCA_EXPECTED_RELEASE_ASSETS:-}" ]]; then
  read -r -a expected <<< "$ARCA_EXPECTED_RELEASE_ASSETS"
else
  expected=("${default_expected[@]}")
fi

tmp_dir="$(mktemp -d)"
expected_file="$tmp_dir/expected"
actual_file="$tmp_dir/actual"
trap 'rm -rf "$tmp_dir"' EXIT

printf '%s\n' "${expected[@]}" | sort > "$expected_file"
find "$asset_dir" -maxdepth 1 -type f -exec basename {} \; | sort > "$actual_file"

if ! diff -u "$expected_file" "$actual_file"; then
  echo "release asset set does not match expected files" >&2
  exit 1
fi

for name in "${expected[@]}"; do
  if [[ ! -f "$asset_dir/$name" ]]; then
    echo "missing release asset: $name" >&2
    exit 1
  fi
done

require_checksum_manifest() {
  local checksum="$1"
  local expected_name
  local line_count
  local line
  local hash
  local name
  local extra

  expected_name="$(basename "${checksum%.sha256}")"
  line_count="$(awk 'END { print NR }' "$checksum")"
  if [[ "$line_count" -ne 1 ]]; then
    echo "checksum file must contain exactly one line: $(basename "$checksum")" >&2
    exit 1
  fi

  IFS= read -r line < "$checksum"
  read -r hash name extra <<< "$line"
  if [[ -n "${extra:-}" || -z "${hash:-}" || -z "${name:-}" ]]; then
    echo "checksum file has invalid format: $(basename "$checksum")" >&2
    exit 1
  fi
  if [[ ! "$hash" =~ ^[0-9a-f]{64}$ ]]; then
    echo "checksum file has invalid SHA-256 digest: $(basename "$checksum")" >&2
    exit 1
  fi
  if [[ "$name" != "$expected_name" ]]; then
    echo "checksum file $(basename "$checksum") references unexpected file: $name" >&2
    exit 1
  fi
}

for checksum in "$asset_dir"/*.sha256; do
  [[ -e "$checksum" ]] || continue
  require_checksum_manifest "$checksum"
  (
    cd "$asset_dir"
    verify_checksum "$(basename "$checksum")"
  )
done

list_zip() {
  local archive="$1"
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$archive" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    bad = archive.testzip()
    if bad is not None:
        raise SystemExit(f"zip member failed CRC check: {bad}")
    for name in archive.namelist():
        print(name.rstrip("/"))
PY
  elif command -v unzip >/dev/null 2>&1; then
    unzip -tqq "$archive" >/dev/null
    unzip -Z1 "$archive" | sed 's,/$,,'
  else
    echo "python3 or unzip is required to inspect zip release assets" >&2
    exit 1
  fi
}

list_archive_entries() {
  local archive="$1"
  case "$archive" in
    *.tar.gz) tar -tzf "$archive" | sed 's,/$,,' ;;
    *.zip) list_zip "$archive" ;;
    *)
      echo "unsupported release asset archive type: $archive" >&2
      exit 1
      ;;
  esac
}

extract_zip_entry() {
  local archive="$1"
  local entry="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$archive" "$entry" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    sys.stdout.buffer.write(archive.read(sys.argv[2]))
PY
  elif command -v unzip >/dev/null 2>&1; then
    unzip -p "$archive" "$entry"
  else
    echo "python3 or unzip is required to inspect zip release assets" >&2
    exit 1
  fi
}

extract_archive_entry() {
  local archive="$1"
  local entry="$2"
  case "$archive" in
    *.tar.gz) tar -xOzf "$archive" "$entry" ;;
    *.zip) extract_zip_entry "$archive" "$entry" ;;
    *)
      echo "unsupported release asset archive type: $archive" >&2
      exit 1
      ;;
  esac
}

verify_archive_payloads() {
  local name="$1"
  local archive="$asset_dir/$name"

  [[ -f "$archive" ]] || return 0
  case "$archive" in
    *.tar.gz) tar -xOzf "$archive" >/dev/null ;;
    *.zip) list_zip "$archive" >/dev/null ;;
    *)
      echo "unsupported release asset archive type: $archive" >&2
      exit 1
      ;;
  esac
}

require_archive_safe_paths() {
  local name="$1"
  local archive="$asset_dir/$name"
  local entries="$tmp_dir/$name.safe.entries"
  local entry
  local part

  [[ -f "$archive" ]] || return 0
  list_archive_entries "$archive" | sort -u > "$entries"

  while IFS= read -r entry; do
    if [[ -z "$entry" ]]; then
      echo "release asset $name has an empty archive entry" >&2
      exit 1
    fi
    case "$entry" in
      /* | *\\*)
        echo "release asset $name has unsafe archive entry: $entry" >&2
        exit 1
        ;;
    esac

    IFS='/' read -r -a parts <<< "$entry"
    for part in "${parts[@]}"; do
      case "$part" in
        "" | "." | ".." | *:*)
          echo "release asset $name has unsafe archive entry: $entry" >&2
          exit 1
          ;;
      esac
    done
  done < "$entries"
}

require_archive_entries() {
  local name="$1"
  shift
  local archive="$asset_dir/$name"
  local entries="$tmp_dir/$name.entries"

  [[ -f "$archive" ]] || return 0
  list_archive_entries "$archive" | sort -u > "$entries"

  for entry in "$@"; do
    if ! grep -Fxq "$entry" "$entries"; then
      echo "release asset $name is missing required entry: $entry" >&2
      exit 1
    fi
  done
}

require_archive_prefix() {
  local name="$1"
  local prefix="$2"
  local archive="$asset_dir/$name"
  local entries="$tmp_dir/$name.prefix.entries"

  [[ -f "$archive" ]] || return 0
  list_archive_entries "$archive" | sort -u > "$entries"

  while IFS= read -r entry; do
    case "$entry" in
      "$prefix" | "$prefix"/*) ;;
      *)
        echo "release asset $name has unexpected top-level entry: $entry" >&2
        exit 1
        ;;
    esac
  done < "$entries"
}

require_archive_entry_matches_file() {
  local name="$1"
  local entry="$2"
  local expected_file="$3"
  local archive="$asset_dir/$name"
  local actual_file

  [[ -f "$archive" ]] || return 0
  actual_file="$(mktemp "$tmp_dir/archive-entry.XXXXXX")"
  extract_archive_entry "$archive" "$entry" > "$actual_file"
  if ! cmp -s "$expected_file" "$actual_file"; then
    echo "release asset $name entry $entry does not match $expected_file" >&2
    exit 1
  fi
}

require_tar_entry_executable() {
  local name="$1"
  local entry="$2"
  local archive="$asset_dir/$name"
  local mode
  local owner_x
  local group_x
  local other_x

  [[ -f "$archive" ]] || return 0
  case "$archive" in
    *.tar.gz) ;;
    *)
      echo "release asset $name is not a tar.gz archive" >&2
      exit 1
      ;;
  esac

  mode="$(tar -tvzf "$archive" "$entry" | awk 'NR == 1 { print $1 }')"
  if [[ -z "$mode" ]]; then
    echo "release asset $name is missing executable entry: $entry" >&2
    exit 1
  fi

  owner_x="${mode:3:1}"
  group_x="${mode:6:1}"
  other_x="${mode:9:1}"
  if [[ "$owner_x$group_x$other_x" != *x* && "$owner_x$group_x$other_x" != *s* && "$owner_x$group_x$other_x" != *t* ]]; then
    echo "release asset $name entry $entry is not executable" >&2
    exit 1
  fi
}

require_package_docs() {
  local name="$1"

  require_archive_entry_matches_file "$name" arca/README.md docs/package-readme.md
  require_archive_entry_matches_file "$name" arca/LICENSE LICENSE
  require_archive_entry_matches_file "$name" arca/THIRD_PARTY_LICENSES.md docs/third-party-licenses.md
}

verify_archive_payloads arca-linux-x86_64.tar.gz
require_archive_safe_paths arca-linux-x86_64.tar.gz
require_archive_prefix arca-linux-x86_64.tar.gz arca
require_archive_entries arca-linux-x86_64.tar.gz \
  arca/arca \
  arca/README.md \
  arca/LICENSE \
  arca/THIRD_PARTY_LICENSES.md
require_tar_entry_executable arca-linux-x86_64.tar.gz arca/arca
require_package_docs arca-linux-x86_64.tar.gz

verify_archive_payloads arca-macos-native.tar.gz
require_archive_safe_paths arca-macos-native.tar.gz
require_archive_prefix arca-macos-native.tar.gz arca
require_archive_entries arca-macos-native.tar.gz \
  arca/arca \
  arca/README.md \
  arca/LICENSE \
  arca/THIRD_PARTY_LICENSES.md
require_tar_entry_executable arca-macos-native.tar.gz arca/arca
require_package_docs arca-macos-native.tar.gz

verify_archive_payloads arca-windows-x86_64.zip
require_archive_safe_paths arca-windows-x86_64.zip
require_archive_prefix arca-windows-x86_64.zip arca
require_archive_entries arca-windows-x86_64.zip \
  arca/arca.exe \
  arca/README.md \
  arca/LICENSE \
  arca/THIRD_PARTY_LICENSES.md
require_package_docs arca-windows-x86_64.zip

verify_archive_payloads arca-compat-fixtures.tar.gz
require_archive_safe_paths arca-compat-fixtures.tar.gz
require_archive_prefix arca-compat-fixtures.tar.gz compat-fixtures
require_archive_entries arca-compat-fixtures.tar.gz \
  compat-fixtures/EXPECTED.txt \
  compat-fixtures/README.txt \
  compat-fixtures/SHA256SUMS \
  compat-fixtures/verify-compat-extract.mjs \
  compat-fixtures/arca-plain.zip \
  compat-fixtures/arca-aes.zip \
  compat-fixtures/arca-zipcrypto.zip \
  compat-fixtures/arca.tar.gz \
  compat-fixtures/expected/a.txt \
  compat-fixtures/expected/sub/b.txt \
  "compat-fixtures/expected/space name.txt"

if [[ -f "$asset_dir/arca-compat-fixtures.tar.gz" ]]; then
  compat_dir="$tmp_dir/compat-check"
  mkdir -p "$compat_dir"
  tar -C "$compat_dir" -xzf "$asset_dir/arca-compat-fixtures.tar.gz" \
    compat-fixtures/EXPECTED.txt \
    compat-fixtures/SHA256SUMS \
    compat-fixtures/verify-compat-extract.mjs \
    compat-fixtures/arca-plain.zip \
    compat-fixtures/arca-aes.zip \
    compat-fixtures/arca-zipcrypto.zip \
    compat-fixtures/arca.tar.gz \
    compat-fixtures/expected/a.txt \
    compat-fixtures/expected/sub/b.txt \
    "compat-fixtures/expected/space name.txt"
  (
    cd "$compat_dir/compat-fixtures"
    verify_checksum SHA256SUMS
  )
fi

verify_archive_payloads arca-source.tar.gz
require_archive_safe_paths arca-source.tar.gz
require_archive_prefix arca-source.tar.gz arca-source
require_archive_entries arca-source.tar.gz \
  arca-source/Cargo.toml \
  arca-source/Cargo.lock \
  arca-source/LICENSE \
  arca-source/README.md \
  arca-source/docs/package-readme.md \
  arca-source/docs/third-party-licenses.md \
  arca-source/.github/workflows/ci.yml \
  arca-source/.github/workflows/release.yml \
  arca-source/crates/arca-core/src/lib.rs \
  arca-source/crates/arca-cli/src/main.rs \
  arca-source/scripts/package.sh \
  arca-source/scripts/package.ps1 \
  arca-source/scripts/verify-compat-extract.mjs \
  arca-source/scripts/version-check.mjs
require_archive_entry_matches_file arca-source.tar.gz arca-source/LICENSE LICENSE
require_archive_entry_matches_file arca-source.tar.gz arca-source/README.md README.md
require_archive_entry_matches_file \
  arca-source.tar.gz \
  arca-source/docs/package-readme.md \
  docs/package-readme.md
require_archive_entry_matches_file \
  arca-source.tar.gz \
  arca-source/docs/third-party-licenses.md \
  docs/third-party-licenses.md

echo "release assets ok"
