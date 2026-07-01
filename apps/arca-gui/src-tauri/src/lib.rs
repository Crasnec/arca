use std::path::PathBuf;

use arca_core::{
    ArcaError, ArchiveManifest as CoreArchiveManifest, CompressOptions, DeleteSelectionOptions,
    DirectEditAddPlan as CoreDirectEditAddPlan,
    DirectEditPendingEntry as CoreDirectEditPendingEntry,
    DirectEditPlannedEntry as CoreDirectEditPlannedEntry, DirectEditSaveOptions, Encryption,
    ExitCode, ExtractOptions, ExtractSelectionOptions, Password, PlanDirectEditAddOptions,
    TestOptions, TestSelectionOptions, compress_with_context, delete_selection_with_context,
    extract_selection_with_context, extract_with_context, inspect_archive_with_context,
    plan_direct_edit_add_with_context as core_plan_direct_edit_add_with_context,
    save_direct_edit_with_context as core_save_direct_edit_with_context,
    test_selection_with_context, test_with_context,
};
use serde::{Deserialize, Serialize};

mod file_associations;
mod menus;
mod operations;
mod startup;

use file_associations::{
    archive_format_capabilities, file_association_status, set_all_file_associations,
    set_file_association,
};

use menus::{
    build_native_menu, handle_native_menu_event, set_native_menu_locale, show_entry_context_menu,
};

use operations::{
    OperationRegistry, begin_operation, begin_tracked_operation, cancel_operation, core_context,
    discard_operation, fail_tracked_operation, finish_tracked_operation, handle_app_run_event,
    handle_window_close_requested,
};

use startup::{
    configure_startup_window, handle_single_instance_startup, startup_requests,
    startup_requests_from_args, startup_shell_operation_request,
};

const MAX_GUI_JOBS: usize = 4;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthStatus {
    app: &'static str,
    core_linked: bool,
    native_backend: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandError {
    code: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractResult {
    output_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateResult {
    archive_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractSelectedEntriesRequest {
    archive_path: String,
    output_path: Option<String>,
    entries: Vec<String>,
    password: Option<String>,
    overwrite: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiArchiveManifest {
    archive_path: String,
    archive_name: String,
    format_kind: String,
    format_suffix: String,
    digest_sha256: String,
    entries: Vec<GuiListEntry>,
    entry_count: usize,
    total_uncompressed_size: u64,
    total_compressed_size: Option<u64>,
    encrypted_entry_count: usize,
    validation: GuiArchiveValidation,
    direct_edit: GuiDirectEditStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiListEntry {
    path: String,
    entry_type: String,
    uncompressed_size: u64,
    compressed_size: Option<u64>,
    encrypted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiArchiveValidation {
    metadata_validated: bool,
    payload_validated: bool,
    password_required: bool,
    fully_validated: bool,
    state: &'static str,
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiDirectEditStatus {
    allowed: bool,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiDirectEditAddPlan {
    additions: Vec<GuiDirectEditPlannedEntry>,
    replacements: Vec<GuiDirectEditPlannedEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GuiDirectEditPlannedEntry {
    archive_path: String,
    entry_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveDirectEditRequest {
    archive_path: String,
    expected_digest_sha256: String,
    delete_entries: Vec<String>,
    add_inputs: Vec<String>,
    add_entries: Vec<String>,
    replace_entries: Vec<String>,
}

impl CommandError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: "usage",
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal",
            message: message.into(),
        }
    }

    #[cfg(not(windows))]
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: "unsupported",
            message: message.into(),
        }
    }

    fn interrupted(message: impl Into<String>) -> Self {
        Self {
            code: "interrupted",
            message: message.into(),
        }
    }
}

impl From<ArcaError> for CommandError {
    fn from(error: ArcaError) -> Self {
        let code = match ExitCode::from(&error) {
            ExitCode::Success => "success",
            ExitCode::General => "general",
            ExitCode::Usage => "usage",
            ExitCode::Unsupported => "unsupported",
            ExitCode::Security => "security",
            ExitCode::Password => "password",
            ExitCode::Integrity => "integrity",
            ExitCode::Interrupted => "interrupted",
        };
        Self {
            code,
            message: error.to_string(),
        }
    }
}

impl GuiArchiveManifest {
    fn from_core(manifest: CoreArchiveManifest) -> Self {
        let validation = archive_validation(manifest.encrypted_entry_count);
        Self {
            archive_path: manifest.archive_path.display().to_string(),
            archive_name: manifest.archive_name,
            format_kind: manifest.format_kind,
            format_suffix: manifest.format_suffix,
            digest_sha256: manifest.digest_sha256,
            entries: manifest
                .entries
                .into_iter()
                .map(|entry| GuiListEntry {
                    path: entry.path,
                    entry_type: entry.entry_type,
                    uncompressed_size: entry.uncompressed_size,
                    compressed_size: entry.compressed_size,
                    encrypted: entry.encrypted,
                })
                .collect(),
            entry_count: manifest.entry_count,
            total_uncompressed_size: manifest.total_uncompressed_size,
            total_compressed_size: manifest.total_compressed_size,
            encrypted_entry_count: manifest.encrypted_entry_count,
            validation,
            direct_edit: GuiDirectEditStatus {
                allowed: manifest.direct_edit.allowed,
                reason: manifest.direct_edit.reason,
            },
        }
    }
}

impl From<CoreDirectEditAddPlan> for GuiDirectEditAddPlan {
    fn from(plan: CoreDirectEditAddPlan) -> Self {
        Self {
            additions: plan.additions.into_iter().map(Into::into).collect(),
            replacements: plan.replacements.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CoreDirectEditPlannedEntry> for GuiDirectEditPlannedEntry {
    fn from(entry: CoreDirectEditPlannedEntry) -> Self {
        Self {
            archive_path: entry.archive_path,
            entry_type: entry.entry_type,
        }
    }
}

impl From<GuiDirectEditPlannedEntry> for CoreDirectEditPendingEntry {
    fn from(entry: GuiDirectEditPlannedEntry) -> Self {
        Self {
            archive_path: entry.archive_path,
            entry_type: entry.entry_type,
        }
    }
}

fn archive_validation(encrypted_entry_count: usize) -> GuiArchiveValidation {
    if encrypted_entry_count > 0 {
        return GuiArchiveValidation {
            metadata_validated: true,
            payload_validated: false,
            password_required: true,
            fully_validated: false,
            state: "metadataOnlyPasswordRequired",
            reason: "Archive metadata is validated, but encrypted payloads require a password before Test or Extract can fully validate them.".into(),
        };
    }

    GuiArchiveValidation {
        metadata_validated: true,
        payload_validated: false,
        password_required: false,
        fully_validated: false,
        state: "metadataOnly",
        reason: "Archive metadata is validated. Run Test to validate payload contents.".into(),
    }
}

#[tauri::command]
fn health() -> HealthStatus {
    HealthStatus {
        app: "arca-gui",
        core_linked: true,
        native_backend: arca_native::native_backend_enabled(),
    }
}

#[tauri::command]
fn close_current_window(window: tauri::Window) -> Result<(), CommandError> {
    window
        .close()
        .map_err(|error| CommandError::internal(format!("window close failed: {error}")))
}

#[tauri::command]
async fn list_archive(
    archive_path: String,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<GuiArchiveManifest, CommandError> {
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let archive_path = PathBuf::from(archive_path);
    if archive_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive path is required",
        );
    }

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        inspect_archive_with_context(archive_path, context)
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map(GuiArchiveManifest::from_core)
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

#[tauri::command]
async fn test_archive(
    archive_path: String,
    password: Option<String>,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<(), CommandError> {
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let archive_path = PathBuf::from(archive_path);
    if archive_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive path is required",
        );
    }
    let password = password_from_string(password);

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        test_with_context(
            TestOptions {
                archive: archive_path,
                jobs: default_jobs(),
                password,
            },
            context,
        )
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

#[tauri::command]
async fn test_selected_entries(
    archive_path: String,
    entries: Vec<String>,
    password: Option<String>,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<(), CommandError> {
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let archive_path = PathBuf::from(archive_path);
    if archive_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive path is required",
        );
    }
    if entries.is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "at least one archive entry is required",
        );
    }
    let password = password_from_string(password);

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        test_selection_with_context(
            TestSelectionOptions {
                archive: archive_path,
                jobs: default_jobs(),
                password,
                entries,
            },
            context,
        )
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

#[tauri::command]
async fn extract_archive(
    archive_path: String,
    output_path: Option<String>,
    password: Option<String>,
    overwrite: bool,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<ExtractResult, CommandError> {
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let archive_path = PathBuf::from(archive_path);
    if archive_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive path is required",
        );
    }
    let output_path = output_path
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from);
    let password = password_from_string(password);

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        extract_with_context(
            ExtractOptions {
                archive: archive_path,
                output: output_path,
                overwrite,
                jobs: default_jobs(),
                password,
            },
            context,
        )
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map(|output| ExtractResult {
        output_path: output.display().to_string(),
    })
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

#[tauri::command]
async fn extract_selected_entries(
    request: ExtractSelectedEntriesRequest,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<ExtractResult, CommandError> {
    let ExtractSelectedEntriesRequest {
        archive_path,
        output_path,
        entries,
        password,
        overwrite,
    } = request;
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let archive_path = PathBuf::from(archive_path);
    if archive_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive path is required",
        );
    }
    if entries.is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "at least one archive entry is required",
        );
    }
    let output_path = output_path
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from);
    let password = password_from_string(password);

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        extract_selection_with_context(
            ExtractSelectionOptions {
                archive: archive_path,
                output: output_path,
                overwrite,
                jobs: default_jobs(),
                password,
                entries,
            },
            context,
        )
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map(|output| ExtractResult {
        output_path: output.display().to_string(),
    })
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

#[tauri::command]
async fn create_archive(
    output_path: String,
    inputs: Vec<String>,
    password: Option<String>,
    overwrite: bool,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<CreateResult, CommandError> {
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let output_path = PathBuf::from(output_path);
    if output_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive output path is required",
        );
    }
    if inputs.is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "at least one input file or folder is required",
        );
    }
    let inputs = inputs.into_iter().map(PathBuf::from).collect();
    let encryption = match password_from_string(password) {
        Some(password) => Encryption::Aes256(password),
        None => Encryption::None,
    };

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        compress_with_context(
            CompressOptions {
                inputs,
                output: Some(output_path),
                overwrite,
                level: None,
                jobs: default_jobs(),
                excludes: Vec::new(),
                encryption,
                auto_tar: false,
            },
            context,
        )
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map(|output| CreateResult {
        archive_path: output.display().to_string(),
    })
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

#[tauri::command]
async fn delete_selected_entries(
    archive_path: String,
    expected_digest_sha256: String,
    entries: Vec<String>,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<GuiArchiveManifest, CommandError> {
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let archive_path = PathBuf::from(archive_path);
    if archive_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive path is required",
        );
    }
    if expected_digest_sha256.trim().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive digest is required",
        );
    }
    if entries.is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "at least one archive entry is required",
        );
    }

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        delete_selection_with_context(
            DeleteSelectionOptions {
                archive: archive_path,
                expected_digest_sha256,
                entries,
            },
            context,
        )
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map(GuiArchiveManifest::from_core)
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

#[tauri::command]
async fn plan_direct_edit_add(
    archive_path: String,
    inputs: Vec<String>,
    pending_delete_entries: Vec<String>,
    pending_add_entries: Vec<GuiDirectEditPlannedEntry>,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<GuiDirectEditAddPlan, CommandError> {
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let archive_path = PathBuf::from(archive_path);
    if archive_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive path is required",
        );
    }
    if inputs.is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "at least one input file or folder is required",
        );
    }
    let inputs = inputs.into_iter().map(PathBuf::from).collect();
    let pending_add_entries = pending_add_entries.into_iter().map(Into::into).collect();

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        core_plan_direct_edit_add_with_context(
            PlanDirectEditAddOptions {
                archive: archive_path,
                inputs,
                pending_delete_entries,
                pending_add_entries,
            },
            context,
        )
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map(GuiDirectEditAddPlan::from)
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

#[tauri::command]
async fn save_direct_edit(
    request: SaveDirectEditRequest,
    operation_id: Option<u64>,
    app: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
) -> Result<GuiArchiveManifest, CommandError> {
    let SaveDirectEditRequest {
        archive_path,
        expected_digest_sha256,
        delete_entries,
        add_inputs,
        add_entries,
        replace_entries,
    } = request;
    let operation = begin_tracked_operation(&app, operations.inner(), operation_id)?;
    let archive_path = PathBuf::from(archive_path);
    if archive_path.as_os_str().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive path is required",
        );
    }
    if expected_digest_sha256.trim().is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "archive digest is required",
        );
    }
    if delete_entries.is_empty() && add_entries.is_empty() {
        return fail_tracked_operation(
            &app,
            operations.inner(),
            operation.as_ref(),
            "there are no pending changes to save",
        );
    }
    let add_inputs = add_inputs.into_iter().map(PathBuf::from).collect();

    let context = core_context(&app, operation.as_ref());
    let result = tauri::async_runtime::spawn_blocking(move || {
        core_save_direct_edit_with_context(
            DirectEditSaveOptions {
                archive: archive_path,
                expected_digest_sha256,
                delete_entries,
                add_inputs,
                add_entries,
                replace_entries,
            },
            context,
        )
    })
    .await
    .map_err(|error| CommandError::internal(format!("archive worker failed: {error}")))?
    .map(GuiArchiveManifest::from_core)
    .map_err(CommandError::from);
    finish_tracked_operation(&app, operations.inner(), operation.as_ref(), &result)?;
    result
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAX_GUI_JOBS)
}

fn password_from_string(password: Option<String>) -> Option<Password> {
    password.map(|password| Password::new(password.into_bytes()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().manage(OperationRegistry::default());
    #[cfg(desktop)]
    let builder = {
        let requests = startup_requests_from_args(std::env::args_os());
        if startup_shell_operation_request(&requests).is_some() {
            builder
        } else {
            builder
                .menu(build_native_menu)
                .on_menu_event(handle_native_menu_event)
        }
    };
    let app = builder
        .on_window_event(handle_window_close_requested)
        .plugin(tauri_plugin_single_instance::init(
            handle_single_instance_startup,
        ))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(desktop)]
            configure_startup_window(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health,
            startup_requests,
            close_current_window,
            begin_operation,
            discard_operation,
            cancel_operation,
            archive_format_capabilities,
            file_association_status,
            set_file_association,
            set_all_file_associations,
            #[cfg(desktop)]
            show_entry_context_menu,
            #[cfg(desktop)]
            set_native_menu_locale,
            list_archive,
            test_archive,
            test_selected_entries,
            extract_archive,
            extract_selected_entries,
            create_archive,
            delete_selected_entries,
            plan_direct_edit_add,
            save_direct_edit
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Arca GUI");
    app.run(handle_app_run_event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_validation_serializes_camel_case_password_state() {
        let value = serde_json::to_value(archive_validation(2)).unwrap();

        assert_eq!(value["metadataValidated"], true);
        assert_eq!(value["payloadValidated"], false);
        assert_eq!(value["passwordRequired"], true);
        assert_eq!(value["fullyValidated"], false);
        assert_eq!(value["state"], "metadataOnlyPasswordRequired");
        assert!(value.get("metadata_validated").is_none());
        assert!(value.get("password_required").is_none());
    }

    #[test]
    fn gui_validation_non_encrypted_archive_is_metadata_only() {
        let value = serde_json::to_value(archive_validation(0)).unwrap();

        assert_eq!(value["metadataValidated"], true);
        assert_eq!(value["passwordRequired"], false);
        assert_eq!(value["fullyValidated"], false);
        assert_eq!(value["state"], "metadataOnly");
    }
}
