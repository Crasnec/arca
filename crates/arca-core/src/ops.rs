use std::cmp::min;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use bzip2::Compression as BzCompression;
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use filetime::FileTime;
use flate2::Compression as GzCompression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tar::{Archive, Builder, EntryType, Header};
use tempfile::Builder as TempBuilder;
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;
use zeroize::Zeroize;
use zip::unstable::write::FileOptionsExt;
use zip::write::FileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

use crate::error::{ArcaError, ArcaResult, io_at};
use crate::format::{
    ArchiveFormat, CompressionKind, FormatKind, default_extract_destination, normalize_output_path,
    required_format, single_stream_output, strip_archive_suffix,
};
use crate::plan::{
    ArchiveEntry, EntryKind, SourceSnapshot, UnixTime, capture_path_snapshot,
    ensure_entries_unchanged, plan_entries, validate_open_file_snapshot, validate_path_snapshot,
};
use crate::policy::{CollisionSet, collision_key, validate_archive_path, validate_symlink_target};

#[derive(Clone)]
pub struct Password {
    bytes: Vec<u8>,
}

impl Password {
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Password(<redacted>)")
    }
}

impl Drop for Password {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

#[derive(Debug, Clone, Default)]
pub enum Encryption {
    #[default]
    None,
    Aes256(Password),
    ZipCrypto(Password),
}

#[derive(Debug, Clone)]
pub struct CompressOptions {
    pub inputs: Vec<PathBuf>,
    pub output: Option<PathBuf>,
    pub overwrite: bool,
    pub level: Option<u8>,
    pub jobs: usize,
    pub excludes: Vec<String>,
    pub encryption: Encryption,
    pub auto_tar: bool,
}

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub archive: PathBuf,
    pub output: Option<PathBuf>,
    pub overwrite: bool,
    pub jobs: usize,
    pub password: Option<Password>,
}

#[derive(Debug, Clone)]
pub struct TestOptions {
    pub archive: PathBuf,
    pub jobs: usize,
    pub password: Option<Password>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListEntry {
    pub path: String,
    pub entry_type: String,
    pub uncompressed_size: u64,
    pub compressed_size: Option<u64>,
    pub encrypted: bool,
}

const DEFAULT_MAX_ARCHIVE_ENTRIES: usize = 200_000;
const DEFAULT_MAX_ENTRY_UNPACKED_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const DEFAULT_MAX_TOTAL_UNPACKED_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const DEFAULT_MAX_COMPRESSION_RATIO: u64 = 10_000;
const DEFAULT_MAX_SYMLINK_TARGET_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, Copy)]
struct ResourceLimits {
    max_entries: usize,
    max_entry_unpacked_bytes: u64,
    max_total_unpacked_bytes: u64,
    max_compression_ratio: u64,
    max_symlink_target_bytes: u64,
}

impl ResourceLimits {
    fn from_env() -> ArcaResult<Self> {
        Ok(Self {
            max_entries: env_limit_usize("ARCA_MAX_ENTRIES", DEFAULT_MAX_ARCHIVE_ENTRIES)?,
            max_entry_unpacked_bytes: env_limit_u64(
                "ARCA_MAX_ENTRY_UNPACKED_BYTES",
                DEFAULT_MAX_ENTRY_UNPACKED_BYTES,
            )?,
            max_total_unpacked_bytes: env_limit_u64(
                "ARCA_MAX_UNPACKED_BYTES",
                DEFAULT_MAX_TOTAL_UNPACKED_BYTES,
            )?,
            max_compression_ratio: env_limit_u64(
                "ARCA_MAX_COMPRESSION_RATIO",
                DEFAULT_MAX_COMPRESSION_RATIO,
            )?,
            max_symlink_target_bytes: env_limit_u64(
                "ARCA_MAX_SYMLINK_TARGET_BYTES",
                DEFAULT_MAX_SYMLINK_TARGET_BYTES,
            )?,
        })
    }

    fn check_entry_count(self, entries: usize) -> ArcaResult<()> {
        if entries > self.max_entries {
            return Err(ArcaError::Security(format!(
                "archive entry count limit exceeded: {entries} > {}",
                self.max_entries
            )));
        }
        Ok(())
    }

    fn check_entry_size(self, path: &str, size: u64) -> ArcaResult<()> {
        if size > self.max_entry_unpacked_bytes {
            return Err(ArcaError::Security(format!(
                "archive entry size limit exceeded for {path}: {size} > {}",
                self.max_entry_unpacked_bytes
            )));
        }
        Ok(())
    }

    fn add_total(self, current: u64, path: &str, size: u64) -> ArcaResult<u64> {
        let total = current.checked_add(size).ok_or_else(|| {
            ArcaError::Security(format!("archive unpacked size overflow at {path}"))
        })?;
        if total > self.max_total_unpacked_bytes {
            return Err(ArcaError::Security(format!(
                "archive unpacked size limit exceeded at {path}: {total} > {}",
                self.max_total_unpacked_bytes
            )));
        }
        Ok(total)
    }

    fn check_compression_ratio(
        self,
        path: &str,
        uncompressed: u64,
        compressed: u64,
    ) -> ArcaResult<()> {
        if uncompressed == 0 {
            return Ok(());
        }
        if compressed == 0 || uncompressed > compressed.saturating_mul(self.max_compression_ratio) {
            return Err(ArcaError::Security(format!(
                "archive compression ratio limit exceeded for {path}: {uncompressed}/{compressed} > {}",
                self.max_compression_ratio
            )));
        }
        Ok(())
    }

    fn check_symlink_target_size(self, path: &str, size: u64) -> ArcaResult<()> {
        if size > self.max_symlink_target_bytes {
            return Err(ArcaError::Security(format!(
                "symlink target size limit exceeded for {path}: {size} > {}",
                self.max_symlink_target_bytes
            )));
        }
        Ok(())
    }
}

fn env_limit_u64(name: &str, default: u64) -> ArcaResult<u64> {
    match std::env::var(name) {
        Ok(value) => parse_positive_u64(name, &value),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(ArcaError::Usage(format!("{name} must be valid UTF-8")))
        }
    }
}

fn env_limit_usize(name: &str, default: usize) -> ArcaResult<usize> {
    match std::env::var(name) {
        Ok(value) => {
            let parsed = parse_positive_u64(name, &value)?;
            usize::try_from(parsed).map_err(|_| {
                ArcaError::Usage(format!("{name} is too large for this platform: {parsed}"))
            })
        }
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(ArcaError::Usage(format!("{name} must be valid UTF-8")))
        }
    }
}

fn parse_positive_u64(name: &str, value: &str) -> ArcaResult<u64> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| ArcaError::Usage(format!("{name} must be a positive integer")))?;
    if parsed == 0 {
        return Err(ArcaError::Usage(format!(
            "{name} must be a positive integer"
        )));
    }
    Ok(parsed)
}

pub fn compress(options: CompressOptions) -> ArcaResult<PathBuf> {
    validate_jobs(options.jobs)?;
    let mut output = normalize_output_path(options.output, &options.inputs)?;
    let mut format = required_format(&output)?;

    if options.jobs > 1 && !matches!(format.kind, FormatKind::Zip) {
        return Err(ArcaError::Unsupported(
            "compression --jobs > 1 is only supported for .zip".into(),
        ));
    }

    if format.is_single_stream() && options.inputs.iter().any(|input| input.is_dir()) {
        if !options.auto_tar {
            return Err(ArcaError::Unsupported(format!(
                "{} is a single-file format; use a tar-based suffix or --auto-tar",
                format.suffix
            )));
        }
        output = auto_tar_output(&output, format)?;
        format = required_format(&output)?;
    }

    if matches!(format.kind, FormatKind::Tar) && options.level.is_some() {
        return Err(ArcaError::Unsupported(
            "--level is not supported for .tar".into(),
        ));
    }
    if !matches!(format.kind, FormatKind::Zip) && !matches!(options.encryption, Encryption::None) {
        return Err(ArcaError::Unsupported(
            "password options are only supported for .zip".into(),
        ));
    }
    if options.jobs > 1 && matches!(options.encryption, Encryption::ZipCrypto(_)) {
        return Err(ArcaError::Unsupported(
            "compression --jobs > 1 is not supported with ZipCrypto".into(),
        ));
    }
    if output.exists() && !options.overwrite {
        return Err(ArcaError::Usage(format!(
            "output already exists: {}",
            output.display()
        )));
    }

    if format.is_single_stream() {
        compress_single_stream(
            &options.inputs,
            &output,
            format,
            options.level,
            options.overwrite,
        )?;
        return Ok(output);
    }

    let entries = plan_entries(&options.inputs, &options.excludes, &output)?;
    write_archive_atomic(&output, options.overwrite, |file| match format.kind {
        FormatKind::Zip => {
            write_zip(
                file,
                &entries,
                options.level,
                &options.encryption,
                options.jobs,
            )?;
            ensure_entries_unchanged(&entries, &options.inputs, &options.excludes, &output)
        }
        FormatKind::Tar => {
            write_tar(file, &entries)?;
            ensure_entries_unchanged(&entries, &options.inputs, &options.excludes, &output)
        }
        FormatKind::TarCompressed(codec) => {
            match codec {
                CompressionKind::Gzip => {
                    let encoder = GzEncoder::new(file, gz_level(options.level));
                    write_tar(encoder, &entries)?;
                }
                CompressionKind::Bzip2 => {
                    let encoder = BzEncoder::new(file, bz_level(options.level));
                    write_tar(encoder, &entries)?;
                }
                CompressionKind::Xz => {
                    let encoder = XzEncoder::new(file, xz_level(options.level));
                    write_tar(encoder, &entries)?;
                }
            };
            ensure_entries_unchanged(&entries, &options.inputs, &options.excludes, &output)
        }
        FormatKind::SingleStream(_) => unreachable!("single stream handled earlier"),
    })?;
    Ok(output)
}

pub fn extract(options: ExtractOptions) -> ArcaResult<PathBuf> {
    validate_jobs(options.jobs)?;
    let format = required_format(&options.archive)?;
    let limits = ResourceLimits::from_env()?;
    if format.is_single_stream() {
        let output = single_stream_output(&options.archive, options.output.as_deref())?;
        extract_single_stream(&options.archive, &output, format, options.overwrite, limits)?;
        return Ok(output);
    }
    let destination = options
        .output
        .unwrap_or(default_extract_destination(&options.archive, format)?);
    validate_container_destination(&destination, options.overwrite)?;

    match format.kind {
        FormatKind::Zip => extract_zip(
            &options.archive,
            &destination,
            options.overwrite,
            options.password.as_ref(),
            options.jobs,
            limits,
        )?,
        FormatKind::Tar => {
            let file = File::open(&options.archive).map_err(|err| io_at(&options.archive, err))?;
            extract_tar_file(
                &options.archive,
                file,
                &destination,
                options.overwrite,
                limits,
            )?;
        }
        FormatKind::TarCompressed(codec) => {
            extract_compressed_tar(
                &options.archive,
                codec,
                &destination,
                options.overwrite,
                limits,
            )?;
        }
        FormatKind::SingleStream(_) => unreachable!("single stream handled earlier"),
    }
    Ok(destination)
}

pub fn list(archive_path: PathBuf) -> ArcaResult<Vec<ListEntry>> {
    let format = required_format(&archive_path)?;
    let limits = ResourceLimits::from_env()?;
    match format.kind {
        FormatKind::Zip => list_zip(&archive_path, limits),
        FormatKind::Tar => {
            let file = File::open(&archive_path).map_err(|err| io_at(&archive_path, err))?;
            list_tar(file, limits)
        }
        FormatKind::TarCompressed(codec) => match codec {
            CompressionKind::Gzip => list_tar(
                LimitedReader::new(
                    GzDecoder::new(open_reader(&archive_path)?),
                    limits.max_total_unpacked_bytes,
                ),
                limits,
            ),
            CompressionKind::Bzip2 => list_tar(
                LimitedReader::new(
                    BzDecoder::new(open_reader(&archive_path)?),
                    limits.max_total_unpacked_bytes,
                ),
                limits,
            ),
            CompressionKind::Xz => list_tar(
                LimitedReader::new(
                    XzDecoder::new(open_reader(&archive_path)?),
                    limits.max_total_unpacked_bytes,
                ),
                limits,
            ),
        },
        FormatKind::SingleStream(codec) => {
            let output = single_stream_list_path(&archive_path, format)?;
            let size = fs::metadata(&archive_path)
                .map_err(|err| io_at(&archive_path, err))?
                .len();
            let uncompressed_size = single_stream_uncompressed_size(&archive_path, codec, limits)?;
            limits.check_compression_ratio(&output, uncompressed_size, size)?;
            Ok(vec![ListEntry {
                path: output,
                entry_type: "file".into(),
                uncompressed_size,
                compressed_size: Some(size),
                encrypted: false,
            }])
        }
    }
}

pub fn test(options: TestOptions) -> ArcaResult<()> {
    validate_jobs(options.jobs)?;
    let format = required_format(&options.archive)?;
    let limits = ResourceLimits::from_env()?;
    match format.kind {
        FormatKind::Zip => test_zip(
            &options.archive,
            options.password.as_ref(),
            options.jobs,
            limits,
        )?,
        FormatKind::Tar => {
            let file = File::open(&options.archive).map_err(|err| io_at(&options.archive, err))?;
            test_tar(file, &options.archive, limits)?;
        }
        FormatKind::TarCompressed(codec) => match codec {
            CompressionKind::Gzip => {
                test_tar(
                    LimitedReader::new(
                        GzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                )?;
            }
            CompressionKind::Bzip2 => {
                test_tar(
                    LimitedReader::new(
                        BzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                )?;
            }
            CompressionKind::Xz => {
                test_tar(
                    LimitedReader::new(
                        XzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                )?;
            }
        },
        FormatKind::SingleStream(codec) => {
            single_stream_uncompressed_size(&options.archive, codec, limits)?;
        }
    }
    Ok(())
}

fn single_stream_list_path(archive_path: &Path, format: ArchiveFormat) -> ArcaResult<String> {
    strip_archive_suffix(archive_path, format)?
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| ArcaError::NonUtf8Path(archive_path.to_path_buf()))
}

fn single_stream_uncompressed_size(
    archive_path: &Path,
    codec: CompressionKind,
    limits: ResourceLimits,
) -> ArcaResult<u64> {
    let mut reader: Box<dyn Read> = match codec {
        CompressionKind::Gzip => Box::new(GzDecoder::new(open_reader(archive_path)?)),
        CompressionKind::Bzip2 => Box::new(BzDecoder::new(open_reader(archive_path)?)),
        CompressionKind::Xz => Box::new(XzDecoder::new(open_reader(archive_path)?)),
    };
    let copied = copy_archive_payload_to_output(
        &mut reader,
        &mut io::sink(),
        archive_path,
        Path::new("<sink>"),
        limits.max_total_unpacked_bytes,
    )?;
    let compressed = fs::metadata(archive_path)
        .map_err(|err| io_at(archive_path, err))?
        .len();
    limits.check_compression_ratio(&archive_path.display().to_string(), copied, compressed)?;
    Ok(copied)
}

fn write_archive_atomic<F>(output: &Path, overwrite: bool, write_fn: F) -> ArcaResult<()>
where
    F: FnOnce(File) -> ArcaResult<()>,
{
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|err| io_at(parent, err))?;
    let temp = TempBuilder::new()
        .prefix(".arca-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|err| io_at(parent, err))?;
    let temp_path = temp.path().to_path_buf();
    let file = temp.reopen().map_err(|err| io_at(&temp_path, err))?;
    write_fn(file)?;
    persist_temp_file(temp, output, overwrite)?;
    Ok(())
}

fn validate_jobs(jobs: usize) -> ArcaResult<()> {
    if jobs == 0 {
        return Err(ArcaError::Usage("--jobs must be at least 1".into()));
    }
    Ok(())
}

fn write_zip<W: Write + Seek>(
    writer: W,
    entries: &[ArchiveEntry],
    level: Option<u8>,
    encryption: &Encryption,
    jobs: usize,
) -> ArcaResult<()> {
    if effective_jobs(jobs, entries.len()) > 1 {
        return write_zip_parallel(writer, entries, level, encryption, jobs);
    }

    let mut zip = ZipWriter::new(writer);
    for entry in entries {
        match entry.kind {
            EntryKind::Directory => {
                validate_path_snapshot(entry)?;
                zip.add_directory(
                    format!("{}/", entry.archive_path),
                    zip_options_for_entry(entry, level, encryption)?,
                )?;
            }
            EntryKind::Symlink => {
                validate_path_snapshot(entry)?;
                let target = entry
                    .symlink_target
                    .as_ref()
                    .ok_or_else(|| ArcaError::Security("missing symlink target".into()))?;
                zip.add_symlink(
                    &entry.archive_path,
                    target,
                    zip_options_for_entry(entry, level, encryption)?,
                )?;
            }
            EntryKind::File => {
                let mut input =
                    File::open(&entry.source).map_err(|err| io_at(&entry.source, err))?;
                validate_open_file_snapshot(entry, &input)?;
                zip.start_file(
                    &entry.archive_path,
                    zip_options_for_entry(entry, level, encryption)?,
                )?;
                io::copy(&mut input, &mut zip).map_err(|err| io_at(&entry.source, err))?;
                validate_open_file_snapshot(entry, &input)?;
            }
        }
    }
    zip.finish()?;
    Ok(())
}

fn write_zip_parallel<W: Write + Seek>(
    writer: W,
    entries: &[ArchiveEntry],
    level: Option<u8>,
    encryption: &Encryption,
    jobs: usize,
) -> ArcaResult<()> {
    let compressed_files = compress_zip_files_parallel(entries, level, encryption, jobs)?;
    let mut compressed_iter = compressed_files.into_iter();
    let mut zip = ZipWriter::new(writer);

    for entry in entries {
        match entry.kind {
            EntryKind::Directory => {
                validate_path_snapshot(entry)?;
                zip.add_directory(
                    format!("{}/", entry.archive_path),
                    zip_options_for_entry(entry, level, encryption)?,
                )?;
            }
            EntryKind::Symlink => {
                validate_path_snapshot(entry)?;
                let target = entry
                    .symlink_target
                    .as_ref()
                    .ok_or_else(|| ArcaError::Security("missing symlink target".into()))?;
                zip.add_symlink(
                    &entry.archive_path,
                    target,
                    zip_options_for_entry(entry, level, encryption)?,
                )?;
            }
            EntryKind::File => {
                let compressed = compressed_iter
                    .next()
                    .ok_or_else(|| ArcaError::Integrity("missing compressed zip entry".into()))?;
                let cursor = Cursor::new(compressed.bytes);
                let mut archive = ZipArchive::new(cursor)?;
                let file = archive.by_index_raw(0)?;
                zip.raw_copy_file(file)?;
                validate_path_snapshot(entry)?;
            }
        }
    }

    if compressed_iter.next().is_some() {
        return Err(ArcaError::Integrity(
            "unexpected extra compressed zip entry".into(),
        ));
    }
    zip.finish()?;
    Ok(())
}

#[derive(Debug)]
struct PreparedZipFile {
    bytes: Vec<u8>,
}

fn compress_zip_files_parallel(
    entries: &[ArchiveEntry],
    level: Option<u8>,
    encryption: &Encryption,
    jobs: usize,
) -> ArcaResult<Vec<PreparedZipFile>> {
    let files: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .cloned()
        .collect();
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let worker_count = effective_jobs(jobs, files.len());
    let chunk_size = files.len().div_ceil(worker_count);
    let mut per_worker =
        std::thread::scope(|scope| -> ArcaResult<Vec<Vec<(usize, PreparedZipFile)>>> {
            let mut handles = Vec::new();
            for (worker_index, chunk) in files.chunks(chunk_size).enumerate() {
                let chunk = chunk.to_vec();
                handles.push(scope.spawn(move || {
                    let mut prepared = Vec::new();
                    for (offset, entry) in chunk.iter().enumerate() {
                        let index = worker_index * chunk_size + offset;
                        prepared.push((index, prepare_zip_file_entry(entry, level, encryption)?));
                    }
                    Ok::<_, ArcaError>(prepared)
                }));
            }

            let mut output = Vec::new();
            for handle in handles {
                match handle.join() {
                    Ok(result) => output.push(result?),
                    Err(_) => {
                        return Err(ArcaError::Other("zip compression worker panicked".into()));
                    }
                }
            }
            Ok(output)
        })?;

    let mut flattened = per_worker.drain(..).flatten().collect::<Vec<_>>();
    flattened.sort_by_key(|(index, _)| *index);
    Ok(flattened.into_iter().map(|(_, file)| file).collect())
}

fn prepare_zip_file_entry(
    entry: &ArchiveEntry,
    level: Option<u8>,
    encryption: &Encryption,
) -> ArcaResult<PreparedZipFile> {
    let mut input = File::open(&entry.source).map_err(|err| io_at(&entry.source, err))?;
    validate_open_file_snapshot(entry, &input)?;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(
        &entry.archive_path,
        zip_options_for_entry(entry, level, encryption)?,
    )?;
    io::copy(&mut input, &mut zip).map_err(|err| io_at(&entry.source, err))?;
    validate_open_file_snapshot(entry, &input)?;
    let cursor = zip.finish()?;
    Ok(PreparedZipFile {
        bytes: cursor.into_inner(),
    })
}

fn zip_options_for_entry<'a>(
    entry: &ArchiveEntry,
    level: Option<u8>,
    encryption: &'a Encryption,
) -> ArcaResult<FileOptions<'a, 'static, ()>> {
    let mut options = zip_options(level, encryption)?;
    options =
        options.large_file(entry.kind == EntryKind::File && entry.size >= u64::from(u32::MAX));
    options = options.unix_permissions(entry_mode(entry));
    if let Some(modified) = entry
        .snapshot
        .modified
        .and_then(zip_datetime_from_unix_time)
    {
        options = options.last_modified_time(modified);
    }
    Ok(options)
}

fn zip_options<'a>(
    level: Option<u8>,
    encryption: &'a Encryption,
) -> ArcaResult<FileOptions<'a, 'static, ()>> {
    let mut options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(level.map(i64::from));
    options = match encryption {
        Encryption::None => options,
        Encryption::Aes256(password) => {
            options.with_aes_encryption_bytes(zip::AesMode::Aes256, password.as_bytes())
        }
        Encryption::ZipCrypto(password) => {
            options.with_deprecated_encryption(password.as_bytes())?
        }
    };
    Ok(options)
}

fn write_tar<W: Write>(writer: W, entries: &[ArchiveEntry]) -> ArcaResult<()> {
    let mut tar = Builder::new(writer);
    tar.follow_symlinks(false);
    for entry in entries {
        match entry.kind {
            EntryKind::Directory => {
                validate_path_snapshot(entry)?;
                let mut header = tar_header(entry, EntryType::Directory);
                tar.append_data(&mut header, &entry.archive_path, io::empty())
                    .map_err(|err| io_at(&entry.source, err))?;
            }
            EntryKind::File => {
                let mut input =
                    File::open(&entry.source).map_err(|err| io_at(&entry.source, err))?;
                validate_open_file_snapshot(entry, &input)?;
                let mut header = tar_header(entry, EntryType::Regular);
                header.set_size(entry.size);
                tar.append_data(&mut header, &entry.archive_path, &mut input)
                    .map_err(|err| io_at(&entry.source, err))?;
                validate_open_file_snapshot(entry, &input)?;
            }
            EntryKind::Symlink => {
                validate_path_snapshot(entry)?;
                let target = entry
                    .symlink_target
                    .as_ref()
                    .ok_or_else(|| ArcaError::Security("missing symlink target".into()))?;
                let mut header = tar_header(entry, EntryType::Symlink);
                tar.append_link(&mut header, &entry.archive_path, target)
                    .map_err(|err| io_at(&entry.source, err))?;
            }
        }
    }
    tar.finish()?;
    Ok(())
}

fn tar_header(entry: &ArchiveEntry, entry_type: EntryType) -> Header {
    let mut header = Header::new_gnu();
    header.set_entry_type(entry_type);
    header.set_size(0);
    header.set_mode(entry_mode(entry));
    if let Some(modified) = entry.snapshot.modified
        && modified.seconds >= 0
    {
        header.set_mtime(modified.seconds as u64);
    }
    header
}

fn compress_single_stream(
    inputs: &[PathBuf],
    output: &Path,
    format: ArchiveFormat,
    level: Option<u8>,
    overwrite: bool,
) -> ArcaResult<()> {
    if inputs.len() != 1 {
        return Err(ArcaError::Usage(
            "single-stream compression requires exactly one input file".into(),
        ));
    }
    let snapshot = capture_path_snapshot(&inputs[0])?;
    reject_single_stream_self_output(&inputs[0], output)?;
    if snapshot.kind != EntryKind::File {
        return Err(ArcaError::Usage(
            "single-stream compression requires a regular file".into(),
        ));
    }
    if output.exists() && !overwrite {
        return Err(ArcaError::Usage(format!(
            "output already exists: {}",
            output.display()
        )));
    }
    write_archive_atomic(output, overwrite, |file| {
        let mut input = File::open(&inputs[0]).map_err(|err| io_at(&inputs[0], err))?;
        validate_single_stream_snapshot(&inputs[0], &input, &snapshot)?;
        match format.kind {
            FormatKind::SingleStream(CompressionKind::Gzip) => {
                let mut enc = GzEncoder::new(file, gz_level(level));
                io::copy(&mut input, &mut enc).map_err(|err| io_at(&inputs[0], err))?;
                enc.finish()?;
            }
            FormatKind::SingleStream(CompressionKind::Bzip2) => {
                let mut enc = BzEncoder::new(file, bz_level(level));
                io::copy(&mut input, &mut enc).map_err(|err| io_at(&inputs[0], err))?;
                enc.finish()?;
            }
            FormatKind::SingleStream(CompressionKind::Xz) => {
                let mut enc = XzEncoder::new(file, xz_level(level));
                io::copy(&mut input, &mut enc).map_err(|err| io_at(&inputs[0], err))?;
                enc.finish()?;
            }
            _ => unreachable!("not a single stream format"),
        }
        validate_single_stream_snapshot(&inputs[0], &input, &snapshot)?;
        let current = capture_path_snapshot(&inputs[0])?;
        if current != snapshot {
            return Err(ArcaError::Integrity(format!(
                "input file changed during compression: {}",
                inputs[0].display()
            )));
        }
        Ok(())
    })
}

fn extract_single_stream(
    archive: &Path,
    output: &Path,
    format: ArchiveFormat,
    overwrite: bool,
    limits: ResourceLimits,
) -> ArcaResult<()> {
    reject_single_stream_extract_self_output(archive, output)?;
    validate_file_publish_target(output, overwrite)?;
    ensure_parent(output)?;
    let temp = temp_file_near(output)?;
    let mut writer = BufWriter::new(temp.reopen().map_err(|err| io_at(output, err))?);
    match format.kind {
        FormatKind::SingleStream(CompressionKind::Gzip) => {
            let mut dec = GzDecoder::new(open_reader(archive)?);
            copy_archive_payload_to_output(
                &mut dec,
                &mut writer,
                archive,
                output,
                limits.max_total_unpacked_bytes,
            )?;
        }
        FormatKind::SingleStream(CompressionKind::Bzip2) => {
            let mut dec = BzDecoder::new(open_reader(archive)?);
            copy_archive_payload_to_output(
                &mut dec,
                &mut writer,
                archive,
                output,
                limits.max_total_unpacked_bytes,
            )?;
        }
        FormatKind::SingleStream(CompressionKind::Xz) => {
            let mut dec = XzDecoder::new(open_reader(archive)?);
            copy_archive_payload_to_output(
                &mut dec,
                &mut writer,
                archive,
                output,
                limits.max_total_unpacked_bytes,
            )?;
        }
        _ => unreachable!("not a single stream format"),
    }
    writer.flush()?;
    drop(writer);
    let uncompressed = fs::metadata(temp.path())
        .map_err(|err| io_at(output, err))?
        .len();
    let compressed = fs::metadata(archive)
        .map_err(|err| io_at(archive, err))?
        .len();
    limits.check_compression_ratio(&archive.display().to_string(), uncompressed, compressed)?;
    persist_temp_file(temp, output, overwrite)?;
    Ok(())
}

fn copy_archive_payload_to_output<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    archive: &Path,
    output: &Path,
    max_bytes: u64,
) -> ArcaResult<u64> {
    let mut total = 0;
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buf)
            .map_err(|err| archive_payload_error(archive, err))?;
        if read == 0 {
            return Ok(total);
        }
        let read = read as u64;
        if read > max_bytes.saturating_sub(total) {
            return Err(ArcaError::Security(format!(
                "archive payload size limit exceeded in {}: > {}",
                archive.display(),
                max_bytes
            )));
        }
        writer
            .write_all(&buf[..read as usize])
            .map_err(|err| io_at(output, err))?;
        total += read;
    }
}

struct LimitedReader<R> {
    inner: R,
    remaining: u64,
    limit: u64,
}

impl<R> LimitedReader<R> {
    fn new(inner: R, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
            limit,
        }
    }
}

impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            let mut probe = [0_u8; 1];
            return match self.inner.read(&mut probe) {
                Ok(0) => Ok(0),
                Ok(_) => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "archive unpacked stream size limit exceeded: {}",
                        self.limit
                    ),
                )),
                Err(err) => Err(err),
            };
        }
        let max = if self.remaining > usize::MAX as u64 {
            buf.len()
        } else {
            min(buf.len(), self.remaining as usize)
        };
        let read = self.inner.read(&mut buf[..max])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

fn extract_zip(
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    password: Option<&Password>,
    jobs: usize,
    limits: ResourceLimits,
) -> ArcaResult<()> {
    let digest = digest_file(archive_path)?;
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    let manifest = scan_zip(&mut archive, archive_path, limits)?;
    ensure_parent(destination)?;
    extract_zip_checked(
        archive_path,
        destination,
        overwrite,
        password,
        jobs,
        ValidatedZip {
            manifest: &manifest,
            digest,
            limits,
        },
    )?;
    Ok(())
}

struct ValidatedZip<'a> {
    manifest: &'a [ScannedEntry],
    digest: [u8; 32],
    limits: ResourceLimits,
}

fn extract_zip_checked(
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    password: Option<&Password>,
    jobs: usize,
    validated: ValidatedZip<'_>,
) -> ArcaResult<()> {
    let staging = TempBuilder::new()
        .prefix(".arca-stage-")
        .tempdir_in(destination.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|err| io_at(destination, err))?;
    extract_zip_with_manifest(
        archive_path,
        staging.path(),
        overwrite,
        password,
        jobs,
        validated.manifest,
        validated.limits,
    )?;
    let digest2 = digest_file(archive_path)?;
    if validated.digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    publish_staging(staging.path(), destination, overwrite)?;
    Ok(())
}

fn extract_zip_with_manifest(
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    password: Option<&Password>,
    jobs: usize,
    manifest: &[ScannedEntry],
    limits: ResourceLimits,
) -> ArcaResult<()> {
    let mut directories = Vec::new();

    for scanned in manifest {
        if scanned.kind == EntryKind::Directory {
            let out = destination.join(&scanned.path);
            fs::create_dir_all(&out).map_err(|err| io_at(&out, err))?;
            directories.push(EntryMetadata::from_scanned(&out, scanned, false));
        }
    }

    let work: Vec<_> = manifest
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, entry)| entry.kind != EntryKind::Directory)
        .collect();
    let worker_count = effective_jobs(jobs, work.len());

    if worker_count <= 1 {
        extract_zip_entries(
            archive_path,
            destination,
            overwrite,
            password,
            limits,
            work.iter().map(|(index, entry)| (*index, entry)),
        )?;
        apply_directory_metadata(directories)?;
        return Ok(());
    }

    let chunk_size = work.len().div_ceil(worker_count);
    std::thread::scope(|scope| -> ArcaResult<()> {
        let mut handles = Vec::new();
        for chunk in work.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            handles.push(scope.spawn(move || {
                extract_zip_entries(
                    archive_path,
                    destination,
                    overwrite,
                    password,
                    limits,
                    chunk.iter().map(|(index, entry)| (*index, entry)),
                )
            }));
        }
        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(_) => {
                    return Err(ArcaError::Other("zip extraction worker panicked".into()));
                }
            }
        }
        Ok(())
    })?;
    apply_directory_metadata(directories)?;
    Ok(())
}

fn extract_zip_entries<'a, I>(
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    password: Option<&Password>,
    limits: ResourceLimits,
    entries: I,
) -> ArcaResult<()>
where
    I: IntoIterator<Item = (usize, &'a ScannedEntry)>,
{
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    for (index, scanned) in entries {
        let mut file = zip_file_by_index(&mut archive, index, password)?;
        let out = destination.join(&scanned.path);
        match scanned.kind {
            EntryKind::Directory => {}
            EntryKind::Symlink => {
                let target = read_symlink_target(&mut file, archive_path, limits)?;
                validate_symlink_target(&target)?;
                create_symlink(&target, &out, overwrite)?;
                apply_entry_metadata(&out, scanned.mode, scanned.modified, true)?;
            }
            EntryKind::File => {
                write_entry_file(
                    &mut file,
                    &out,
                    overwrite,
                    scanned.mode,
                    scanned.modified,
                    archive_path,
                    scanned.size,
                )?;
            }
        }
    }
    Ok(())
}

fn scan_zip<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    archive_path: &Path,
    limits: ResourceLimits,
) -> ArcaResult<Vec<ScannedEntry>> {
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut scanned = Vec::new();
    let mut total = 0;
    limits.check_entry_count(archive.len())?;
    for index in 0..archive.len() {
        let file = archive.by_index_raw(index)?;
        let name = file.name()?.to_string();
        let name = name.trim_end_matches('/').to_owned();
        if name.is_empty() {
            continue;
        }
        validate_archive_path(&name)?;
        collisions.insert_archive_path(&name)?;
        let kind = zip_entry_kind(&file);
        hierarchy.insert(&name, kind)?;
        let size = file.size();
        let compressed_size = file.compressed_size();
        check_scanned_entry_limits(&name, kind, size, limits)?;
        if kind != EntryKind::Directory {
            limits.check_compression_ratio(&name, size, compressed_size)?;
        }
        total = limits.add_total(total, &name, size)?;
        scanned.push(ScannedEntry {
            path: PathBuf::from(name),
            kind,
            link_target: None,
            size,
            mode: file.unix_mode().map(|mode| mode & 0o777),
            modified: file.last_modified().and_then(zip_datetime_to_unix_time),
        });
    }
    let compressed = fs::metadata(archive_path)
        .map_err(|err| io_at(archive_path, err))?
        .len();
    limits.check_compression_ratio(&archive_path.display().to_string(), total, compressed)?;
    Ok(scanned)
}

fn zip_entry_kind<R: Read>(file: &zip::read::ZipFile<'_, R>) -> EntryKind {
    if file.is_dir() {
        return EntryKind::Directory;
    }
    if file
        .unix_mode()
        .is_some_and(|mode| mode & 0o170000 == 0o120000)
    {
        EntryKind::Symlink
    } else {
        EntryKind::File
    }
}

fn zip_file_by_index<'a, R: Read + Seek>(
    archive: &'a mut ZipArchive<R>,
    index: usize,
    password: Option<&Password>,
) -> ArcaResult<zip::read::ZipFile<'a, R>> {
    match password {
        Some(password) => Ok(archive.by_index_decrypt(index, password.as_bytes())?),
        None => Ok(archive.by_index(index)?),
    }
}

fn check_scanned_entry_limits(
    path: &str,
    kind: EntryKind,
    size: u64,
    limits: ResourceLimits,
) -> ArcaResult<()> {
    if kind == EntryKind::Directory {
        return Ok(());
    }
    if kind == EntryKind::Symlink {
        limits.check_symlink_target_size(path, size)?;
    }
    limits.check_entry_size(path, size)
}

fn read_symlink_target<R: Read>(
    reader: &mut R,
    archive_path: &Path,
    limits: ResourceLimits,
) -> ArcaResult<String> {
    let mut limited = reader.take(limits.max_symlink_target_bytes + 1);
    let mut target = String::new();
    limited
        .read_to_string(&mut target)
        .map_err(|err| archive_payload_error(archive_path, err))?;
    if target.len() as u64 > limits.max_symlink_target_bytes {
        return Err(ArcaError::Security(format!(
            "symlink target size limit exceeded in {}: {} > {}",
            archive_path.display(),
            target.len(),
            limits.max_symlink_target_bytes
        )));
    }
    Ok(target)
}

fn test_zip(
    archive_path: &Path,
    password: Option<&Password>,
    jobs: usize,
    limits: ResourceLimits,
) -> ArcaResult<()> {
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    scan_zip(&mut archive, archive_path, limits)?;
    let len = archive.len();
    let worker_count = effective_jobs(jobs, len);
    if worker_count <= 1 {
        test_zip_entries(archive_path, password, limits, 0..len)?;
        return Ok(());
    }

    let chunk_size = len.div_ceil(worker_count);
    std::thread::scope(|scope| -> ArcaResult<()> {
        let mut handles = Vec::new();
        for start in (0..len).step_by(chunk_size) {
            let end = min(start + chunk_size, len);
            handles.push(
                scope.spawn(move || test_zip_entries(archive_path, password, limits, start..end)),
            );
        }
        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(_) => return Err(ArcaError::Other("zip test worker panicked".into())),
            }
        }
        Ok(())
    })
}

fn test_zip_entries<I>(
    archive_path: &Path,
    password: Option<&Password>,
    limits: ResourceLimits,
    entries: I,
) -> ArcaResult<()>
where
    I: IntoIterator<Item = usize>,
{
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    for index in entries {
        let mut file = zip_file_by_index(&mut archive, index, password)?;
        if zip_entry_kind(&file) == EntryKind::Symlink {
            let target = read_symlink_target(&mut file, archive_path, limits)?;
            validate_symlink_target(&target)?;
        } else {
            let expected = file.size();
            let copied = copy_archive_payload_to_output(
                &mut file,
                &mut io::sink(),
                archive_path,
                Path::new("<sink>"),
                expected,
            )?;
            if copied != expected {
                return Err(ArcaError::Integrity(format!(
                    "archive entry size changed while testing {}",
                    archive_path.display()
                )));
            }
        }
    }
    Ok(())
}

fn effective_jobs(jobs: usize, work_items: usize) -> usize {
    min(jobs.max(1), work_items.max(1))
}

#[derive(Debug, Clone)]
struct ScannedEntry {
    path: PathBuf,
    kind: EntryKind,
    link_target: Option<String>,
    size: u64,
    mode: Option<u32>,
    modified: Option<UnixTime>,
}

#[derive(Default)]
struct ArchiveHierarchy {
    entries: Vec<(String, EntryKind)>,
}

impl ArchiveHierarchy {
    fn insert(&mut self, path: &str, kind: EntryKind) -> ArcaResult<()> {
        let key = collision_key(path)?;
        for (existing, existing_kind) in &self.entries {
            if path_is_child_of(&key, existing) && *existing_kind != EntryKind::Directory {
                return Err(ArcaError::Security(format!(
                    "archive path has non-directory parent: {path}"
                )));
            }
            if path_is_child_of(existing, &key) && kind != EntryKind::Directory {
                return Err(ArcaError::Security(format!(
                    "archive path is parent of existing entry but is not a directory: {path}"
                )));
            }
        }
        self.entries.push((key, kind));
        Ok(())
    }
}

fn path_is_child_of(path: &str, parent: &str) -> bool {
    path.len() > parent.len()
        && path.starts_with(parent)
        && path.as_bytes().get(parent.len()) == Some(&b'/')
}

fn extract_compressed_tar(
    archive_path: &Path,
    codec: CompressionKind,
    destination: &Path,
    overwrite: bool,
    limits: ResourceLimits,
) -> ArcaResult<()> {
    let (manifest, digest) = scan_compressed_tar(archive_path, codec, limits)?;
    ensure_parent(destination)?;
    let staging = TempBuilder::new()
        .prefix(".arca-stage-")
        .tempdir_in(destination.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|err| io_at(destination, err))?;
    match codec {
        CompressionKind::Gzip => extract_tar_with_manifest(
            LimitedReader::new(
                GzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            staging.path(),
            overwrite,
            &manifest,
            archive_path,
            limits,
        )?,
        CompressionKind::Bzip2 => extract_tar_with_manifest(
            LimitedReader::new(
                BzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            staging.path(),
            overwrite,
            &manifest,
            archive_path,
            limits,
        )?,
        CompressionKind::Xz => extract_tar_with_manifest(
            LimitedReader::new(
                XzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            staging.path(),
            overwrite,
            &manifest,
            archive_path,
            limits,
        )?,
    }
    let digest2 = digest_file(archive_path)?;
    if digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    publish_staging(staging.path(), destination, overwrite)?;
    Ok(())
}

fn scan_compressed_tar(
    archive_path: &Path,
    codec: CompressionKind,
    limits: ResourceLimits,
) -> ArcaResult<(Vec<ScannedEntry>, [u8; 32])> {
    let digest = digest_file(archive_path)?;
    let manifest = match codec {
        CompressionKind::Gzip => pre_scan_tar(
            LimitedReader::new(
                GzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            limits,
        )?,
        CompressionKind::Bzip2 => pre_scan_tar(
            LimitedReader::new(
                BzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            limits,
        )?,
        CompressionKind::Xz => pre_scan_tar(
            LimitedReader::new(
                XzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            limits,
        )?,
    };
    let total = manifest.iter().map(|entry| entry.size).sum();
    let compressed = fs::metadata(archive_path)
        .map_err(|err| io_at(archive_path, err))?
        .len();
    limits.check_compression_ratio(&archive_path.display().to_string(), total, compressed)?;
    Ok((manifest, digest))
}

fn extract_tar_file(
    archive_path: &Path,
    mut file: File,
    destination: &Path,
    overwrite: bool,
    limits: ResourceLimits,
) -> ArcaResult<()> {
    let digest = digest_file(archive_path)?;
    let manifest = pre_scan_tar(&mut file, limits)?;
    file.seek(SeekFrom::Start(0))?;
    ensure_parent(destination)?;
    extract_tar_file_checked(
        file,
        archive_path,
        destination,
        overwrite,
        &manifest,
        digest,
        limits,
    )?;
    Ok(())
}

fn extract_tar_file_checked(
    file: File,
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    manifest: &[ScannedEntry],
    digest: [u8; 32],
    limits: ResourceLimits,
) -> ArcaResult<()> {
    let staging = TempBuilder::new()
        .prefix(".arca-stage-")
        .tempdir_in(destination.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|err| io_at(destination, err))?;
    extract_tar_with_manifest(
        file,
        staging.path(),
        overwrite,
        manifest,
        archive_path,
        limits,
    )?;
    let digest2 = digest_file(archive_path)?;
    if digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    publish_staging(staging.path(), destination, overwrite)?;
    Ok(())
}

fn extract_tar_with_manifest<R: Read>(
    reader: R,
    destination: &Path,
    overwrite: bool,
    manifest: &[ScannedEntry],
    archive_path: &Path,
    limits: ResourceLimits,
) -> ArcaResult<()> {
    let expected: Vec<_> = manifest.iter().map(manifest_key).collect();
    let mut seen = Vec::new();
    let mut directories = Vec::new();
    for scanned in manifest {
        if scanned.kind == EntryKind::Directory {
            let out = destination.join(&scanned.path);
            fs::create_dir_all(&out).map_err(|err| io_at(&out, err))?;
            directories.push(EntryMetadata::from_scanned(&out, scanned, false));
        }
    }

    let mut archive = Archive::new(reader);
    for item in archive.entries().map_err(tar_archive_error)? {
        let mut entry = item.map_err(tar_archive_error)?;
        let scanned = scan_tar_entry(&entry, limits)?;
        seen.push(manifest_key(&scanned));
        let out = destination.join(&scanned.path);
        match scanned.kind {
            EntryKind::Directory => {}
            EntryKind::Symlink => {
                let target = scanned
                    .link_target
                    .ok_or_else(|| ArcaError::Security("missing symlink target".into()))?;
                create_symlink(&target, &out, overwrite)?;
                apply_entry_metadata(&out, scanned.mode, scanned.modified, true)?;
            }
            EntryKind::File => {
                write_entry_file(
                    &mut entry,
                    &out,
                    overwrite,
                    scanned.mode,
                    scanned.modified,
                    archive_path,
                    scanned.size,
                )?;
            }
        }
    }
    if seen != expected {
        return Err(ArcaError::Integrity(
            "tar entry manifest changed between validation and extraction".into(),
        ));
    }
    apply_directory_metadata(directories)?;
    Ok(())
}

fn test_tar<R: Read>(reader: R, archive_path: &Path, limits: ResourceLimits) -> ArcaResult<()> {
    let mut archive = Archive::new(reader);
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut total = 0;
    for item in archive.entries().map_err(tar_archive_error)? {
        let mut entry = item.map_err(tar_archive_error)?;
        let scanned = scan_tar_entry(&entry, limits)?;
        let path_str = scanned.path.to_string_lossy();
        collisions.insert_archive_path(&path_str)?;
        hierarchy.insert(&path_str, scanned.kind)?;
        total = limits.add_total(total, &path_str, scanned.size)?;
        if scanned.kind == EntryKind::File {
            let copied = copy_archive_payload_to_output(
                &mut entry,
                &mut io::sink(),
                archive_path,
                Path::new("<sink>"),
                scanned.size,
            )?;
            if copied != scanned.size {
                return Err(ArcaError::Integrity(format!(
                    "tar entry size changed while testing {}",
                    archive_path.display()
                )));
            }
        }
    }
    Ok(())
}

fn pre_scan_tar<R: Read>(reader: R, limits: ResourceLimits) -> ArcaResult<Vec<ScannedEntry>> {
    let mut archive = Archive::new(reader);
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut scanned = Vec::new();
    let mut total = 0;
    for item in archive.entries().map_err(tar_archive_error)? {
        let entry = item.map_err(tar_archive_error)?;
        let scanned_entry = scan_tar_entry(&entry, limits)?;
        let path_str = scanned_entry.path.to_string_lossy();
        collisions.insert_archive_path(&path_str)?;
        hierarchy.insert(&path_str, scanned_entry.kind)?;
        total = limits.add_total(total, &path_str, scanned_entry.size)?;
        scanned.push(scanned_entry);
        limits.check_entry_count(scanned.len())?;
    }
    Ok(scanned)
}

fn scan_tar_entry<R: Read>(
    entry: &tar::Entry<'_, R>,
    limits: ResourceLimits,
) -> ArcaResult<ScannedEntry> {
    let path = entry.path().map_err(tar_archive_error)?;
    let path_str = path
        .to_str()
        .ok_or_else(|| ArcaError::NonUtf8Path(path.to_path_buf()))?
        .trim_end_matches('/')
        .to_owned();
    validate_archive_path(&path_str)?;
    let entry_type = entry.header().entry_type();
    let header_size = entry.header().size().map_err(tar_archive_error)?;
    let kind = if entry_type.is_dir() {
        EntryKind::Directory
    } else if entry_type.is_file() {
        EntryKind::File
    } else if entry_type.is_symlink() {
        EntryKind::Symlink
    } else if entry_type.is_hard_link() {
        return Err(ArcaError::Security(format!(
            "tar hardlink entries are not supported: {path_str}"
        )));
    } else {
        return Err(ArcaError::Security(format!(
            "tar special entry is not supported: {path_str}"
        )));
    };
    let link_target = if kind == EntryKind::Symlink {
        let target = entry
            .link_name()
            .map_err(tar_archive_error)?
            .ok_or_else(|| ArcaError::Security("missing symlink target".into()))?;
        let target = target
            .to_str()
            .ok_or_else(|| ArcaError::NonUtf8Path(target.to_path_buf()))?
            .to_owned();
        validate_symlink_target(&target)?;
        limits.check_symlink_target_size(&path_str, target.len() as u64)?;
        Some(target)
    } else {
        None
    };
    let size = if kind == EntryKind::Symlink {
        link_target.as_ref().map_or(0, |target| target.len() as u64)
    } else if kind == EntryKind::Directory {
        0
    } else {
        header_size
    };
    check_scanned_entry_limits(&path_str, kind, size, limits)?;
    Ok(ScannedEntry {
        path: PathBuf::from(path_str),
        kind,
        link_target,
        size,
        mode: entry.header().mode().ok().map(|mode| mode & 0o777),
        modified: entry
            .header()
            .mtime()
            .ok()
            .and_then(|seconds| i64::try_from(seconds).ok())
            .map(|seconds| UnixTime { seconds, nanos: 0 }),
    })
}

fn list_zip(path: &Path, limits: ResourceLimits) -> ArcaResult<Vec<ListEntry>> {
    let mut archive = ZipArchive::new(File::open(path).map_err(|err| io_at(path, err))?)?;
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut entries = Vec::new();
    let mut total = 0;
    limits.check_entry_count(archive.len())?;
    for index in 0..archive.len() {
        let file = archive.by_index_raw(index)?;
        let raw_name = file.name()?.to_string();
        let name = raw_name.trim_end_matches('/').to_owned();
        if name.is_empty() {
            continue;
        }
        validate_archive_path(&name)?;
        collisions.insert_archive_path(&name)?;
        let kind = zip_entry_kind(&file);
        hierarchy.insert(&name, kind)?;
        let uncompressed_size = file.size();
        let compressed_size = file.compressed_size();
        check_scanned_entry_limits(&name, kind, uncompressed_size, limits)?;
        if kind != EntryKind::Directory {
            limits.check_compression_ratio(&name, uncompressed_size, compressed_size)?;
        }
        total = limits.add_total(total, &name, uncompressed_size)?;
        let encrypted = file.encrypted();
        drop(file);
        if kind == EntryKind::Symlink && !encrypted {
            let mut file = archive.by_index(index)?;
            let target = read_symlink_target(&mut file, path, limits)?;
            validate_symlink_target(&target)?;
        }
        entries.push(ListEntry {
            path: raw_name,
            entry_type: match kind {
                EntryKind::Directory => "directory",
                EntryKind::File => "file",
                EntryKind::Symlink => "symlink",
            }
            .into(),
            uncompressed_size,
            compressed_size: Some(compressed_size),
            encrypted,
        });
    }
    let compressed = fs::metadata(path).map_err(|err| io_at(path, err))?.len();
    limits.check_compression_ratio(&path.display().to_string(), total, compressed)?;
    Ok(entries)
}

fn list_tar<R: Read>(reader: R, limits: ResourceLimits) -> ArcaResult<Vec<ListEntry>> {
    let mut archive = Archive::new(reader);
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut entries = Vec::new();
    let mut total = 0;
    for item in archive.entries().map_err(tar_archive_error)? {
        let entry = item.map_err(tar_archive_error)?;
        let scanned = scan_tar_entry(&entry, limits)?;
        let path_str = scanned.path.to_string_lossy();
        collisions.insert_archive_path(&path_str)?;
        hierarchy.insert(&path_str, scanned.kind)?;
        total = limits.add_total(total, &path_str, scanned.size)?;
        entries.push(ListEntry {
            path: path_str.into_owned(),
            entry_type: match scanned.kind {
                EntryKind::Directory => "directory",
                EntryKind::File => "file",
                EntryKind::Symlink => "symlink",
            }
            .into(),
            uncompressed_size: scanned.size,
            compressed_size: None,
            encrypted: false,
        });
        limits.check_entry_count(entries.len())?;
    }
    Ok(entries)
}

fn write_entry_file<R: Read>(
    reader: &mut R,
    out: &Path,
    overwrite: bool,
    mode: Option<u32>,
    modified: Option<UnixTime>,
    archive_path: &Path,
    expected_size: u64,
) -> ArcaResult<()> {
    if let Ok(existing) = fs::symlink_metadata(out) {
        if existing.file_type().is_symlink() {
            return Err(ArcaError::Security(format!(
                "refusing to overwrite existing symlink: {}",
                out.display()
            )));
        }
        if !overwrite {
            return Err(ArcaError::Usage(format!(
                "output file already exists: {}",
                out.display()
            )));
        }
    }
    ensure_parent(out)?;
    let temp = temp_file_near(out)?;
    {
        let mut writer = BufWriter::new(temp.reopen().map_err(|err| io_at(out, err))?);
        let copied =
            copy_archive_payload_to_output(reader, &mut writer, archive_path, out, expected_size)?;
        if copied != expected_size {
            return Err(ArcaError::Integrity(format!(
                "archive entry size changed while extracting {}",
                archive_path.display()
            )));
        }
        writer.flush()?;
    }
    persist_temp_file(temp, out, overwrite)?;
    apply_entry_metadata(out, mode, modified, false)?;
    Ok(())
}

#[derive(Debug)]
struct EntryMetadata {
    path: PathBuf,
    mode: Option<u32>,
    modified: Option<UnixTime>,
    symlink: bool,
}

impl EntryMetadata {
    fn from_scanned(path: &Path, entry: &ScannedEntry, symlink: bool) -> Self {
        Self {
            path: path.to_path_buf(),
            mode: entry.mode,
            modified: entry.modified,
            symlink,
        }
    }
}

fn entry_metadata_from_fs(target: &Path, meta: &fs::Metadata) -> ArcaResult<EntryMetadata> {
    let file_type = meta.file_type();
    let (kind, symlink) = if file_type.is_symlink() {
        (EntryKind::Symlink, true)
    } else if file_type.is_dir() {
        (EntryKind::Directory, false)
    } else if file_type.is_file() {
        (EntryKind::File, false)
    } else {
        return Err(ArcaError::Security(format!(
            "staging contains unsupported file type: {}",
            target.display()
        )));
    };
    let snapshot = SourceSnapshot::from_metadata(meta, kind);
    Ok(EntryMetadata {
        path: target.to_path_buf(),
        mode: snapshot.mode,
        modified: snapshot.modified,
        symlink,
    })
}

fn apply_directory_metadata(mut directories: Vec<EntryMetadata>) -> ArcaResult<()> {
    directories.sort_by_key(|entry| std::cmp::Reverse(entry.path.components().count()));
    for entry in directories {
        apply_entry_metadata(&entry.path, entry.mode, entry.modified, entry.symlink)?;
    }
    Ok(())
}

fn apply_entry_metadata(
    path: &Path,
    mode: Option<u32>,
    modified: Option<UnixTime>,
    symlink: bool,
) -> ArcaResult<()> {
    #[cfg(unix)]
    if !symlink && let Some(mode) = mode {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(mode & 0o777);
        fs::set_permissions(path, permissions).map_err(|err| io_at(path, err))?;
    }

    if let Some(modified) = modified {
        let file_time = FileTime::from_unix_time(modified.seconds, modified.nanos);
        if symlink {
            filetime::set_symlink_file_times(path, file_time, file_time)
                .map_err(|err| io_at(path, err))?;
        } else {
            filetime::set_file_times(path, file_time, file_time).map_err(|err| io_at(path, err))?;
        }
    }
    Ok(())
}

fn create_symlink(target: &str, out: &Path, overwrite: bool) -> ArcaResult<()> {
    validate_symlink_target(target)?;
    if let Ok(existing) = fs::symlink_metadata(out) {
        if existing.file_type().is_symlink() || !overwrite {
            return Err(ArcaError::Security(format!(
                "refusing to overwrite existing destination with symlink: {}",
                out.display()
            )));
        }
        return Err(ArcaError::Security(format!(
            "refusing to replace existing non-symlink with symlink: {}",
            out.display()
        )));
    }
    ensure_parent(out)?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, out).map_err(|err| io_at(out, err))?;
        Ok(())
    }
    #[cfg(windows)]
    {
        if symlink_target_is_existing_directory(target, out) {
            std::os::windows::fs::symlink_dir(target, out).map_err(|err| io_at(out, err))?;
        } else {
            std::os::windows::fs::symlink_file(target, out).map_err(|err| io_at(out, err))?;
        }
        Ok(())
    }
}

#[cfg(windows)]
fn symlink_target_is_existing_directory(target: &str, out: &Path) -> bool {
    out.parent()
        .map(|parent| parent.join(Path::new(target)).is_dir())
        .unwrap_or(false)
}

fn publish_staging(staging: &Path, destination: &Path, overwrite: bool) -> ArcaResult<()> {
    let items = walk_flat(staging)?;
    validate_publish_targets(&items, staging, destination, overwrite)?;
    fs::create_dir_all(destination).map_err(|err| io_at(destination, err))?;
    let mut directories = Vec::new();
    for item in items {
        let rel = item
            .strip_prefix(staging)
            .map_err(|err| ArcaError::Other(err.to_string()))?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(rel);
        let meta = fs::symlink_metadata(&item).map_err(|err| io_at(&item, err))?;
        let entry_metadata = entry_metadata_from_fs(&target, &meta)?;
        if meta.is_dir() {
            fs::create_dir_all(&target).map_err(|err| io_at(&target, err))?;
            directories.push(entry_metadata);
        } else if meta.file_type().is_symlink() {
            let link = fs::read_link(&item).map_err(|err| io_at(&item, err))?;
            let link = link
                .to_str()
                .ok_or_else(|| ArcaError::NonUtf8Path(link.clone()))?
                .to_owned();
            create_symlink(&link, &target, overwrite)?;
            apply_entry_metadata(&target, entry_metadata.mode, entry_metadata.modified, true)?;
        } else {
            publish_staged_file(&item, &target, overwrite, &entry_metadata)?;
        }
    }
    apply_directory_metadata(directories)?;
    Ok(())
}

fn validate_publish_targets(
    items: &[PathBuf],
    staging: &Path,
    destination: &Path,
    overwrite: bool,
) -> ArcaResult<()> {
    for item in items {
        let rel = item
            .strip_prefix(staging)
            .map_err(|err| ArcaError::Other(err.to_string()))?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(rel);
        validate_publish_parent(&target, destination)?;
        let meta = fs::symlink_metadata(item).map_err(|err| io_at(item, err))?;
        if meta.is_dir() {
            validate_directory_publish_target(&target)?;
        } else if meta.file_type().is_symlink() {
            let link = fs::read_link(item).map_err(|err| io_at(item, err))?;
            let link = link
                .to_str()
                .ok_or_else(|| ArcaError::NonUtf8Path(link.clone()))?;
            validate_symlink_target(link)?;
            validate_symlink_publish_target(&target, overwrite)?;
        } else {
            validate_file_publish_target(&target, overwrite)?;
        }
    }
    Ok(())
}

fn validate_publish_parent(target: &Path, destination: &Path) -> ArcaResult<()> {
    validate_existing_directory_component(destination)?;
    let parent = target.parent().unwrap_or(destination);
    let rel_parent = parent
        .strip_prefix(destination)
        .map_err(|err| ArcaError::Other(err.to_string()))?;
    let mut current = destination.to_path_buf();
    for component in rel_parent.components() {
        current.push(component.as_os_str());
        validate_existing_directory_component(&current)?;
    }
    Ok(())
}

fn validate_existing_directory_component(path: &Path) -> ArcaResult<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
            Err(ArcaError::Security(format!(
                "refusing to publish through existing non-directory: {}",
                path.display()
            )))
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(io_at(path, err)),
    }
}

fn validate_directory_publish_target(target: &Path) -> ArcaResult<()> {
    match fs::symlink_metadata(target) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
            Err(ArcaError::Security(format!(
                "refusing to overwrite existing non-directory: {}",
                target.display()
            )))
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(io_at(target, err)),
    }
}

fn validate_symlink_publish_target(target: &Path, overwrite: bool) -> ArcaResult<()> {
    if let Ok(existing) = fs::symlink_metadata(target) {
        if existing.file_type().is_symlink() || !overwrite {
            return Err(ArcaError::Security(format!(
                "refusing to overwrite existing destination with symlink: {}",
                target.display()
            )));
        }
        return Err(ArcaError::Security(format!(
            "refusing to replace existing non-symlink with symlink: {}",
            target.display()
        )));
    }
    Ok(())
}

fn validate_container_destination(destination: &Path, overwrite: bool) -> ArcaResult<()> {
    if let Ok(existing) = fs::symlink_metadata(destination) {
        if existing.file_type().is_symlink() {
            return Err(ArcaError::Security(format!(
                "refusing to extract through existing destination symlink: {}",
                destination.display()
            )));
        }
        if !existing.is_dir() {
            if overwrite {
                return Err(ArcaError::Security(format!(
                    "refusing to replace existing non-directory destination: {}",
                    destination.display()
                )));
            }
            return Err(ArcaError::Usage(format!(
                "output destination already exists: {}",
                destination.display()
            )));
        }
        if !overwrite {
            return Err(ArcaError::Usage(format!(
                "output destination already exists: {}",
                destination.display()
            )));
        }
    }
    Ok(())
}

fn publish_staged_file(
    source: &Path,
    target: &Path,
    overwrite: bool,
    metadata: &EntryMetadata,
) -> ArcaResult<()> {
    if let Ok(existing) = fs::symlink_metadata(target) {
        if existing.file_type().is_symlink() {
            return Err(ArcaError::Security(format!(
                "refusing to overwrite existing symlink: {}",
                target.display()
            )));
        }
        if !existing.is_file() {
            return Err(ArcaError::Security(format!(
                "refusing to overwrite existing non-file: {}",
                target.display()
            )));
        }
        if !overwrite {
            return Err(ArcaError::Usage(format!(
                "output file already exists: {}",
                target.display()
            )));
        }
        replace_existing_file(source, target, metadata)?;
        return Ok(());
    }

    ensure_parent(target)?;
    fs::rename(source, target).map_err(|err| io_at(target, err))?;
    Ok(())
}

fn replace_existing_file(source: &Path, target: &Path, metadata: &EntryMetadata) -> ArcaResult<()> {
    ensure_parent(target)?;
    let temp = temp_file_near(target)?;
    {
        let mut input = File::open(source).map_err(|err| io_at(source, err))?;
        let mut output = BufWriter::new(temp.reopen().map_err(|err| io_at(target, err))?);
        io::copy(&mut input, &mut output).map_err(|err| io_at(target, err))?;
        output.flush()?;
    }
    persist_temp_file(temp, target, true)?;
    apply_entry_metadata(target, metadata.mode, metadata.modified, false)?;
    Ok(())
}

fn persist_temp_file(
    temp: tempfile::NamedTempFile,
    target: &Path,
    overwrite: bool,
) -> ArcaResult<()> {
    validate_file_publish_target(target, overwrite)?;
    temp.persist(target)
        .map_err(|err| io_at(target, err.error))?;
    Ok(())
}

fn validate_file_publish_target(target: &Path, overwrite: bool) -> ArcaResult<()> {
    if let Ok(existing) = fs::symlink_metadata(target) {
        if existing.file_type().is_symlink() {
            return Err(ArcaError::Security(format!(
                "refusing to overwrite existing symlink: {}",
                target.display()
            )));
        }
        if !existing.is_file() {
            return Err(ArcaError::Security(format!(
                "refusing to overwrite existing non-file: {}",
                target.display()
            )));
        }
        if !overwrite {
            return Err(ArcaError::Usage(format!(
                "output file already exists: {}",
                target.display()
            )));
        }
    }
    Ok(())
}

fn walk_flat(root: &Path) -> ArcaResult<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for item in walkdir::WalkDir::new(root).sort_by_file_name() {
        let item = item.map_err(|err| ArcaError::Other(err.to_string()))?;
        paths.push(item.path().to_path_buf());
    }
    Ok(paths)
}

fn ensure_parent(path: &Path) -> ArcaResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| io_at(parent, err))?;
    }
    Ok(())
}

fn temp_file_near(path: &Path) -> ArcaResult<tempfile::NamedTempFile> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    TempBuilder::new()
        .prefix(".arca-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|err| io_at(parent, err))
}

fn open_reader(path: &Path) -> ArcaResult<BufReader<File>> {
    Ok(BufReader::new(
        File::open(path).map_err(|err| io_at(path, err))?,
    ))
}

fn digest_file(path: &Path) -> ArcaResult<[u8; 32]> {
    let mut file = File::open(path).map_err(|err| io_at(path, err))?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf).map_err(|err| io_at(path, err))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hasher.finalize().into())
}

fn archive_payload_error(path: &Path, error: io::Error) -> ArcaError {
    ArcaError::Integrity(format!(
        "failed to read archive payload in {}: {error}",
        path.display()
    ))
}

fn tar_archive_error(error: io::Error) -> ArcaError {
    ArcaError::Integrity(format!("failed to read tar archive: {error}"))
}

fn manifest_key(
    entry: &ScannedEntry,
) -> (
    PathBuf,
    EntryKind,
    Option<String>,
    Option<u32>,
    Option<UnixTime>,
) {
    (
        entry.path.clone(),
        entry.kind,
        entry.link_target.clone(),
        entry.mode,
        entry.modified,
    )
}

fn auto_tar_output(output: &Path, format: ArchiveFormat) -> ArcaResult<PathBuf> {
    let replacement = match format.kind {
        FormatKind::SingleStream(CompressionKind::Gzip) => ".tar.gz",
        FormatKind::SingleStream(CompressionKind::Bzip2) => ".tar.bz2",
        FormatKind::SingleStream(CompressionKind::Xz) => ".tar.xz",
        _ => return Ok(output.to_path_buf()),
    };
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ArcaError::NonUtf8Path(output.to_path_buf()))?;
    let stem = &name[..name.len() - format.suffix.len()];
    Ok(output.with_file_name(format!("{stem}{replacement}")))
}

fn validate_single_stream_snapshot(
    path: &Path,
    file: &File,
    snapshot: &SourceSnapshot,
) -> ArcaResult<()> {
    let meta = file.metadata().map_err(|err| io_at(path, err))?;
    let current = SourceSnapshot::from_metadata(&meta, EntryKind::File);
    if &current != snapshot {
        return Err(ArcaError::Integrity(format!(
            "input file changed during compression: {}",
            path.display()
        )));
    }
    Ok(())
}

fn reject_single_stream_self_output(input: &Path, output: &Path) -> ArcaResult<()> {
    let input_abs = input.canonicalize().map_err(|err| io_at(input, err))?;
    let output_abs = match output.canonicalize() {
        Ok(path) => path,
        Err(_) => absolute_output_path_for_compare(output)?,
    };
    if input_abs == output_abs {
        return Err(ArcaError::Usage(format!(
            "output archive cannot equal input file: {}",
            output.display()
        )));
    }
    Ok(())
}

fn reject_single_stream_extract_self_output(archive: &Path, output: &Path) -> ArcaResult<()> {
    let archive_abs = archive.canonicalize().map_err(|err| io_at(archive, err))?;
    let output_abs = match output.canonicalize() {
        Ok(path) => path,
        Err(_) => absolute_output_path_for_compare(output)?,
    };
    if archive_abs == output_abs {
        return Err(ArcaError::Usage(format!(
            "extraction output cannot equal archive: {}",
            output.display()
        )));
    }
    Ok(())
}

fn absolute_output_path_for_compare(path: &Path) -> ArcaResult<PathBuf> {
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

fn entry_mode(entry: &ArchiveEntry) -> u32 {
    entry
        .snapshot
        .mode
        .unwrap_or_else(|| default_mode(entry.kind))
}

fn default_mode(kind: EntryKind) -> u32 {
    match kind {
        EntryKind::File => 0o644,
        EntryKind::Directory => 0o755,
        EntryKind::Symlink => 0o777,
    }
}

fn zip_datetime_from_unix_time(time: UnixTime) -> Option<DateTime> {
    let days = time.seconds.div_euclid(86_400);
    let second_of_day = time.seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days)?;
    if !(1980..=2107).contains(&year) {
        return None;
    }
    let hour = (second_of_day / 3_600) as u8;
    let minute = ((second_of_day % 3_600) / 60) as u8;
    let second = (second_of_day % 60) as u8;
    DateTime::from_date_and_time(year as u16, month as u8, day as u8, hour, minute, second).ok()
}

fn zip_datetime_to_unix_time(time: DateTime) -> Option<UnixTime> {
    if !time.is_valid() {
        return None;
    }
    let days = days_from_civil(time.year() as i32, time.month() as u32, time.day() as u32)?;
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(i64::from(time.hour()) * 3_600)?
        .checked_add(i64::from(time.minute()) * 60)?
        .checked_add(i64::from(time.second()))?;
    Some(UnixTime { seconds, nanos: 0 })
}

fn civil_from_days(days_since_epoch: i64) -> Option<(i32, u32, u32)> {
    let z = days_since_epoch.checked_add(719_468)?;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    Some((i32::try_from(year).ok()?, month as u32, day as u32))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let year = i64::from(year) - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn gz_level(level: Option<u8>) -> GzCompression {
    GzCompression::new(u32::from(level.unwrap_or(6).min(9)))
}

fn bz_level(level: Option<u8>) -> BzCompression {
    BzCompression::new(u32::from(level.unwrap_or(6).min(9)))
}

fn xz_level(level: Option<u8>) -> u32 {
    u32::from(level.unwrap_or(6).min(9))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_tar_publish_rejects_archive_changed_after_validation() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("input.tar");
        write_test_tar(&archive, b"before\n");

        let digest = digest_file(&archive).unwrap();
        let mut scan_file = File::open(&archive).unwrap();
        let limits = test_limits();
        let manifest = pre_scan_tar(&mut scan_file, limits).unwrap();
        drop(scan_file);

        write_test_tar(&archive, b"after\n");
        let out = dir.path().join("out");
        let err = extract_tar_file_checked(
            File::open(&archive).unwrap(),
            &archive,
            &out,
            false,
            &manifest,
            digest,
            limits,
        )
        .unwrap_err();
        assert!(
            matches!(err, ArcaError::Integrity(_)),
            "expected integrity error, got {err}"
        );
        assert!(!out.exists(), "changed archive should not publish output");
    }

    #[test]
    fn zip_publish_rejects_archive_changed_after_validation() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("input.zip");
        write_test_zip(&archive, b"before\n");

        let digest = digest_file(&archive).unwrap();
        let mut zip = ZipArchive::new(File::open(&archive).unwrap()).unwrap();
        let limits = test_limits();
        let manifest = scan_zip(&mut zip, &archive, limits).unwrap();
        drop(zip);

        write_test_zip(&archive, b"after\n");
        let out = dir.path().join("out");
        let err = extract_zip_checked(
            &archive,
            &out,
            false,
            None,
            1,
            ValidatedZip {
                manifest: &manifest,
                digest,
                limits,
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, ArcaError::Integrity(_)),
            "expected integrity error, got {err}"
        );
        assert!(!out.exists(), "changed archive should not publish output");
    }

    fn write_test_tar(path: &Path, contents: &[u8]) {
        let file = File::create(path).unwrap();
        let mut tar = Builder::new(file);
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Regular);
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        tar.append_data(&mut header, "data.txt", contents).unwrap();
        tar.finish().unwrap();
    }

    fn write_test_zip(path: &Path, contents: &[u8]) {
        let file = File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("data.txt", FileOptions::<()>::default())
            .unwrap();
        zip.write_all(contents).unwrap();
        zip.finish().unwrap();
    }

    fn test_limits() -> ResourceLimits {
        ResourceLimits {
            max_entries: 1_000,
            max_entry_unpacked_bytes: 1024 * 1024,
            max_total_unpacked_bytes: 1024 * 1024,
            max_compression_ratio: 10_000,
            max_symlink_target_bytes: 16 * 1024,
        }
    }
}
