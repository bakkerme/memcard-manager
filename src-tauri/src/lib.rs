mod card;
mod card_session;
mod usb;

use card::{
    capture_card, cards_directory, configure_collection, configured_collection,
    configured_directory, read_card_backups, read_collection, resolve_collection, saves_directory,
    sync_saves, update_card_label, CardColor, LibraryView, SyncResult,
};

use card::{backup_dir, compose_new_card, display_path, CardSource, CardView, Ps1Card};
use card_session::{CardSessions, LoadedCard};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use usb::HardwareStatus;

struct AppState {
    card: Mutex<CardSessions>,
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
    f(guard.active()?)
}

fn usb_context<'a>(slot: &'a mut Option<rusb::Context>) -> Result<&'a rusb::Context, String> {
    if slot.is_none() {
        *slot = Some(rusb::Context::new().map_err(|e| format!("Could not init libusb ({e})."))?);
    }
    Ok(slot.as_ref().unwrap())
}

#[tauri::command]
fn open_card(bytes: Vec<u8>, name: String, state: State<AppState>) -> Result<LoadedCard, String> {
    let card = Ps1Card::open(&bytes, &name, false).map_err(|e| e.0)?;
    Ok(state.card.lock().map_err(|e| e.to_string())?.open(card))
}

#[tauri::command]
fn open_path(path: String, state: State<AppState>) -> Result<LoadedCard, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Could not read file ({e})."))?;
    let name = std::path::Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("card.mcr")
        .to_string();
    open_card(bytes, name, state)
}

#[tauri::command]
fn activate_card(session_id: String, state: State<AppState>) -> Result<(), String> {
    state
        .card
        .lock()
        .map_err(|e| e.to_string())?
        .activate(&session_id)
}

#[tauri::command]
fn close_card(
    session_id: String,
    next_session_id: Option<String>,
    state: State<AppState>,
) -> Result<(), String> {
    state
        .card
        .lock()
        .map_err(|e| e.to_string())?
        .close(&session_id, next_session_id.as_deref())
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
fn backup_card(
    directory: String,
    name: String,
    color: CardColor,
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<BackupResult, String> {
    let root = require_collection(&app, &directory)?;
    with_card(&state, |card| {
        let path = capture_card(&root, card, &name, color)?;
        Ok(BackupResult {
            display_path: display_path(&path),
            filename: path.file_name().unwrap().to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
        })
    })
}

fn library_config(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join("local-backups.json"))
        .map_err(|e| format!("Could not find settings folder ({e})."))
}

fn collection_config(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(library_config(app)?.with_file_name("collection.json"))
}

fn previous_card_directory(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(backup_dir(
        app.path().document_dir().map_err(|e| e.to_string())?,
    ))
}

fn require_collection(
    app: &tauri::AppHandle,
    directory: &str,
) -> Result<std::path::PathBuf, String> {
    let root = configured_collection(&collection_config(app)?)?
        .ok_or("Choose a collection folder for both Backup and Sync first.")?;
    if root != std::path::Path::new(directory) {
        return Err("The collection folder changed. Refresh the collection and try again.".into());
    }
    Ok(root)
}

#[tauri::command]
async fn local_backups(app: tauri::AppHandle) -> Result<LibraryView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let previous_cards = previous_card_directory(&app)?;
        if let Some(root) = resolve_collection(
            &collection_config(&app)?,
            &library_config(&app)?,
            &previous_cards,
        )? {
            return read_collection(&root);
        }
        let mut view = LibraryView::default();
        view.cards = read_card_backups(&previous_cards, &mut view.warnings)?;
        Ok(view)
    })
    .await
    .map_err(|e| format!("Collection worker failed ({e})."))?
}

#[tauri::command]
async fn configure_local_backups(
    directory: String,
    app: tauri::AppHandle,
) -> Result<LibraryView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = std::path::Path::new(&directory);
        let existing = configured_collection(&collection_config(&app)?)?;
        let previous_saves = match &existing {
            Some(previous) => Some(saves_directory(previous)),
            None => configured_directory(&library_config(&app)?)?,
        };
        let previous_cards = match &existing {
            Some(previous) => cards_directory(previous),
            None => previous_card_directory(&app)?,
        };
        configure_collection(
            &collection_config(&app)?,
            root,
            previous_saves.as_deref(),
            &previous_cards,
        )?;
        read_collection(root)
    })
    .await
    .map_err(|e| format!("Collection worker failed ({e})."))?
}

#[tauri::command]
async fn sync_card(directory: String, app: tauri::AppHandle) -> Result<SyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = require_collection(&app, &directory)?;
        let state = app.state::<AppState>();
        with_card(&state, |card| sync_saves(card, &saves_directory(&root)))
    })
    .await
    .map_err(|e| format!("Sync worker failed ({e})."))?
}

#[tauri::command]
async fn label_card_backup(
    path: String,
    name: String,
    color: CardColor,
    app: tauri::AppHandle,
) -> Result<LibraryView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = configured_collection(&collection_config(&app)?)?
            .ok_or("Choose a collection folder before labeling card backups.")?;
        update_card_label(&root, std::path::Path::new(&path), &name, color)?;
        read_collection(&root)
    })
    .await
    .map_err(|e| format!("Card label worker failed ({e})."))?
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
async fn read_adaptor(app: tauri::AppHandle) -> Result<LoadedCard, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        read_adaptor_blocking(&app, &state)
    })
    .await
    .map_err(|e| format!("Card read worker failed ({e})."))?
}

fn read_adaptor_blocking(app: &tauri::AppHandle, state: &AppState) -> Result<LoadedCard, String> {
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
    let view = state.card.lock().map_err(|e| e.to_string())?.open(card);
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
            card: Mutex::new(CardSessions::default()),
            usb: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            open_card,
            open_path,
            activate_card,
            close_card,
            compose_card,
            backup_card,
            label_card_backup,
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
