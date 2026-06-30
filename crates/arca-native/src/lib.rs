//! Optional native backend integration boundary for Arca.
//!
//! The first implementation uses pure Rust archive backends. This crate exists so
//! FFI-based replacements can be added without leaking native concerns into
//! `arca-core`.

/// Returns whether a native backend is currently compiled in.
#[must_use]
pub const fn native_backend_enabled() -> bool {
    false
}
