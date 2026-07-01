# Release Checklist

Arca releases are GPL-3.0-or-later native CLI builds produced from tags. The GUI foundation is
validated in CI and the source archive, but GUI bundles are separate future release artifacts.

## Before Tagging

- Run `cargo fmt --all --check`.
- Run `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Run `node ./scripts/license-check.mjs --check docs/third-party-licenses.md`.
- Run `node ./scripts/version-check.mjs`; on tag builds this also checks `GITHUB_REF_NAME` against
  the `arca-cli` Cargo version.
- Run `npm run gui:smoke`.
- Run `npm run gui:typecheck`.
- Run `npm run gui:web:build`.
- Run `cargo test --workspace --locked`.
- Run `ARCA_REQUIRE_7ZIP=1 ./scripts/smoke.sh` on Linux or macOS.
- Run `$env:ARCA_REQUIRE_7ZIP = "1"; ./scripts/smoke.ps1` on Windows PowerShell.
- Run `bash ./scripts/package.sh` on Linux or macOS.
- Run `pwsh ./scripts/package.ps1` on Windows.
- Run `ARCA_REQUIRE_7ZIP=1 bash ./scripts/compat-fixtures.sh` on Linux.
- Run `bash ./scripts/source-archive.sh` on Linux; this also verifies extracted source metadata and
  compiles workspace tests with `--no-run`, runs `npm ci`, and runs GUI smoke plus GUI web build
  checks from the extracted source bundle.
- Run `bash ./scripts/verify-release-assets.sh <release-assets-dir>` before publishing; this checks
  the exact asset set, one-line SHA-256 manifests that reference their matching archives, readable
  archive payloads, ZIP payload CRCs, archive entry path safety, expected top-level archive
  directories, required package/source/fixture entries, packaged GPL/readme/third-party notice
  content, GUI source/icon/NSIS hook entries in the source archive, and the fixture bundle's
  internal `SHA256SUMS`.
- Complete `docs/platform-test-plan.md` for the release candidate and record CI plus manual
  platform evidence.
- Confirm the CI workflow is green on Linux, macOS, and Windows.
- Confirm AES ZIP, ZipCrypto ZIP, tar-gzip, plain ZIP, and single-stream gzip roundtrips work.
- Confirm 7-Zip compatibility smoke is not skipped in CI or release workflows.
- Confirm the exact `zip` backend pin in `Cargo.toml` is still intentional.
- Confirm GUI foundation hardening remains true: strict Tauri CSP, no production devtools, no broad
  frontend filesystem capability, direct Rust command boundary, `list_archive`/`test_archive`
  /`extract_archive` using `arca-core`, no React plaintext password state, and npm dependency
  license coverage.
- Confirm GUI Tauri commands return GUI DTOs with `camelCase` payloads, including manifest
  validation state, without changing CLI JSON output.
- Confirm Direct Editing add-plan DTOs do not expose local source paths to React; the frontend keeps
  only user-selected inputs and archive entry names needed for save.
- Confirm full-archive Test and Extract successes promote the open GUI session to fully validated,
  refresh the open manifest/digest when no Direct Editing changes are pending, and selected-entry
  operations do not overstate whole-archive payload validation.
- Confirm unsaved Direct Editing changes suppress fully-validated status in the Workbench until the
  archive is saved and tested again.
- Confirm GUI startup archive arguments are filtered to supported existing archive files before
  auto-open so file association launches do not bypass the Rust archive format policy.
- Confirm GUI drag-and-drop open is wired through Tauri webview drop events and still opens archives
  via the Rust `list_archive` command instead of frontend filesystem permissions.
- Confirm GUI selected-entry test/extract uses Rust `test_selection`/`extract_selection` command
  paths, including password retry behavior and explicit overwrite confirmation.
- Confirm GUI file dialogs grant only `dialog:allow-open`/`dialog:allow-save` and pass selected
  paths into Rust commands; no broad frontend filesystem permission should be present.
- Confirm GUI new archive creation uses the Rust `create_archive` command backed by
  `arca-core::compress`, including AES-256 ZIP password handling without React password state.
  AES-256 password input should only be enabled after the output path uses `.zip` so non-ZIP
  creation does not collect a password before core rejects it.
- Confirm GUI single-stream creation for `.gz`, `.bz2`, and `.xz` allows only one file input and
  disables folder addition before the command reaches core.
- Confirm GUI output Replace/Replace All prompts only appear for usage-level `already exists` errors
  and never convert security errors such as symlink overwrite refusal into overwrite retries.
- Confirm GUI create-dialog drag-and-drop adds inputs, ordinary workbench drops still open archives,
  and the internal context menu can test/extract selected entries, test the archive, and copy paths.
- Confirm GUI Plain ZIP delete/add/replace stays pending until explicit Save, add Replacement Prompt
  outcomes come from the Rust `plan_direct_edit_add` command, expose per-entry Skip/Replace plus
  Skip All/Replace All choices, additional add batches are checked against existing pending add
  entries by the Rust planner, Save passes the open archive digest to the Rust `save_direct_edit`
  command, stale digest saves fail, and encrypted ZIP archives remain read-only.
- Confirm GUI multiple pending Direct Editing changes can be undone and redone step-by-step before
  Save.
- Confirm GUI opening another archive or starting a new archive with pending Direct Editing changes
  shows an unsaved-change prompt and discards those changes only after explicit confirmation.
- Confirm GUI Workbench shortcuts cover open, new archive, save pending changes, undo, redo, and
  delete selected entries without hijacking text-field editing.
- Confirm GUI desktop bundle icons are generated, listed in `tauri.conf.json`, and the file
  associations register only supported archive extensions with Viewer roles and Linux MIME mappings.
- Confirm GUI Windows NSIS shell Context Menu hook registers only supported archive extensions,
  removes those keys on uninstall, and launches `open`, `extract`, and `test` startup requests
  through the GUI/Rust command boundary. `npm run gui:smoke` statically checks install/uninstall
  extension symmetry plus the exact Explorer labels, icon registration, quoted command template,
  and startup flags.
- Confirm a later file-association or Windows shell Context Menu launch routes into the running GUI
  through `tauri-plugin-single-instance`, focuses the main window, and preserves the requested
  open/test/extract action after any unsaved-change confirmation.
- Confirm GUI archive commands pass through the Rust Operation Registry, emit operation progress
  events, claim each operation handle at most once, finish claimed handles on validation failures,
  discard only unclaimed handles when the frontend fails before invoking an archive command, deliver
  Cancel Requests into `arca-core` cancellation tokens, and reject cancel requests once an operation
  reaches `Committing`.
- Confirm GUI operation progress is visible in the status bar, shows percentages when totals are
  available, and falls back to an indeterminate state for unknown totals.
- Confirm core progress reports operation-specific extract/test phases and determinate totals for
  archive creation, extraction, testing, ZIP listing, single-stream payloads, and Direct Editing ZIP
  rewrites where totals are available. Parallel ZIP compression, extraction, and testing should use
  aggregate progress counters rather than worker-local totals.
- Confirm GUI window close requests during `Committing` are blocked by Rust window-event handling
  and show a waiting message without granting frontend `core:window:*` permissions.
- Confirm GUI app-exit requests during `Committing` are blocked by Rust run-event handling and
  retried after active operations drain.
- Confirm core mutating operations still acquire interprocess Target Locks for archive creation,
  extraction publishing, single-stream extraction publishing, and Direct Editing save publishing.
- Confirm cancellation cleanup tests cover container archive creation, single-stream archive
  creation, container extraction, single-stream extraction, and Direct Editing save staging paths
  without publishing outputs or leaving `.arca-*` temp entries.
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
The source bundle includes the GUI source tree, root npm manifests, Tauri config, frontend source,
icon source, generated desktop icon assets, Windows NSIS shell Context Menu hook, and npm dependency
license data even before GUI binary bundles are published. The release asset verifier requires the
generated desktop icon assets, `windows/nsis-shell-context.nsh`, and `scripts/gui-smoke.mjs` to be
present in `arca-source.tar.gz`.
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
- Windows Explorer shell Context Menu open/extract/test for GUI installer candidates.
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
