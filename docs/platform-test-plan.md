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
npm run gui:smoke
npm run gui:typecheck
npm run gui:web:build
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

Linux local preflight for the GUI Rust workspace requires Tauri/WebKitGTK development packages
before running full-workspace Cargo checks: `libdbus-1-dev`, `libwebkit2gtk-4.1-dev`,
`libayatana-appindicator3-dev`, `librsvg2-dev`, and `patchelf`.

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
- GUI M0 smoke check and frontend production build
- workspace tests
- smoke tests with 7-Zip required
- package dry run
- Linux compatibility fixture build
- Linux source archive build
- Linux release asset verification dry run

The release asset verifier must require GUI source files, generated desktop icon assets,
`apps/arca-gui/src-tauri/windows/nsis-shell-context.nsh`, and `scripts/gui-smoke.mjs` inside
`arca-source.tar.gz` so source releases preserve the GUI and shell-integration validation material.

For the GUI foundation, CI is expected to prove the desktop app source tree is present, Tauri
capabilities do not grant broad frontend filesystem permissions, the CSP stays restricted, npm
licenses are included in `docs/third-party-licenses.md`, read-only `list_archive`/`test_archive`
plus extract commands call `arca-core` directly, and the React workbench builds. Tauri command
responses should use GUI-specific `camelCase` DTOs, including open-manifest validation state, while
CLI JSON output remains unchanged. Direct Editing add-plan DTOs should not expose local source paths
to React; the frontend should retain only user-selected inputs and archive entry names needed for
Save. Full-archive Test and Extract successes should promote the open
GUI session to fully validated and refresh the open manifest/digest when no Direct Editing changes
are pending; selected-entry operations must not claim whole-archive payload validation. Unsaved
Direct Editing changes should suppress fully-validated status until the archive is saved and tested
again. Password-required
test/extract operations must prompt without keeping plaintext passwords in React state. Startup
archive arguments are filtered to existing files with supported archive suffixes before auto-open.
Drag-and-drop open uses Tauri webview drop events and still routes the dropped path through the Rust
`list_archive` command boundary. Selected-entry test/extract commands must use `arca-core` selection
APIs and preserve the same archive validation, staging, and password rules as whole-archive
operations. File-dialog open and extract destination picking grant only
`dialog:allow-open`/`dialog:allow-save`; selected paths still flow into Rust archive commands rather
than frontend filesystem access. New archive creation must use the Rust `create_archive` command
backed by `arca-core::compress`, including AES-256 ZIP password passing without React password
state. AES-256 password input should only be enabled for `.zip` outputs so non-ZIP creation does
not read a password before core policy rejects encryption. Extract/create overwrite must stay opt-in
through a Replace/Replace All prompt and must not turn security errors such as symlink overwrite
refusal into replaceable prompts. Single-stream creation for `.gz`, `.bz2`, and `.xz` should allow
only one file input and disable folder addition before invoking core. The Workbench
context menu should test/extract selected entries, test the whole archive, and copy selected archive
paths without broad filesystem or shell permissions. New archive input drag-and-drop should add paths
only while the create dialog is open; otherwise drops continue to open archives through the Rust
`list_archive` command boundary. Plain ZIP delete/add/replace should stay pending until explicit
Save. Add planning must call the Rust `plan_direct_edit_add` command so Replacement Prompt outcomes
come from core archive policy rather than frontend string matching, while the prompt exposes
per-entry Skip/Replace plus Skip All/Replace All choices for bulk additions. Additional add batches
before Save must pass existing pending add entries back into the Rust planner so pending-vs-new
conflicts are rejected by core policy. Save must call the Rust `save_direct_edit` command with the
open archive digest so stale archives are rejected before rewrite. Multiple pending Direct Editing
changes must be undoable and redoable step-by-step before Save. Opening another archive or starting
a new archive with pending Direct Editing changes must
show an unsaved-change prompt and discard those changes only after explicit confirmation.
Workbench shortcuts should cover open, new archive, save pending changes, undo, redo, and delete
selected entries without treating text-field editing as archive deletion. Generated desktop bundle
icons must be present and wired in `tauri.conf.json`, and file associations must advertise only
supported archive suffixes with Viewer-only roles plus Linux MIME mappings. Windows shell Context
Menu registration must be wired through the NSIS installer hook for the same supported suffix set,
and `npm run gui:smoke` must statically verify install/uninstall extension symmetry plus the exact
Explorer labels, icon registration, quoted command template, and startup flags. The app must parse
the resulting startup requests as open/test/extract actions before routing them through Rust archive
commands. Core mutating operations must hold interprocess Target Locks
around archive creation, extraction publishing, single-stream extraction publishing, and Direct
Editing save publishing so CLI, GUI, and shell-triggered operations do not publish to the same target
concurrently. GUI archive commands must pass through the Rust Operation Registry so operation
handles are claimed at most once, frontend failures before archive-command invocation discard only
unclaimed handles, malformed invocations finish claimed handles, and operation progress events plus
Cancel Requests cross the Tauri boundary. Core digest, scan, copy, compression, extraction, testing,
and Direct Editing rewrite loops
must observe cancellation cooperatively, while cancel requests during `Committing` are rejected and
window close or app-exit requests are blocked until the commit finishes. Deferred app exit must be
retried after active operations drain. Single-instance routing must forward later file-association
and shell Context Menu startup requests into the running GUI process without granting frontend
filesystem or shell permissions. The frontend must show operation progress in the status bar,
including percentages when core totals are available and an indeterminate state otherwise.
Core progress uses operation-specific extract/test phases and reports determinate totals for
archive creation, extraction, testing, ZIP listing, single-stream payloads, and Direct Editing ZIP
rewrites where totals are available. Parallel ZIP compression, extraction, and testing use shared
aggregate progress counters so worker chunks do not reset status-bar percentages. Core cancellation
cleanup coverage now exercises container archive creation, single-stream archive creation,
container extraction, single-stream extraction, and Direct Editing save staging paths.

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

For GUI installer candidates, install the Arca NSIS bundle and right-click `arca-plain.zip` in
Explorer. The exact registry hook shape is statically covered by `npm run gui:smoke`; the manual
check is for Explorer rendering and installed-app behavior. Verify that `Open in Arca`,
`Extract with Arca`, and `Test with Arca` are present, use the Arca icon, and launch the GUI with
the selected archive path. `Extract with Arca` must extract through the GUI/Rust command boundary
and show the same password/replace prompts as toolbar extraction. `Test with Arca` must report
pass/fail without shelling out to the CLI.

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
- GUI installer shell Context Menu entries appear for supported archive files and route to
  open/extract/test startup actions.

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
