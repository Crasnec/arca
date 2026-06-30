#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
archive="${ARCA_COMPAT_ARCHIVE:-arca-compat-fixtures.tar.gz}"
if [[ -n "${ARCA_BIN:-}" ]]; then
  binary="$ARCA_BIN"
else
  binary="$root/target/release/arca"
  cargo build --release --locked --bin arca --manifest-path "$root/Cargo.toml"
fi
work="$root/dist/compat-fixtures"
check="$root/dist/compat-check"

case "$archive" in
  /*) archive_path="$archive" ;;
  *) archive_path="$root/$archive" ;;
esac

if [[ ! -x "$binary" ]]; then
  echo "release binary not found or not executable: $binary" >&2
  exit 1
fi

seven_zip="${SEVEN_ZIP_BIN:-}"
if [[ -z "$seven_zip" ]]; then
  seven_zip="$(command -v 7z || command -v 7zz || command -v 7za || true)"
fi
require_seven_zip="${ARCA_REQUIRE_7ZIP:-0}"

rm -rf "$work" "$check"
mkdir -p "$work/expected/sub" "$check"
printf 'hello arca\n' > "$work/expected/a.txt"
printf 'nested\n' > "$work/expected/sub/b.txt"
printf 'file with spaces\n' > "$work/expected/space name.txt"
cat > "$work/EXPECTED.txt" <<'EXPECTED'
Expected extracted files for arca-plain.zip:

- a.txt: hello arca
- sub/b.txt: nested
- space name.txt: file with spaces

The extracted file tree must match expected/ exactly.
EXPECTED
cp "$root/scripts/verify-compat-extract.mjs" "$work/verify-compat-extract.mjs"

"$binary" compress "$work/expected" -o "$work/arca-plain.zip" --jobs 4 --quiet
printf 'secret\n' | "$binary" compress "$work/expected" -o "$work/arca-aes.zip" \
  --jobs 4 --password-stdin --quiet
printf 'secret\n' | "$binary" compress "$work/expected" -o "$work/arca-zipcrypto.zip" \
  --password-stdin --zipcrypto --quiet
"$binary" compress "$work/expected" -o "$work/arca.tar.gz" --quiet

"$binary" test "$work/arca-plain.zip" --jobs 4 --quiet
printf 'secret\n' | "$binary" test "$work/arca-aes.zip" --jobs 4 --password-stdin --quiet
printf 'secret\n' | "$binary" test "$work/arca-zipcrypto.zip" --password-stdin --quiet
"$binary" test "$work/arca.tar.gz" --quiet

"$binary" extract "$work/arca-plain.zip" -o "$check/plain" --jobs 4 --quiet
diff -r "$work/expected" "$check/plain"
if command -v node >/dev/null 2>&1; then
  node "$work/verify-compat-extract.mjs" "$check/plain" "$work/expected" >/dev/null
fi
printf 'secret\n' | "$binary" extract "$work/arca-aes.zip" -o "$check/aes" \
  --jobs 4 --password-stdin --quiet
diff -r "$work/expected" "$check/aes"
printf 'secret\n' | "$binary" extract "$work/arca-zipcrypto.zip" -o "$check/zipcrypto" \
  --password-stdin --quiet
diff -r "$work/expected" "$check/zipcrypto"
"$binary" extract "$work/arca.tar.gz" -o "$check/tgz" --quiet
diff -r "$work/expected" "$check/tgz"

if [[ -n "$seven_zip" ]]; then
  mkdir -p "$check/7z-plain" "$check/7z-aes" "$check/7z-zipcrypto"
  "$seven_zip" x "$work/arca-plain.zip" "-o$check/7z-plain" -y >/dev/null
  diff -r "$work/expected" "$check/7z-plain"
  "$seven_zip" x "$work/arca-aes.zip" "-o$check/7z-aes" -psecret -y >/dev/null
  diff -r "$work/expected" "$check/7z-aes"
  "$seven_zip" x "$work/arca-zipcrypto.zip" "-o$check/7z-zipcrypto" -psecret -y >/dev/null
  diff -r "$work/expected" "$check/7z-zipcrypto"
elif [[ "$require_seven_zip" == "1" ]]; then
  echo "7-Zip not found but ARCA_REQUIRE_7ZIP=1" >&2
  exit 1
fi

cat > "$work/README.txt" <<'README'
Arca manual compatibility fixtures

Expected files are in expected/.
EXPECTED.txt lists the expected file names and contents.

Manual checks:
- Windows Explorer: open/extract arca-plain.zip and compare the extracted files with expected/.
- macOS Archive Utility: open/extract arca-plain.zip and compare the extracted files with expected/.
- Optional scripted comparison: node verify-compat-extract.mjs <extracted-dir> expected
- 7-Zip: extract arca-plain.zip, arca-aes.zip, and arca-zipcrypto.zip.
- Password for arca-aes.zip and arca-zipcrypto.zip: secret

The plain ZIP check passes only if the extracted tree contains exactly:
- a.txt
- sub/b.txt
- space name.txt

Explorer and Archive Utility may not support AES or ZipCrypto archives.
Use those encrypted fixtures for 7-Zip and Arca password checks.
README

(
  cd "$work"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum EXPECTED.txt verify-compat-extract.mjs \
      arca-plain.zip arca-aes.zip arca-zipcrypto.zip arca.tar.gz \
      expected/a.txt expected/sub/b.txt "expected/space name.txt" > SHA256SUMS
  else
    shasum -a 256 EXPECTED.txt verify-compat-extract.mjs \
      arca-plain.zip arca-aes.zip arca-zipcrypto.zip arca.tar.gz \
      expected/a.txt expected/sub/b.txt "expected/space name.txt" > SHA256SUMS
  fi
)

rm -f "$archive_path" "$archive_path.sha256"
tar -C "$root/dist" -czf "$archive_path" compat-fixtures

if command -v sha256sum >/dev/null 2>&1; then
  hash="$(sha256sum "$archive_path" | awk '{print $1}')"
else
  hash="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
fi
printf '%s  %s\n' "$hash" "$(basename "$archive_path")" > "$archive_path.sha256"

printf '%s\n' "$archive_path"
