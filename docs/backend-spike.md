# Backend Spike

Date: 2026-06-30

## Decision

Use pure Rust backends for the initial implementation:

- `zip = "=9.0.0-pre2"` with `aes-crypto`, `deflate-flate2-zlib-rs`, and `time`.
- `tar = 0.4.46`.
- `flate2 = 1.1.9`.
- `bzip2 = 0.6.1`.
- `xz2 = 0.1.7`.

`arca-native` remains as the boundary for future FFI/native backend replacement if compatibility testing shows the pure Rust path is not sufficient.

## Validated Locally

- ZIP create/list/test/extract.
- tar+gzip create/test/extract.
- tar+bzip2 create/extract.
- tar+xz create/extract.
- single-file gzip/bzip2/xz create/test/extract.
- ZIP AES-256 create/test/extract using `--password-stdin`.
- ZIP ZipCrypto create/test/extract using `--password-stdin --zipcrypto`.
- Creation-side safe path rejection for Windows reserved names.

## Not Yet Validated

- 7-Zip compatibility for AES-256 and ZipCrypto.
- Windows Explorer compatibility for ZipCrypto.
- macOS Archive Utility behavior.
- Linux/macOS/Windows CI packaging.
- Large-file bounded-memory behavior under load.

## Risk

`zip = "=9.0.0-pre2"` exposes ZipCrypto creation through an unstable extension trait, so the dependency is pinned to the exact pre-release. Before a public release, either keep the exact pin with compatibility fixtures or replace it with a stable backend/native library.
