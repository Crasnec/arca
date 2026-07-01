use std::cmp::min;
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

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

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    canceled: Arc<AtomicBool>,
}

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> ArcaResult<()> {
        if self.is_canceled() {
            return Err(ArcaError::Canceled("operation was canceled".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreProgressPhase {
    Starting,
    Scanning,
    Reading,
    Writing,
    Testing,
    Extracting,
    Committing,
    Finished,
}

#[derive(Clone, Debug)]
pub struct CoreProgress {
    pub phase: CoreProgressPhase,
    pub message: String,
    pub processed: Option<u64>,
    pub total: Option<u64>,
}

impl CoreProgress {
    #[must_use]
    pub fn new(phase: CoreProgressPhase, message: impl Into<String>) -> Self {
        Self {
            phase,
            message: message.into(),
            processed: None,
            total: None,
        }
    }

    #[must_use]
    pub fn with_counts(mut self, processed: u64, total: Option<u64>) -> Self {
        self.processed = Some(processed);
        self.total = total;
        self
    }
}

#[derive(Clone)]
pub struct ProgressSink {
    handler: Arc<dyn Fn(CoreProgress) + Send + Sync>,
}

impl ProgressSink {
    #[must_use]
    pub fn new(handler: impl Fn(CoreProgress) + Send + Sync + 'static) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }

    pub fn emit(&self, progress: CoreProgress) {
        (self.handler)(progress);
    }
}

impl fmt::Debug for ProgressSink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProgressSink(<callback>)")
    }
}

#[derive(Clone, Debug, Default)]
pub struct OperationContext {
    cancellation: CancellationToken,
    progress: Option<ProgressSink>,
}

impl OperationContext {
    #[must_use]
    pub fn new(cancellation: CancellationToken) -> Self {
        Self {
            cancellation,
            progress: None,
        }
    }

    #[must_use]
    pub fn with_progress_sink(mut self, progress: ProgressSink) -> Self {
        self.progress = Some(progress);
        self
    }

    #[must_use]
    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn check_cancelled(&self) -> ArcaResult<()> {
        self.cancellation.check()
    }

    pub fn progress(&self, phase: CoreProgressPhase, message: impl Into<String>) {
        if let Some(progress) = &self.progress {
            progress.emit(CoreProgress::new(phase, message));
        }
    }

    pub fn progress_counts(
        &self,
        phase: CoreProgressPhase,
        message: impl Into<String>,
        processed: u64,
        total: Option<u64>,
    ) {
        if let Some(progress) = &self.progress {
            progress.emit(CoreProgress::new(phase, message).with_counts(processed, total));
        }
    }
}

#[derive(Clone, Debug)]
struct SharedProgressCounter {
    processed: Arc<Mutex<u64>>,
    total: u64,
    phase: CoreProgressPhase,
    message: &'static str,
}

impl SharedProgressCounter {
    fn new(phase: CoreProgressPhase, message: &'static str, total: u64) -> Self {
        Self {
            processed: Arc::new(Mutex::new(0)),
            total,
            phase,
            message,
        }
    }

    fn increment_by(&self, amount: u64, context: &OperationContext) {
        let mut processed = self
            .processed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *processed = (*processed).saturating_add(amount).min(self.total);
        context.progress_counts(self.phase, self.message, *processed, Some(self.total));
    }

    fn finish(&self, context: &OperationContext) {
        let mut processed = self
            .processed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *processed = self.total;
        context.progress_counts(self.phase, self.message, self.total, Some(self.total));
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
pub struct ExtractSelectionOptions {
    pub archive: PathBuf,
    pub output: Option<PathBuf>,
    pub overwrite: bool,
    pub jobs: usize,
    pub password: Option<Password>,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestOptions {
    pub archive: PathBuf,
    pub jobs: usize,
    pub password: Option<Password>,
}

#[derive(Debug, Clone)]
pub struct TestSelectionOptions {
    pub archive: PathBuf,
    pub jobs: usize,
    pub password: Option<Password>,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteSelectionOptions {
    pub archive: PathBuf,
    pub expected_digest_sha256: String,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PlanDirectEditAddOptions {
    pub archive: PathBuf,
    pub inputs: Vec<PathBuf>,
    pub pending_delete_entries: Vec<String>,
    pub pending_add_entries: Vec<DirectEditPendingEntry>,
}

#[derive(Debug, Clone)]
pub struct DirectEditSaveOptions {
    pub archive: PathBuf,
    pub expected_digest_sha256: String,
    pub delete_entries: Vec<String>,
    pub add_inputs: Vec<PathBuf>,
    pub add_entries: Vec<String>,
    pub replace_entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectEditAddPlan {
    pub additions: Vec<DirectEditPlannedEntry>,
    pub replacements: Vec<DirectEditPlannedEntry>,
}

#[derive(Debug, Clone)]
pub struct DirectEditPendingEntry {
    pub archive_path: String,
    pub entry_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectEditPlannedEntry {
    pub archive_path: String,
    pub source_path: PathBuf,
    pub entry_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListEntry {
    pub path: String,
    pub entry_type: String,
    pub uncompressed_size: u64,
    pub compressed_size: Option<u64>,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectEditStatus {
    pub allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveManifest {
    pub archive_path: PathBuf,
    pub archive_name: String,
    pub format_kind: String,
    pub format_suffix: String,
    pub digest_sha256: String,
    pub entries: Vec<ListEntry>,
    pub entry_count: usize,
    pub total_uncompressed_size: u64,
    pub total_compressed_size: Option<u64>,
    pub encrypted_entry_count: usize,
    pub direct_edit: DirectEditStatus,
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
    compress_with_context(options, OperationContext::default())
}

pub fn compress_with_context(
    options: CompressOptions,
    context: OperationContext,
) -> ArcaResult<PathBuf> {
    context.progress(CoreProgressPhase::Starting, "Starting compression");
    context.check_cancelled()?;
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
            &context,
        )?;
        context.progress(CoreProgressPhase::Finished, "Compression finished");
        return Ok(output);
    }

    context.progress(CoreProgressPhase::Scanning, "Planning archive inputs");
    let entries = plan_entries(&options.inputs, &options.excludes, &output)?;
    context.check_cancelled()?;
    let _target_lock = TargetLock::acquire(&output)?;
    write_archive_atomic(&output, options.overwrite, &context, |file| {
        match format.kind {
            FormatKind::Zip => {
                write_zip(
                    file,
                    &entries,
                    options.level,
                    &options.encryption,
                    options.jobs,
                    &context,
                )?;
                ensure_entries_unchanged(&entries, &options.inputs, &options.excludes, &output)
            }
            FormatKind::Tar => {
                write_tar(file, &entries, &context)?;
                ensure_entries_unchanged(&entries, &options.inputs, &options.excludes, &output)
            }
            FormatKind::TarCompressed(codec) => {
                match codec {
                    CompressionKind::Gzip => {
                        let encoder = GzEncoder::new(file, gz_level(options.level));
                        write_tar(encoder, &entries, &context)?;
                    }
                    CompressionKind::Bzip2 => {
                        let encoder = BzEncoder::new(file, bz_level(options.level));
                        write_tar(encoder, &entries, &context)?;
                    }
                    CompressionKind::Xz => {
                        let encoder = XzEncoder::new(file, xz_level(options.level));
                        write_tar(encoder, &entries, &context)?;
                    }
                };
                ensure_entries_unchanged(&entries, &options.inputs, &options.excludes, &output)
            }
            FormatKind::SingleStream(_) => unreachable!("single stream handled earlier"),
        }
    })?;
    context.progress(CoreProgressPhase::Finished, "Compression finished");
    Ok(output)
}

pub fn extract(options: ExtractOptions) -> ArcaResult<PathBuf> {
    extract_with_context(options, OperationContext::default())
}

pub fn extract_with_context(
    options: ExtractOptions,
    context: OperationContext,
) -> ArcaResult<PathBuf> {
    context.progress(CoreProgressPhase::Starting, "Starting extraction");
    context.check_cancelled()?;
    validate_jobs(options.jobs)?;
    let format = required_format(&options.archive)?;
    let limits = ResourceLimits::from_env()?;
    if format.is_single_stream() {
        let output = single_stream_output(&options.archive, options.output.as_deref())?;
        let _target_lock = TargetLock::acquire(&output)?;
        extract_single_stream(
            &options.archive,
            &output,
            format,
            options.overwrite,
            limits,
            &context,
        )?;
        context.progress(CoreProgressPhase::Finished, "Extraction finished");
        return Ok(output);
    }
    let destination = options
        .output
        .unwrap_or(default_extract_destination(&options.archive, format)?);
    validate_container_destination(&destination, options.overwrite)?;
    let _target_lock = TargetLock::acquire(&destination)?;

    match format.kind {
        FormatKind::Zip => extract_zip(
            &options.archive,
            &destination,
            options.overwrite,
            options.password.as_ref(),
            options.jobs,
            limits,
            &context,
        )?,
        FormatKind::Tar => {
            let file = File::open(&options.archive).map_err(|err| io_at(&options.archive, err))?;
            extract_tar_file(
                &options.archive,
                file,
                &destination,
                options.overwrite,
                limits,
                &context,
            )?;
        }
        FormatKind::TarCompressed(codec) => {
            extract_compressed_tar(
                &options.archive,
                codec,
                &destination,
                options.overwrite,
                limits,
                &context,
            )?;
        }
        FormatKind::SingleStream(_) => unreachable!("single stream handled earlier"),
    }
    context.progress(CoreProgressPhase::Finished, "Extraction finished");
    Ok(destination)
}

pub fn extract_selection(options: ExtractSelectionOptions) -> ArcaResult<PathBuf> {
    extract_selection_with_context(options, OperationContext::default())
}

pub fn extract_selection_with_context(
    options: ExtractSelectionOptions,
    context: OperationContext,
) -> ArcaResult<PathBuf> {
    context.progress(CoreProgressPhase::Starting, "Starting selected extraction");
    context.check_cancelled()?;
    validate_jobs(options.jobs)?;
    let selection = ArchiveSelection::new(&options.entries)?;
    let format = required_format(&options.archive)?;
    let limits = ResourceLimits::from_env()?;
    if format.is_single_stream() {
        selection.require_single_stream_match(&options.archive, format)?;
        let output = single_stream_output(&options.archive, options.output.as_deref())?;
        let _target_lock = TargetLock::acquire(&output)?;
        extract_single_stream(
            &options.archive,
            &output,
            format,
            options.overwrite,
            limits,
            &context,
        )?;
        context.progress(CoreProgressPhase::Finished, "Selected extraction finished");
        return Ok(output);
    }

    let destination = options
        .output
        .unwrap_or(default_extract_destination(&options.archive, format)?);
    validate_container_destination(&destination, options.overwrite)?;
    let _target_lock = TargetLock::acquire(&destination)?;

    match format.kind {
        FormatKind::Zip => extract_zip_selection(
            &options.archive,
            &destination,
            options.overwrite,
            options.password.as_ref(),
            options.jobs,
            limits,
            &selection,
            &context,
        )?,
        FormatKind::Tar => {
            let file = File::open(&options.archive).map_err(|err| io_at(&options.archive, err))?;
            extract_tar_file_selection(
                &options.archive,
                file,
                &destination,
                options.overwrite,
                limits,
                &selection,
                &context,
            )?;
        }
        FormatKind::TarCompressed(codec) => {
            extract_compressed_tar_selection(
                &options.archive,
                codec,
                &destination,
                options.overwrite,
                limits,
                &selection,
                &context,
            )?;
        }
        FormatKind::SingleStream(_) => unreachable!("single stream handled earlier"),
    }
    context.progress(CoreProgressPhase::Finished, "Selected extraction finished");
    Ok(destination)
}

pub fn list(archive_path: PathBuf) -> ArcaResult<Vec<ListEntry>> {
    list_with_context(archive_path, OperationContext::default())
}

pub fn list_with_context(
    archive_path: PathBuf,
    context: OperationContext,
) -> ArcaResult<Vec<ListEntry>> {
    context.progress(CoreProgressPhase::Scanning, "Listing archive");
    context.check_cancelled()?;
    let format = required_format(&archive_path)?;
    let limits = ResourceLimits::from_env()?;
    match format.kind {
        FormatKind::Zip => list_zip(&archive_path, limits, &context),
        FormatKind::Tar => {
            let file = File::open(&archive_path).map_err(|err| io_at(&archive_path, err))?;
            list_tar(file, limits, &context)
        }
        FormatKind::TarCompressed(codec) => match codec {
            CompressionKind::Gzip => list_tar(
                LimitedReader::new(
                    GzDecoder::new(open_reader(&archive_path)?),
                    limits.max_total_unpacked_bytes,
                ),
                limits,
                &context,
            ),
            CompressionKind::Bzip2 => list_tar(
                LimitedReader::new(
                    BzDecoder::new(open_reader(&archive_path)?),
                    limits.max_total_unpacked_bytes,
                ),
                limits,
                &context,
            ),
            CompressionKind::Xz => list_tar(
                LimitedReader::new(
                    XzDecoder::new(open_reader(&archive_path)?),
                    limits.max_total_unpacked_bytes,
                ),
                limits,
                &context,
            ),
        },
        FormatKind::SingleStream(codec) => {
            let output = single_stream_list_path(&archive_path, format)?;
            let size = fs::metadata(&archive_path)
                .map_err(|err| io_at(&archive_path, err))?
                .len();
            let uncompressed_size = single_stream_uncompressed_size(
                &archive_path,
                codec,
                limits,
                CoreProgressPhase::Scanning,
                "Scanning single-stream payload",
                &context,
            )?;
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

pub fn inspect_archive(archive_path: PathBuf) -> ArcaResult<ArchiveManifest> {
    inspect_archive_with_context(archive_path, OperationContext::default())
}

pub fn inspect_archive_with_context(
    archive_path: PathBuf,
    context: OperationContext,
) -> ArcaResult<ArchiveManifest> {
    let format = required_format(&archive_path)?;
    let digest_before = digest_file_with_context(&archive_path, &context)?;
    let entries = list_with_context(archive_path.clone(), context.clone())?;
    let digest_after = digest_file_with_context(&archive_path, &context)?;
    if digest_before != digest_after {
        return Err(ArcaError::Integrity(format!(
            "archive changed while listing: {}",
            archive_path.display()
        )));
    }

    let total_uncompressed_size = entries.iter().try_fold(0_u64, |total, entry| {
        total.checked_add(entry.uncompressed_size).ok_or_else(|| {
            ArcaError::Security(format!("archive unpacked size overflow at {}", entry.path))
        })
    })?;
    let mut total_compressed_size = Some(0_u64);
    for entry in &entries {
        let Some(total) = total_compressed_size else {
            break;
        };
        let Some(size) = entry.compressed_size else {
            total_compressed_size = None;
            break;
        };
        total_compressed_size = Some(total.checked_add(size).ok_or_else(|| {
            ArcaError::Security(format!(
                "archive compressed size overflow at {}",
                entry.path
            ))
        })?);
    }
    let encrypted_entry_count = entries.iter().filter(|entry| entry.encrypted).count();

    Ok(ArchiveManifest {
        archive_name: archive_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or_else(|| ArcaError::NonUtf8Path(archive_path.clone()))?,
        archive_path,
        format_kind: format_kind_label(format.kind).to_owned(),
        format_suffix: format.suffix.to_owned(),
        digest_sha256: hex_digest(digest_before),
        entry_count: entries.len(),
        total_uncompressed_size,
        total_compressed_size,
        encrypted_entry_count,
        direct_edit: direct_edit_status(format, encrypted_entry_count),
        entries,
    })
}

pub fn test(options: TestOptions) -> ArcaResult<()> {
    test_with_context(options, OperationContext::default())
}

pub fn test_with_context(options: TestOptions, context: OperationContext) -> ArcaResult<()> {
    context.progress(CoreProgressPhase::Starting, "Starting archive test");
    context.check_cancelled()?;
    validate_jobs(options.jobs)?;
    let format = required_format(&options.archive)?;
    let limits = ResourceLimits::from_env()?;
    match format.kind {
        FormatKind::Zip => test_zip(
            &options.archive,
            options.password.as_ref(),
            options.jobs,
            limits,
            &context,
        )?,
        FormatKind::Tar => {
            let digest = digest_file_with_context(&options.archive, &context)?;
            let mut file =
                File::open(&options.archive).map_err(|err| io_at(&options.archive, err))?;
            let manifest = pre_scan_tar(&mut file, limits, &context)?;
            file.seek(SeekFrom::Start(0))?;
            test_tar_with_manifest(file, &options.archive, limits, &manifest, &context)?;
            let digest2 = digest_file_with_context(&options.archive, &context)?;
            if digest != digest2 {
                return Err(ArcaError::Integrity(
                    "archive changed between validation and testing".into(),
                ));
            }
        }
        FormatKind::TarCompressed(codec) => match codec {
            CompressionKind::Gzip => {
                let (manifest, digest) =
                    scan_compressed_tar(&options.archive, codec, limits, &context)?;
                test_tar_with_manifest(
                    LimitedReader::new(
                        GzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                    &manifest,
                    &context,
                )?;
                let digest2 = digest_file_with_context(&options.archive, &context)?;
                if digest != digest2 {
                    return Err(ArcaError::Integrity(
                        "archive changed between validation and testing".into(),
                    ));
                }
            }
            CompressionKind::Bzip2 => {
                let (manifest, digest) =
                    scan_compressed_tar(&options.archive, codec, limits, &context)?;
                test_tar_with_manifest(
                    LimitedReader::new(
                        BzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                    &manifest,
                    &context,
                )?;
                let digest2 = digest_file_with_context(&options.archive, &context)?;
                if digest != digest2 {
                    return Err(ArcaError::Integrity(
                        "archive changed between validation and testing".into(),
                    ));
                }
            }
            CompressionKind::Xz => {
                let (manifest, digest) =
                    scan_compressed_tar(&options.archive, codec, limits, &context)?;
                test_tar_with_manifest(
                    LimitedReader::new(
                        XzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                    &manifest,
                    &context,
                )?;
                let digest2 = digest_file_with_context(&options.archive, &context)?;
                if digest != digest2 {
                    return Err(ArcaError::Integrity(
                        "archive changed between validation and testing".into(),
                    ));
                }
            }
        },
        FormatKind::SingleStream(codec) => {
            single_stream_uncompressed_size(
                &options.archive,
                codec,
                limits,
                CoreProgressPhase::Testing,
                "Testing single-stream payload",
                &context,
            )?;
        }
    }
    context.progress(CoreProgressPhase::Finished, "Archive test finished");
    Ok(())
}

pub fn test_selection(options: TestSelectionOptions) -> ArcaResult<()> {
    test_selection_with_context(options, OperationContext::default())
}

pub fn test_selection_with_context(
    options: TestSelectionOptions,
    context: OperationContext,
) -> ArcaResult<()> {
    context.progress(CoreProgressPhase::Starting, "Starting selected entry test");
    context.check_cancelled()?;
    validate_jobs(options.jobs)?;
    let selection = ArchiveSelection::new(&options.entries)?;
    let format = required_format(&options.archive)?;
    let limits = ResourceLimits::from_env()?;
    match format.kind {
        FormatKind::Zip => test_zip_selection(
            &options.archive,
            options.password.as_ref(),
            options.jobs,
            limits,
            &selection,
            &context,
        )?,
        FormatKind::Tar => {
            let digest = digest_file_with_context(&options.archive, &context)?;
            let mut file =
                File::open(&options.archive).map_err(|err| io_at(&options.archive, err))?;
            let manifest = pre_scan_tar(&mut file, limits, &context)?;
            let selected = selection.select_manifest(&manifest)?;
            file.seek(SeekFrom::Start(0))?;
            test_tar_selected_entries(
                file,
                &options.archive,
                limits,
                &manifest,
                &selected,
                &context,
            )?;
            let digest2 = digest_file_with_context(&options.archive, &context)?;
            if digest != digest2 {
                return Err(ArcaError::Integrity(
                    "archive changed between validation and testing".into(),
                ));
            }
        }
        FormatKind::TarCompressed(codec) => match codec {
            CompressionKind::Gzip => {
                let (manifest, digest) =
                    scan_compressed_tar(&options.archive, codec, limits, &context)?;
                let selected = selection.select_manifest(&manifest)?;
                test_tar_selected_entries(
                    LimitedReader::new(
                        GzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                    &manifest,
                    &selected,
                    &context,
                )?;
                let digest2 = digest_file_with_context(&options.archive, &context)?;
                if digest != digest2 {
                    return Err(ArcaError::Integrity(
                        "archive changed between validation and testing".into(),
                    ));
                }
            }
            CompressionKind::Bzip2 => {
                let (manifest, digest) =
                    scan_compressed_tar(&options.archive, codec, limits, &context)?;
                let selected = selection.select_manifest(&manifest)?;
                test_tar_selected_entries(
                    LimitedReader::new(
                        BzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                    &manifest,
                    &selected,
                    &context,
                )?;
                let digest2 = digest_file_with_context(&options.archive, &context)?;
                if digest != digest2 {
                    return Err(ArcaError::Integrity(
                        "archive changed between validation and testing".into(),
                    ));
                }
            }
            CompressionKind::Xz => {
                let (manifest, digest) =
                    scan_compressed_tar(&options.archive, codec, limits, &context)?;
                let selected = selection.select_manifest(&manifest)?;
                test_tar_selected_entries(
                    LimitedReader::new(
                        XzDecoder::new(open_reader(&options.archive)?),
                        limits.max_total_unpacked_bytes,
                    ),
                    &options.archive,
                    limits,
                    &manifest,
                    &selected,
                    &context,
                )?;
                let digest2 = digest_file_with_context(&options.archive, &context)?;
                if digest != digest2 {
                    return Err(ArcaError::Integrity(
                        "archive changed between validation and testing".into(),
                    ));
                }
            }
        },
        FormatKind::SingleStream(_) => {
            selection.require_single_stream_match(&options.archive, format)?;
            test_with_context(
                TestOptions {
                    archive: options.archive,
                    jobs: options.jobs,
                    password: options.password,
                },
                context.clone(),
            )?;
        }
    }
    context.progress(CoreProgressPhase::Finished, "Selected entry test finished");
    Ok(())
}

pub fn delete_selection(options: DeleteSelectionOptions) -> ArcaResult<ArchiveManifest> {
    delete_selection_with_context(options, OperationContext::default())
}

pub fn delete_selection_with_context(
    options: DeleteSelectionOptions,
    context: OperationContext,
) -> ArcaResult<ArchiveManifest> {
    save_direct_edit_with_context(
        DirectEditSaveOptions {
            archive: options.archive,
            expected_digest_sha256: options.expected_digest_sha256,
            delete_entries: options.entries,
            add_inputs: Vec::new(),
            add_entries: Vec::new(),
            replace_entries: Vec::new(),
        },
        context,
    )
}

pub fn plan_direct_edit_add(options: PlanDirectEditAddOptions) -> ArcaResult<DirectEditAddPlan> {
    plan_direct_edit_add_with_context(options, OperationContext::default())
}

pub fn plan_direct_edit_add_with_context(
    options: PlanDirectEditAddOptions,
    context: OperationContext,
) -> ArcaResult<DirectEditAddPlan> {
    context.progress(
        CoreProgressPhase::Scanning,
        "Planning direct edit additions",
    );
    context.check_cancelled()?;
    let format = required_format(&options.archive)?;
    if !matches!(format.kind, FormatKind::Zip) {
        return Err(ArcaError::Unsupported(
            "Direct Editing add is only supported for Plain ZIP archives".into(),
        ));
    }
    if options.inputs.is_empty() {
        return Err(ArcaError::Usage(
            "at least one input file or folder is required".into(),
        ));
    }
    let limits = ResourceLimits::from_env()?;
    let file = File::open(&options.archive).map_err(|err| io_at(&options.archive, err))?;
    let mut archive = ZipArchive::new(file)?;
    reject_encrypted_zip(&mut archive)?;
    let manifest = scan_zip(&mut archive, &options.archive, limits, &context)?;
    let delete_paths = selected_manifest_paths(&manifest, &options.pending_delete_entries)?;
    let pending_adds = normalize_direct_edit_pending_entries(&options.pending_add_entries)?;
    context.check_cancelled()?;
    let planned = plan_entries(&options.inputs, &[], &options.archive)?;
    validate_direct_edit_pending_add_conflicts(&pending_adds, &planned)?;
    plan_direct_edit_entries(&manifest, &delete_paths, &planned)
}

pub fn save_direct_edit(options: DirectEditSaveOptions) -> ArcaResult<ArchiveManifest> {
    save_direct_edit_with_context(options, OperationContext::default())
}

pub fn save_direct_edit_with_context(
    options: DirectEditSaveOptions,
    context: OperationContext,
) -> ArcaResult<ArchiveManifest> {
    context.progress(CoreProgressPhase::Starting, "Starting direct edit save");
    context.check_cancelled()?;
    let format = required_format(&options.archive)?;
    if !matches!(format.kind, FormatKind::Zip) {
        return Err(ArcaError::Unsupported(
            "Direct Editing save is only supported for Plain ZIP archives".into(),
        ));
    }
    let limits = ResourceLimits::from_env()?;
    validate_file_publish_target(&options.archive, true)?;
    let _target_lock = TargetLock::acquire(&options.archive)?;

    let digest = digest_file_with_context(&options.archive, &context)?;
    let digest_hex = hex_digest(digest);
    if options.expected_digest_sha256 != digest_hex {
        return Err(ArcaError::Integrity(format!(
            "archive changed before saving: {}",
            options.archive.display()
        )));
    }

    let file = File::open(&options.archive).map_err(|err| io_at(&options.archive, err))?;
    let mut archive = ZipArchive::new(file)?;
    reject_encrypted_zip(&mut archive)?;
    let manifest = scan_zip(&mut archive, &options.archive, limits, &context)?;
    let delete_paths = selected_manifest_paths(&manifest, &options.delete_entries)?;
    let add_entries = normalize_archive_path_set(&options.add_entries)?;
    let replace_entries = normalize_archive_path_set(&options.replace_entries)?;
    let planned = if options.add_inputs.is_empty() {
        Vec::new()
    } else {
        context.progress(CoreProgressPhase::Scanning, "Planning direct edit inputs");
        plan_entries(&options.add_inputs, &[], &options.archive)?
    };
    let included = planned
        .iter()
        .filter(|entry| add_entries.contains(entry.archive_path.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let planned_paths = planned
        .iter()
        .map(|entry| Ok(entry.archive_path.clone()))
        .collect::<ArcaResult<BTreeSet<_>>>()?;
    let missing_adds = add_entries
        .difference(&planned_paths)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_adds.is_empty() {
        return Err(ArcaError::Usage(format!(
            "planned add entry not found: {}",
            missing_adds.join(", ")
        )));
    }
    validate_direct_edit_replacements(&manifest, &delete_paths, &included, &replace_entries)?;
    let replaced_paths = direct_edit_replaced_paths(&manifest, &delete_paths, &included)?;
    validate_direct_edit_final_paths(&manifest, &delete_paths, &replaced_paths, &included)?;

    let parent = options.archive.parent().unwrap_or_else(|| Path::new("."));
    let temp = TempBuilder::new()
        .prefix(".arca-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|err| io_at(parent, err))?;
    let temp_path = temp.path().to_path_buf();
    {
        let writer = temp.reopen().map_err(|err| io_at(&temp_path, err))?;
        rewrite_plain_zip_with_changes(
            &options.archive,
            writer,
            &delete_paths,
            &replaced_paths,
            &included,
            &context,
        )?;
    }
    test_zip(&temp_path, None, 1, limits, &context)?;

    let digest_after_write = digest_file_with_context(&options.archive, &context)?;
    if digest_after_write != digest {
        return Err(ArcaError::Integrity(format!(
            "archive changed before saving: {}",
            options.archive.display()
        )));
    }
    context.check_cancelled()?;
    context.progress(CoreProgressPhase::Committing, "Publishing direct edit save");
    persist_temp_file(temp, &options.archive, true)?;
    let manifest = inspect_archive_with_context(options.archive, context.clone())?;
    context.progress(CoreProgressPhase::Finished, "Direct edit save finished");
    Ok(manifest)
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
    progress_phase: CoreProgressPhase,
    progress_message: &'static str,
    context: &OperationContext,
) -> ArcaResult<u64> {
    let mut reader: Box<dyn Read> = match codec {
        CompressionKind::Gzip => Box::new(GzDecoder::new(open_reader(archive_path)?)),
        CompressionKind::Bzip2 => Box::new(BzDecoder::new(open_reader(archive_path)?)),
        CompressionKind::Xz => Box::new(XzDecoder::new(open_reader(archive_path)?)),
    };
    let copied = copy_archive_payload_to_output(
        &mut reader,
        &mut io::sink(),
        PayloadCopyPlan {
            archive: archive_path,
            output: Path::new("<sink>"),
            max_bytes: limits.max_total_unpacked_bytes,
            progress_phase,
            progress_message,
            context,
        },
    )?;
    let compressed = fs::metadata(archive_path)
        .map_err(|err| io_at(archive_path, err))?
        .len();
    limits.check_compression_ratio(&archive_path.display().to_string(), copied, compressed)?;
    Ok(copied)
}

fn format_kind_label(kind: FormatKind) -> &'static str {
    match kind {
        FormatKind::Zip => "zip",
        FormatKind::Tar => "tar",
        FormatKind::TarCompressed(CompressionKind::Gzip) => "tar.gz",
        FormatKind::TarCompressed(CompressionKind::Bzip2) => "tar.bz2",
        FormatKind::TarCompressed(CompressionKind::Xz) => "tar.xz",
        FormatKind::SingleStream(CompressionKind::Gzip) => "gz",
        FormatKind::SingleStream(CompressionKind::Bzip2) => "bz2",
        FormatKind::SingleStream(CompressionKind::Xz) => "xz",
    }
}

fn entry_kind_label(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Directory => "directory",
        EntryKind::File => "file",
        EntryKind::Symlink => "symlink",
    }
}

fn direct_edit_status(format: ArchiveFormat, encrypted_entry_count: usize) -> DirectEditStatus {
    if !matches!(format.kind, FormatKind::Zip) {
        return DirectEditStatus {
            allowed: false,
            reason: Some("Direct Editing is planned only for Plain ZIP archives".into()),
        };
    }
    if encrypted_entry_count > 0 {
        return DirectEditStatus {
            allowed: false,
            reason: Some("Encrypted ZIP archives are read-only in the GUI".into()),
        };
    }
    DirectEditStatus {
        allowed: true,
        reason: None,
    }
}

fn write_archive_atomic<F>(
    output: &Path,
    overwrite: bool,
    context: &OperationContext,
    write_fn: F,
) -> ArcaResult<()>
where
    F: FnOnce(File) -> ArcaResult<()>,
{
    context.check_cancelled()?;
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
    context.check_cancelled()?;
    context.progress(CoreProgressPhase::Committing, "Publishing archive");
    persist_temp_file(temp, output, overwrite)?;
    Ok(())
}

fn validate_jobs(jobs: usize) -> ArcaResult<()> {
    if jobs == 0 {
        return Err(ArcaError::Usage("--jobs must be at least 1".into()));
    }
    Ok(())
}

#[derive(Debug)]
struct TargetLock {
    path: PathBuf,
}

impl TargetLock {
    fn acquire(target: &Path) -> ArcaResult<Self> {
        ensure_parent(target)?;
        let identity = target_lock_identity(target)?;
        let lock_path = target_lock_path(&identity);
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                return Err(ArcaError::Busy(format!(
                    "target is locked by another Arca operation: {}",
                    identity.display()
                )));
            }
            Err(err) => return Err(io_at(&lock_path, err)),
        };
        writeln!(
            file,
            "pid={}\ntarget={}",
            std::process::id(),
            identity.display()
        )
        .map_err(|err| io_at(&lock_path, err))?;
        Ok(Self { path: lock_path })
    }
}

impl Drop for TargetLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn target_lock_identity(target: &Path) -> ArcaResult<PathBuf> {
    match target.canonicalize() {
        Ok(path) => Ok(path),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            absolute_output_path_for_compare(target)
        }
        Err(err) => Err(io_at(target, err)),
    }
}

fn target_lock_path(identity: &Path) -> PathBuf {
    let parent = identity.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!(
        ".arca-lock-{}.lock",
        path_identity_digest(identity)
    ))
}

fn path_identity_digest(identity: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"arca-target-lock-v1\0");
    hash_path_identity(&mut hasher, identity);
    hex_digest(hasher.finalize().into())
}

#[cfg(unix)]
fn hash_path_identity(hasher: &mut Sha256, identity: &Path) {
    use std::os::unix::ffi::OsStrExt;
    hasher.update(identity.as_os_str().as_bytes());
}

#[cfg(windows)]
fn hash_path_identity(hasher: &mut Sha256, identity: &Path) {
    use std::os::windows::ffi::OsStrExt;
    for unit in identity.as_os_str().encode_wide() {
        hasher.update(unit.to_le_bytes());
    }
}

#[cfg(not(any(unix, windows)))]
fn hash_path_identity(hasher: &mut Sha256, identity: &Path) {
    hasher.update(identity.to_string_lossy().as_bytes());
}

fn write_zip<W: Write + Seek>(
    writer: W,
    entries: &[ArchiveEntry],
    level: Option<u8>,
    encryption: &Encryption,
    jobs: usize,
    context: &OperationContext,
) -> ArcaResult<()> {
    context.progress(CoreProgressPhase::Writing, "Writing ZIP archive");
    if effective_jobs(jobs, entries.len()) > 1 {
        return write_zip_parallel(writer, entries, level, encryption, jobs, context);
    }

    let mut zip = ZipWriter::new(writer);
    for (index, entry) in entries.iter().enumerate() {
        context.check_cancelled()?;
        context.progress_counts(
            CoreProgressPhase::Writing,
            "Writing ZIP entries",
            index as u64,
            Some(entries.len() as u64),
        );
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
                copy_unbounded_with_context(
                    &mut input,
                    &mut zip,
                    &entry.source,
                    &entry.source,
                    context,
                )?;
                validate_open_file_snapshot(entry, &input)?;
            }
        }
    }
    context.check_cancelled()?;
    zip.finish()?;
    Ok(())
}

fn write_zip_parallel<W: Write + Seek>(
    writer: W,
    entries: &[ArchiveEntry],
    level: Option<u8>,
    encryption: &Encryption,
    jobs: usize,
    context: &OperationContext,
) -> ArcaResult<()> {
    let compressed_files = compress_zip_files_parallel(entries, level, encryption, jobs, context)?;
    let mut compressed_iter = compressed_files.into_iter();
    let mut zip = ZipWriter::new(writer);

    for (index, entry) in entries.iter().enumerate() {
        context.check_cancelled()?;
        context.progress_counts(
            CoreProgressPhase::Writing,
            "Writing ZIP entries",
            index as u64,
            Some(entries.len() as u64),
        );
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
    context: &OperationContext,
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
    let progress = SharedProgressCounter::new(
        CoreProgressPhase::Writing,
        "Compressing ZIP file data",
        files
            .iter()
            .fold(0_u64, |total, entry| total.saturating_add(entry.size)),
    );
    let mut per_worker =
        std::thread::scope(|scope| -> ArcaResult<Vec<Vec<(usize, PreparedZipFile)>>> {
            let mut handles = Vec::new();
            for (worker_index, chunk) in files.chunks(chunk_size).enumerate() {
                let chunk = chunk.to_vec();
                let context = context.clone();
                let progress = progress.clone();
                handles.push(scope.spawn(move || {
                    let mut prepared = Vec::new();
                    for (offset, entry) in chunk.iter().enumerate() {
                        context.check_cancelled()?;
                        let index = worker_index * chunk_size + offset;
                        prepared.push((
                            index,
                            prepare_zip_file_entry(
                                entry,
                                level,
                                encryption,
                                Some(&progress),
                                &context,
                            )?,
                        ));
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
    progress.finish(context);

    let mut flattened = per_worker.drain(..).flatten().collect::<Vec<_>>();
    flattened.sort_by_key(|(index, _)| *index);
    Ok(flattened.into_iter().map(|(_, file)| file).collect())
}

fn prepare_zip_file_entry(
    entry: &ArchiveEntry,
    level: Option<u8>,
    encryption: &Encryption,
    progress: Option<&SharedProgressCounter>,
    context: &OperationContext,
) -> ArcaResult<PreparedZipFile> {
    context.check_cancelled()?;
    let mut input = File::open(&entry.source).map_err(|err| io_at(&entry.source, err))?;
    validate_open_file_snapshot(entry, &input)?;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(
        &entry.archive_path,
        zip_options_for_entry(entry, level, encryption)?,
    )?;
    copy_unbounded_with_shared_progress(
        &mut input,
        &mut zip,
        &entry.source,
        &entry.source,
        progress,
        context,
    )?;
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

fn write_tar<W: Write>(
    writer: W,
    entries: &[ArchiveEntry],
    context: &OperationContext,
) -> ArcaResult<()> {
    context.progress(CoreProgressPhase::Writing, "Writing tar archive");
    let mut tar = Builder::new(writer);
    tar.follow_symlinks(false);
    for (index, entry) in entries.iter().enumerate() {
        context.check_cancelled()?;
        context.progress_counts(
            CoreProgressPhase::Writing,
            "Writing tar entries",
            index as u64,
            Some(entries.len() as u64),
        );
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
                {
                    let mut cancelable = CancelableReader::new(&mut input, context.clone());
                    tar.append_data(&mut header, &entry.archive_path, &mut cancelable)
                        .map_err(|err| io_at(&entry.source, err))?;
                }
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
    context: &OperationContext,
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
    let _target_lock = TargetLock::acquire(output)?;
    write_archive_atomic(output, overwrite, context, |file| {
        let mut input = File::open(&inputs[0]).map_err(|err| io_at(&inputs[0], err))?;
        validate_single_stream_snapshot(&inputs[0], &input, &snapshot)?;
        match format.kind {
            FormatKind::SingleStream(CompressionKind::Gzip) => {
                let mut enc = GzEncoder::new(file, gz_level(level));
                copy_unbounded_with_context(&mut input, &mut enc, &inputs[0], output, context)?;
                enc.finish()?;
            }
            FormatKind::SingleStream(CompressionKind::Bzip2) => {
                let mut enc = BzEncoder::new(file, bz_level(level));
                copy_unbounded_with_context(&mut input, &mut enc, &inputs[0], output, context)?;
                enc.finish()?;
            }
            FormatKind::SingleStream(CompressionKind::Xz) => {
                let mut enc = XzEncoder::new(file, xz_level(level));
                copy_unbounded_with_context(&mut input, &mut enc, &inputs[0], output, context)?;
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
    context: &OperationContext,
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
                PayloadCopyPlan {
                    archive,
                    output,
                    max_bytes: limits.max_total_unpacked_bytes,
                    progress_phase: CoreProgressPhase::Extracting,
                    progress_message: "Extracting single-stream payload",
                    context,
                },
            )?;
        }
        FormatKind::SingleStream(CompressionKind::Bzip2) => {
            let mut dec = BzDecoder::new(open_reader(archive)?);
            copy_archive_payload_to_output(
                &mut dec,
                &mut writer,
                PayloadCopyPlan {
                    archive,
                    output,
                    max_bytes: limits.max_total_unpacked_bytes,
                    progress_phase: CoreProgressPhase::Extracting,
                    progress_message: "Extracting single-stream payload",
                    context,
                },
            )?;
        }
        FormatKind::SingleStream(CompressionKind::Xz) => {
            let mut dec = XzDecoder::new(open_reader(archive)?);
            copy_archive_payload_to_output(
                &mut dec,
                &mut writer,
                PayloadCopyPlan {
                    archive,
                    output,
                    max_bytes: limits.max_total_unpacked_bytes,
                    progress_phase: CoreProgressPhase::Extracting,
                    progress_message: "Extracting single-stream payload",
                    context,
                },
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
    context.check_cancelled()?;
    context.progress(CoreProgressPhase::Committing, "Publishing extracted file");
    persist_temp_file(temp, output, overwrite)?;
    Ok(())
}

struct PayloadCopyPlan<'a> {
    archive: &'a Path,
    output: &'a Path,
    max_bytes: u64,
    progress_phase: CoreProgressPhase,
    progress_message: &'static str,
    context: &'a OperationContext,
}

fn copy_archive_payload_to_output<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    plan: PayloadCopyPlan<'_>,
) -> ArcaResult<u64> {
    let PayloadCopyPlan {
        archive,
        output,
        max_bytes,
        progress_phase,
        progress_message,
        context,
    } = plan;

    let mut total = 0;
    let mut buf = [0_u8; 64 * 1024];
    loop {
        context.check_cancelled()?;
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
        context.progress_counts(progress_phase, progress_message, total, Some(max_bytes));
    }
}

fn copy_unbounded_with_context<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    input_path: &Path,
    output_path: &Path,
    context: &OperationContext,
) -> ArcaResult<u64> {
    copy_unbounded_with_shared_progress(reader, writer, input_path, output_path, None, context)
}

fn copy_unbounded_with_shared_progress<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    input_path: &Path,
    output_path: &Path,
    shared_progress: Option<&SharedProgressCounter>,
    context: &OperationContext,
) -> ArcaResult<u64> {
    let mut total = 0;
    let mut buf = [0_u8; 64 * 1024];
    let expected_size = fs::metadata(input_path).ok().map(|metadata| metadata.len());
    loop {
        context.check_cancelled()?;
        let read = reader
            .read(&mut buf)
            .map_err(|err| io_at(input_path, err))?;
        if read == 0 {
            return Ok(total);
        }
        writer
            .write_all(&buf[..read])
            .map_err(|err| io_at(output_path, err))?;
        let read = read as u64;
        total += read;
        if let Some(shared_progress) = shared_progress {
            shared_progress.increment_by(read, context);
        } else {
            context.progress_counts(
                CoreProgressPhase::Writing,
                "Copying file data",
                total,
                expected_size,
            );
        }
    }
}

struct CancelableReader<R> {
    inner: R,
    context: OperationContext,
}

impl<R> CancelableReader<R> {
    fn new(inner: R, context: OperationContext) -> Self {
        Self { inner, context }
    }
}

impl<R: Read> Read for CancelableReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.context.cancellation.is_canceled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "operation was canceled",
            ));
        }
        self.inner.read(buf)
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
    context: &OperationContext,
) -> ArcaResult<()> {
    let digest = digest_file_with_context(archive_path, context)?;
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    let manifest = scan_zip(&mut archive, archive_path, limits, context)?;
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
        context,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn extract_zip_selection(
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    password: Option<&Password>,
    jobs: usize,
    limits: ResourceLimits,
    selection: &ArchiveSelection,
    context: &OperationContext,
) -> ArcaResult<()> {
    let digest = digest_file_with_context(archive_path, context)?;
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    let manifest = scan_zip(&mut archive, archive_path, limits, context)?;
    let selected = selection.select_manifest(&manifest)?;
    ensure_parent(destination)?;
    let staging = TempBuilder::new()
        .prefix(".arca-stage-")
        .tempdir_in(destination.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|err| io_at(destination, err))?;
    extract_zip_selected_entries(
        archive_path,
        staging.path(),
        overwrite,
        password,
        jobs,
        limits,
        &selected,
        context,
    )?;
    let digest2 = digest_file_with_context(archive_path, context)?;
    if digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    context.check_cancelled()?;
    context.progress(
        CoreProgressPhase::Committing,
        "Publishing selected extracted files",
    );
    publish_staging(staging.path(), destination, overwrite)?;
    Ok(())
}

struct ValidatedZip<'a> {
    manifest: &'a [ScannedEntry],
    digest: [u8; 32],
    limits: ResourceLimits,
}

#[allow(clippy::too_many_arguments)]
fn extract_zip_checked(
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    password: Option<&Password>,
    jobs: usize,
    validated: ValidatedZip<'_>,
    context: &OperationContext,
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
        context,
    )?;
    let digest2 = digest_file_with_context(archive_path, context)?;
    if validated.digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    context.check_cancelled()?;
    context.progress(CoreProgressPhase::Committing, "Publishing extracted files");
    publish_staging(staging.path(), destination, overwrite)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn extract_zip_with_manifest(
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    password: Option<&Password>,
    jobs: usize,
    manifest: &[ScannedEntry],
    limits: ResourceLimits,
    context: &OperationContext,
) -> ArcaResult<()> {
    let mut directories = Vec::new();

    for scanned in manifest {
        context.check_cancelled()?;
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
    let progress = SharedProgressCounter::new(
        CoreProgressPhase::Extracting,
        "Extracting ZIP entries",
        work.len() as u64,
    );

    if worker_count <= 1 {
        extract_zip_entries(
            ZipExtractPlan {
                archive_path,
                destination,
                overwrite,
                password,
                limits,
                progress: Some(progress.clone()),
                context,
            },
            work.iter().map(|(index, entry)| (*index, entry)),
        )?;
        progress.finish(context);
        apply_directory_metadata(directories)?;
        return Ok(());
    }

    let chunk_size = work.len().div_ceil(worker_count);
    std::thread::scope(|scope| -> ArcaResult<()> {
        let mut handles = Vec::new();
        for chunk in work.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            let context = context.clone();
            let progress = progress.clone();
            handles.push(scope.spawn(move || {
                extract_zip_entries(
                    ZipExtractPlan {
                        archive_path,
                        destination,
                        overwrite,
                        password,
                        limits,
                        progress: Some(progress),
                        context: &context,
                    },
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
    progress.finish(context);
    apply_directory_metadata(directories)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn extract_zip_selected_entries(
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    password: Option<&Password>,
    jobs: usize,
    limits: ResourceLimits,
    selected: &[(usize, &ScannedEntry)],
    context: &OperationContext,
) -> ArcaResult<()> {
    let mut directories = Vec::new();
    for (_, scanned) in selected {
        context.check_cancelled()?;
        if scanned.kind == EntryKind::Directory {
            let out = destination.join(&scanned.path);
            fs::create_dir_all(&out).map_err(|err| io_at(&out, err))?;
            directories.push(EntryMetadata::from_scanned(&out, scanned, false));
        }
    }

    let work: Vec<_> = selected
        .iter()
        .copied()
        .filter(|(_, entry)| entry.kind != EntryKind::Directory)
        .collect();
    let worker_count = effective_jobs(jobs, work.len());
    let progress = SharedProgressCounter::new(
        CoreProgressPhase::Extracting,
        "Extracting selected ZIP entries",
        work.len() as u64,
    );

    if worker_count <= 1 {
        extract_zip_entries(
            ZipExtractPlan {
                archive_path,
                destination,
                overwrite,
                password,
                limits,
                progress: Some(progress.clone()),
                context,
            },
            work,
        )?;
        progress.finish(context);
        apply_directory_metadata(directories)?;
        return Ok(());
    }

    let chunk_size = work.len().div_ceil(worker_count);
    std::thread::scope(|scope| -> ArcaResult<()> {
        let mut handles = Vec::new();
        for chunk in work.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            let context = context.clone();
            let progress = progress.clone();
            handles.push(scope.spawn(move || {
                extract_zip_entries(
                    ZipExtractPlan {
                        archive_path,
                        destination,
                        overwrite,
                        password,
                        limits,
                        progress: Some(progress),
                        context: &context,
                    },
                    chunk,
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
    progress.finish(context);
    apply_directory_metadata(directories)?;
    Ok(())
}

struct ZipExtractPlan<'a> {
    archive_path: &'a Path,
    destination: &'a Path,
    overwrite: bool,
    password: Option<&'a Password>,
    limits: ResourceLimits,
    progress: Option<SharedProgressCounter>,
    context: &'a OperationContext,
}

fn extract_zip_entries<'entry, I>(plan: ZipExtractPlan<'_>, entries: I) -> ArcaResult<()>
where
    I: IntoIterator<Item = (usize, &'entry ScannedEntry)>,
{
    let ZipExtractPlan {
        archive_path,
        destination,
        overwrite,
        password,
        limits,
        progress,
        context,
    } = plan;

    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    for (index, scanned) in entries {
        context.check_cancelled()?;
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
                    context,
                )?;
            }
        }
        if let Some(progress) = &progress {
            progress.increment_by(1, context);
        }
    }
    Ok(())
}

fn selected_manifest_paths(
    manifest: &[ScannedEntry],
    entries: &[String],
) -> ArcaResult<BTreeSet<String>> {
    if entries.is_empty() {
        return Ok(BTreeSet::new());
    }
    let selection = ArchiveSelection::new(entries)?;
    selection
        .select_manifest(manifest)?
        .iter()
        .map(|(_, entry)| scanned_path_str(entry).map(str::to_owned))
        .collect()
}

fn normalize_direct_edit_pending_entries(
    entries: &[DirectEditPendingEntry],
) -> ArcaResult<Vec<(String, EntryKind)>> {
    entries
        .iter()
        .map(|entry| {
            validate_archive_path(&entry.archive_path)?;
            Ok((
                entry.archive_path.trim_end_matches('/').to_owned(),
                direct_edit_pending_entry_kind(&entry.entry_type)?,
            ))
        })
        .collect()
}

fn direct_edit_pending_entry_kind(entry_type: &str) -> ArcaResult<EntryKind> {
    match entry_type {
        "directory" => Ok(EntryKind::Directory),
        "file" => Ok(EntryKind::File),
        "symlink" => Ok(EntryKind::Symlink),
        other => Err(ArcaError::Usage(format!(
            "unsupported pending direct edit entry type: {other}"
        ))),
    }
}

fn normalize_archive_path_set(entries: &[String]) -> ArcaResult<BTreeSet<String>> {
    entries
        .iter()
        .map(|entry| normalize_selection_path(entry))
        .collect()
}

fn validate_direct_edit_pending_add_conflicts(
    pending_adds: &[(String, EntryKind)],
    planned: &[ArchiveEntry],
) -> ArcaResult<()> {
    for entry in planned {
        for (pending_path, pending_kind) in pending_adds {
            if direct_edit_paths_conflict(
                &entry.archive_path,
                entry.kind,
                pending_path,
                *pending_kind,
            )? {
                return Err(ArcaError::Usage(format!(
                    "archive entry already has pending changes: {}",
                    pending_path
                )));
            }
        }
    }
    Ok(())
}

fn plan_direct_edit_entries(
    manifest: &[ScannedEntry],
    delete_paths: &BTreeSet<String>,
    planned: &[ArchiveEntry],
) -> ArcaResult<DirectEditAddPlan> {
    let mut additions = Vec::new();
    let mut replacements = Vec::new();
    for entry in planned {
        if direct_edit_entry_conflicts_with_manifest(entry, manifest, delete_paths)? {
            replacements.push(direct_edit_planned_entry(entry));
        } else {
            additions.push(direct_edit_planned_entry(entry));
        }
    }
    Ok(DirectEditAddPlan {
        additions,
        replacements,
    })
}

fn direct_edit_planned_entry(entry: &ArchiveEntry) -> DirectEditPlannedEntry {
    DirectEditPlannedEntry {
        archive_path: entry.archive_path.clone(),
        source_path: entry.source.clone(),
        entry_type: entry_kind_label(entry.kind).into(),
    }
}

fn direct_edit_entry_conflicts_with_manifest(
    entry: &ArchiveEntry,
    manifest: &[ScannedEntry],
    delete_paths: &BTreeSet<String>,
) -> ArcaResult<bool> {
    for existing in manifest {
        let existing_path = scanned_path_str(existing)?;
        if delete_paths.contains(existing_path) {
            continue;
        }
        if direct_edit_paths_conflict(
            &entry.archive_path,
            entry.kind,
            existing_path,
            existing.kind,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn direct_edit_paths_conflict(
    planned_path: &str,
    planned_kind: EntryKind,
    existing_path: &str,
    existing_kind: EntryKind,
) -> ArcaResult<bool> {
    let planned_key = collision_key(planned_path)?;
    let existing_key = collision_key(existing_path)?;
    Ok(planned_key == existing_key
        || (path_is_child_of(&planned_key, &existing_key) && existing_kind != EntryKind::Directory)
        || (path_is_child_of(&existing_key, &planned_key) && planned_kind != EntryKind::Directory))
}

fn validate_direct_edit_replacements(
    manifest: &[ScannedEntry],
    delete_paths: &BTreeSet<String>,
    included: &[ArchiveEntry],
    replace_entries: &BTreeSet<String>,
) -> ArcaResult<()> {
    for entry in included {
        if direct_edit_entry_conflicts_with_manifest(entry, manifest, delete_paths)?
            && !replace_entries.contains(entry.archive_path.as_str())
        {
            return Err(ArcaError::Usage(format!(
                "replacement was not confirmed for archive entry: {}",
                entry.archive_path
            )));
        }
    }
    Ok(())
}

fn direct_edit_replaced_paths(
    manifest: &[ScannedEntry],
    delete_paths: &BTreeSet<String>,
    included: &[ArchiveEntry],
) -> ArcaResult<BTreeSet<String>> {
    let mut replaced = BTreeSet::new();
    for entry in included {
        for existing in manifest {
            let existing_path = scanned_path_str(existing)?;
            if delete_paths.contains(existing_path) {
                continue;
            }
            if direct_edit_paths_conflict(
                &entry.archive_path,
                entry.kind,
                existing_path,
                existing.kind,
            )? {
                replaced.insert(existing_path.to_owned());
            }
        }
    }
    Ok(replaced)
}

fn validate_direct_edit_final_paths(
    manifest: &[ScannedEntry],
    delete_paths: &BTreeSet<String>,
    replaced_paths: &BTreeSet<String>,
    included: &[ArchiveEntry],
) -> ArcaResult<()> {
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    for entry in manifest {
        let path = scanned_path_str(entry)?;
        if delete_paths.contains(path) || replaced_paths.contains(path) {
            continue;
        }
        collisions.insert_archive_path(path)?;
        hierarchy.insert(path, entry.kind)?;
    }
    for entry in included {
        collisions.insert_archive_path(&entry.archive_path)?;
        hierarchy.insert(&entry.archive_path, entry.kind)?;
    }
    Ok(())
}

fn rewrite_plain_zip_with_changes<W: Write + Seek>(
    archive_path: &Path,
    writer: W,
    delete_paths: &BTreeSet<String>,
    replaced_paths: &BTreeSet<String>,
    additions: &[ArchiveEntry],
    context: &OperationContext,
) -> ArcaResult<()> {
    context.progress(CoreProgressPhase::Writing, "Rewriting ZIP archive");
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut source = ZipArchive::new(file)?;
    reject_encrypted_zip(&mut source)?;
    let mut output = ZipWriter::new(writer);
    let rewrite_total = source.len().saturating_add(additions.len()) as u64;
    let mut rewritten = 0_u64;
    for index in 0..source.len() {
        context.check_cancelled()?;
        let file = source.by_index_raw(index)?;
        let path = file.name()?.trim_end_matches('/').to_owned();
        if path.is_empty() {
            return Err(ArcaError::Security(
                "cannot rewrite ZIP archive with an empty entry name".into(),
            ));
        }
        validate_archive_path(&path)?;
        if delete_paths.contains(&path) || replaced_paths.contains(&path) {
            rewritten += 1;
            context.progress_counts(
                CoreProgressPhase::Writing,
                "Rewriting ZIP entries",
                rewritten,
                Some(rewrite_total),
            );
            continue;
        }
        output.raw_copy_file(file)?;
        rewritten += 1;
        context.progress_counts(
            CoreProgressPhase::Writing,
            "Rewriting ZIP entries",
            rewritten,
            Some(rewrite_total),
        );
    }
    for entry in additions {
        context.check_cancelled()?;
        write_direct_edit_zip_entry(&mut output, entry, context)?;
        rewritten += 1;
        context.progress_counts(
            CoreProgressPhase::Writing,
            "Rewriting ZIP entries",
            rewritten,
            Some(rewrite_total),
        );
    }
    context.progress_counts(
        CoreProgressPhase::Writing,
        "Rewriting ZIP entries",
        rewrite_total,
        Some(rewrite_total),
    );
    context.check_cancelled()?;
    output.finish()?;
    Ok(())
}

fn write_direct_edit_zip_entry<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    entry: &ArchiveEntry,
    context: &OperationContext,
) -> ArcaResult<()> {
    context.check_cancelled()?;
    match entry.kind {
        EntryKind::Directory => {
            validate_path_snapshot(entry)?;
            zip.add_directory(
                format!("{}/", entry.archive_path),
                zip_options_for_entry(entry, None, &Encryption::None)?,
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
                zip_options_for_entry(entry, None, &Encryption::None)?,
            )?;
        }
        EntryKind::File => {
            let mut input = File::open(&entry.source).map_err(|err| io_at(&entry.source, err))?;
            validate_open_file_snapshot(entry, &input)?;
            zip.start_file(
                &entry.archive_path,
                zip_options_for_entry(entry, None, &Encryption::None)?,
            )?;
            copy_unbounded_with_context(&mut input, zip, &entry.source, &entry.source, context)?;
            validate_open_file_snapshot(entry, &input)?;
        }
    }
    Ok(())
}

fn reject_encrypted_zip<R: Read + Seek>(archive: &mut ZipArchive<R>) -> ArcaResult<()> {
    for index in 0..archive.len() {
        let file = archive.by_index_raw(index)?;
        if file.encrypted() {
            return Err(ArcaError::Unsupported(
                "Direct Editing is disabled for encrypted ZIP archives".into(),
            ));
        }
    }
    Ok(())
}

fn scan_zip<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    archive_path: &Path,
    limits: ResourceLimits,
    context: &OperationContext,
) -> ArcaResult<Vec<ScannedEntry>> {
    context.progress(CoreProgressPhase::Scanning, "Scanning ZIP archive");
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut scanned = Vec::new();
    let mut total = 0;
    limits.check_entry_count(archive.len())?;
    for index in 0..archive.len() {
        context.check_cancelled()?;
        context.progress_counts(
            CoreProgressPhase::Scanning,
            "Scanning ZIP entries",
            index as u64,
            Some(archive.len() as u64),
        );
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
    context: &OperationContext,
) -> ArcaResult<()> {
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    scan_zip(&mut archive, archive_path, limits, context)?;
    let len = archive.len();
    let indexes = (0..len).collect::<Vec<_>>();
    let progress = SharedProgressCounter::new(
        CoreProgressPhase::Testing,
        "Testing ZIP entries",
        len as u64,
    );
    let worker_count = effective_jobs(jobs, len);
    if worker_count <= 1 {
        test_zip_entries(
            archive_path,
            password,
            limits,
            indexes,
            Some(progress.clone()),
            context,
        )?;
        progress.finish(context);
        return Ok(());
    }

    let chunk_size = len.div_ceil(worker_count);
    std::thread::scope(|scope| -> ArcaResult<()> {
        let mut handles = Vec::new();
        for chunk in indexes.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            let context = context.clone();
            let progress = progress.clone();
            handles.push(scope.spawn(move || {
                test_zip_entries(
                    archive_path,
                    password,
                    limits,
                    chunk,
                    Some(progress),
                    &context,
                )
            }));
        }
        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(_) => return Err(ArcaError::Other("zip test worker panicked".into())),
            }
        }
        Ok(())
    })?;
    progress.finish(context);
    Ok(())
}

fn test_zip_selection(
    archive_path: &Path,
    password: Option<&Password>,
    jobs: usize,
    limits: ResourceLimits,
    selection: &ArchiveSelection,
    context: &OperationContext,
) -> ArcaResult<()> {
    let digest = digest_file_with_context(archive_path, context)?;
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    let manifest = scan_zip(&mut archive, archive_path, limits, context)?;
    let selected = selection.select_manifest(&manifest)?;
    let indexes: Vec<_> = selected.iter().map(|(index, _)| *index).collect();
    let progress = SharedProgressCounter::new(
        CoreProgressPhase::Testing,
        "Testing selected ZIP entries",
        indexes.len() as u64,
    );
    let worker_count = effective_jobs(jobs, indexes.len());
    if worker_count <= 1 {
        test_zip_entries(
            archive_path,
            password,
            limits,
            indexes,
            Some(progress.clone()),
            context,
        )?;
        progress.finish(context);
        let digest2 = digest_file_with_context(archive_path, context)?;
        if digest != digest2 {
            return Err(ArcaError::Integrity(
                "archive changed between validation and testing".into(),
            ));
        }
        return Ok(());
    }

    let chunk_size = indexes.len().div_ceil(worker_count);
    std::thread::scope(|scope| -> ArcaResult<()> {
        let mut handles = Vec::new();
        for chunk in indexes.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            let context = context.clone();
            let progress = progress.clone();
            handles.push(scope.spawn(move || {
                test_zip_entries(
                    archive_path,
                    password,
                    limits,
                    chunk,
                    Some(progress),
                    &context,
                )
            }));
        }
        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(_) => return Err(ArcaError::Other("zip test worker panicked".into())),
            }
        }
        Ok(())
    })?;
    progress.finish(context);
    let digest2 = digest_file_with_context(archive_path, context)?;
    if digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and testing".into(),
        ));
    }
    Ok(())
}

fn test_zip_entries<I>(
    archive_path: &Path,
    password: Option<&Password>,
    limits: ResourceLimits,
    entries: I,
    progress: Option<SharedProgressCounter>,
    context: &OperationContext,
) -> ArcaResult<()>
where
    I: IntoIterator<Item = usize>,
{
    let file = File::open(archive_path).map_err(|err| io_at(archive_path, err))?;
    let mut archive = ZipArchive::new(file)?;
    let entries = entries.into_iter().collect::<Vec<_>>();
    for index in entries {
        context.check_cancelled()?;
        let mut file = zip_file_by_index(&mut archive, index, password)?;
        if zip_entry_kind(&file) == EntryKind::Symlink {
            let target = read_symlink_target(&mut file, archive_path, limits)?;
            validate_symlink_target(&target)?;
        } else {
            let expected = file.size();
            let copied = copy_archive_payload_to_output(
                &mut file,
                &mut io::sink(),
                PayloadCopyPlan {
                    archive: archive_path,
                    output: Path::new("<sink>"),
                    max_bytes: expected,
                    progress_phase: CoreProgressPhase::Testing,
                    progress_message: "Testing ZIP entry payload",
                    context,
                },
            )?;
            if copied != expected {
                return Err(ArcaError::Integrity(format!(
                    "archive entry size changed while testing {}",
                    archive_path.display()
                )));
            }
        }
        if let Some(progress) = &progress {
            progress.increment_by(1, context);
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

#[derive(Debug)]
struct ArchiveSelection {
    requested: BTreeSet<String>,
}

impl ArchiveSelection {
    fn new(entries: &[String]) -> ArcaResult<Self> {
        if entries.is_empty() {
            return Err(ArcaError::Usage(
                "at least one archive entry must be selected".into(),
            ));
        }
        let mut requested = BTreeSet::new();
        for entry in entries {
            let normalized = normalize_selection_path(entry)?;
            requested.insert(normalized);
        }
        Ok(Self { requested })
    }

    fn select_manifest<'a>(
        &self,
        manifest: &'a [ScannedEntry],
    ) -> ArcaResult<Vec<(usize, &'a ScannedEntry)>> {
        let mut selected_directories = Vec::new();
        let mut found = BTreeSet::new();
        for entry in manifest {
            let path = scanned_path_str(entry)?;
            if self.requested.contains(path) {
                found.insert(path.to_owned());
                if entry.kind == EntryKind::Directory {
                    selected_directories.push(path.to_owned());
                }
            }
        }

        let missing: Vec<_> = self.requested.difference(&found).cloned().collect();
        if !missing.is_empty() {
            return Err(ArcaError::Usage(format!(
                "selected archive entry not found: {}",
                missing.join(", ")
            )));
        }

        let selected = manifest
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                let Ok(path) = scanned_path_str(entry) else {
                    return false;
                };
                self.requested.contains(path)
                    || selected_directories
                        .iter()
                        .any(|directory| path_is_child_of(path, directory))
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err(ArcaError::Usage(
                "selected archive entries did not match extractable content".into(),
            ));
        }
        Ok(selected)
    }

    fn require_single_stream_match(
        &self,
        archive_path: &Path,
        format: ArchiveFormat,
    ) -> ArcaResult<()> {
        let virtual_path = single_stream_list_path(archive_path, format)?;
        if self.requested.len() == 1 && self.requested.contains(&virtual_path) {
            return Ok(());
        }
        Err(ArcaError::Usage(format!(
            "selected archive entry not found: {}",
            self.requested
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

fn normalize_selection_path(path: &str) -> ArcaResult<String> {
    let normalized = path.trim_end_matches('/').to_owned();
    if normalized.is_empty() {
        return Err(ArcaError::Usage(
            "selected archive entry path must not be empty".into(),
        ));
    }
    validate_archive_path(&normalized)?;
    Ok(normalized)
}

fn scanned_path_str(entry: &ScannedEntry) -> ArcaResult<&str> {
    entry
        .path
        .to_str()
        .ok_or_else(|| ArcaError::NonUtf8Path(entry.path.clone()))
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
    context: &OperationContext,
) -> ArcaResult<()> {
    let (manifest, digest) = scan_compressed_tar(archive_path, codec, limits, context)?;
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
            context,
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
            context,
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
            context,
        )?,
    }
    let digest2 = digest_file_with_context(archive_path, context)?;
    if digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    context.check_cancelled()?;
    context.progress(CoreProgressPhase::Committing, "Publishing extracted files");
    publish_staging(staging.path(), destination, overwrite)?;
    Ok(())
}

fn extract_compressed_tar_selection(
    archive_path: &Path,
    codec: CompressionKind,
    destination: &Path,
    overwrite: bool,
    limits: ResourceLimits,
    selection: &ArchiveSelection,
    context: &OperationContext,
) -> ArcaResult<()> {
    let (manifest, digest) = scan_compressed_tar(archive_path, codec, limits, context)?;
    let selected = selection.select_manifest(&manifest)?;
    ensure_parent(destination)?;
    let staging = TempBuilder::new()
        .prefix(".arca-stage-")
        .tempdir_in(destination.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|err| io_at(destination, err))?;
    match codec {
        CompressionKind::Gzip => extract_tar_with_manifest_selection(
            LimitedReader::new(
                GzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            staging.path(),
            overwrite,
            &manifest,
            &selected,
            archive_path,
            limits,
            context,
        )?,
        CompressionKind::Bzip2 => extract_tar_with_manifest_selection(
            LimitedReader::new(
                BzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            staging.path(),
            overwrite,
            &manifest,
            &selected,
            archive_path,
            limits,
            context,
        )?,
        CompressionKind::Xz => extract_tar_with_manifest_selection(
            LimitedReader::new(
                XzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            staging.path(),
            overwrite,
            &manifest,
            &selected,
            archive_path,
            limits,
            context,
        )?,
    }
    let digest2 = digest_file_with_context(archive_path, context)?;
    if digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    context.check_cancelled()?;
    context.progress(
        CoreProgressPhase::Committing,
        "Publishing selected extracted files",
    );
    publish_staging(staging.path(), destination, overwrite)?;
    Ok(())
}

fn scan_compressed_tar(
    archive_path: &Path,
    codec: CompressionKind,
    limits: ResourceLimits,
    context: &OperationContext,
) -> ArcaResult<(Vec<ScannedEntry>, [u8; 32])> {
    let digest = digest_file_with_context(archive_path, context)?;
    let manifest = match codec {
        CompressionKind::Gzip => pre_scan_tar(
            LimitedReader::new(
                GzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            limits,
            context,
        )?,
        CompressionKind::Bzip2 => pre_scan_tar(
            LimitedReader::new(
                BzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            limits,
            context,
        )?,
        CompressionKind::Xz => pre_scan_tar(
            LimitedReader::new(
                XzDecoder::new(open_reader(archive_path)?),
                limits.max_total_unpacked_bytes,
            ),
            limits,
            context,
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
    context: &OperationContext,
) -> ArcaResult<()> {
    let digest = digest_file_with_context(archive_path, context)?;
    let manifest = pre_scan_tar(&mut file, limits, context)?;
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
        context,
    )?;
    Ok(())
}

fn extract_tar_file_selection(
    archive_path: &Path,
    mut file: File,
    destination: &Path,
    overwrite: bool,
    limits: ResourceLimits,
    selection: &ArchiveSelection,
    context: &OperationContext,
) -> ArcaResult<()> {
    let digest = digest_file_with_context(archive_path, context)?;
    let manifest = pre_scan_tar(&mut file, limits, context)?;
    let selected = selection.select_manifest(&manifest)?;
    file.seek(SeekFrom::Start(0))?;
    ensure_parent(destination)?;
    let staging = TempBuilder::new()
        .prefix(".arca-stage-")
        .tempdir_in(destination.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|err| io_at(destination, err))?;
    extract_tar_with_manifest_selection(
        file,
        staging.path(),
        overwrite,
        &manifest,
        &selected,
        archive_path,
        limits,
        context,
    )?;
    let digest2 = digest_file_with_context(archive_path, context)?;
    if digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    context.check_cancelled()?;
    context.progress(
        CoreProgressPhase::Committing,
        "Publishing selected extracted files",
    );
    publish_staging(staging.path(), destination, overwrite)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn extract_tar_file_checked(
    file: File,
    archive_path: &Path,
    destination: &Path,
    overwrite: bool,
    manifest: &[ScannedEntry],
    digest: [u8; 32],
    limits: ResourceLimits,
    context: &OperationContext,
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
        context,
    )?;
    let digest2 = digest_file_with_context(archive_path, context)?;
    if digest != digest2 {
        return Err(ArcaError::Integrity(
            "archive changed between validation and extraction".into(),
        ));
    }
    context.check_cancelled()?;
    context.progress(CoreProgressPhase::Committing, "Publishing extracted files");
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
    context: &OperationContext,
) -> ArcaResult<()> {
    let expected: Vec<_> = manifest.iter().map(manifest_key).collect();
    let mut seen = Vec::new();
    let mut directories = Vec::new();
    for scanned in manifest {
        context.check_cancelled()?;
        if scanned.kind == EntryKind::Directory {
            let out = destination.join(&scanned.path);
            fs::create_dir_all(&out).map_err(|err| io_at(&out, err))?;
            directories.push(EntryMetadata::from_scanned(&out, scanned, false));
        }
    }

    let mut archive = Archive::new(reader);
    for item in archive.entries().map_err(tar_archive_error)? {
        context.check_cancelled()?;
        let mut entry = item.map_err(tar_archive_error)?;
        let scanned = scan_tar_entry(&entry, limits)?;
        seen.push(manifest_key(&scanned));
        context.progress_counts(
            CoreProgressPhase::Extracting,
            "Extracting tar entries",
            seen.len() as u64,
            Some(expected.len() as u64),
        );
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
                    context,
                )?;
            }
        }
    }
    if seen != expected {
        return Err(ArcaError::Integrity(
            "tar entry manifest changed between validation and extraction".into(),
        ));
    }
    context.progress_counts(
        CoreProgressPhase::Extracting,
        "Extracting tar entries",
        expected.len() as u64,
        Some(expected.len() as u64),
    );
    apply_directory_metadata(directories)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn extract_tar_with_manifest_selection<R: Read>(
    reader: R,
    destination: &Path,
    overwrite: bool,
    manifest: &[ScannedEntry],
    selected: &[(usize, &ScannedEntry)],
    archive_path: &Path,
    limits: ResourceLimits,
    context: &OperationContext,
) -> ArcaResult<()> {
    let expected: Vec<_> = manifest.iter().map(manifest_key).collect();
    let selected_indexes: BTreeSet<_> = selected.iter().map(|(index, _)| *index).collect();
    let mut seen = Vec::new();
    let mut directories = Vec::new();
    let mut selected_processed = 0_u64;
    for (_, scanned) in selected {
        context.check_cancelled()?;
        if scanned.kind == EntryKind::Directory {
            let out = destination.join(&scanned.path);
            fs::create_dir_all(&out).map_err(|err| io_at(&out, err))?;
            directories.push(EntryMetadata::from_scanned(&out, scanned, false));
            selected_processed += 1;
            context.progress_counts(
                CoreProgressPhase::Extracting,
                "Extracting selected tar entries",
                selected_processed,
                Some(selected.len() as u64),
            );
        }
    }

    let mut archive = Archive::new(reader);
    for item in archive.entries().map_err(tar_archive_error)? {
        context.check_cancelled()?;
        let index = seen.len();
        let mut entry = item.map_err(tar_archive_error)?;
        let scanned = scan_tar_entry(&entry, limits)?;
        seen.push(manifest_key(&scanned));
        if !selected_indexes.contains(&index) {
            continue;
        }
        if scanned.kind != EntryKind::Directory {
            selected_processed += 1;
            context.progress_counts(
                CoreProgressPhase::Extracting,
                "Extracting selected tar entries",
                selected_processed,
                Some(selected.len() as u64),
            );
        }
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
                    context,
                )?;
            }
        }
    }
    if seen != expected {
        return Err(ArcaError::Integrity(
            "tar entry manifest changed between validation and extraction".into(),
        ));
    }
    context.progress_counts(
        CoreProgressPhase::Extracting,
        "Extracting selected tar entries",
        selected.len() as u64,
        Some(selected.len() as u64),
    );
    apply_directory_metadata(directories)?;
    Ok(())
}

fn test_tar_with_manifest<R: Read>(
    reader: R,
    archive_path: &Path,
    limits: ResourceLimits,
    manifest: &[ScannedEntry],
    context: &OperationContext,
) -> ArcaResult<()> {
    context.progress(CoreProgressPhase::Testing, "Testing tar archive");
    let expected: Vec<_> = manifest.iter().map(manifest_key).collect();
    let mut archive = Archive::new(reader);
    let mut seen = Vec::new();
    for item in archive.entries().map_err(tar_archive_error)? {
        context.check_cancelled()?;
        let mut entry = item.map_err(tar_archive_error)?;
        let scanned = scan_tar_entry(&entry, limits)?;
        seen.push(manifest_key(&scanned));
        context.progress_counts(
            CoreProgressPhase::Testing,
            "Testing tar entries",
            seen.len() as u64,
            Some(expected.len() as u64),
        );
        if scanned.kind == EntryKind::File {
            let copied = copy_archive_payload_to_output(
                &mut entry,
                &mut io::sink(),
                PayloadCopyPlan {
                    archive: archive_path,
                    output: Path::new("<sink>"),
                    max_bytes: scanned.size,
                    progress_phase: CoreProgressPhase::Testing,
                    progress_message: "Testing tar entry payload",
                    context,
                },
            )?;
            if copied != scanned.size {
                return Err(ArcaError::Integrity(format!(
                    "tar entry size changed while testing {}",
                    archive_path.display()
                )));
            }
        }
    }
    if seen != expected {
        return Err(ArcaError::Integrity(
            "tar entry manifest changed between validation and testing".into(),
        ));
    }
    context.progress_counts(
        CoreProgressPhase::Testing,
        "Testing tar entries",
        expected.len() as u64,
        Some(expected.len() as u64),
    );
    Ok(())
}

fn test_tar_selected_entries<R: Read>(
    reader: R,
    archive_path: &Path,
    limits: ResourceLimits,
    manifest: &[ScannedEntry],
    selected: &[(usize, &ScannedEntry)],
    context: &OperationContext,
) -> ArcaResult<()> {
    let expected: Vec<_> = manifest.iter().map(manifest_key).collect();
    let selected_indexes: BTreeSet<_> = selected.iter().map(|(index, _)| *index).collect();
    let mut archive = Archive::new(reader);
    let mut seen = Vec::new();
    let mut selected_processed = 0_u64;
    for item in archive.entries().map_err(tar_archive_error)? {
        context.check_cancelled()?;
        let index = seen.len();
        let mut entry = item.map_err(tar_archive_error)?;
        let scanned = scan_tar_entry(&entry, limits)?;
        seen.push(manifest_key(&scanned));
        if !selected_indexes.contains(&index) {
            continue;
        }
        selected_processed += 1;
        context.progress_counts(
            CoreProgressPhase::Testing,
            "Testing selected tar entries",
            selected_processed,
            Some(selected.len() as u64),
        );
        if scanned.kind == EntryKind::File {
            let copied = copy_archive_payload_to_output(
                &mut entry,
                &mut io::sink(),
                PayloadCopyPlan {
                    archive: archive_path,
                    output: Path::new("<sink>"),
                    max_bytes: scanned.size,
                    progress_phase: CoreProgressPhase::Testing,
                    progress_message: "Testing selected tar entry payload",
                    context,
                },
            )?;
            if copied != scanned.size {
                return Err(ArcaError::Integrity(format!(
                    "tar entry size changed while testing {}",
                    archive_path.display()
                )));
            }
        }
    }
    if seen != expected {
        return Err(ArcaError::Integrity(
            "tar entry manifest changed between validation and testing".into(),
        ));
    }
    context.progress_counts(
        CoreProgressPhase::Testing,
        "Testing selected tar entries",
        selected.len() as u64,
        Some(selected.len() as u64),
    );
    Ok(())
}

fn pre_scan_tar<R: Read>(
    reader: R,
    limits: ResourceLimits,
    context: &OperationContext,
) -> ArcaResult<Vec<ScannedEntry>> {
    context.progress(CoreProgressPhase::Scanning, "Scanning tar archive");
    let mut archive = Archive::new(reader);
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut scanned = Vec::new();
    let mut total = 0;
    for item in archive.entries().map_err(tar_archive_error)? {
        context.check_cancelled()?;
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

fn list_zip(
    path: &Path,
    limits: ResourceLimits,
    context: &OperationContext,
) -> ArcaResult<Vec<ListEntry>> {
    let mut archive = ZipArchive::new(File::open(path).map_err(|err| io_at(path, err))?)?;
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut entries = Vec::new();
    let mut total = 0;
    limits.check_entry_count(archive.len())?;
    for index in 0..archive.len() {
        context.check_cancelled()?;
        context.progress_counts(
            CoreProgressPhase::Scanning,
            "Listing ZIP entries",
            index as u64,
            Some(archive.len() as u64),
        );
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
    context.progress_counts(
        CoreProgressPhase::Scanning,
        "Listing ZIP entries",
        archive.len() as u64,
        Some(archive.len() as u64),
    );
    let compressed = fs::metadata(path).map_err(|err| io_at(path, err))?.len();
    limits.check_compression_ratio(&path.display().to_string(), total, compressed)?;
    Ok(entries)
}

fn list_tar<R: Read>(
    reader: R,
    limits: ResourceLimits,
    context: &OperationContext,
) -> ArcaResult<Vec<ListEntry>> {
    let mut archive = Archive::new(reader);
    let mut collisions = CollisionSet::default();
    let mut hierarchy = ArchiveHierarchy::default();
    let mut entries = Vec::new();
    let mut total = 0;
    for item in archive.entries().map_err(tar_archive_error)? {
        context.check_cancelled()?;
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

#[allow(clippy::too_many_arguments)]
fn write_entry_file<R: Read>(
    reader: &mut R,
    out: &Path,
    overwrite: bool,
    mode: Option<u32>,
    modified: Option<UnixTime>,
    archive_path: &Path,
    expected_size: u64,
    context: &OperationContext,
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
        let copied = copy_archive_payload_to_output(
            reader,
            &mut writer,
            PayloadCopyPlan {
                archive: archive_path,
                output: out,
                max_bytes: expected_size,
                progress_phase: CoreProgressPhase::Extracting,
                progress_message: "Extracting archive entry payload",
                context,
            },
        )?;
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
    #[cfg(not(unix))]
    let _ = mode;

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

#[cfg(test)]
fn digest_file(path: &Path) -> ArcaResult<[u8; 32]> {
    digest_file_with_context(path, &OperationContext::default())
}

fn digest_file_with_context(path: &Path, context: &OperationContext) -> ArcaResult<[u8; 32]> {
    context.progress(CoreProgressPhase::Reading, "Reading archive digest");
    let mut file = File::open(path).map_err(|err| io_at(path, err))?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    let expected = fs::metadata(path).ok().map(|meta| meta.len());
    loop {
        context.check_cancelled()?;
        let read = file.read(&mut buf).map_err(|err| io_at(path, err))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        total += read as u64;
        context.progress_counts(
            CoreProgressPhase::Reading,
            "Reading archive digest",
            total,
            expected,
        );
    }
    Ok(hasher.finalize().into())
}

fn hex_digest(digest: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn archive_payload_error(path: &Path, error: io::Error) -> ArcaError {
    if error.kind() == io::ErrorKind::Interrupted {
        return ArcaError::Canceled("operation was canceled".into());
    }
    ArcaError::Integrity(format!(
        "failed to read archive payload in {}: {error}",
        path.display()
    ))
}

fn tar_archive_error(error: io::Error) -> ArcaError {
    if error.kind() == io::ErrorKind::Interrupted {
        return ArcaError::Canceled("operation was canceled".into());
    }
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
        let context = OperationContext::default();
        let manifest = pre_scan_tar(&mut scan_file, limits, &context).unwrap();
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
            &context,
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
        let context = OperationContext::default();
        let manifest = scan_zip(&mut zip, &archive, limits, &context).unwrap();
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
            &context,
        )
        .unwrap_err();
        assert!(
            matches!(err, ArcaError::Integrity(_)),
            "expected integrity error, got {err}"
        );
        assert!(!out.exists(), "changed archive should not publish output");
    }

    #[test]
    fn target_lock_rejects_second_holder_until_released() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("archive.zip");

        let first = TargetLock::acquire(&target).unwrap();
        let err = TargetLock::acquire(&target).unwrap_err();
        assert!(
            matches!(err, ArcaError::Busy(_)),
            "expected busy error, got {err}"
        );

        drop(first);
        TargetLock::acquire(&target).unwrap();
    }

    #[test]
    fn cancellation_token_stops_payload_copy() {
        let context = OperationContext::new(CancellationToken::new());
        context.cancellation().cancel();
        let mut reader = io::Cursor::new(vec![1_u8; 1024]);
        let mut writer = Vec::new();
        let err = copy_archive_payload_to_output(
            &mut reader,
            &mut writer,
            PayloadCopyPlan {
                archive: Path::new("archive.zip"),
                output: Path::new("out"),
                max_bytes: 1024,
                progress_phase: CoreProgressPhase::Extracting,
                progress_message: "Extracting archive entry payload",
                context: &context,
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, ArcaError::Canceled(_)),
            "expected canceled error, got {err}"
        );
        assert!(writer.is_empty(), "canceled copy should not write payload");
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
