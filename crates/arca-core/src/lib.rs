pub mod error;
pub mod format;
pub mod ops;
pub mod plan;
pub mod policy;

pub use error::{ArcaError, ArcaResult, ExitCode};
pub use format::{
    ArchiveFormat, ArchiveFormatDescriptor, CompressionKind, FormatKind, FormatSignature,
    archive_file_extensions, archive_formats,
};
pub use ops::{
    ArchiveManifest, CancellationToken, CompressOptions, CoreProgress, CoreProgressPhase,
    DeleteSelectionOptions, DirectEditAddPlan, DirectEditPendingEntry, DirectEditPlannedEntry,
    DirectEditSaveOptions, DirectEditStatus, Encryption, ExtractOptions, ExtractSelectionOptions,
    OperationContext, Password, PlanDirectEditAddOptions, ProgressSink, TestOptions,
    TestSelectionOptions, compress, compress_with_context, delete_selection,
    delete_selection_with_context, extract, extract_selection, extract_selection_with_context,
    extract_with_context, inspect_archive, inspect_archive_with_context, list, list_with_context,
    plan_direct_edit_add, plan_direct_edit_add_with_context, save_direct_edit,
    save_direct_edit_with_context, test, test_selection, test_selection_with_context,
    test_with_context,
};
pub use plan::{ArchiveEntry, EntryKind, plan_entries};
