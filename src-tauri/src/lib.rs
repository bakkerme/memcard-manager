mod card;
mod usb;

use card::{backup_bytes, compose_new_card, CardFormat, CardSource, CardView, Ps1Card};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, State};
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

fn with_card<T>(state: &AppState, f: impl FnOnce(&Ps1Card) -> Result<T, String>) -> Result<T, String> {
    let guard = state.card.lock().map_err(|_| "Card lock poisoned.".to_string())?;
    let card = guard.as_ref().ok_or_else(|| "No card is open.".to_string())?;
    f(card)
}

fn usb_context<'a>(
    slot: &'a mut Option<rusb::Context>,
) -> Result<&'a rusb::Context, String> {
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
fn backup_card(format: Option<String>, state: State<AppState>) -> Result<ExportResult, String> {
    with_card(&state, |card| {
        let fmt = match format.as_deref() {
            Some("gme") => CardFormat::Gme,
            Some("vgs") => CardFormat::Vgs,
            Some("raw") | Some("mcr") => CardFormat::Raw,
            None => {
                if matches!(card.source, CardSource::Usb) {
                    CardFormat::Raw
                } else {
                    card.format
                }
            }
            other => {
                return Err(format!("Unknown backup format {:?}.", other));
            }
        };
        let ext = match fmt {
            CardFormat::Raw => "mcr",
            CardFormat::Gme => "gme",
            CardFormat::Vgs => "vgs",
        };
        Ok(ExportResult {
            filename: format!("{}-backup.{ext}", card.source_name),
            view: None,
            bytes: backup_bytes(card, fmt),
        })
    })
}

#[tauri::command]
fn write_bytes(path: String, bytes: Vec<u8>) -> Result<(), String> {
    std::fs::write(&path, bytes).map_err(|e| format!("Could not write file ({e})."))
}

#[tauri::command]
fn probe_adaptor(state: State<AppState>) -> Result<HardwareStatus, String> {
    let mut slot = state.usb.lock().map_err(|e| e.to_string())?;
    let ctx = usb_context(&mut slot)?;
    Ok(usb::probe(ctx))
}

#[tauri::command]
fn read_adaptor(app: tauri::AppHandle, state: State<AppState>) -> Result<CardView, String> {
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
    let mut card = Ps1Card::open_from(&bytes, "adaptor", false, CardSource::Usb).map_err(|e| e.0)?;
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
    let usb = rusb::Context::new().ok();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            card: Mutex::new(None),
            usb: Mutex::new(usb),
        })
        .invoke_handler(tauri::generate_handler![
            open_card,
            open_path,
            compose_card,
            backup_card,
            write_bytes,
            probe_adaptor,
            read_adaptor
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
