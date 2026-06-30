# Release Checklist

Arca releases are GPL-3.0-or-later native CLI builds produced from tags.

## Before Tagging

- Run `cargo fmt --all --check`.
- Run `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Run `node ./scripts/license-check.mjs --check docs/third-party-licenses.md`.
- Run `node ./scripts/version-check.mjs`; on tag builds this also checks `GITHUB_REF_NAME` against
  the `arca-cli` Cargo version.
- Run `cargo test --workspace --locked`.
- Run `ARCA_REQUIRE_7ZIP=1 ./scripts/smoke.sh` on Linux or macOS.
- Run `$env:ARCA_REQUIRE_7ZIP = "1"; ./scripts/smoke.ps1` on Windows PowerShell.
- Run `bash ./scripts/package.sh` on Linux or macOS.
- Run `pwsh ./scripts/package.ps1` on Windows.
- Run `ARCA_REQUIRE_7ZIP=1 bash ./scripts/compat-fixtures.sh` on Linux.
- Run `bash ./scripts/source-archive.sh` on Linux; this also verifies extracted source metadata and
  compiles workspace tests with `--no-run`.
- Run `bash ./scripts/verify-release-assets.sh <release-assets-dir>` before publishing; this checks
  the exact asset set, one-line SHA-256 manifests that reference their matching archives, readable
  archive payloads, ZIP payload CRCs, archive entry path safety, expected top-level archive
  directories, required package/source/fixture entries, packaged GPL/readme/third-party notice
  content, and the fixture bundle's internal `SHA256SUMS`.
- Complete `docs/platform-test-plan.md` for the release candidate and record CI plus manual
  platform evidence.
- Confirm the CI workflow is green on Linux, macOS, and Windows.
- Confirm AES ZIP, ZipCrypto ZIP, tar-gzip, plain ZIP, and single-stream gzip roundtrips work.
- Confirm 7-Zip compatibility smoke is not skipped in CI or release workflows.
- Confirm the exact `zip` backend pin in `Cargo.toml` is still intentional.
- Confirm binary release archives contain the package README, `LICENSE`, `THIRD_PARTY_LICENSES.md`,
  and a runnable `arca`/`arca.exe`.
- Confirm malicious archive fixture tests cover unsafe paths, collisions, non-directory prefix
  conflicts, unsafe symlinks, tar hardlinks, tar special files, non-UTF-8 tar paths, and failed
  extraction not publishing output or partial overwrite changes before v1.
- Confirm corrupt ZIP payloads, truncated tar payloads, truncated compressed tar archives, and
  truncated single-stream archives are rejected as integrity failures before v1.
- Confirm Zip Bomb guardrails are active for entry count, per-entry unpacked size, total unpacked
  size, compression ratio, and symlink target size.

## Tagging

Use semantic-ish tags for public builds:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow uploads:

- `arca-linux-x86_64.tar.gz`
- `arca-macos-native.tar.gz`
- `arca-windows-x86_64.zip`
- `arca-compat-fixtures.tar.gz`
- `arca-source.tar.gz`
- SHA-256 checksum files for each archive

The first three archives are user-facing app packages. `arca-source.tar.gz` is the matching GPL
source bundle. `arca-compat-fixtures.tar.gz` is a compatibility test fixture bundle, not an app
package. Keep that distinction clear in release notes and manual download instructions.

The release workflow uses `scripts/package.sh` and `scripts/package.ps1`, the same package scripts
used by CI dry runs. Those scripts extract the generated binary archive and verify required files
plus an exact `arca --version` match against the Cargo package version before writing checksums.
Unix package builds set the packaged `arca` mode to `755`.
The release workflow also runs `scripts/version-check.mjs`, so tag builds fail before packaging if
the pushed `v*` tag does not match the `arca-cli` Cargo package version.
It also uploads `arca-compat-fixtures.tar.gz` for Windows Explorer and macOS Archive Utility checks.
That fixture bundle includes `EXPECTED.txt`, `README.txt`, `SHA256SUMS`,
`verify-compat-extract.mjs`, expected files, and Arca-generated ZIP fixtures so manual GUI
extraction checks have a fixed expected result and an optional byte-for-byte comparison helper.
It uploads `arca-source.tar.gz` so binary releases have a matching, build-checked source bundle.
GitHub Release publication runs in a separate job after every package matrix job succeeds, so a
failed platform build should not create a partial public release.
The publish job verifies the exact release asset set and every `.sha256` checksum before uploading
files to GitHub Releases. It rejects checksum manifests that do not reference their matching
archives, then inspects each downloaded archive for readable archive payloads, ZIP payload CRCs,
archive entry path safety, expected top-level directories, required package/source/fixture entries,
Unix package binary executable bits, packaged GPL/readme/third-party notice content, and
compatibility fixture internal checksums.

## Manual Compatibility Checks

Before calling a build v1, verify at least one release candidate manually with:

- Windows Explorer ZIP open/extract.
- macOS Archive Utility ZIP open/extract.
- Use `arca-compat-fixtures.tar.gz`, compare extracted `arca-plain.zip` contents with `expected/`,
  and confirm they match `EXPECTED.txt`.
- Optionally run `node verify-compat-extract.mjs <extracted-dir> expected` inside the fixture bundle
  after a GUI extraction.
- 7-Zip create/extract on Linux, macOS, and Windows.
- Wrong-password behavior for AES ZIP and ZipCrypto ZIP.
- Corrupt ZIP payload, truncated tar payload, truncated compressed tar, and truncated single-stream
  integrity failures.
- Extraction rejection for traversal, absolute paths, Windows reserved names, symlink escapes,
  non-directory prefix conflicts, tar hardlinks, and tar special files.
- Failed extraction does not publish a destination directory, partial output, or partial overwrite
  changes.
