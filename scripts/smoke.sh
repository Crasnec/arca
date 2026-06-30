#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/arca-smoke.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

bin="${ARCA_BIN:-}"
if [[ -z "$bin" ]]; then
  cargo build --locked --bin arca --manifest-path "$root/Cargo.toml"
  bin="$root/target/debug/arca"
fi

seven_zip="${SEVEN_ZIP_BIN:-}"
if [[ -z "$seven_zip" ]]; then
  seven_zip="$(command -v 7z || command -v 7zz || command -v 7za || true)"
fi
require_seven_zip="${ARCA_REQUIRE_7ZIP:-0}"

write_traversal_zip() {
  local out="$1"
  {
    printf '\x50\x4b\x03\x04'
    printf '\x14\x00\x00\x00\x00\x00\x00\x00\x00\x00'
    printf '\x3e\x06\x3a\x18'
    printf '\x04\x00\x00\x00\x04\x00\x00\x00'
    printf '\x0e\x00\x00\x00'
    printf '../outside.txt'
    printf 'bad\n'
    printf '\x50\x4b\x01\x02'
    printf '\x14\x03\x14\x00\x00\x00\x00\x00\x00\x00\x00\x00'
    printf '\x3e\x06\x3a\x18'
    printf '\x04\x00\x00\x00\x04\x00\x00\x00'
    printf '\x0e\x00\x00\x00\x00\x00\x00\x00\x00\x00'
    printf '\x00\x00\x00\x00\x00\x00\x00\x00'
    printf '../outside.txt'
    printf '\x50\x4b\x05\x06'
    printf '\x00\x00\x00\x00\x01\x00\x01\x00'
    printf '\x3c\x00\x00\x00\x30\x00\x00\x00\x00\x00'
  } > "$out"
}

mkdir -p "$tmp/src/sub"
printf 'hello arca\n' > "$tmp/src/a.txt"
printf 'nested\n' > "$tmp/src/sub/b.txt"

"$bin" compress "$tmp/src" -o "$tmp/src.zip" --jobs 4 --quiet
"$bin" test "$tmp/src.zip" --jobs 4 --quiet
"$bin" extract "$tmp/src.zip" -o "$tmp/zip-out" --jobs 4 --quiet
diff -r "$tmp/src" "$tmp/zip-out"

"$bin" compress "$tmp/src" -o "$tmp/src.tar.gz" --quiet
"$bin" test "$tmp/src.tar.gz" --quiet
"$bin" extract "$tmp/src.tar.gz" -o "$tmp/tgz-out" --quiet
diff -r "$tmp/src" "$tmp/tgz-out"

for suffix in tar.bz2 tar.xz; do
  "$bin" compress "$tmp/src" -o "$tmp/src.$suffix" --quiet
  "$bin" test "$tmp/src.$suffix" --quiet
  "$bin" extract "$tmp/src.$suffix" -o "$tmp/$suffix-out" --quiet
  diff -r "$tmp/src" "$tmp/$suffix-out"
done

printf 'secret\n' | "$bin" compress "$tmp/src" -o "$tmp/aes.zip" --jobs 4 --password-stdin --quiet
printf 'secret\n' | "$bin" test "$tmp/aes.zip" --jobs 4 --password-stdin --quiet
if printf 'wrong\n' | "$bin" test "$tmp/aes.zip" --password-stdin --quiet 2> "$tmp/aes-wrong.err"; then
  echo "expected AES ZIP wrong password to be rejected" >&2
  exit 1
fi
grep -qi "password" "$tmp/aes-wrong.err"
printf 'secret\n' | "$bin" extract "$tmp/aes.zip" -o "$tmp/aes-out" --jobs 4 --password-stdin --quiet
diff -r "$tmp/src" "$tmp/aes-out"

printf 'secret\n' | "$bin" compress "$tmp/src" -o "$tmp/zipcrypto.zip" --password-stdin --zipcrypto --quiet
printf 'secret\n' | "$bin" test "$tmp/zipcrypto.zip" --password-stdin --quiet
if printf 'wrong\n' | "$bin" test "$tmp/zipcrypto.zip" --password-stdin --quiet 2> "$tmp/zipcrypto-wrong.err"; then
  echo "expected ZipCrypto ZIP wrong password to be rejected" >&2
  exit 1
fi
grep -qi "password" "$tmp/zipcrypto-wrong.err"
printf 'secret\n' | "$bin" extract "$tmp/zipcrypto.zip" -o "$tmp/zipcrypto-out" --password-stdin --quiet
diff -r "$tmp/src" "$tmp/zipcrypto-out"

"$bin" compress "$tmp/src/a.txt" -o "$tmp/a.txt.gz" --quiet
"$bin" test "$tmp/a.txt.gz" --quiet
"$bin" extract "$tmp/a.txt.gz" -o "$tmp/a.out" --quiet
diff "$tmp/src/a.txt" "$tmp/a.out"

for suffix in bz2 xz; do
  "$bin" compress "$tmp/src/a.txt" -o "$tmp/a.txt.$suffix" --quiet
  "$bin" test "$tmp/a.txt.$suffix" --quiet
  "$bin" extract "$tmp/a.txt.$suffix" -o "$tmp/a.$suffix.out" --quiet
  diff "$tmp/src/a.txt" "$tmp/a.$suffix.out"
done

mkdir -p "$tmp/unsafe"
printf 'bad\n' > "$tmp/unsafe/CON.txt"
if "$bin" compress "$tmp/unsafe" -o "$tmp/unsafe.zip" --quiet 2> "$tmp/unsafe.err"; then
  echo "expected reserved Windows name to be rejected" >&2
  exit 1
fi
grep -qi "reserved" "$tmp/unsafe.err"

write_traversal_zip "$tmp/evil.zip"
if "$bin" test "$tmp/evil.zip" --quiet 2> "$tmp/evil-test.err"; then
  echo "expected traversal ZIP test to be rejected" >&2
  exit 1
fi
grep -qi "invalid archive path component" "$tmp/evil-test.err"
if "$bin" extract "$tmp/evil.zip" -o "$tmp/evil-out" --quiet 2> "$tmp/evil-extract.err"; then
  echo "expected traversal ZIP extraction to be rejected" >&2
  exit 1
fi
grep -qi "invalid archive path component" "$tmp/evil-extract.err"
test ! -e "$tmp/evil-out"
test ! -e "$tmp/outside.txt"

if [[ -n "$seven_zip" ]]; then
  (
    cd "$tmp/src"
    "$seven_zip" a -tzip "$tmp/by-7z.zip" . >/dev/null
  )
  "$bin" extract "$tmp/by-7z.zip" -o "$tmp/from-7z" --quiet
  diff -r "$tmp/src" "$tmp/from-7z"

  "$bin" compress "$tmp/src" -o "$tmp/arca-for-7z.zip" --jobs 4 --quiet
  mkdir -p "$tmp/7z-out"
  "$seven_zip" x "$tmp/arca-for-7z.zip" "-o$tmp/7z-out" >/dev/null
  diff -r "$tmp/src" "$tmp/7z-out"

  mkdir -p "$tmp/7z-aes-out"
  "$seven_zip" x "$tmp/aes.zip" "-o$tmp/7z-aes-out" -psecret -y >/dev/null
  diff -r "$tmp/src" "$tmp/7z-aes-out"

  mkdir -p "$tmp/7z-zipcrypto-out"
  "$seven_zip" x "$tmp/zipcrypto.zip" "-o$tmp/7z-zipcrypto-out" -psecret -y >/dev/null
  diff -r "$tmp/src" "$tmp/7z-zipcrypto-out"
else
  if [[ "$require_seven_zip" == "1" ]]; then
    echo "7-Zip not found but ARCA_REQUIRE_7ZIP=1" >&2
    exit 1
  fi
  echo "7-Zip not found; external compatibility smoke skipped"
fi

echo "arca smoke ok"
