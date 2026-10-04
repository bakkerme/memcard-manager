mod card;
mod usb;

use card::{
    configure_directory, configured_directory, read_library, sync_saves, LibraryView, SyncResult,
};

use card::{
    backup_bytes, backup_dir, backup_filename, backup_stem, compose_new_card, display_path,
    local_timestamp, write_backup, CardFormat, CardSource, CardView, Ps1Card,
};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use usb::HardwareStatus;

struct AppState {
    card: Mutex<Option<Ps1Card>>,
    usb: Mutex<Option<rusb::Context>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportResult {
    bytes: Vec<u8>,
    filename: String,
    view: Option<CardView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupResult {
    path: String,
    display_path: String,
    filename: String,
}

fn with_card<T>(
    state: &AppState,
    f: impl FnOnce(&Ps1Card) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state
        .card
        .lock()
        .map_err(|_| "Card lock poisoned.".to_string())?;
    let card = guard
        .as_ref()
        .ok_or_else(|| "No card is open.".to_string())?;
    f(card)
}

fn usb_context<'a>(slot: &'a mut Option<rusb::Context>) -> Result<&'a rusb::Context, String> {
    if slot.is_none() {
        *slot = Some(rusb::Context::new().map_err(|e| format!("Could not init libusb ({e})."))?);
    }
    Ok(slot.as_ref().unwrap())
}

#[tauri::command]
fn open_card(bytes: Vec<u8>, name: String, state: State<AppState>) -> Result<CardView, String> {
    let card = Ps1Card::open(&bytes, &name, false).map_err(|e| e.0)?;
    let view = card.view();
    *state.card.lock().map_err(|e| e.to_string())? = Some(card);
    Ok(view)
}

#[tauri::command]
fn open_path(path: String, state: State<AppState>) -> Result<CardView, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Could not read file ({e})."))?;
    let name = std::path::Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("card.mcr")
        .to_string();
    open_card(bytes, name, state)
}

#[tauri::command]
fn compose_card(master_slots: Vec<u8>, state: State<AppState>) -> Result<ExportResult, String> {
    let slots: Vec<usize> = master_slots.iter().map(|s| *s as usize).collect();
    let dest = with_card(&state, |source| {
        compose_new_card(source, &slots, None).map_err(|e| e.0)
    })?;
    Ok(ExportResult {
        filename: format!("{}.mcr", dest.source_name),
        view: Some(dest.view()),
        bytes: dest.save_raw(true),
    })
}

#[tauri::command]
fn backup_card(app: tauri::AppHandle, state: State<AppState>) -> Result<BackupResult, String> {
    let documents = app
        .path()
        .document_dir()
        .map_err(|e| format!("Could not find Documents ({e})."))?;
    let dir = backup_dir(documents);
    with_card(&state, |card| {
        let stem = backup_stem(&card.source_name, matches!(card.source, CardSource::Usb));
        let filename = backup_filename(&stem, &local_timestamp());
        let path = write_backup(&dir, &filename, &backup_bytes(card, CardFormat::Raw))?;
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&filename)
            .to_string();
        Ok(BackupResult {
            display_path: display_path(&path),
            path: path.to_string_lossy().into_owned(),
            filename,
        })
    })
}

fn library_config(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join("local-backups.json"))
        .map_err(|e| format!("Could not find settings folder ({e})."))
}

#[tauri::command]
async fn local_backups(app: tauri::AppHandle) -> Result<LibraryView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        match configured_directory(&library_config(&app)?)? {
            Some(directory) => read_library(&directory),
            None => Ok(LibraryView::default()),
        }
    })
    .await
    .map_err(|e| format!("Local backups worker failed ({e})."))?
}

#[tauri::command]
async fn configure_local_backups(
    directory: String,
    app: tauri::AppHandle,
) -> Result<LibraryView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let directory = std::path::Path::new(&directory);
        let view = read_library(directory)?;
        configure_directory(&library_config(&app)?, directory)?;
        Ok(view)
    })
    .await
    .map_err(|e| format!("Local backups worker failed ({e})."))?
}

#[tauri::command]
async fn sync_card(directory: String, app: tauri::AppHandle) -> Result<SyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let directory = std::path::Path::new(&directory);
        let result = with_card(&state, |card| sync_saves(card, directory))?;
        configure_directory(&library_config(&app)?, directory).map_err(|e| {
            format!("Saves were exported, but the destination setting could not be saved. {e}")
        })?;
        Ok(result)
    })
    .await
    .map_err(|e| format!("Sync worker failed ({e})."))?
}

#[tauri::command]
fn reveal_path(path: String, app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| format!("Could not reveal in Finder ({e})."))
}

#[tauri::command]
fn write_bytes(path: String, bytes: Vec<u8>) -> Result<(), String> {
    std::fs::write(&path, bytes).map_err(|e| format!("Could not write file ({e})."))
}

#[tauri::command]
async fn probe_adaptor(app: tauri::AppHandle) -> Result<HardwareStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        probe_adaptor_blocking(&state)
    })
    .await
    .map_err(|e| format!("Adaptor worker failed ({e})."))?
}

fn probe_adaptor_blocking(state: &AppState) -> Result<HardwareStatus, String> {
    let mut slot = state.usb.lock().map_err(|e| e.to_string())?;
    let ctx = usb_context(&mut slot)?;
    Ok(usb::probe(ctx))
}

#[tauri::command]
async fn read_adaptor(app: tauri::AppHandle) -> Result<CardView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        read_adaptor_blocking(&app, &state)
    })
    .await
    .map_err(|e| format!("Card read worker failed ({e})."))?
}

fn read_adaptor_blocking(app: &tauri::AppHandle, state: &AppState) -> Result<CardView, String> {
    let mut slot = state.usb.lock().map_err(|e| e.to_string())?;
    let ctx = usb_context(&mut slot)?;
    let probed = usb::find_identity(ctx).ok().flatten();
    let _ = app.emit(
        "usb-progress",
        HardwareStatus {
            state: "reading".into(),
            message: "Reading frame 0 / 1024".into(),
            identity: probed.clone(),
            frame: 0,
            total: usb::FRAME_COUNT,
        },
    );
    let progress_id = probed.clone();
    let app_progress = app.clone();
    let (bytes, identity) = usb::read_card(ctx, move |frame| {
        if frame == 0 || frame == usb::FRAME_COUNT - 1 || frame % 8 == 0 {
            let _ = app_progress.emit(
                "usb-progress",
                HardwareStatus {
                    state: "reading".into(),
                    message: format!("Reading frame {} / {}", frame + 1, usb::FRAME_COUNT),
                    identity: progress_id.clone(),
                    frame: frame + 1,
                    total: usb::FRAME_COUNT,
                },
            );
        }
    })?;
    drop(slot);
    let mut card =
        Ps1Card::open_from(&bytes, "adaptor", false, CardSource::Usb).map_err(|e| e.0)?;
    if !identity.product.is_empty() {
        card.source_name = identity.product.clone();
    } else {
        card.source_name = "adaptor".into();
    }
    let view = card.view();
    *state.card.lock().map_err(|e| e.to_string())? = Some(card);
    let _ = app.emit(
        "usb-progress",
        HardwareStatus {
            state: "live".into(),
            message: format!(
                "Live · {:04X}:{:04X}{}",
                identity.vid,
                identity.pid,
                if identity.product.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", identity.product)
                }
            ),
            identity: Some(identity),
            frame: usb::FRAME_COUNT,
            total: usb::FRAME_COUNT,
        },
    );
    Ok(view)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            card: Mutex::new(None),
            usb: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            open_card,
            open_path,
            compose_card,
            backup_card,
            sync_card,
            local_backups,
            configure_local_backups,
            reveal_path,
            write_bytes,
            probe_adaptor,
            read_adaptor
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
