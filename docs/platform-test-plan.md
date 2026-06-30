# Platform Test Plan

This plan is the release gate for cross-platform Arca builds. It separates automated CI coverage
from manual OS integration checks that GitHub-hosted runners cannot prove.

## Scope

Test every release candidate on:

- Linux x86_64, through GitHub Actions `ubuntu-latest` and a local Linux preflight.
- macOS native, through GitHub Actions `macos-latest` plus Archive Utility manual checks.
- Windows x86_64, through GitHub Actions `windows-latest` plus Explorer manual checks.

The candidate is not v1-ready until CI is green and the manual checks below have recorded evidence.

## Local Preflight

Run from the repository root before pushing:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
node ./scripts/license-check.mjs --check docs/third-party-licenses.md
node ./scripts/version-check.mjs
ARCA_REQUIRE_7ZIP=1 ./scripts/smoke.sh
bash ./scripts/package.sh
ARCA_REQUIRE_7ZIP=1 bash ./scripts/compat-fixtures.sh
bash ./scripts/source-archive.sh
mkdir -p release-assets
cp arca-linux-x86_64.tar.gz arca-linux-x86_64.tar.gz.sha256 release-assets/
cp arca-compat-fixtures.tar.gz arca-compat-fixtures.tar.gz.sha256 release-assets/
cp arca-source.tar.gz arca-source.tar.gz.sha256 release-assets/
ARCA_EXPECTED_RELEASE_ASSETS="arca-linux-x86_64.tar.gz arca-linux-x86_64.tar.gz.sha256 arca-compat-fixtures.tar.gz arca-compat-fixtures.tar.gz.sha256 arca-source.tar.gz arca-source.tar.gz.sha256" \
  bash ./scripts/verify-release-assets.sh release-assets
```

Clean generated files after the preflight:

```bash
rm -rf arca-linux-x86_64.tar.gz arca-linux-x86_64.tar.gz.sha256 \
  arca-compat-fixtures.tar.gz arca-compat-fixtures.tar.gz.sha256 \
  arca-source.tar.gz arca-source.tar.gz.sha256 release-assets dist
```

## GitHub Actions Gate

After pushing the candidate branch, trigger or inspect the CI workflow:

```bash
gh workflow run ci.yml -R crasnec/arca --ref main
gh run list -R crasnec/arca --workflow CI --limit 5
gh run watch -R crasnec/arca <run-id> --exit-status
```

The CI gate must pass these jobs on Linux, macOS, and Windows:

- formatting
- clippy with `-D warnings`
- license and version checks
- workspace tests
- smoke tests with 7-Zip required
- package dry run
- Linux compatibility fixture build
- Linux source archive build
- Linux release asset verification dry run

Before tagging, run the release workflow in dispatch mode to prove all platform packages build:

```bash
gh workflow run release.yml -R crasnec/arca --ref main
gh run list -R crasnec/arca --workflow Release --limit 5
gh run watch -R crasnec/arca <run-id> --exit-status
```

The dispatch run must upload:

- `arca-linux-x86_64.tar.gz`
- `arca-macos-native.tar.gz`
- `arca-windows-x86_64.zip`
- `arca-compat-fixtures.tar.gz`
- `arca-source.tar.gz`
- matching `.sha256` files

## Linux Manual Check

Use the Linux package from the release workflow artifacts:

```bash
sha256sum -c arca-linux-x86_64.tar.gz.sha256
tar -xzf arca-linux-x86_64.tar.gz
./arca/arca --version
./arca/arca compress arca/README.md -o linux-smoke.zip --overwrite
./arca/arca test linux-smoke.zip
./arca/arca extract linux-smoke.zip -o linux-smoke-out --overwrite
7z t linux-smoke.zip
```

Pass criteria:

- checksum verification succeeds
- packaged binary runs
- Arca-created ZIP tests and extracts
- 7-Zip can test the Arca-created ZIP

## macOS Manual Check

Use the macOS package and compatibility fixture artifacts:

```bash
shasum -a 256 -c arca-macos-native.tar.gz.sha256
tar -xzf arca-macos-native.tar.gz
./arca/arca --version
./arca/arca compress arca/README.md -o macos-smoke.zip --overwrite
./arca/arca test macos-smoke.zip
./arca/arca extract macos-smoke.zip -o macos-smoke-out --overwrite
```

Then extract `arca-compat-fixtures.tar.gz`, open `compat-fixtures/arca-plain.zip` with Archive
Utility, and compare the extracted contents with `compat-fixtures/expected/`.

Optional byte-for-byte helper:

```bash
node compat-fixtures/verify-compat-extract.mjs <archive-utility-output-dir> compat-fixtures/expected
```

Pass criteria:

- checksum verification succeeds
- packaged binary runs
- Arca-created ZIP tests and extracts
- Archive Utility extracts `arca-plain.zip`
- extracted files match `EXPECTED.txt`

## Windows Manual Check

Use the Windows package and compatibility fixture artifacts in PowerShell:

```powershell
Get-FileHash .\arca-windows-x86_64.zip -Algorithm SHA256
Expand-Archive .\arca-windows-x86_64.zip -DestinationPath .\arca-package
.\arca-package\arca\arca.exe --version
.\arca-package\arca\arca.exe compress .\arca-package\arca\README.md -o .\windows-smoke.zip --overwrite
.\arca-package\arca\arca.exe test .\windows-smoke.zip
.\arca-package\arca\arca.exe extract .\windows-smoke.zip -o .\windows-smoke-out --overwrite
```

Compare the printed hash with `arca-windows-x86_64.zip.sha256`.

Then extract `arca-compat-fixtures.tar.gz`, open `compat-fixtures\arca-plain.zip` with Windows
Explorer, and compare the extracted contents with `compat-fixtures\expected\`.

Optional byte-for-byte helper:

```powershell
node .\compat-fixtures\verify-compat-extract.mjs <explorer-output-dir> .\compat-fixtures\expected
```

Pass criteria:

- checksum matches the `.sha256` file
- packaged binary runs
- Arca-created ZIP tests and extracts
- Windows Explorer extracts `arca-plain.zip`
- extracted files match `EXPECTED.txt`

## Security Regression Gate

The automated tests and smoke checks must continue to prove:

- Zip Slip paths are rejected before publishing output.
- Zip Bomb limits reject excessive entry counts, per-entry size, total unpacked size, compression
  ratio, and symlink target size.
- Nested archives are not recursively extracted; if extracted later, the same limits apply again.
- Symlink escapes, tar hardlinks, tar special files, and non-directory prefix conflicts are rejected.
- Filename bypasses are rejected, including absolute paths, Windows drive paths, ADS/colon paths,
  trailing dot or space components, control characters, and Unicode normalization collisions.

If any item fails, do not tag. Fix the implementation, add or update a regression test, and rerun
the full gate.

## Evidence Record

Record one block per release candidate:

```text
Candidate:
Commit:
CI run URL:
Release dispatch run URL:

Linux:
  Runner CI: pass/fail
  Manual package hash: pass/fail
  Manual binary smoke: pass/fail
  7-Zip check: pass/fail

macOS:
  Runner CI: pass/fail
  Manual package hash: pass/fail
  Manual binary smoke: pass/fail
  Archive Utility fixture: pass/fail

Windows:
  Runner CI: pass/fail
  Manual package hash: pass/fail
  Manual binary smoke: pass/fail
  Explorer fixture: pass/fail

Security regression gate: pass/fail
Notes:
```
