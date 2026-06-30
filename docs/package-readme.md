# Arca

Arca is a GPLv3 cross-platform archive CLI.

## Quick Start

```bash
arca compress <inputs...> -o archive.zip
arca extract archive.zip -o output
arca list archive.zip
arca test archive.zip
```

On Windows, run `arca.exe` instead of `arca`.

## Which File To Download

Use the binary package that matches your platform:

- `arca-linux-x86_64.tar.gz`: Linux x86_64.
- `arca-macos-native.tar.gz`: macOS native GitHub runner build.
- `arca-windows-x86_64.zip`: Windows x86_64.

The other release archives are not app downloads:

- `arca-source.tar.gz`: matching GPL source bundle for the release.
- `arca-compat-fixtures.tar.gz`: compatibility fixtures for Windows Explorer, macOS Archive
  Utility, and 7-Zip checks.

## Supported Formats

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

## Passwords

ZIP encryption is supported.

```bash
arca compress secrets -o secrets.zip --password
printf 'secret\n' | arca compress secrets -o secrets.zip --password-stdin
```

AES-256 ZIP is the default. `--zipcrypto` enables traditional ZipCrypto for compatibility and prints
a warning. ZipCrypto is weak and should only be used when another tool requires it.
Non-ZIP formats are not encrypted in this release. Password flags on non-ZIP `compress`, `extract`,
and `test` commands are rejected before password input is read.

## Safety

Arca rejects archive paths that are unsafe or unpredictable across Linux, macOS, and Windows,
including traversal, absolute paths, Windows drive paths, reserved names, ADS/colon paths,
trailing-space or trailing-dot components, non-UTF-8 paths, path collisions, non-directory prefix
conflicts, unsafe symlinks, and tar hardlinks or special entries.

Container extraction writes into a staging directory first and publishes output only after
validation and extraction succeed.
Existing output trees are preflighted before overwrite publishing so failed extraction does not
partially replace files.
`list`, `test`, and `extract` reject unsafe paths and readable unsafe symlink targets before trusting
archive contents.

## Resource Limits

`list`, `test`, and `extract` enforce Zip Bomb guardrails: entry count, per-entry unpacked size,
total unpacked size, compression ratio, and symlink target size. Nested archive files are not
recursively extracted; if you extract a nested archive later, the same limits apply again.

The default limits can be adjusted with `ARCA_MAX_ENTRIES`, `ARCA_MAX_ENTRY_UNPACKED_BYTES`,
`ARCA_MAX_UNPACKED_BYTES`, `ARCA_MAX_COMPRESSION_RATIO`, and
`ARCA_MAX_SYMLINK_TARGET_BYTES`.

## Verify This Download

Each release archive has a matching `.sha256` file.

```bash
sha256sum -c arca-linux-x86_64.tar.gz.sha256
shasum -a 256 -c arca-macos-native.tar.gz.sha256
```

On Windows PowerShell:

```powershell
Get-FileHash .\arca-windows-x86_64.zip -Algorithm SHA256
```

Compare the printed hash with `arca-windows-x86_64.zip.sha256`.

## License

Arca is licensed under GPL-3.0-or-later. See `LICENSE`.
Third-party notices are included in `THIRD_PARTY_LICENSES.md`.
