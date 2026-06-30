pub mod error;
pub mod format;
pub mod ops;
pub mod plan;
pub mod policy;

pub use error::{ArcaError, ArcaResult, ExitCode};
pub use format::{ArchiveFormat, CompressionKind, FormatKind};
pub use ops::{
    CompressOptions, Encryption, ExtractOptions, Password, TestOptions, compress, extract, list,
    test,
};
pub use plan::{ArchiveEntry, EntryKind, plan_entries};
