#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use std::ffi::OsString;

use arca_core::format;
use serde::Serialize;

use crate::CommandError;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileAssociationStatus {
    supported: bool,
    entries: Vec<FileAssociationEntry>,
    message: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileAssociationEntry {
    extension: String,
    enabled: bool,
    default_handler: bool,
    registered_handler: bool,
    open_with_handler: bool,
    open_command: bool,
    extract_command: bool,
    test_command: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuiArchiveFormatCapability {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    suffixes: Vec<&'static str>,
    create_suffixes: Vec<&'static str>,
    extensions: Vec<&'static str>,
    mime_type: &'static str,
    supports_create: bool,
    supports_extract: bool,
    supports_test: bool,
    supports_direct_edit: bool,
    signatures: Vec<GuiFormatSignature>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiFormatSignature {
    offset: u64,
    bytes_hex: String,
}

#[tauri::command]
pub(crate) fn file_association_status() -> Result<FileAssociationStatus, CommandError> {
    native_file_association_status()
}

#[tauri::command]
pub(crate) fn archive_format_capabilities() -> Vec<GuiArchiveFormatCapability> {
    format::archive_formats()
        .iter()
        .map(gui_archive_format_capability)
        .collect()
}

#[tauri::command]
pub(crate) fn set_file_association(
    extension: String,
    enabled: bool,
) -> Result<FileAssociationEntry, CommandError> {
    let extension = normalized_file_association_extension(&extension)?;
    native_set_file_association(&extension, enabled)
}

#[tauri::command]
pub(crate) fn set_all_file_associations(
    enabled: bool,
) -> Result<FileAssociationStatus, CommandError> {
    native_set_all_file_associations(enabled)
}

fn gui_archive_format_capability(
    descriptor: &format::ArchiveFormatDescriptor,
) -> GuiArchiveFormatCapability {
    GuiArchiveFormatCapability {
        id: descriptor.id,
        name: descriptor.name,
        description: descriptor.description,
        suffixes: descriptor.suffixes.to_vec(),
        create_suffixes: descriptor.create_suffixes.to_vec(),
        extensions: descriptor.extensions.to_vec(),
        mime_type: descriptor.mime_type,
        supports_create: descriptor.supports_create(),
        supports_extract: true,
        supports_test: true,
        supports_direct_edit: descriptor.supports_direct_edit(),
        signatures: descriptor
            .signatures
            .iter()
            .map(|signature| GuiFormatSignature {
                offset: signature.offset,
                bytes_hex: bytes_hex(signature.bytes),
            })
            .collect(),
    }
}

fn bytes_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn file_association_extensions() -> impl Iterator<Item = &'static str> {
    format::archive_formats()
        .iter()
        .flat_map(|descriptor| descriptor.extensions.iter().copied())
}

fn normalized_file_association_extension(extension: &str) -> Result<String, CommandError> {
    let normalized = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if format::descriptor_for_extension(&normalized).is_some() {
        return Ok(normalized);
    }
    Err(CommandError::usage(format!(
        "unsupported archive file extension: {extension}"
    )))
}

#[cfg(target_os = "linux")]
const LINUX_DESKTOP_ID: &str = "com.crasnec.arca.desktop";

#[cfg(target_os = "linux")]
const LINUX_DESKTOP_ID_CANDIDATES: &[&str] = &[
    LINUX_DESKTOP_ID,
    "arca-gui.desktop",
    "Arca.desktop",
    "arca.desktop",
];

#[cfg(target_os = "linux")]
fn native_file_association_status() -> Result<FileAssociationStatus, CommandError> {
    if linux_xdg_config_home().is_err() || linux_xdg_data_home().is_err() {
        return Ok(linux_unsupported_file_association_status(
            "File associations require XDG user directories on Linux.",
        ));
    }

    Ok(FileAssociationStatus {
        supported: true,
        entries: file_association_extensions()
            .map(linux_file_association_entry)
            .collect::<Result<Vec<_>, _>>()?,
        message: None,
    })
}

#[cfg(target_os = "linux")]
fn native_set_file_association(
    extension: &str,
    enabled: bool,
) -> Result<FileAssociationEntry, CommandError> {
    if enabled {
        linux_ensure_desktop_entry()?;
    }
    linux_set_mime_association(extension, enabled)?;
    linux_file_association_entry(extension)
}

#[cfg(target_os = "linux")]
fn native_set_all_file_associations(enabled: bool) -> Result<FileAssociationStatus, CommandError> {
    if enabled {
        linux_ensure_desktop_entry()?;
    }
    for extension in file_association_extensions() {
        linux_set_mime_association(extension, enabled)?;
    }
    native_file_association_status()
}

#[cfg(target_os = "linux")]
fn linux_unsupported_file_association_status(message: &'static str) -> FileAssociationStatus {
    FileAssociationStatus {
        supported: false,
        entries: file_association_extensions()
            .map(|extension| FileAssociationEntry {
                extension: extension.to_owned(),
                enabled: false,
                default_handler: false,
                registered_handler: false,
                open_with_handler: false,
                open_command: false,
                extract_command: false,
                test_command: false,
            })
            .collect(),
        message: Some(message),
    }
}

#[cfg(target_os = "linux")]
fn linux_file_association_entry(extension: &str) -> Result<FileAssociationEntry, CommandError> {
    let mime_types = linux_mime_types_for_extension(extension)?;
    let default_handler = linux_mimeapps_has_default_handler(&mime_types)?;
    let registered_handler = linux_desktop_entry_registered()?;
    let open_with_handler = linux_mimeapps_has_open_with_handler(&mime_types)?;
    let open_command = linux_desktop_entry_has_open_command()?;
    Ok(FileAssociationEntry {
        extension: extension.to_owned(),
        enabled: default_handler && registered_handler && open_with_handler && open_command,
        default_handler,
        registered_handler,
        open_with_handler,
        open_command,
        extract_command: registered_handler,
        test_command: registered_handler,
    })
}

#[cfg(target_os = "linux")]
fn linux_set_mime_association(extension: &str, enabled: bool) -> Result<(), CommandError> {
    let mime_types = linux_mime_types_for_extension(extension)?;
    for path in linux_mimeapps_paths()? {
        let original = linux_read_text_file(&path)?.unwrap_or_default();
        let mut next = original.clone();
        for mime_type in &mime_types {
            if enabled {
                next = mimeapps_prepend_desktop_id(
                    &next,
                    "Default Applications",
                    mime_type,
                    LINUX_DESKTOP_ID,
                );
                next = mimeapps_prepend_desktop_id(
                    &next,
                    "Added Associations",
                    mime_type,
                    LINUX_DESKTOP_ID,
                );
            } else {
                next = mimeapps_remove_desktop_ids(
                    &next,
                    "Default Applications",
                    mime_type,
                    LINUX_DESKTOP_ID_CANDIDATES,
                );
                next = mimeapps_remove_desktop_ids(
                    &next,
                    "Added Associations",
                    mime_type,
                    LINUX_DESKTOP_ID_CANDIDATES,
                );
            }
        }
        if next != original {
            linux_write_text_file(&path, &next)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_mimeapps_has_default_handler(mime_types: &[&'static str]) -> Result<bool, CommandError> {
    linux_mimeapps_mime_types_match(mime_types, "Default Applications", |ids| {
        ids.first()
            .map(|desktop_id| linux_is_arca_desktop_id(desktop_id))
            .unwrap_or(false)
    })
}

#[cfg(target_os = "linux")]
fn linux_mimeapps_has_open_with_handler(mime_types: &[&'static str]) -> Result<bool, CommandError> {
    linux_mimeapps_mime_types_match(mime_types, "Added Associations", |ids| {
        ids.iter()
            .any(|desktop_id| linux_is_arca_desktop_id(desktop_id))
    })
}

#[cfg(target_os = "linux")]
fn linux_mimeapps_mime_types_match(
    mime_types: &[&'static str],
    section: &str,
    predicate: impl Fn(&[String]) -> bool,
) -> Result<bool, CommandError> {
    let paths = linux_mimeapps_paths()?;
    for mime_type in mime_types {
        let mut mime_type_matches = false;
        for path in &paths {
            let Some(source) = linux_read_text_file(path)? else {
                continue;
            };
            let Some(value) = mimeapps_value(&source, section, mime_type) else {
                continue;
            };
            let desktop_ids = split_desktop_ids(&value);
            if predicate(&desktop_ids) {
                mime_type_matches = true;
                break;
            }
        }
        if !mime_type_matches {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(target_os = "linux")]
fn linux_mime_types_for_extension(extension: &str) -> Result<Vec<&'static str>, CommandError> {
    let descriptor = format::descriptor_for_extension(extension).ok_or_else(|| {
        CommandError::usage(format!("unsupported archive file extension: {extension}"))
    })?;
    let mut mime_types = Vec::new();
    push_unique_mime_type(&mut mime_types, descriptor.mime_type);
    for alias in linux_mime_aliases_for_extension(extension) {
        push_unique_mime_type(&mut mime_types, alias);
    }
    Ok(mime_types)
}

#[cfg(target_os = "linux")]
fn linux_mime_aliases_for_extension(extension: &str) -> &'static [&'static str] {
    match extension {
        "zip" => &["application/x-zip-compressed"],
        "gz" => &["application/x-gzip"],
        "bz2" => &["application/x-bzip"],
        "tbz2" => &["application/x-bzip2-compressed-tar"],
        "tgz" => &["application/x-gtar-compressed"],
        _ => &[],
    }
}

#[cfg(target_os = "linux")]
fn push_unique_mime_type(mime_types: &mut Vec<&'static str>, mime_type: &'static str) {
    if !mime_types.contains(&mime_type) {
        mime_types.push(mime_type);
    }
}

#[cfg(target_os = "linux")]
fn linux_ensure_desktop_entry() -> Result<(), CommandError> {
    let path = linux_user_desktop_entry_path()?;
    let exe = linux_current_exe_path()?;
    let contents = linux_desktop_entry_contents(&exe)?;
    linux_write_text_file(&path, &contents)
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_registered() -> Result<bool, CommandError> {
    for path in linux_desktop_entry_candidate_paths()? {
        if path.is_file() {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_has_open_command() -> Result<bool, CommandError> {
    for path in linux_desktop_entry_candidate_paths()? {
        let Some(contents) = linux_read_text_file(&path)? else {
            continue;
        };
        if contents.contains("Exec=") && contents.contains("--arca-shell-open") {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_contents(exe: &Path) -> Result<String, CommandError> {
    let exec_path = linux_desktop_exec_path(exe)?;
    let mut mime_types = Vec::new();
    for extension in file_association_extensions() {
        for mime_type in linux_mime_types_for_extension(extension)? {
            push_unique_mime_type(&mut mime_types, mime_type);
        }
    }
    Ok(format!(
        concat!(
            "[Desktop Entry]\n",
            "Version=1.0\n",
            "Type=Application\n",
            "Name=Arca\n",
            "GenericName=Archive Manager\n",
            "Comment=Open and manage archive files\n",
            "Exec={exec_path} --arca-shell-open %f\n",
            "Icon=arca-gui\n",
            "Terminal=false\n",
            "Categories=Utility;Archiving;\n",
            "MimeType={mime_types};\n",
            "NoDisplay=false\n",
            "StartupNotify=true\n"
        ),
        exec_path = exec_path,
        mime_types = mime_types.join(";")
    ))
}

#[cfg(target_os = "linux")]
fn linux_desktop_exec_path(path: &Path) -> Result<String, CommandError> {
    let raw = path
        .to_str()
        .ok_or_else(|| CommandError::internal("current executable path is not valid UTF-8"))?;
    let escaped = raw
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
    Ok(format!("\"{escaped}\""))
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry_candidate_paths() -> Result<Vec<PathBuf>, CommandError> {
    let mut paths = Vec::new();
    let mut dirs = vec![linux_xdg_data_home()?.join("applications")];
    dirs.extend(
        linux_xdg_data_dirs()
            .into_iter()
            .map(|path| path.join("applications")),
    );
    for dir in dirs {
        for desktop_id in LINUX_DESKTOP_ID_CANDIDATES {
            paths.push(dir.join(desktop_id));
        }
    }
    Ok(paths)
}

#[cfg(target_os = "linux")]
fn linux_user_desktop_entry_path() -> Result<PathBuf, CommandError> {
    Ok(linux_xdg_data_home()?
        .join("applications")
        .join(LINUX_DESKTOP_ID))
}

#[cfg(target_os = "linux")]
fn linux_mimeapps_paths() -> Result<Vec<PathBuf>, CommandError> {
    Ok(vec![
        linux_xdg_config_home()?.join("mimeapps.list"),
        linux_xdg_config_home()?.join("kde-mimeapps.list"),
        linux_xdg_data_home()?
            .join("applications")
            .join("mimeapps.list"),
    ])
}

#[cfg(target_os = "linux")]
fn linux_xdg_config_home() -> Result<PathBuf, CommandError> {
    linux_xdg_home("XDG_CONFIG_HOME", ".config")
}

#[cfg(target_os = "linux")]
fn linux_xdg_data_home() -> Result<PathBuf, CommandError> {
    linux_xdg_home("XDG_DATA_HOME", ".local/share")
}

#[cfg(target_os = "linux")]
fn linux_xdg_home(variable: &str, fallback: &str) -> Result<PathBuf, CommandError> {
    if let Some(value) = std::env::var_os(variable).filter(|value| !value.as_os_str().is_empty()) {
        return Ok(PathBuf::from(value));
    }
    let home = std::env::var_os("HOME")
        .filter(|value| !value.as_os_str().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| CommandError::unsupported("Linux XDG user directory is not available"))?;
    Ok(home.join(fallback))
}

#[cfg(target_os = "linux")]
fn linux_xdg_data_dirs() -> Vec<PathBuf> {
    let value = std::env::var_os("XDG_DATA_DIRS")
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| OsString::from("/usr/local/share:/usr/share"));
    std::env::split_paths(&value)
        .filter(|path| !path.as_os_str().is_empty())
        .collect()
}

#[cfg(target_os = "linux")]
fn linux_current_exe_path() -> Result<PathBuf, CommandError> {
    std::env::current_exe().map_err(|error| {
        CommandError::internal(format!("Linux desktop entry path failed: {error}"))
    })
}

#[cfg(target_os = "linux")]
fn linux_read_text_file(path: &Path) -> Result<Option<String>, CommandError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(CommandError::internal(format!(
            "Linux MIME association read failed for {}: {error}",
            path.display()
        ))),
    }
}

#[cfg(target_os = "linux")]
fn linux_write_text_file(path: &Path, contents: &str) -> Result<(), CommandError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            CommandError::internal(format!(
                "Linux MIME association directory update failed for {}: {error}",
                parent.display()
            ))
        })?;
    }
    std::fs::write(path, contents).map_err(|error| {
        CommandError::internal(format!(
            "Linux MIME association write failed for {}: {error}",
            path.display()
        ))
    })
}

#[cfg(target_os = "linux")]
fn linux_is_arca_desktop_id(desktop_id: &str) -> bool {
    LINUX_DESKTOP_ID_CANDIDATES.contains(&desktop_id)
}

#[cfg(target_os = "linux")]
fn mimeapps_prepend_desktop_id(source: &str, section: &str, key: &str, desktop_id: &str) -> String {
    let current = mimeapps_value(source, section, key).unwrap_or_default();
    let mut ids = vec![desktop_id.to_owned()];
    for current_id in split_desktop_ids(&current) {
        if current_id != desktop_id {
            ids.push(current_id);
        }
    }
    mimeapps_set_value(source, section, key, &join_desktop_ids(&ids))
}

#[cfg(target_os = "linux")]
fn mimeapps_remove_desktop_ids(
    source: &str,
    section: &str,
    key: &str,
    desktop_ids: &[&str],
) -> String {
    let Some(current) = mimeapps_value(source, section, key) else {
        return source.to_owned();
    };
    let remaining = split_desktop_ids(&current)
        .into_iter()
        .filter(|desktop_id| !desktop_ids.contains(&desktop_id.as_str()))
        .collect::<Vec<_>>();
    if remaining.is_empty() {
        mimeapps_remove_key(source, section, key)
    } else {
        mimeapps_set_value(source, section, key, &join_desktop_ids(&remaining))
    }
}

#[cfg(target_os = "linux")]
fn mimeapps_value(source: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in source.lines() {
        if let Some(header) = mimeapps_section(line) {
            in_section = header == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((line_key, value)) = mimeapps_key_value(line) else {
            continue;
        };
        if line_key == key {
            return Some(value.trim().to_owned());
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn mimeapps_set_value(source: &str, section: &str, key: &str, value: &str) -> String {
    let mut lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
    let mut in_section = false;
    let mut insert_index = None;
    for index in 0..lines.len() {
        if let Some(header) = mimeapps_section(&lines[index]) {
            if in_section {
                insert_index = Some(index);
                break;
            }
            in_section = header == section;
            continue;
        }
        if !in_section {
            continue;
        }
        if mimeapps_line_key(&lines[index])
            .map(|line_key| line_key == key)
            .unwrap_or(false)
        {
            lines[index] = format!("{key}={value}");
            return lines_to_mimeapps(lines);
        }
    }

    if in_section {
        lines.insert(
            insert_index.unwrap_or(lines.len()),
            format!("{key}={value}"),
        );
        return lines_to_mimeapps(lines);
    }

    if !lines.is_empty() && lines.last().map(|line| !line.is_empty()).unwrap_or(false) {
        lines.push(String::new());
    }
    lines.push(format!("[{section}]"));
    lines.push(format!("{key}={value}"));
    lines_to_mimeapps(lines)
}

#[cfg(target_os = "linux")]
fn mimeapps_remove_key(source: &str, section: &str, key: &str) -> String {
    let mut lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
    let mut in_section = false;
    for index in 0..lines.len() {
        if let Some(header) = mimeapps_section(&lines[index]) {
            if in_section {
                break;
            }
            in_section = header == section;
            continue;
        }
        if !in_section {
            continue;
        }
        if mimeapps_line_key(&lines[index])
            .map(|line_key| line_key == key)
            .unwrap_or(false)
        {
            lines.remove(index);
            break;
        }
    }
    lines_to_mimeapps(lines)
}

#[cfg(target_os = "linux")]
fn mimeapps_section(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .map(str::trim)
}

#[cfg(target_os = "linux")]
fn mimeapps_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with('[') {
        return None;
    }
    let (key, value) = trimmed.split_once('=')?;
    Some((key.trim(), value))
}

#[cfg(target_os = "linux")]
fn mimeapps_line_key(line: &str) -> Option<&str> {
    mimeapps_key_value(line).map(|(key, _)| key)
}

#[cfg(target_os = "linux")]
fn split_desktop_ids(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|desktop_id| !desktop_id.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(target_os = "linux")]
fn join_desktop_ids(desktop_ids: &[String]) -> String {
    let mut value = desktop_ids.join(";");
    if !value.is_empty() {
        value.push(';');
    }
    value
}

#[cfg(target_os = "linux")]
fn lines_to_mimeapps(lines: Vec<String>) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

#[cfg(all(not(windows), not(target_os = "linux")))]
fn native_file_association_status() -> Result<FileAssociationStatus, CommandError> {
    Ok(FileAssociationStatus {
        supported: false,
        entries: file_association_extensions()
            .map(|extension| FileAssociationEntry {
                extension: extension.to_owned(),
                enabled: false,
                default_handler: false,
                registered_handler: false,
                open_with_handler: false,
                open_command: false,
                extract_command: false,
                test_command: false,
            })
            .collect(),
        message: Some("File association settings are available on Windows and Linux."),
    })
}

#[cfg(all(not(windows), not(target_os = "linux")))]
fn native_set_file_association(
    _extension: &str,
    _enabled: bool,
) -> Result<FileAssociationEntry, CommandError> {
    Err(CommandError::unsupported(
        "file association settings are available on Windows and Linux",
    ))
}

#[cfg(all(not(windows), not(target_os = "linux")))]
fn native_set_all_file_associations(_enabled: bool) -> Result<FileAssociationStatus, CommandError> {
    Err(CommandError::unsupported(
        "file association settings are available on Windows and Linux",
    ))
}

#[cfg(windows)]
fn native_file_association_status() -> Result<FileAssociationStatus, CommandError> {
    Ok(FileAssociationStatus {
        supported: true,
        entries: file_association_extensions()
            .map(windows_file_association_entry)
            .collect::<Result<Vec<_>, _>>()?,
        message: None,
    })
}

#[cfg(windows)]
fn native_set_file_association(
    extension: &str,
    enabled: bool,
) -> Result<FileAssociationEntry, CommandError> {
    if enabled {
        windows_register_file_association(extension)?;
    } else {
        windows_unregister_file_association(extension)?;
    }
    windows_notify_association_changed();
    windows_file_association_entry(extension)
}

#[cfg(windows)]
fn native_set_all_file_associations(enabled: bool) -> Result<FileAssociationStatus, CommandError> {
    for extension in file_association_extensions() {
        if enabled {
            windows_register_file_association(extension)?;
        } else {
            windows_unregister_file_association(extension)?;
        }
    }
    windows_notify_association_changed();
    native_file_association_status()
}

#[cfg(windows)]
fn windows_file_association_entry(extension: &str) -> Result<FileAssociationEntry, CommandError> {
    let default_handler = windows_default_handler_enabled(extension)?;
    let registered_handler = windows_prog_id_registered(extension)?;
    let open_with_handler = windows_open_with_handler_enabled(extension)?;
    let open_command = windows_shell_action_registered(extension, "ArcaOpen")?;
    let extract_command = windows_shell_action_registered(extension, "ArcaExtract")?;
    let test_command = windows_shell_action_registered(extension, "ArcaTest")?;
    Ok(FileAssociationEntry {
        extension: extension.to_owned(),
        enabled: default_handler
            && registered_handler
            && open_with_handler
            && open_command
            && extract_command
            && test_command,
        default_handler,
        registered_handler,
        open_with_handler,
        open_command,
        extract_command,
        test_command,
    })
}

#[cfg(windows)]
fn windows_register_file_association(extension: &str) -> Result<(), CommandError> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let descriptor = format::descriptor_for_extension(extension).ok_or_else(|| {
        CommandError::usage(format!("unsupported archive file extension: {extension}"))
    })?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let prog_id = windows_prog_id(extension);
    let exe = current_exe_string()?;

    let extension_key_path = format!(r"Software\Classes\.{extension}");
    let (extension_key, _) = hkcu
        .create_subkey(extension_key_path.as_str())
        .map_err(registry_error)?;
    extension_key
        .set_value("", &prog_id)
        .map_err(registry_error)?;

    let open_with_key_path = format!(r"Software\Classes\.{extension}\OpenWithProgids");
    let (open_with_key, _) = hkcu
        .create_subkey(open_with_key_path.as_str())
        .map_err(registry_error)?;
    open_with_key
        .set_value(prog_id.as_str(), &"")
        .map_err(registry_error)?;

    let prog_key_path = format!(r"Software\Classes\{prog_id}");
    let (prog_key, _) = hkcu
        .create_subkey(prog_key_path.as_str())
        .map_err(registry_error)?;
    prog_key
        .set_value("", &format!("Arca {}", descriptor.name))
        .map_err(registry_error)?;
    let icon_key_path = format!(r"Software\Classes\{prog_id}\DefaultIcon");
    let (icon_key, _) = hkcu
        .create_subkey(icon_key_path.as_str())
        .map_err(registry_error)?;
    icon_key
        .set_value("", &format!("{exe},0"))
        .map_err(registry_error)?;
    let open_command_key_path = format!(r"Software\Classes\{prog_id}\shell\open\command");
    let (open_command_key, _) = hkcu
        .create_subkey(open_command_key_path.as_str())
        .map_err(registry_error)?;
    open_command_key
        .set_value("", &windows_command(&exe, "--arca-shell-open"))
        .map_err(registry_error)?;

    windows_register_shell_action(
        extension,
        "ArcaOpen",
        "Open in Arca",
        "--arca-shell-open",
        &exe,
    )?;
    windows_register_shell_action(
        extension,
        "ArcaExtract",
        "Extract with Arca",
        "--arca-shell-extract",
        &exe,
    )?;
    windows_register_shell_action(
        extension,
        "ArcaTest",
        "Test with Arca",
        "--arca-shell-test",
        &exe,
    )?;
    Ok(())
}

#[cfg(windows)]
fn windows_unregister_file_association(extension: &str) -> Result<(), CommandError> {
    use std::io::ErrorKind;
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let prog_id = windows_prog_id(extension);

    let extension_key_path = format!(r"Software\Classes\.{extension}");
    if let Ok(extension_key) = hkcu.open_subkey_with_flags(
        extension_key_path.as_str(),
        winreg::enums::KEY_READ | winreg::enums::KEY_WRITE,
    ) {
        if extension_key
            .get_value::<String, _>("")
            .map(|value| value == prog_id)
            .unwrap_or(false)
        {
            let _ = extension_key.delete_value("");
        }
    }

    let open_with_key_path = format!(r"Software\Classes\.{extension}\OpenWithProgids");
    if let Ok(open_with_key) =
        hkcu.open_subkey_with_flags(open_with_key_path.as_str(), winreg::enums::KEY_WRITE)
    {
        let _ = open_with_key.delete_value(prog_id.as_str());
    }

    for key in [
        format!(r"Software\Classes\{prog_id}"),
        windows_shell_action_key(extension, "ArcaOpen"),
        windows_shell_action_key(extension, "ArcaExtract"),
        windows_shell_action_key(extension, "ArcaTest"),
    ] {
        match hkcu.delete_subkey_all(key.as_str()) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(registry_error(error)),
        }
    }
    Ok(())
}

#[cfg(windows)]
fn windows_register_shell_action(
    extension: &str,
    key: &str,
    label: &str,
    flag: &str,
    exe: &str,
) -> Result<(), CommandError> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let action_key_path = windows_shell_action_key(extension, key);
    let (action_key, _) = hkcu
        .create_subkey(action_key_path.as_str())
        .map_err(registry_error)?;
    action_key.set_value("", &label).map_err(registry_error)?;
    action_key
        .set_value("Icon", &format!("{exe},0"))
        .map_err(registry_error)?;
    let command_key_path = format!(r"{action_key_path}\command");
    let (command_key, _) = hkcu
        .create_subkey(command_key_path.as_str())
        .map_err(registry_error)?;
    command_key
        .set_value("", &windows_command(exe, flag))
        .map_err(registry_error)
}

#[cfg(windows)]
fn windows_default_handler_enabled(extension: &str) -> Result<bool, CommandError> {
    windows_registry_value(format!(r"Software\Classes\.{extension}"), "")
        .map(|value| value == windows_prog_id(extension))
}

#[cfg(windows)]
fn windows_prog_id_registered(extension: &str) -> Result<bool, CommandError> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    Ok(RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(format!(r"Software\Classes\{}", windows_prog_id(extension)).as_str())
        .is_ok())
}

#[cfg(windows)]
fn windows_open_with_handler_enabled(extension: &str) -> Result<bool, CommandError> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let key_path = format!(r"Software\Classes\.{extension}\OpenWithProgids");
    let prog_id = windows_prog_id(extension);
    Ok(RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(key_path.as_str())
        .and_then(|key| key.get_value::<String, _>(prog_id.as_str()))
        .is_ok())
}

#[cfg(windows)]
fn windows_shell_action_registered(extension: &str, key: &str) -> Result<bool, CommandError> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    Ok(RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(windows_shell_action_key(extension, key).as_str())
        .is_ok())
}

#[cfg(windows)]
fn windows_registry_value(path: String, name: &str) -> Result<String, CommandError> {
    use std::io::ErrorKind;
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    match RegKey::predef(HKEY_CURRENT_USER).open_subkey(path.as_str()) {
        Ok(key) => match key.get_value(name) {
            Ok(value) => Ok(value),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(String::new()),
            Err(error) => Err(registry_error(error)),
        },
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(registry_error(error)),
    }
}

#[cfg(windows)]
fn windows_prog_id(extension: &str) -> String {
    format!("Arca.{extension}")
}

#[cfg(windows)]
fn windows_shell_action_key(extension: &str, key: &str) -> String {
    format!(r"Software\Classes\SystemFileAssociations\.{extension}\shell\{key}")
}

#[cfg(windows)]
fn windows_command(exe: &str, flag: &str) -> String {
    format!(r#""{exe}" {flag} "%1""#)
}

#[cfg(windows)]
fn current_exe_string() -> Result<String, CommandError> {
    std::env::current_exe()
        .map_err(registry_error)?
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| CommandError::internal("current executable path is not valid UTF-8"))
}

#[cfg(windows)]
fn registry_error(error: std::io::Error) -> CommandError {
    CommandError::internal(format!("Windows registry operation failed: {error}"))
}

#[cfg(windows)]
fn windows_notify_association_changed() {
    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}
