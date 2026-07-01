use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::error::{ArcaError, ArcaResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionKind {
    Gzip,
    Bzip2,
    Xz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    Zip,
    Tar,
    TarCompressed(CompressionKind),
    SingleStream(CompressionKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveFormat {
    pub kind: FormatKind,
    pub suffix: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatSignature {
    pub offset: u64,
    pub bytes: &'static [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveFormatDescriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub kind: FormatKind,
    pub suffixes: &'static [&'static str],
    pub create_suffixes: &'static [&'static str],
    pub extensions: &'static [&'static str],
    pub mime_type: &'static str,
    pub signatures: &'static [FormatSignature],
}

impl ArchiveFormat {
    #[must_use]
    pub const fn is_container(self) -> bool {
        matches!(
            self.kind,
            FormatKind::Zip | FormatKind::Tar | FormatKind::TarCompressed(_)
        )
    }

    #[must_use]
    pub const fn is_single_stream(self) -> bool {
        matches!(self.kind, FormatKind::SingleStream(_))
    }
}

impl ArchiveFormatDescriptor {
    #[must_use]
    pub const fn supports_create(self) -> bool {
        !self.create_suffixes.is_empty()
    }

    #[must_use]
    pub const fn supports_direct_edit(self) -> bool {
        matches!(self.kind, FormatKind::Zip)
    }
}

const ZIP_SIGNATURES: &[FormatSignature] = &[
    FormatSignature {
        offset: 0,
        bytes: b"PK\x03\x04",
    },
    FormatSignature {
        offset: 0,
        bytes: b"PK\x05\x06",
    },
    FormatSignature {
        offset: 0,
        bytes: b"PK\x07\x08",
    },
];
const TAR_SIGNATURES: &[FormatSignature] = &[FormatSignature {
    offset: 257,
    bytes: b"ustar",
}];
const GZIP_SIGNATURES: &[FormatSignature] = &[FormatSignature {
    offset: 0,
    bytes: b"\x1f\x8b",
}];
const BZIP2_SIGNATURES: &[FormatSignature] = &[FormatSignature {
    offset: 0,
    bytes: b"BZh",
}];
const XZ_SIGNATURES: &[FormatSignature] = &[FormatSignature {
    offset: 0,
    bytes: b"\xfd7zXZ\x00",
}];

const ARCHIVE_FORMATS: &[ArchiveFormatDescriptor] = &[
    ArchiveFormatDescriptor {
        id: "zip",
        name: "ZIP Archive",
        description: "ZIP archive file",
        kind: FormatKind::Zip,
        suffixes: &[".zip"],
        create_suffixes: &[".zip"],
        extensions: &["zip"],
        mime_type: "application/zip",
        signatures: ZIP_SIGNATURES,
    },
    ArchiveFormatDescriptor {
        id: "tar",
        name: "Tar Archive",
        description: "Uncompressed tar archive",
        kind: FormatKind::Tar,
        suffixes: &[".tar"],
        create_suffixes: &[".tar"],
        extensions: &["tar"],
        mime_type: "application/x-tar",
        signatures: TAR_SIGNATURES,
    },
    ArchiveFormatDescriptor {
        id: "tar.gz",
        name: "Tar Gzip Archive",
        description: "Tar archive compressed with gzip",
        kind: FormatKind::TarCompressed(CompressionKind::Gzip),
        suffixes: &[".tar.gz", ".tgz"],
        create_suffixes: &[".tar.gz", ".tgz"],
        extensions: &["tgz"],
        mime_type: "application/x-compressed-tar",
        signatures: GZIP_SIGNATURES,
    },
    ArchiveFormatDescriptor {
        id: "tar.bz2",
        name: "Tar Bzip2 Archive",
        description: "Tar archive compressed with bzip2",
        kind: FormatKind::TarCompressed(CompressionKind::Bzip2),
        suffixes: &[".tar.bz2", ".tbz2"],
        create_suffixes: &[".tar.bz2", ".tbz2"],
        extensions: &["tbz2"],
        mime_type: "application/x-bzip-compressed-tar",
        signatures: BZIP2_SIGNATURES,
    },
    ArchiveFormatDescriptor {
        id: "tar.xz",
        name: "Tar XZ Archive",
        description: "Tar archive compressed with xz",
        kind: FormatKind::TarCompressed(CompressionKind::Xz),
        suffixes: &[".tar.xz", ".txz"],
        create_suffixes: &[".tar.xz", ".txz"],
        extensions: &["txz"],
        mime_type: "application/x-xz-compressed-tar",
        signatures: XZ_SIGNATURES,
    },
    ArchiveFormatDescriptor {
        id: "gz",
        name: "Gzip Stream",
        description: "Single-file gzip stream",
        kind: FormatKind::SingleStream(CompressionKind::Gzip),
        suffixes: &[".gz"],
        create_suffixes: &[],
        extensions: &["gz"],
        mime_type: "application/gzip",
        signatures: GZIP_SIGNATURES,
    },
    ArchiveFormatDescriptor {
        id: "bz2",
        name: "Bzip2 Stream",
        description: "Single-file bzip2 stream",
        kind: FormatKind::SingleStream(CompressionKind::Bzip2),
        suffixes: &[".bz2"],
        create_suffixes: &[],
        extensions: &["bz2"],
        mime_type: "application/x-bzip2",
        signatures: BZIP2_SIGNATURES,
    },
    ArchiveFormatDescriptor {
        id: "xz",
        name: "XZ Stream",
        description: "Single-file xz stream",
        kind: FormatKind::SingleStream(CompressionKind::Xz),
        suffixes: &[".xz"],
        create_suffixes: &[],
        extensions: &["xz"],
        mime_type: "application/x-xz",
        signatures: XZ_SIGNATURES,
    },
];

#[must_use]
pub const fn archive_formats() -> &'static [ArchiveFormatDescriptor] {
    ARCHIVE_FORMATS
}

pub fn archive_file_extensions() -> impl Iterator<Item = &'static str> {
    archive_formats()
        .iter()
        .flat_map(|descriptor| descriptor.extensions.iter().copied())
}

pub fn detect_format(path: &Path) -> ArcaResult<Option<ArchiveFormat>> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ArcaError::NonUtf8Path(path.to_path_buf()))?;
    let lower = name.to_lowercase();
    let mut matched = None;
    for descriptor in archive_formats() {
        for &suffix in descriptor.suffixes {
            if lower.ends_with(suffix)
                && matched
                    .map(|format: ArchiveFormat| suffix.len() > format.suffix.len())
                    .unwrap_or(true)
            {
                matched = Some(ArchiveFormat {
                    kind: descriptor.kind,
                    suffix,
                });
            }
        }
    }
    Ok(matched)
}

pub fn required_format(path: &Path) -> ArcaResult<ArchiveFormat> {
    detect_format(path)?.ok_or_else(|| {
        ArcaError::Unsupported(format!("unsupported archive suffix for {}", path.display()))
    })
}

pub fn descriptor_for_extension(extension: &str) -> Option<&'static ArchiveFormatDescriptor> {
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    archive_formats().iter().find(|descriptor| {
        descriptor
            .extensions
            .iter()
            .any(|candidate| *candidate == extension)
    })
}

#[must_use]
pub fn descriptor_for_format(format: ArchiveFormat) -> Option<&'static ArchiveFormatDescriptor> {
    archive_formats().iter().find(|descriptor| {
        descriptor.kind == format.kind && descriptor.suffixes.contains(&format.suffix)
    })
}

pub fn format_matches_signature(path: &Path, format: ArchiveFormat) -> ArcaResult<bool> {
    let Some(descriptor) = descriptor_for_format(format) else {
        return Ok(false);
    };
    descriptor_matches_signature(path, descriptor)
}

pub fn descriptor_matches_signature(
    path: &Path,
    descriptor: &ArchiveFormatDescriptor,
) -> ArcaResult<bool> {
    let mut file = File::open(path).map_err(|source| ArcaError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    for signature in descriptor.signatures {
        if file.seek(SeekFrom::Start(signature.offset)).is_err() {
            continue;
        }
        let mut actual = vec![0_u8; signature.bytes.len()];
        if file.read_exact(&mut actual).is_ok() && actual == signature.bytes {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn detect_format_with_signature(path: &Path) -> ArcaResult<Option<ArchiveFormat>> {
    let Some(format) = detect_format(path)? else {
        return Ok(None);
    };
    Ok(format_matches_signature(path, format)?.then_some(format))
}

pub fn default_compress_output(inputs: &[PathBuf]) -> ArcaResult<PathBuf> {
    if inputs.is_empty() {
        return Err(ArcaError::Usage(
            "compress requires at least one input".into(),
        ));
    }
    let base = if inputs.len() == 1 {
        inputs[0]
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ArcaError::NonUtf8Path(inputs[0].clone()))?
            .to_owned()
    } else {
        "archive".to_owned()
    };
    Ok(PathBuf::from(format!("{base}.zip")))
}

pub fn normalize_output_path(output: Option<PathBuf>, inputs: &[PathBuf]) -> ArcaResult<PathBuf> {
    let mut path = match output {
        Some(path) => path,
        None => default_compress_output(inputs)?,
    };
    if detect_format(&path)?.is_none() {
        if path.extension().is_some() {
            return Err(ArcaError::Unsupported(format!(
                "unsupported archive suffix for {}",
                path.display()
            )));
        }
        path.set_extension("zip");
    }
    Ok(path)
}

pub fn strip_archive_suffix(path: &Path, format: ArchiveFormat) -> ArcaResult<PathBuf> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ArcaError::NonUtf8Path(path.to_path_buf()))?;
    let cut = name.len() - format.suffix.len();
    let stem = &name[..cut];
    let stem = if stem.is_empty() { "archive" } else { stem };
    Ok(path.with_file_name(stem))
}

pub fn default_extract_destination(path: &Path, format: ArchiveFormat) -> ArcaResult<PathBuf> {
    strip_archive_suffix(path, format)
}

pub fn single_stream_output(archive: &Path, output: Option<&Path>) -> ArcaResult<PathBuf> {
    let format = required_format(archive)?;
    if !format.is_single_stream() {
        return Err(ArcaError::Usage(
            "single stream output requested for container format".into(),
        ));
    }
    let default_output = strip_archive_suffix(archive, format)?;
    let derived = default_output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ArcaError::NonUtf8Path(archive.to_path_buf()))?
        .to_owned();
    match output {
        Some(path) if path.is_dir() => Ok(path.join(derived)),
        Some(path) => Ok(path.to_path_buf()),
        None => Ok(default_output),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_suffix_wins() {
        let fmt = required_format(Path::new("backup.tar.gz")).unwrap();
        assert_eq!(fmt.kind, FormatKind::TarCompressed(CompressionKind::Gzip));
    }

    #[test]
    fn registered_formats_expose_file_extensions_and_signatures() {
        let zip = descriptor_for_extension("zip").unwrap();

        assert_eq!(zip.id, "zip");
        assert_eq!(zip.kind, FormatKind::Zip);
        assert!(zip.supports_create());
        assert!(zip.supports_direct_edit());
        assert!(
            zip.signatures
                .iter()
                .any(|signature| signature.bytes == b"PK\x03\x04")
        );
    }

    #[test]
    fn archive_extensions_are_derived_from_registered_formats() {
        let extensions = archive_file_extensions().collect::<Vec<_>>();

        assert_eq!(
            extensions,
            vec!["zip", "tar", "tgz", "tbz2", "txz", "gz", "bz2", "xz"]
        );
    }

    #[test]
    fn suffixless_output_defaults_to_zip() {
        let out =
            normalize_output_path(Some(PathBuf::from("backup")), &[PathBuf::from("docs")]).unwrap();
        assert_eq!(out, PathBuf::from("backup.zip"));
    }

    #[test]
    fn unknown_suffix_fails() {
        let err =
            normalize_output_path(Some(PathBuf::from("backup.rar")), &[PathBuf::from("docs")])
                .unwrap_err();
        assert!(matches!(err, ArcaError::Unsupported(_)));
    }
}
