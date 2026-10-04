use super::snapshot::{read_metadata, save_content_id, snapshot_directory, SnapshotSource};
use super::{
    display_path,
    engine::{Ps1Card, SaveInfo, BLOCK_SIZE, MCS_HEADER_SIZE, SLOT_COUNT},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySave {
    pub path: String,
    pub relative_path: String,
    pub save: SaveInfo,
    pub snapshots: Vec<LibrarySnapshot>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub path: String,
    pub content_id: String,
    pub captured_at: Option<String>,
    pub sources: Vec<SnapshotSource>,
    pub current: bool,
    pub save: SaveInfo,
}

fn read_snapshots(latest: &Path, warnings: &mut Vec<String>) -> Vec<LibrarySnapshot> {
    let dir = snapshot_directory(latest);
    for folder in [dir.parent().unwrap(), &dir] {
        match std::fs::symlink_metadata(folder) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return vec![],
            Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
            _ => {
                warnings.push(format!(
                    "Could not read snapshot folder {}.",
                    folder.display()
                ));
                return vec![];
            }
        }
    }
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            warnings.push(format!("Could not read snapshots ({e})."));
            return vec![];
        }
    };
    let current = std::fs::read(latest)
        .ok()
        .and_then(|bytes| save_content_id(&bytes).ok());
    let mut snapshots = vec![];
    for entry in entries {
        let parsed = (|| -> Result<Option<LibrarySnapshot>, String> {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !entry.file_type().map_err(|e| e.to_string())?.is_file()
                || path.extension().is_none_or(|ext| ext != "mcs")
            {
                return Ok(None);
            }
            if entry.metadata().map_err(|e| e.to_string())?.len()
                > (MCS_HEADER_SIZE + SLOT_COUNT * BLOCK_SIZE) as u64
            {
                return Err("Snapshot exceeds PS1 save size.".into());
            }
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let id = save_content_id(&bytes)?;
            if path.file_stem().and_then(|name| name.to_str()) != Some(&id) {
                return Err(format!(
                    "Snapshot {} does not match its content ID.",
                    path.display()
                ));
            }
            let save = parse_mcs(&bytes)?;
            let metadata = match read_metadata(&path.with_extension("json")) {
                Ok(meta) if meta.content_id == id => Some(meta),
                Ok(_) => {
                    warnings.push(format!(
                        "Snapshot {} has mismatched metadata.",
                        path.display()
                    ));
                    None
                }
                Err(error) => {
                    warnings.push(format!("Snapshot {}: {error}", path.display()));
                    None
                }
            };
            Ok(Some(LibrarySnapshot {
                path: path.to_string_lossy().into_owned(),
                current: current.as_ref() == Some(&id),
                content_id: id,
                captured_at: metadata.as_ref().map(|meta| meta.captured_at.clone()),
                sources: metadata.map(|meta| meta.sources).unwrap_or_default(),
                save,
            }))
        })();
        match parsed {
            Ok(Some(snapshot)) => snapshots.push(snapshot),
            Ok(None) => {}
            Err(error) => warnings.push(error),
        }
    }
    snapshots.sort_by(|a, b| {
        b.captured_at
            .cmp(&a.captured_at)
            .then(a.content_id.cmp(&b.content_id))
    });
    snapshots
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryView {
    pub directory: Option<String>,
    pub display_path: Option<String>,
    pub saves: Vec<LibrarySave>,
    pub warnings: Vec<String>,
    pub collection_configured: bool,
    pub cards: Vec<super::CardBackup>,
}

#[derive(Serialize, Deserialize)]
struct LibraryConfig {
    directory: String,
}

pub fn configured_directory(config: &Path) -> Result<Option<PathBuf>, String> {
    let bytes = match std::fs::read(config) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "Could not read the local backups setting ({e}). Choose a folder again."
            ))
        }
    };
    let saved: LibraryConfig = serde_json::from_slice(&bytes).map_err(|e| {
        format!("Could not read the local backups setting ({e}). Choose a folder again.")
    })?;
    Ok(Some(PathBuf::from(saved.directory)))
}

pub fn configure_directory(config: &Path, directory: &Path) -> Result<(), String> {
    let parent = config.parent().ok_or("Invalid settings path.")?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Could not create settings folder ({e})."))?;
    let bytes = serde_json::to_vec(&LibraryConfig {
        directory: directory.to_string_lossy().into_owned(),
    })
    .map_err(|e| e.to_string())?;
    let temporary = config.with_extension("tmp");
    std::fs::write(&temporary, bytes)
        .map_err(|e| format!("Could not save local backups setting ({e})."))?;
    std::fs::rename(&temporary, config)
        .map_err(|e| format!("Could not save local backups setting ({e})."))
}

fn parse_mcs(bytes: &[u8]) -> Result<SaveInfo, String> {
    if bytes.len() < MCS_HEADER_SIZE + BLOCK_SIZE
        || bytes.len() > MCS_HEADER_SIZE + SLOT_COUNT * BLOCK_SIZE
        || (bytes.len() - MCS_HEADER_SIZE) % BLOCK_SIZE != 0
    {
        return Err("Expected a 128-byte MCS header and 1–15 complete save blocks.".into());
    }
    if !matches!(bytes[0], 0x51 | 0xa1) {
        return Err("Not a PS1 MCS save.".into());
    }
    // Reuse the attributed MemcardRex parser on a temporary card, never AppState's open card.
    let mut card = Ps1Card::create_formatted("local-save");
    card.set_save_bytes(0, bytes)
        .map_err(|_| "Save exceeds card capacity.".to_string())?;
    let mut save = card
        .view()
        .saves
        .into_iter()
        .next()
        .ok_or("No save data found.")?;
    save.deleted = bytes[0] == 0xa1;
    Ok(save)
}

pub fn read_library(directory: &Path) -> Result<LibraryView, String> {
    if !directory.is_dir() {
        return Err(format!(
            "Local backups folder {} is unavailable. Reconnect it or choose another folder.",
            directory.display()
        ));
    }
    let mut view = LibraryView {
        directory: Some(directory.to_string_lossy().into_owned()),
        display_path: Some(display_path(directory)),
        ..Default::default()
    };
    scan(directory, directory, 0, &mut view)?;
    view.saves.sort_by(|a, b| {
        a.save
            .prod_code
            .cmp(&b.save.prod_code)
            .then(a.save.title.cmp(&b.save.title))
            .then(a.relative_path.cmp(&b.relative_path))
    });
    Ok(view)
}

fn scan(root: &Path, directory: &Path, depth: usize, view: &mut LibraryView) -> Result<(), String> {
    let entries = std::fs::read_dir(directory).map_err(|e| {
        format!(
            "Could not read local backups folder {} ({e}).",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                view.warnings
                    .push(format!("Could not read a folder entry ({e})."));
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(kind) => kind,
            Err(e) => {
                view.warnings.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if entry.file_name() == ".snapshots" {
                continue;
            }
            if depth >= 16 {
                view.warnings
                    .push(format!("Skipped deeply nested folder {}.", path.display()));
                continue;
            }
            if let Err(error) = scan(root, &path, depth + 1, view) {
                view.warnings.push(error);
            }
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mcs"))
        {
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let parsed = (|| -> Result<SaveInfo, String> {
                let length = entry.metadata().map_err(|e| e.to_string())?.len();
                if length > (MCS_HEADER_SIZE + SLOT_COUNT * BLOCK_SIZE) as u64 {
                    return Err("File is too large for a PS1 save.".into());
                }
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                parse_mcs(&bytes)
            })();
            match parsed {
                Ok(save) => view.saves.push(LibrarySave {
                    path: path.to_string_lossy().into_owned(),
                    relative_path,
                    snapshots: read_snapshots(&path, &mut view.warnings),
                    save,
                }),
                Err(error) => view.warnings.push(format!("{relative_path}: {error}")),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_sync_saves_and_persists_directory_without_modifying_files() {
        let root = std::env::temp_dir().join(format!(
            "memcard-library-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dir = root.join("saves");
        std::fs::create_dir_all(&dir).unwrap();
        let bytes =
            std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../blue.mcr")).unwrap();
        let card = Ps1Card::open(&bytes, "blue", false).unwrap();
        let config = root.join("settings/library.json");
        assert!(configured_directory(&config).unwrap().is_none());
        configure_directory(&config, &dir).unwrap();
        assert_eq!(configured_directory(&config).unwrap().unwrap(), dir);
        super::super::sync_saves(&card, &dir).unwrap();
        std::fs::write(dir.join("broken.mcs"), b"broken").unwrap();
        std::fs::write(dir.join("ignore.mcr"), &bytes).unwrap();
        let library = read_library(&dir).unwrap();
        let active = card
            .view()
            .saves
            .into_iter()
            .filter(|s| !s.deleted)
            .collect::<Vec<_>>();
        assert_eq!(library.saves.len(), active.len());
        assert_eq!(library.warnings.len(), 1);
        for entry in library.saves {
            let source = active
                .iter()
                .find(|s| {
                    s.prod_code == entry.save.prod_code && s.identifier == entry.save.identifier
                })
                .unwrap();
            assert_eq!(entry.save.title, source.title);
            assert_eq!(entry.save.frames, source.frames);
            assert_eq!(entry.save.linked_slots.len(), source.linked_slots.len());
            assert_eq!(
                std::fs::read(entry.path).unwrap(),
                card.get_save_bytes(source.master_slot as usize)
            );
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&dir, dir.join("loop")).unwrap();
            assert_eq!(read_library(&dir).unwrap().saves.len(), active.len());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_invalid_save_sizes_and_magic() {
        for length in [0, 128, 129, 8192, 128 + 8193, 128 + 16 * 8192] {
            assert!(parse_mcs(&vec![0; length]).is_err());
        }
        assert!(parse_mcs(&vec![0; 128 + 8192]).is_err());
    }
}
