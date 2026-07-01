# Arca

Arca is a GPLv3 cross-platform archive tool written in Rust.

This repository contains the v1 CLI implementation and the early Arca Archive Manager GUI
foundation. It is intentionally isolated from the surrounding WZ/Rulebook workspace.

## Commands

```bash
arca compress <inputs...> [-o <archive>] [--overwrite] [--level <0..9>] \
  [--jobs <N>] [--exclude <glob>...] [--password|--password-stdin] \
  [--zipcrypto] [--auto-tar]
arca extract <archive> [-o <dir-or-file>] [--overwrite] [--jobs <N>] [--password|--password-stdin]
arca list <archive> [--json]
arca test <archive> [--jobs <N>] [--password|--password-stdin]
```

There are no short aliases in v1.

## Supported Formats

Creation and extraction currently support:

- `.zip`
- `.tar`
- `.tar.gz`, `.tgz`
- `.tar.bz2`, `.tbz2`
- `.tar.xz`, `.txz`
- single-file `.gz`, `.bz2`, `.xz`

If `compress -o` has no suffix, Arca appends `.zip`. Unknown suffixes fail instead of being
silently reinterpreted.
If `extract -o` is omitted, Arca writes next to the archive with the archive suffix removed. For
single-file `.gz`, `.bz2`, and `.xz`, `-o` may be a file path or an existing directory.

## Safety Model

Arca applies the same archive path policy during creation and extraction. It rejects entries that
would be unsafe or unpredictable across Linux, macOS, and Windows, including:

- path traversal and absolute paths
- Windows drive paths
- Windows reserved names such as `CON.txt`
- ADS/colon paths
- trailing-space or trailing-dot components
- non-UTF-8 paths
- case/Unicode-normalization collisions
- non-directory prefix conflicts such as `link` plus `link/file`
- symlink targets that escape the archive root/destination
- tar hardlinks, device nodes, FIFOs, sockets, sparse files, and special entries

`list`, `test`, and `extract` validate archive entry paths before trusting metadata. They also
validate symlink targets when the target can be read without a password. Container extraction writes
into a staging directory first and publishes only after validation and extraction succeed.
Existing publish targets are preflighted before any file replacement so failed overwrite extraction
does not partially modify an existing output tree.
ZIP and tar extraction also use a second pass plus digest/manifest checks before publishing staged
output.

## Resource Limits

`list`, `test`, and `extract` enforce default resource limits before trusting archive payloads:

- maximum archive entries: `200000`
- maximum unpacked size for one entry: `17179869184` bytes
- maximum total unpacked size: `68719476736` bytes
- maximum compression ratio: `10000`
- maximum symlink target size: `16384` bytes

These defaults are Zip Bomb guardrails for v1. They can be adjusted with
`ARCA_MAX_ENTRIES`, `ARCA_MAX_ENTRY_UNPACKED_BYTES`, `ARCA_MAX_UNPACKED_BYTES`,
`ARCA_MAX_COMPRESSION_RATIO`, and `ARCA_MAX_SYMLINK_TARGET_BYTES`.
Nested archive files are treated as ordinary files; Arca does not recursively extract an archive
found inside another archive. If a nested archive is extracted in a later command, the same limits
apply again.

## Encryption

ZIP encryption is supported:

- `--password` prompts on the TTY.
- `--password-stdin` reads the first stdin line.
- AES-256 is the default.
- `--zipcrypto` explicitly enables traditional ZipCrypto and prints a warning.
- ZipCrypto creation is serialized; `compress --jobs > 1 --zipcrypto` is rejected.

Non-ZIP formats are not encrypted in v1. Password flags on non-ZIP `compress`, `extract`, and
`test` commands are rejected before password input is read.

## Backend Pin

Arca currently pins the Rust `zip` backend to `=9.0.0-pre2` because ZipCrypto creation is exposed
through the backend API used by v1.
That pin is intentional for reproducible release builds and is covered by AES, ZipCrypto,
wrong-password, and 7-Zip compatibility smoke tests.
Revisit this before changing the ZIP backend or dependency version.

## Graphical Interface

The GUI lives in `apps/arca-gui` as a Tauri 2 + React + TypeScript desktop Workbench foundation.
The current foundation provides the app/workspace layout, Tauri configuration, a strict initial
capability set, a non-archive health command, archive manifest/test/extract commands backed by
`arca-core`, new archive creation through `arca-core::compress`, AES-256 ZIP creation prompts,
explicit replace prompts for extract/create existing outputs, password prompts for password-required
test/extract operations, generated desktop app icons, file-association extension/MIME registration
for supported archive suffixes, startup archive argument handling for file-association/double-click
entry, file-dialog archive opening and extract destination picking through narrow Tauri dialog
permissions, drag-and-drop archive opening, selected-entry test/extract with a Workbench Context
Menu, Windows-first Explorer Context Menu installer hook wiring for open/extract/test actions,
single-instance routing of later file-association/shell launches into the running GUI process,
frontend build, source archive plumbing, and npm license coverage.

Archive workflows in the GUI are still under development. Tauri commands must call `arca-core`
directly for archive operations; the GUI must not shell out to the CLI, grant broad frontend
filesystem permissions, load remote content, or enable production devtools.

Useful GUI checks:

```bash
npm run gui:smoke
npm run gui:typecheck
npm run gui:web:build
```

Full Tauri Rust builds on Linux require WebKitGTK/appindicator development packages such as
`libdbus-1-dev`, `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, and
`patchelf`.

## Current Implementation Status

Implemented:

- Cargo workspace split into `arca-core`, `arca-cli`, and `arca-native`.
- Format detection and default output naming.
- ZIP, tar, tar+gzip/bzip2/xz, and single-file gzip/bzip2/xz create/extract/list/test paths.
- ZIP AES-256 and ZipCrypto create/extract through the Rust `zip` backend.
- Creation-side and extraction-side safe path policy.
- Output self-inclusion rejection.
- Basic `--exclude`, `--level`, `--overwrite`, and `--auto-tar`.
- Input tree change detection before publishing a newly compressed archive.
- ZIP/tar mtime restoration and Unix mode restoration for supported platforms.
- Real `--jobs` parallelism for ZIP creation (unencrypted/AES), extraction, and integrity testing.
- Cross-platform GitHub Actions CI definition for Linux, macOS, and Windows.
- Repeatable smoke scripts under `scripts/`, including optional 7-Zip compatibility checks.
- CI/release version checks for workspace package consistency and tag/version matching.
- CLI integration tests for explicit subcommands, JSON listing, and option validation.
- Malicious archive fixture tests for unsafe ZIP paths, ZIP/tar path collisions, non-directory
  prefix conflicts, unsafe ZIP symlinks, tar hardlinks, tar special entries, and non-UTF-8 tar paths
  on Unix.
- Integrity tests for corrupt ZIP payloads, truncated tar payloads, truncated compressed tar
  archives, and truncated single-stream archives.
- Zip Bomb guardrails for entry count, per-entry unpacked size, total unpacked size, compression
  ratio, and symlink target size, including non-recursive nested archive handling.
- User-facing package README for binary release archives.
- Tag-based GitHub Release packaging workflow with SHA-256 checksums.
- Release asset verifier covering app packages, the matching source bundle, and compatibility
  fixtures as distinct release assets.
- GUI foundation under `apps/arca-gui`, including Tauri/React workspace wiring, strict initial
  Tauri capability/CSP checks, `list_archive`, `test_archive`, and non-overwrite `extract_archive`
  commands plus selected-entry `test_selected_entries`/`extract_selected_entries` commands that
  call `arca-core` directly, `create_archive` calling `arca-core::compress`, password retry/create
  prompts that avoid React password state and only enable AES-256 creation passwords for `.zip`
  outputs, single-stream creation UI limits for `.gz`/`.bz2`/`.xz`, GUI-specific camelCase Tauri
  DTOs that avoid exposing Direct Editing source paths and include open-manifest validation state
  plus full-archive Test/Extract validation promotion with manifest refresh that
  is suppressed while Direct Editing changes are pending, explicit replace prompts that retry
  extract/create with `overwrite` only after user confirmation, startup archive argument handling
  for supported archive paths, file-dialog archive opening and extract destination picking through
  `dialog:allow-open`/`dialog:allow-save`, drag-and-drop archive opening, frontend type/build
  checks, create-dialog drag-and-drop input addition, internal context-menu copy/test/extract
  actions, range selection, Plain ZIP pending delete/add/replace with explicit Save through
  digest-checked `plan_direct_edit_add` and `save_direct_edit` commands, core-planned Replacement
  Prompt outcomes with per-entry Skip/Replace plus Skip All/Replace All choices, multi-batch add
  planning that rejects conflicts with existing pending additions in core, multi-step
  pending-change undo/redo history, unsaved-change confirmation before
  open/new archive transitions, Workbench keyboard shortcuts for common open/new/save/undo/redo/delete
  actions, generated desktop bundle icons, archive file-association extension/MIME validation, and
  npm license/source archive coverage. The GUI also has a Windows NSIS shell Context Menu hook for
  supported archive suffixes plus `startup_requests` handling for `open`, `test`, and `extract`
  shell actions; GUI smoke statically verifies the hook's install/uninstall extension symmetry,
  labels, icon registration, quoted command template, and startup flags. It also has official Tauri
  single-instance routing that forwards later startup requests into the running GUI process. A
  Rust-owned operation registry gives GUI commands operation handles,
  single-use handle claiming, unclaimed-handle discard, validation-failure cleanup,
  coarse start/running/scan/read/write/test/commit/finish/fail events, cancel-request delivery
  through the Tauri command boundary, and cooperative core cancellation checks inside digest,
  scan, copy, extraction, testing, compression, and Direct Editing rewrite loops. The Workbench
  status bar shows determinate progress percentages when the core reports totals and indeterminate
  progress otherwise. Core progress now uses operation-specific extract/test phases and reports
  determinate totals for archive creation, extraction, testing, ZIP listing, single-stream payloads,
  Direct Editing ZIP rewrites, and aggregate parallel ZIP work where totals are available. The GUI
  blocks window close and app-exit requests during non-cancellable
  `Committing` operations, shows an app-style waiting message without granting frontend
  window-close permissions, and retries deferred app exit after active operations drain. Core
  mutating operations acquire interprocess Target Locks for archive outputs, extraction
  destinations, single-stream outputs, and Direct Editing saves so CLI, GUI, and shell-triggered
  runs cannot publish to the same target at the same time.
- GPLv3 license text included in `LICENSE`.

Still incomplete before a public v1 release:

- A green run of the GitHub-hosted CI matrix after this workspace is pushed.
- Manual external checks against Windows Explorer ZIP and macOS Archive Utility.
- GUI archive workflows beyond the current manifest/test/extract/drag-open/create-drag-input/selected
  operation/file-dialog/create/replace-prompt/context-menu/pending-delete-add-replace-save
  boundary: Windows shell Context Menu manual installer validation.
- Published release artifacts from an actual `v*` tag.

Deferred:

- Compression-side parallelism for tar/tar-compressed archives; those writers are still serialized.
- GUI release bundles are separate future artifacts; current CLI release artifacts remain unchanged.

## Verification

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
```

Local smoke checks have covered roundtrips for `.zip`, `.tar.gz`, `.tar.bz2`, `.tar.xz`, `.gz`,
`.bz2`, `.xz`, AES ZIP, ZipCrypto ZIP, 7-Zip ZIP interop where available, creation-side
rejection of `CON.txt`, and traversal ZIP rejection before extraction publishes output.
Unit/integration tests also cover malicious archive rejection for traversal,
unsafe path forms, cross-platform path collisions, non-directory prefix conflicts, unsafe symlink
targets, tar hardlinks, tar special entries, non-UTF-8 tar paths on Unix, and failed extraction not
publishing output or partial overwrite changes. They also cover corrupt ZIP payloads, truncated tar
payloads, truncated compressed tar archives, and truncated single-stream archives being rejected as
integrity failures. CLI integration tests cover explicit command names, JSON list output, and
invalid option combinations. They also cover Zip Bomb resource limits rejecting `list`, `test`, and
`extract` before output is published. Package dry runs
create a release archive, verify its required contents, verify that the packaged binary's
`arca --version` matches the Cargo package version, set Unix package binaries executable, and write
a SHA-256 checksum using the same scripts as the release workflow. Release archives include a
package README, `LICENSE`, and `THIRD_PARTY_LICENSES.md`. Compatibility fixture dry runs create
Arca-generated ZIP fixtures, an `EXPECTED.txt` manifest, and a byte-for-byte comparison helper for
Windows Explorer and macOS Archive Utility checks. Source archive dry runs create a matching
`arca-source.tar.gz` with `Cargo.lock`, validate its metadata, and compile workspace tests from the
extracted source bundle. Release asset verification checks the exact expected artifact set,
one-line SHA-256 manifests that reference their matching archives, readable archive payloads, ZIP
payload CRCs, archive entry path safety, expected top-level archive directories, required
package/source/fixture entries, Unix package binary executable bits, packaged GPL/readme/third-party
notice content, and the
compatibility fixture bundle's internal checksums before publication.
The source archive includes `apps/arca-gui`, root npm manifests, Tauri config, frontend source, the
GUI icon source, and generated desktop icon assets, then runs `npm ci`, GUI smoke, and GUI web build
checks from the extracted source bundle.
