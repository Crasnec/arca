use std::collections::HashSet;

use unicode_normalization::UnicodeNormalization;

use crate::error::{ArcaError, ArcaResult};

const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Default, Clone)]
pub struct CollisionSet {
    seen: HashSet<String>,
}

impl CollisionSet {
    pub fn insert_archive_path(&mut self, path: &str) -> ArcaResult<()> {
        let key = collision_key(path)?;
        if !self.seen.insert(key) {
            return Err(ArcaError::Security(format!(
                "archive path collision for {path}"
            )));
        }
        Ok(())
    }
}

pub fn validate_archive_path(path: &str) -> ArcaResult<()> {
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        return Err(ArcaError::Security("empty archive path".into()));
    }
    if path.starts_with('/') || path.starts_with('\\') || has_windows_drive_prefix(path) {
        return Err(ArcaError::Security(format!(
            "absolute archive path is not allowed: {path}"
        )));
    }
    for component in path.split('/') {
        validate_component(component)?;
    }
    Ok(())
}

pub fn validate_symlink_target(target: &str) -> ArcaResult<()> {
    if target.is_empty() {
        return Err(ArcaError::Security("empty symlink target".into()));
    }
    if target.starts_with('/') || target.starts_with('\\') || has_windows_drive_prefix(target) {
        return Err(ArcaError::Security(format!(
            "absolute symlink target is not allowed: {target}"
        )));
    }
    for component in target.split('/') {
        validate_component(component)?;
        if component == ".." {
            return Err(ArcaError::Security(format!(
                "escaping symlink target is not allowed: {target}"
            )));
        }
    }
    Ok(())
}

pub fn collision_key(path: &str) -> ArcaResult<String> {
    validate_archive_path(path)?;
    let mut parts = Vec::new();
    for component in path.trim_end_matches('/').split('/') {
        let trimmed = component.trim_end_matches([' ', '.']);
        let normalized: String = trimmed.nfc().collect::<String>().to_lowercase();
        parts.push(normalized);
    }
    Ok(parts.join("/"))
}

fn validate_component(component: &str) -> ArcaResult<()> {
    if component.is_empty() || component == "." || component == ".." {
        return Err(ArcaError::Security(format!(
            "invalid archive path component: {component:?}"
        )));
    }
    if component.contains('\\') {
        return Err(ArcaError::Security(format!(
            "backslash is not allowed in archive path component: {component}"
        )));
    }
    if component.contains(':') {
        return Err(ArcaError::Security(format!(
            "colon/ADS path component is not allowed: {component}"
        )));
    }
    if component.ends_with(' ') || component.ends_with('.') {
        return Err(ArcaError::Security(format!(
            "trailing space/dot path component is not allowed: {component}"
        )));
    }
    if component.chars().any(|ch| ch == '\0' || ch.is_control()) {
        return Err(ArcaError::Security(format!(
            "control character is not allowed in archive path component: {component:?}"
        )));
    }
    let base = component
        .split_once('.')
        .map_or(component, |(base, _)| base)
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    if WINDOWS_RESERVED.contains(&base.as_str()) {
        return Err(ArcaError::Security(format!(
            "Windows reserved path component is not allowed: {component}"
        )));
    }
    Ok(())
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let mut chars = path.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_reserved_with_extension() {
        assert!(validate_archive_path("CON.txt").is_err());
    }

    #[test]
    fn rejects_ads_colon() {
        assert!(validate_archive_path("a:b").is_err());
    }

    #[test]
    fn rejects_traversal() {
        assert!(validate_archive_path("../x").is_err());
    }

    #[test]
    fn rejects_common_cross_platform_filename_bypasses() {
        for path in [
            "/absolute.txt",
            "\\absolute.txt",
            "C:/absolute.txt",
            "dir\\file.txt",
            "file:name.txt",
            "name.",
            "name ",
            "line\nbreak.txt",
            "./file.txt",
            "dir//file.txt",
        ] {
            assert!(validate_archive_path(path).is_err(), "accepted {path:?}");
        }
    }

    #[test]
    fn collision_key_catches_case() {
        let mut set = CollisionSet::default();
        set.insert_archive_path("Readme.txt").unwrap();
        assert!(set.insert_archive_path("README.TXT").is_err());
    }

    #[test]
    fn collision_key_catches_unicode_normalization() {
        let mut set = CollisionSet::default();
        set.insert_archive_path("Cafe\u{301}.txt").unwrap();
        assert!(set.insert_archive_path("Caf\u{e9}.txt").is_err());
    }
}
