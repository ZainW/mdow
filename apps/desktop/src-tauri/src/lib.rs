mod fs_tree;
mod models;
mod store;
mod watchers;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Emitter, Manager, State, Window};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::fs_tree::scan_folder;
use crate::models::{FileResult, OpenFolderResult, ScanResult};
use crate::store::AppStore;
use crate::watchers::WatcherHub;

fn map_read_error(err: std::io::Error) -> String {
    match err.kind() {
        ErrorKind::NotFound => "not-found".into(),
        ErrorKind::PermissionDenied => "permission-denied".into(),
        _ => "read-error".into(),
    }
}

fn read_utf8_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(map_read_error)
}

fn resolve_is_dark(theme: &str) -> bool {
    match theme {
        "dark" => true,
        "light" => false,
        _ => matches!(dark_light::detect(), Ok(dark_light::Mode::Dark)),
    }
}

fn emit_theme_changed(app: &AppHandle, theme: &str) {
    let _ = app.emit("theme:changed", resolve_is_dark(theme));
}

#[tauri::command]
fn platform() -> String {
    match std::env::consts::OS {
        "macos" => "darwin".into(),
        "windows" => "win32".into(),
        "linux" => "linux".into(),
        "freebsd" => "freebsd".into(),
        "openbsd" => "openbsd".into(),
        "netbsd" => "netbsd".into(),
        "android" => "android".into(),
        "ios" => "darwin".into(),
        other => other.into(),
    }
}

#[tauri::command]
fn open_file_dialog(
    app: AppHandle,
    store: State<'_, AppStore>,
    watchers: State<'_, WatcherHub>,
) -> Result<Option<FileResult>, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("Markdown", &["md", "markdown", "mdx"])
        .blocking_pick_file();

    let Some(path) = picked else {
        return Ok(None);
    };

    let path_buf = path.into_path().map_err(|err| err.to_string())?;
    let path_str = path_buf.to_string_lossy().into_owned();
    let content = read_utf8_file(Path::new(&path_str))?;
    store.add_recent(&path_str);
    watchers.watch_file(app, PathBuf::from(&path_str));

    Ok(Some(FileResult {
        path: path_str,
        content,
    }))
}

#[tauri::command]
fn read_file(
    app: AppHandle,
    store: State<'_, AppStore>,
    watchers: State<'_, WatcherHub>,
    path: String,
) -> Result<String, String> {
    let content = read_utf8_file(Path::new(&path))?;
    store.add_recent(&path);
    watchers.watch_file(app, PathBuf::from(path));
    Ok(content)
}

#[tauri::command]
fn unwatch_file(watchers: State<'_, WatcherHub>, path: String) {
    watchers.unwatch_file(&path);
}

#[tauri::command]
fn open_folder_dialog(
    app: AppHandle,
    store: State<'_, AppStore>,
    watchers: State<'_, WatcherHub>,
) -> Result<Option<OpenFolderResult>, String> {
    let picked = app.dialog().file().blocking_pick_folder();

    let Some(folder_path) = picked else {
        return Ok(None);
    };

    let folder_buf = folder_path.into_path().map_err(|err| err.to_string())?;
    let path_str = folder_buf.to_string_lossy().into_owned();
    let scan = scan_folder(Path::new(&path_str)).map_err(|err| err.to_string())?;
    store.set_last_folder(Some(&path_str));
    watchers.watch_folder(app, PathBuf::from(&path_str));

    Ok(Some(OpenFolderResult {
        path: path_str,
        tree: scan.tree,
        truncated: scan.truncated,
    }))
}

#[tauri::command]
fn read_folder_tree(
    app: AppHandle,
    store: State<'_, AppStore>,
    watchers: State<'_, WatcherHub>,
    folder_path: String,
) -> Result<ScanResult, String> {
    let scan = scan_folder(Path::new(&folder_path)).map_err(|err| err.to_string())?;
    store.set_last_folder(Some(&folder_path));
    watchers.watch_folder(app, PathBuf::from(folder_path));
    Ok(scan)
}

#[tauri::command]
fn store_get_recents(store: State<'_, AppStore>) -> Vec<String> {
    store.get_recents()
}

#[tauri::command]
fn store_get_state(store: State<'_, AppStore>) -> models::AppState {
    store.get_app_state()
}

#[tauri::command]
fn store_save_state(store: State<'_, AppStore>, patch: serde_json::Value) -> Result<(), String> {
    store.save_app_state(patch).map_err(|err| err.to_string())
}

#[tauri::command]
fn store_add_recent(store: State<'_, AppStore>, file_path: String) {
    store.add_recent(&file_path);
}

#[tauri::command]
fn show_in_folder(app: AppHandle, file_path: String) -> Result<(), String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err("not-found".into());
    }

    #[cfg(target_os = "linux")]
    {
        let parent = path.parent().ok_or_else(|| "read-error".to_string())?;
        app.opener()
            .open_path(parent.to_string_lossy(), None::<&str>)
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        app.opener()
            .reveal_item_in_dir(&file_path)
            .map_err(|err| err.to_string())
    }
}

#[tauri::command]
fn set_theme(app: AppHandle, store: State<'_, AppStore>, theme: String) -> Result<(), String> {
    if !matches!(theme.as_str(), "light" | "dark" | "system") {
        return Ok(());
    }

    store.set_theme(&theme);
    emit_theme_changed(&app, &theme);
    Ok(())
}

#[tauri::command]
async fn set_window_title(window: Window, title: String) {
    let _ = window.set_title(&title);
}

#[tauri::command]
async fn close_window(window: Window) {
    let _ = window.close();
}

#[tauri::command]
async fn check_for_updates() -> Result<(), ()> {
    Ok(())
}

#[tauri::command]
async fn download_update() -> Result<(), ()> {
    Ok(())
}

#[tauri::command]
async fn install_update() -> Result<(), ()> {
    Ok(())
}

#[tauri::command]
async fn set_auto_update_scheduling(_enabled: bool) -> Result<(), ()> {
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let store = AppStore::load(app.handle())?;
            let theme = store.get_app_state().theme;
            app.manage(store);
            app.manage(WatcherHub::default());
            emit_theme_changed(app.handle(), &theme);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            platform,
            open_file_dialog,
            read_file,
            unwatch_file,
            open_folder_dialog,
            read_folder_tree,
            store_get_recents,
            store_get_state,
            store_save_state,
            store_add_recent,
            show_in_folder,
            set_theme,
            set_window_title,
            close_window,
            check_for_updates,
            download_update,
            install_update,
            set_auto_update_scheduling,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
