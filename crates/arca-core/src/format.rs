use std::path::{Path, PathBuf};

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

const FORMATS: &[ArchiveFormat] = &[
    ArchiveFormat {
        kind: FormatKind::TarCompressed(CompressionKind::Gzip),
        suffix: ".tar.gz",
    },
    ArchiveFormat {
        kind: FormatKind::TarCompressed(CompressionKind::Bzip2),
        suffix: ".tar.bz2",
    },
    ArchiveFormat {
        kind: FormatKind::TarCompressed(CompressionKind::Xz),
        suffix: ".tar.xz",
    },
    ArchiveFormat {
        kind: FormatKind::TarCompressed(CompressionKind::Gzip),
        suffix: ".tgz",
    },
    ArchiveFormat {
        kind: FormatKind::TarCompressed(CompressionKind::Bzip2),
        suffix: ".tbz2",
    },
    ArchiveFormat {
        kind: FormatKind::TarCompressed(CompressionKind::Xz),
        suffix: ".txz",
    },
    ArchiveFormat {
        kind: FormatKind::Zip,
        suffix: ".zip",
    },
    ArchiveFormat {
        kind: FormatKind::Tar,
        suffix: ".tar",
    },
    ArchiveFormat {
        kind: FormatKind::SingleStream(CompressionKind::Gzip),
        suffix: ".gz",
    },
    ArchiveFormat {
        kind: FormatKind::SingleStream(CompressionKind::Bzip2),
        suffix: ".bz2",
    },
    ArchiveFormat {
        kind: FormatKind::SingleStream(CompressionKind::Xz),
        suffix: ".xz",
    },
];

pub fn detect_format(path: &Path) -> ArcaResult<Option<ArchiveFormat>> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ArcaError::NonUtf8Path(path.to_path_buf()))?;
    let lower = name.to_lowercase();
    Ok(FORMATS
        .iter()
        .copied()
        .find(|format| lower.ends_with(format.suffix)))
}

pub fn required_format(path: &Path) -> ArcaResult<ArchiveFormat> {
    detect_format(path)?.ok_or_else(|| {
        ArcaError::Unsupported(format!("unsupported archive suffix for {}", path.display()))
    })
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
