use serde::Deserialize;
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder},
};

use crate::CommandError;

const MENU_ACTION_EVENT: &str = "arca-menu-action";

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextMenuPosition {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EntryContextMenuState {
    has_selection: bool,
    can_add_direct_edit: bool,
    can_delete: bool,
    can_open_properties: bool,
    loading: bool,
}

fn normalized_locale(locale: &str) -> &'static str {
    if locale.to_ascii_lowercase().starts_with("ko") {
        "ko"
    } else {
        "en"
    }
}

fn menu_label<'a>(locale: &str, key: &'a str) -> &'a str {
    match (normalized_locale(locale), key) {
        ("ko", "file") => "파일",
        ("ko", "open") => "열기...",
        ("ko", "new_archive") => "새 아카이브...",
        ("ko", "quit") => "끝내기",
        ("ko", "edit") => "편집",
        ("ko", "copy") => "복사",
        ("ko", "delete") => "삭제",
        ("ko", "undo") => "실행 취소",
        ("ko", "redo") => "다시 실행",
        ("ko", "archive") => "아카이브",
        ("ko", "add_files") => "파일 추가...",
        ("ko", "extract") => "풀기",
        ("ko", "test") => "검사",
        ("ko", "info") => "정보",
        ("ko", "view") => "보기",
        ("ko", "find") => "찾기",
        ("ko", "settings") => "설정...",
        ("ko", "help") => "도움말",
        ("ko", "archive_info") => "아카이브 정보",
        ("ko", "extract_selection") => "풀기...",
        ("ko", "extract_here") => "여기에 풀기",
        ("ko", "add_folder") => "폴더 추가...",
        ("ko", "copy_path") => "경로 복사",
        ("ko", "properties") => "속성",
        (_, "file") => "File",
        (_, "open") => "Open...",
        (_, "new_archive") => "New Archive...",
        (_, "quit") => "Quit",
        (_, "edit") => "Edit",
        (_, "copy") => "Copy",
        (_, "delete") => "Delete",
        (_, "undo") => "Undo",
        (_, "redo") => "Redo",
        (_, "archive") => "Archive",
        (_, "add_files") => "Add Files...",
        (_, "extract") => "Extract",
        (_, "test") => "Test",
        (_, "info") => "Info",
        (_, "view") => "View",
        (_, "find") => "Find",
        (_, "settings") => "Settings...",
        (_, "help") => "Help",
        (_, "archive_info") => "Archive Info",
        (_, "extract_selection") => "Extract...",
        (_, "extract_here") => "Extract Here",
        (_, "add_folder") => "Add folder...",
        (_, "copy_path") => "Copy path",
        (_, "properties") => "Properties",
        _ => key,
    }
}

#[cfg(desktop)]
#[tauri::command]
pub(crate) fn show_entry_context_menu<R: tauri::Runtime>(
    window: tauri::WebviewWindow<R>,
    locale: String,
    position: ContextMenuPosition,
    state: EntryContextMenuState,
) -> Result<(), CommandError> {
    let menu = build_entry_context_menu(window.app_handle(), &state, &locale)
        .map_err(|error| CommandError::internal(error.to_string()))?;
    let position = tauri::LogicalPosition::new(position.x.max(0.0), position.y.max(0.0));
    window
        .popup_menu_at(&menu, position)
        .map_err(|error| CommandError::internal(error.to_string()))
}

#[cfg(desktop)]
#[tauri::command]
pub(crate) fn set_native_menu_locale<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    locale: String,
) -> Result<(), CommandError> {
    let menu = build_native_menu_for_locale(&app, &locale)
        .map_err(|error| CommandError::internal(error.to_string()))?;
    app.set_menu(menu)
        .map(|_| ())
        .map_err(|error| CommandError::internal(error.to_string()))
}

#[cfg(desktop)]
pub(crate) fn build_native_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> tauri::Result<Menu<R>> {
    build_native_menu_for_locale(app, "en")
}

#[cfg(desktop)]
fn build_native_menu_for_locale<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    locale: &str,
) -> tauri::Result<Menu<R>> {
    let file = SubmenuBuilder::new(app, menu_label(locale, "file"))
        .text("arca-menu-open", menu_label(locale, "open"))
        .text("arca-menu-new", menu_label(locale, "new_archive"))
        .separator()
        .text("arca-menu-quit", menu_label(locale, "quit"))
        .build()?;
    let edit = SubmenuBuilder::new(app, menu_label(locale, "edit"))
        .text("arca-menu-copy", menu_label(locale, "copy"))
        .text("arca-menu-delete", menu_label(locale, "delete"))
        .separator()
        .text("arca-menu-undo", menu_label(locale, "undo"))
        .text("arca-menu-redo", menu_label(locale, "redo"))
        .build()?;
    let archive = SubmenuBuilder::new(app, menu_label(locale, "archive"))
        .text("arca-menu-add-files", menu_label(locale, "add_files"))
        .text("arca-menu-extract", menu_label(locale, "extract"))
        .text("arca-menu-test", menu_label(locale, "test"))
        .text("arca-menu-info", menu_label(locale, "info"))
        .build()?;
    let view = SubmenuBuilder::new(app, menu_label(locale, "view"))
        .text("arca-menu-find", menu_label(locale, "find"))
        .text("arca-menu-settings", menu_label(locale, "settings"))
        .build()?;
    let help = SubmenuBuilder::new(app, menu_label(locale, "help"))
        .text("arca-menu-info", menu_label(locale, "archive_info"))
        .build()?;

    MenuBuilder::new(app)
        .item(&file)
        .item(&edit)
        .item(&archive)
        .item(&view)
        .item(&help)
        .build()
}

#[cfg(desktop)]
fn build_entry_context_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &EntryContextMenuState,
    locale: &str,
) -> tauri::Result<Menu<R>> {
    let can_use_selection = state.has_selection && !state.loading;
    let can_add = state.can_add_direct_edit && !state.loading;
    let can_delete = state.can_delete && state.has_selection && !state.loading;

    let extract = MenuItemBuilder::with_id(
        "arca-context-extract-selection",
        menu_label(locale, "extract_selection"),
    )
    .enabled(can_use_selection)
    .build(app)?;
    let extract_here = MenuItemBuilder::with_id(
        "arca-context-extract-here",
        menu_label(locale, "extract_here"),
    )
    .enabled(can_use_selection)
    .build(app)?;
    let test = MenuItemBuilder::with_id("arca-context-test-selection", menu_label(locale, "test"))
        .enabled(can_use_selection)
        .build(app)?;
    let add_files =
        MenuItemBuilder::with_id("arca-context-add-files", menu_label(locale, "add_files"))
            .enabled(can_add)
            .build(app)?;
    let add_folder =
        MenuItemBuilder::with_id("arca-context-add-folder", menu_label(locale, "add_folder"))
            .enabled(can_add)
            .build(app)?;
    let copy_path =
        MenuItemBuilder::with_id("arca-context-copy-path", menu_label(locale, "copy_path"))
            .enabled(can_use_selection)
            .build(app)?;
    let delete = MenuItemBuilder::with_id("arca-context-delete", menu_label(locale, "delete"))
        .enabled(can_delete)
        .build(app)?;
    let properties =
        MenuItemBuilder::with_id("arca-context-properties", menu_label(locale, "properties"))
            .enabled(state.can_open_properties && state.has_selection)
            .build(app)?;

    MenuBuilder::new(app)
        .item(&extract)
        .item(&extract_here)
        .item(&test)
        .separator()
        .item(&add_files)
        .item(&add_folder)
        .separator()
        .item(&copy_path)
        .item(&delete)
        .separator()
        .item(&properties)
        .build()
}

#[cfg(desktop)]
pub(crate) fn handle_native_menu_event<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    event: MenuEvent,
) {
    let menu_id = event.id().as_ref();
    if menu_id == "arca-menu-quit" {
        app.exit(0);
        return;
    }
    if menu_id.starts_with("arca-menu-") || menu_id.starts_with("arca-context-") {
        let _ = app.emit(MENU_ACTION_EVENT, menu_id);
    }
}
