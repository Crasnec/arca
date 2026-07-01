# GUI operation registry and cancel requests

Arca's GUI routes archive work through a Rust-owned Operation Registry instead of treating React's
`loading` flag as the operation model. The frontend asks Rust for an operation handle, passes that
handle into the archive command, listens for coarse operation events, and sends Cancel Requests back
to Rust with that handle.

This keeps operation identity on the trusted side of the Tauri boundary and gives core
`ProgressSink`/`CancellationToken` work a stable integration point.

Current scope:

- Operation handles are created by the `begin_operation` command.
- Archive commands accept an optional `operation_id`, claim that handle exactly once, reject reused
  or finished handles, and emit start/running/scan/read/write/test/commit/finish/fail state through
  `arca-operation-progress`.
- If the frontend fails before an archive command claims a handle, its `finally` block calls
  `discard_operation` to remove only unclaimed handles. Discard is a no-op for claimed, running, or
  already-finished operations.
- Archive command validation failures after a handle is claimed still finish the tracked operation,
  so malformed Tauri invocations cannot leave stale active operations behind.
- `cancel_operation` records a Cancel Request and emits a cancel-requested state.
- A request received before the core worker starts is rejected as interrupted.
- `arca-core` digest, scan, copy, compression, extraction, testing, and Direct Editing rewrite loops
  observe `CancellationToken` cooperatively.
- The core emits `Committing` before publish/persist work; cancel requests during that phase are
  rejected by the GUI command layer.
- Window close requests during `Committing` are prevented by a Rust-side Tauri window event handler.
  The frontend receives `arca-close-blocked` and shows a waiting message, without needing
  `core:window:*` permissions.
- App-exit requests during `Committing` are prevented by a Rust-side Tauri run-event handler. The
  requested exit code is kept in the Operation Registry and retried after active operations drain.
- The frontend status bar shows operation progress, with a percentage when `processed` and `total`
  are known and an indeterminate state otherwise.
- Core progress uses operation-specific `Extracting`/`Testing` phases instead of reporting archive
  payload reads as writes, and reports determinate totals for archive creation, extraction, testing,
  ZIP listing, single-stream payloads, Direct Editing ZIP rewrites, and aggregate parallel ZIP work
  where totals are available.
- Core cancellation cleanup tests cover container archive creation, single-stream archive creation,
  container extraction, single-stream extraction, and Direct Editing save staging paths.
