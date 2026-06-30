use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use globset::{Glob, GlobSetBuilder};
use serde::Serialize;
use walkdir::WalkDir;

use crate::error::{ArcaError, ArcaResult, io_at};
use crate::policy::{CollisionSet, validate_archive_path, validate_symlink_target};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchiveEntry {
    pub source: PathBuf,
    pub archive_path: String,
    pub kind: EntryKind,
    pub symlink_target: Option<String>,
    pub size: u64,
    pub snapshot: SourceSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UnixTime {
    pub seconds: i64,
    pub nanos: u32,
}

impl UnixTime {
    #[must_use]
    pub fn from_system_time(time: SystemTime) -> Self {
        match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => Self {
                seconds: duration.as_secs().min(i64::MAX as u64) as i64,
                nanos: duration.subsec_nanos(),
            },
            Err(error) => {
                let duration = error.duration();
                if duration.subsec_nanos() == 0 {
                    Self {
                        seconds: -(duration.as_secs().min(i64::MAX as u64) as i64),
                        nanos: 0,
                    }
                } else {
                    Self {
                        seconds: -((duration.as_secs().min(i64::MAX as u64) as i64) + 1),
                        nanos: 1_000_000_000 - duration.subsec_nanos(),
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn to_system_time(self) -> SystemTime {
        if self.seconds >= 0 {
            UNIX_EPOCH + Duration::new(self.seconds as u64, self.nanos)
        } else if self.nanos == 0 {
            UNIX_EPOCH - Duration::new(self.seconds.unsigned_abs(), 0)
        } else {
            UNIX_EPOCH - Duration::new(self.seconds.unsigned_abs() - 1, 1_000_000_000 - self.nanos)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceSnapshot {
    pub kind: EntryKind,
    pub len: u64,
    pub modified: Option<UnixTime>,
    pub mode: Option<u32>,
    #[cfg(unix)]
    pub dev: u64,
    #[cfg(unix)]
    pub ino: u64,
    #[cfg(unix)]
    pub changed: UnixTime,
}

impl SourceSnapshot {
    #[must_use]
    pub fn from_metadata(meta: &Metadata, kind: EntryKind) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        Self {
            kind,
            len: meta.len(),
            modified: meta.modified().ok().map(UnixTime::from_system_time),
            mode: platform_mode(meta),
            #[cfg(unix)]
            dev: meta.dev(),
            #[cfg(unix)]
            ino: meta.ino(),
            #[cfg(unix)]
            changed: UnixTime {
                seconds: meta.ctime(),
                nanos: meta.ctime_nsec().max(0) as u32,
            },
        }
    }
}

pub fn plan_entries(
    inputs: &[PathBuf],
    excludes: &[String],
    output: &Path,
) -> ArcaResult<Vec<ArchiveEntry>> {
    if inputs.is_empty() {
        return Err(ArcaError::Usage(
            "compress requires at least one input".into(),
        ));
    }
    let output_abs = absolute_output_path(output)?;
    reject_output_inside_inputs(inputs, &output_abs)?;

    let globset = build_globset(excludes)?;
    let single_dir = inputs.len() == 1 && inputs[0].is_dir();
    let mut entries = Vec::new();
    let mut collisions = CollisionSet::default();

    for input in inputs {
        let meta = std::fs::symlink_metadata(input).map_err(|err| io_at(input, err))?;
        if meta.is_file() || meta.file_type().is_symlink() {
            let basename = utf8_file_name(input)?;
            push_entry(input, basename, &globset, &mut collisions, &mut entries)?;
            continue;
        }
        if !meta.is_dir() {
            return Err(ArcaError::Security(format!(
                "unsupported input file type: {}",
                input.display()
            )));
        }

        let root = input;
        for item in WalkDir::new(root)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter()
        {
            let item = item.map_err(|err| ArcaError::Other(err.to_string()))?;
            let source = item.path();
            if single_dir && source == root {
                continue;
            }
            let rel = if single_dir {
                source
                    .strip_prefix(root)
                    .map_err(|err| ArcaError::Other(err.to_string()))?
            } else {
                let parent = root.parent().unwrap_or_else(|| Path::new(""));
                source
                    .strip_prefix(parent)
                    .map_err(|err| ArcaError::Other(err.to_string()))?
            };
            let archive_path = path_to_archive_string(rel)?;
            push_entry(
                source,
                archive_path,
                &globset,
                &mut collisions,
                &mut entries,
            )?;
        }
    }

    Ok(entries)
}

fn push_entry(
    source: &Path,
    archive_path: String,
    excludes: &globset::GlobSet,
    collisions: &mut CollisionSet,
    entries: &mut Vec<ArchiveEntry>,
) -> ArcaResult<()> {
    if excludes.is_match(&archive_path) {
        return Ok(());
    }
    validate_archive_path(&archive_path)?;
    collisions.insert_archive_path(&archive_path)?;

    let meta = std::fs::symlink_metadata(source).map_err(|err| io_at(source, err))?;
    let file_type = meta.file_type();
    #[cfg(unix)]
    reject_unix_special(source, &file_type)?;

    let (kind, symlink_target, size) = if file_type.is_symlink() {
        let target = std::fs::read_link(source).map_err(|err| io_at(source, err))?;
        let target = path_to_archive_string(&target)?;
        validate_symlink_target(&target)?;
        (EntryKind::Symlink, Some(target), 0)
    } else if file_type.is_dir() {
        (EntryKind::Directory, None, 0)
    } else if file_type.is_file() {
        (EntryKind::File, None, meta.len())
    } else {
        return Err(ArcaError::Security(format!(
            "unsupported input file type: {}",
            source.display()
        )));
    };
    let snapshot = SourceSnapshot::from_metadata(&meta, kind);

    entries.push(ArchiveEntry {
        source: source.to_path_buf(),
        archive_path,
        kind,
        symlink_target,
        size,
        snapshot,
    });
    Ok(())
}

pub fn ensure_entries_unchanged(
    expected: &[ArchiveEntry],
    inputs: &[PathBuf],
    excludes: &[String],
    output: &Path,
) -> ArcaResult<()> {
    let current = plan_entries(inputs, excludes, output)?;
    if current != expected {
        return Err(ArcaError::Integrity(
            "input tree changed during compression".into(),
        ));
    }
    Ok(())
}

pub fn capture_path_snapshot(path: &Path) -> ArcaResult<SourceSnapshot> {
    let meta = std::fs::symlink_metadata(path).map_err(|err| io_at(path, err))?;
    let kind = metadata_kind(path, &meta)?;
    Ok(SourceSnapshot::from_metadata(&meta, kind))
}

pub fn validate_path_snapshot(entry: &ArchiveEntry) -> ArcaResult<()> {
    let snapshot = capture_path_snapshot(&entry.source)?;
    if snapshot != entry.snapshot {
        return Err(ArcaError::Integrity(format!(
            "input changed during compression: {}",
            entry.source.display()
        )));
    }
    Ok(())
}

pub fn validate_open_file_snapshot(entry: &ArchiveEntry, file: &File) -> ArcaResult<()> {
    let meta = file.metadata().map_err(|err| io_at(&entry.source, err))?;
    let snapshot = SourceSnapshot::from_metadata(&meta, EntryKind::File);
    if snapshot != entry.snapshot {
        return Err(ArcaError::Integrity(format!(
            "input file changed during compression: {}",
            entry.source.display()
        )));
    }
    Ok(())
}

fn metadata_kind(source: &Path, meta: &Metadata) -> ArcaResult<EntryKind> {
    let file_type = meta.file_type();
    #[cfg(unix)]
    reject_unix_special(source, &file_type)?;
    if file_type.is_symlink() {
        Ok(EntryKind::Symlink)
    } else if file_type.is_dir() {
        Ok(EntryKind::Directory)
    } else if file_type.is_file() {
        Ok(EntryKind::File)
    } else {
        Err(ArcaError::Security(format!(
            "unsupported input file type: {}",
            source.display()
        )))
    }
}

#[cfg(unix)]
fn platform_mode(meta: &Metadata) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    Some(meta.mode() & 0o777)
}

#[cfg(not(unix))]
fn platform_mode(_meta: &Metadata) -> Option<u32> {
    None
}

#[cfg(unix)]
fn reject_unix_special(source: &Path, file_type: &std::fs::FileType) -> ArcaResult<()> {
    use std::os::unix::fs::FileTypeExt;
    if file_type.is_block_device()
        || file_type.is_char_device()
        || file_type.is_fifo()
        || file_type.is_socket()
    {
        return Err(ArcaError::Security(format!(
            "special file entries are not supported: {}",
            source.display()
        )));
    }
    Ok(())
}

fn build_globset(excludes: &[String]) -> ArcaResult<globset::GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in excludes {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}

fn path_to_archive_string(path: &Path) -> ArcaResult<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => {
                let part = part
                    .to_str()
                    .ok_or_else(|| ArcaError::NonUtf8Path(path.to_path_buf()))?;
                parts.push(part.to_owned());
            }
            std::path::Component::CurDir => {}
            _ => {
                return Err(ArcaError::Security(format!(
                    "unsupported path component in {}",
                    path.display()
                )));
            }
        }
    }
    if parts.is_empty() {
        return Err(ArcaError::Security("empty archive path".into()));
    }
    Ok(parts.join("/"))
}

fn utf8_file_name(path: &Path) -> ArcaResult<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| ArcaError::NonUtf8Path(path.to_path_buf()))
}

fn absolute_output_path(path: &Path) -> ArcaResult<PathBuf> {
    let cwd = std::env::current_dir()?;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let parent = absolute.parent().unwrap_or_else(|| Path::new("."));
    let parent = parent.canonicalize().map_err(|err| io_at(parent, err))?;
    let name = absolute.file_name().ok_or_else(|| {
        ArcaError::Usage(format!("output path has no file name: {}", path.display()))
    })?;
    Ok(parent.join(name))
}

fn reject_output_inside_inputs(inputs: &[PathBuf], output_abs: &Path) -> ArcaResult<()> {
    for input in inputs {
        let meta = std::fs::symlink_metadata(input).map_err(|err| io_at(input, err))?;
        let input_abs = input.canonicalize().map_err(|err| io_at(input, err))?;
        if meta.is_dir() {
            if output_abs.starts_with(&input_abs) {
                return Err(ArcaError::Usage(format!(
                    "output archive cannot be inside input tree: {}",
                    output_abs.display()
                )));
            }
        } else if output_abs == input_abs {
            return Err(ArcaError::Usage(format!(
                "output archive cannot equal input file: {}",
                output_abs.display()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_string_uses_forward_slashes() {
        assert_eq!(
            path_to_archive_string(Path::new("a").join("b.txt").as_path()).unwrap(),
            "a/b.txt"
        );
    }
}
