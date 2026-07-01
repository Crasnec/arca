# Windows shell Context Menu uses startup requests

Arca's first operating-system shell Context Menu integration is Windows-first. The NSIS installer
registers Explorer menu entries for supported archive suffixes only, and each entry launches the GUI
with an explicit startup flag: `--arca-shell-open`, `--arca-shell-extract`, or
`--arca-shell-test`.

`npm run gui:smoke` statically verifies the NSIS hook's install/uninstall extension symmetry and
the exact Explorer labels, icon registration, quoted command template, and startup flags. Manual
Windows validation is still required for Explorer rendering and installed-app launch behavior.

The GUI treats those arguments as Startup Requests, not as CLI commands. Rust filters them to
existing files whose suffix maps to a supported archive format before the frontend receives them.
The frontend then routes open, extract, and test through the same Tauri commands that power toolbar
actions, preserving password prompts, replace prompts, archive validation, and the rule that the GUI
must not shell out to the CLI.

Later file-association or shell Context Menu launches are routed into the existing GUI process with
`tauri-plugin-single-instance`. The secondary process argv is parsed through the same Rust Startup
Request filter, then emitted to the frontend as `arca-startup-requests`; the frontend preserves the
requested open/test/extract action and still prompts before discarding unsaved Direct Editing
changes. macOS/Linux shell menu guarantees remain deferred until packaging behavior is validated.
Interprocess Target Locks live in `arca-core` so file-association and shell-triggered operations
cannot race another GUI or CLI process during publish even if single-instance routing is unavailable
on a platform.
