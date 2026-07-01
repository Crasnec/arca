use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use arca_core::format;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, menu::MenuBuilder};

const STARTUP_REQUESTS_EVENT: &str = "arca-startup-requests";
static SHELL_WINDOW_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum StartupAction {
    Open,
    Test,
    Extract,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartupRequest {
    action: StartupAction,
    archive_path: String,
}

pub(crate) fn handle_single_instance_startup<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    argv: Vec<String>,
    cwd: String,
) {
    let cwd = if cwd.is_empty() {
        None
    } else {
        Some(PathBuf::from(cwd))
    };
    let requests = startup_requests_from_args_in_cwd(argv.into_iter().map(OsString::from), cwd);
    if requests.is_empty() {
        return;
    }
    if let Some(request) = startup_shell_operation_request(&requests).cloned() {
        let app_handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let _ = open_shell_operation_window(&app_handle, &request);
        });
        return;
    }

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit(STARTUP_REQUESTS_EVENT, requests);
}

#[cfg(desktop)]
fn open_shell_operation_window<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    request: &StartupRequest,
) -> tauri::Result<()> {
    let size = (480.0, 190.0);
    let label = format!(
        "shell-operation-{}",
        SHELL_WINDOW_ID.fetch_add(1, Ordering::Relaxed)
    );
    let empty_menu = MenuBuilder::new(app).build()?;
    let window = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App(shell_operation_window_url(request).into()),
    )
    .title("Arca")
    .inner_size(size.0, size.1)
    .min_inner_size(size.0, size.1)
    .max_inner_size(size.0, size.1)
    .resizable(false)
    .focused(true)
    .center()
    .menu(empty_menu)
    .build()?;
    let _ = window.set_focus();
    Ok(())
}

fn shell_operation_window_url(request: &StartupRequest) -> String {
    format!(
        "index.html?shellAction={}&archivePath={}",
        shell_operation_action_query(request.action),
        utf8_percent_encode(&request.archive_path, NON_ALPHANUMERIC)
    )
}

fn shell_operation_action_query(action: StartupAction) -> &'static str {
    match action {
        StartupAction::Extract => "extract",
        StartupAction::Test => "test",
        StartupAction::Open => "open",
    }
}

pub(crate) fn startup_shell_operation_request(
    requests: &[StartupRequest],
) -> Option<&StartupRequest> {
    if requests.len() != 1 {
        return None;
    }
    let request = &requests[0];
    match request.action {
        StartupAction::Extract | StartupAction::Test => Some(request),
        StartupAction::Open => None,
    }
}

#[cfg(desktop)]
pub(crate) fn configure_startup_window<R: tauri::Runtime>(
    app: &tauri::App<R>,
) -> tauri::Result<()> {
    let requests = startup_requests_from_args(std::env::args_os());
    if startup_shell_operation_request(&requests).is_none() {
        return Ok(());
    }

    if let Some(window) = app.get_webview_window("main") {
        let size = tauri::LogicalSize::new(480.0, 190.0);
        window.set_title("Arca")?;
        window.set_size(size)?;
        window.set_min_size(Some(size))?;
        window.set_max_size(Some(size))?;
        window.set_resizable(false)?;
        let _ = window.center();
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn startup_requests() -> Vec<StartupRequest> {
    startup_requests_from_args(std::env::args_os())
}

pub(crate) fn startup_requests_from_args(
    args: impl IntoIterator<Item = OsString>,
) -> Vec<StartupRequest> {
    startup_requests_from_args_in_cwd(args, None)
}

fn startup_requests_from_args_in_cwd(
    args: impl IntoIterator<Item = OsString>,
    cwd: Option<PathBuf>,
) -> Vec<StartupRequest> {
    let mut requests = Vec::new();
    let mut args = args.into_iter().skip(1);
    let cwd = cwd.as_deref();

    while let Some(arg) = args.next() {
        if let Some(action) = startup_action_flag(&arg) {
            if let Some(path) = args.next()
                && let Some(request) = startup_request(action, startup_path_from_arg(path, cwd))
            {
                requests.push(request);
            }
            continue;
        }

        if let Some(request) = startup_request(StartupAction::Open, startup_path_from_arg(arg, cwd))
        {
            requests.push(request);
        }
    }

    requests
}

fn startup_path_from_arg(path: OsString, cwd: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_relative()
        && let Some(cwd) = cwd
    {
        return cwd.join(path);
    }
    path
}

fn startup_action_flag(flag: &OsString) -> Option<StartupAction> {
    match flag.to_str()? {
        "--arca-shell-open" => Some(StartupAction::Open),
        "--arca-shell-test" => Some(StartupAction::Test),
        "--arca-shell-extract" => Some(StartupAction::Extract),
        _ => None,
    }
}

fn startup_request(action: StartupAction, path: PathBuf) -> Option<StartupRequest> {
    if !path.is_file() || !matches!(format::detect_format_with_signature(&path), Ok(Some(_))) {
        return None;
    }

    Some(StartupRequest {
        action,
        archive_path: path.to_str()?.to_owned(),
    })
}
