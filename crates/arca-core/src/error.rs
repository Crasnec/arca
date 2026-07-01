use std::path::PathBuf;

use thiserror::Error;

pub type ArcaResult<T> = Result<T, ArcaError>;

#[derive(Debug, Error)]
pub enum ArcaError {
    #[error("usage error: {0}")]
    Usage(String),

    #[error("unsupported format or feature: {0}")]
    Unsupported(String),

    #[error("security policy violation: {0}")]
    Security(String),

    #[error("password or encryption error: {0}")]
    Password(String),

    #[error("corrupt archive or integrity failure: {0}")]
    Integrity(String),

    #[error("operation already in progress: {0}")]
    Busy(String),

    #[error("operation canceled: {0}")]
    Canceled(String),

    #[error("path is not valid UTF-8: {0}")]
    NonUtf8Path(PathBuf),

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("I/O error: {0}")]
    IoPlain(#[from] std::io::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("glob error: {0}")]
    Glob(#[from] globset::Error),

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    General = 1,
    Usage = 2,
    Unsupported = 3,
    Security = 4,
    Password = 5,
    Integrity = 6,
    Interrupted = 130,
}

impl From<&ArcaError> for ExitCode {
    fn from(value: &ArcaError) -> Self {
        match value {
            ArcaError::Usage(_) => ExitCode::Usage,
            ArcaError::Unsupported(_) => ExitCode::Unsupported,
            ArcaError::Security(_) | ArcaError::NonUtf8Path(_) => ExitCode::Security,
            ArcaError::Password(_) => ExitCode::Password,
            ArcaError::Integrity(_) => ExitCode::Integrity,
            ArcaError::Busy(_) | ArcaError::Canceled(_) => ExitCode::Interrupted,
            ArcaError::Zip(zip::result::ZipError::InvalidPassword) => ExitCode::Password,
            ArcaError::Zip(zip::result::ZipError::UnsupportedArchive(msg))
                if *msg == zip::result::ZipError::PASSWORD_REQUIRED =>
            {
                ExitCode::Password
            }
            ArcaError::Zip(_) => ExitCode::Integrity,
            ArcaError::Io { .. }
            | ArcaError::IoPlain(_)
            | ArcaError::Glob(_)
            | ArcaError::Other(_) => ExitCode::General,
        }
    }
}

pub(crate) fn io_at(path: impl Into<PathBuf>, source: std::io::Error) -> ArcaError {
    ArcaError::Io {
        path: path.into(),
        source,
    }
}
