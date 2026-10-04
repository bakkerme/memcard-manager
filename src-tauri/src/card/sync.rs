use super::snapshot::{capture, publish, regular_directory, save_content_id, SnapshotSource};
use super::{backup::display_path, engine::Ps1Card};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub path: String,
    pub display_path: String,
    pub written: usize,
    pub unchanged: usize,
    pub snapshots_added: usize,
}

// Encode unsafe or lowercase names so distinct saves stay distinct on case-insensitive disks.
fn component(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    let bytes = &bytes[..end];
    if !bytes.is_empty()
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'-')
    {
        String::from_utf8(bytes.to_vec()).unwrap()
    } else {
        format!(
            "save~{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    }
}

pub fn sync_saves(card: &Ps1Card, directory: &Path) -> Result<SyncResult, String> {
    if !directory.is_dir() {
        return Err("Choose an existing local directory for Sync.".into());
    }
    let mut result = SyncResult {
        path: directory.to_string_lossy().into_owned(),
        display_path: display_path(directory),
        written: 0,
        unchanged: 0,
        snapshots_added: 0,
    };
    let provenance = SnapshotSource {
        image_id: card.image_id(),
        source_name: card.source_name.clone(),
        source: match card.source {
            super::engine::CardSource::File => "file",
            super::engine::CardSource::Usb => "usb",
        }
        .into(),
    };
    let saves = card.view().saves;
    for save in saves.iter().filter(|save| !save.deleted) {
        let bytes = card.get_save_bytes(save.master_slot as usize);
        // The PS1 directory filename is region + product code + save identifier.
        let game = if save.prod_code.is_empty() {
            "unknown-game".into()
        } else {
            component(&bytes[12..22])
        };
        let folder = directory.join(game);
        // Do not follow a game folder that redirects outside the chosen directory.
        if let Ok(metadata) = std::fs::symlink_metadata(&folder) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(format!(
                    "Game folder {} must be a regular directory.",
                    folder.display()
                ));
            }
        }
        regular_directory(&folder)?;
        let path = folder.join(format!("{}.mcs", component(&bytes[10..30])));
        if let Ok(metadata) = std::fs::symlink_metadata(&path) {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(format!(
                    "Save destination {} must be a regular file.",
                    path.display()
                ));
            }
        }
        let existing = match std::fs::read(&path) {
            Ok(existing) => Some(existing),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("Could not read existing save ({e}).")),
        };
        let incoming_id = save_content_id(&bytes)?;
        if let Some(ref previous) = existing {
            // Migrate the existing latest dump before ever replacing it. Its origin may be unknown.
            result.snapshots_added += usize::from(capture(&path, previous, None)?);
        }
        result.snapshots_added += usize::from(capture(&path, &bytes, Some(provenance.clone()))?);
        if existing
            .as_ref()
            .map(|previous| save_content_id(previous))
            .transpose()?
            .as_deref()
            == Some(&incoming_id)
        {
            result.unchanged += 1;
            continue;
        }
        publish(&path, &bytes, false)?;
        result.written += 1;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::engine::{BLOCK_SIZE, MCS_HEADER_SIZE};

    #[test]
    fn exports_linked_saves_skips_unchanged_and_updates_changed() {
        let source =
            std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../blue.mcr")).unwrap();
        let mut card = Ps1Card::open(&source, "blue.mcr", false).unwrap();
        let original = card.raw.clone();
        let directory = std::env::temp_dir().join(format!(
            "memcard-sync-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let active = card
            .view()
            .saves
            .into_iter()
            .filter(|s| !s.deleted)
            .collect::<Vec<_>>();
        let first = sync_saves(&card, &directory).unwrap();
        assert_eq!(first.written, active.len());
        assert_eq!(card.raw, original);
        for save in &active {
            let bytes = card.get_save_bytes(save.master_slot as usize);
            let folder = if save.prod_code.is_empty() {
                "unknown-game".into()
            } else {
                component(&bytes[12..22])
            };
            let path = directory
                .join(folder)
                .join(format!("{}.mcs", component(&bytes[10..30])));
            assert_eq!(std::fs::read(path).unwrap(), bytes);
            let mut restored = Ps1Card::create_formatted("restored");
            restored.set_save_bytes(0, &bytes).unwrap();
            let restored_view = restored.view();
            assert_eq!(restored_view.saves.len(), 1);
            assert_eq!(restored_view.saves[0].title, save.title);
            assert_eq!(
                restored_view.saves[0].linked_slots.len(),
                save.linked_slots.len()
            );
            assert_eq!(
                &restored.get_save_bytes(0)[MCS_HEADER_SIZE..],
                &bytes[MCS_HEADER_SIZE..]
            );
            assert_eq!(
                bytes.len(),
                MCS_HEADER_SIZE + save.linked_slots.len() * BLOCK_SIZE
            );
        }
        let second = sync_saves(&card, &directory).unwrap();
        assert_eq!(second.written, 0);
        assert_eq!(second.unchanged, active.len());
        let slot = active[0].master_slot as usize;
        card.raw[(slot + 1) * BLOCK_SIZE + 300] ^= 1;
        std::fs::write(directory.join("keep-me.txt"), b"unrelated").unwrap();
        let third = sync_saves(&card, &directory).unwrap();
        assert_eq!(third.written, 1);
        assert_eq!(third.unchanged, active.len() - 1);
        assert!(directory.join("keep-me.txt").exists());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unsafe_names_are_distinct_and_cannot_escape() {
        assert_ne!(
            component(b"SAVE-A").to_lowercase(),
            component(b"SAVE-a").to_lowercase()
        );
        assert_ne!(component(b"a/b"), component(b"a?b"));
        assert_ne!(component(b"a/b"), component(b"save~612f62"));
        assert!(!component(b"../escape").contains('/'));
        assert_eq!(component(b"BASLUS-12345\0\0"), "BASLUS-12345");
    }
    #[test]
    fn case_only_save_names_remain_separate() {
        let raw = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../blue.mcr")).unwrap();
        let source = Ps1Card::open(&raw, "blue", false).unwrap();
        let save = source
            .view()
            .saves
            .into_iter()
            .find(|save| save.linked_slots.len() == 1)
            .unwrap();
        let mut bytes = source.get_save_bytes(save.master_slot as usize);
        bytes[10..30].fill(0);
        let name = b"BASLUS-12345A";
        bytes[10..10 + name.len()].copy_from_slice(name);
        let mut card = Ps1Card::create_formatted("case-names");
        card.set_save_bytes(0, &bytes).unwrap();
        bytes[10 + name.len() - 1] = b'a';
        card.set_save_bytes(1, &bytes).unwrap();
        let dir = test_dir();
        assert_eq!(sync_saves(&card, &dir).unwrap().written, 2);
        let library = super::super::read_library(&dir).unwrap();
        assert_eq!(library.saves.len(), 2);
        assert_ne!(
            library.saves[0].path.to_lowercase(),
            library.saves[1].path.to_lowercase()
        );
        assert_eq!(sync_saves(&card, &dir).unwrap().unchanged, 2);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "memcard-snapshots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn snapshots_preserve_conflicting_cards_dedupe_moves_and_allow_reverting() {
        let raw = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../blue.mcr")).unwrap();
        let original = Ps1Card::open(&raw, "first", false).unwrap();
        let mut card = super::super::compose_new_card(&original, &[13], Some("card-a")).unwrap();
        let dir = test_dir();
        let first = sync_saves(&card, &dir).unwrap();
        assert_eq!(first.snapshots_added, 1);
        let original_image = card.image_id();
        let view = super::super::read_library(&dir).unwrap();
        let latest = std::path::PathBuf::from(&view.saves[0].path);
        let original_bytes = std::fs::read(&latest).unwrap();
        assert_eq!(view.saves[0].snapshots.len(), 1);
        assert_eq!(
            view.saves[0].snapshots[0].sources[0].image_id,
            original_image
        );
        // Change progress under the same PS1 filename, as another card/playthrough might.
        card.raw[BLOCK_SIZE + 300] ^= 1;
        assert_ne!(original_image, card.image_id());
        let next = sync_saves(&card, &dir).unwrap();
        assert_eq!(next.snapshots_added, 1);
        let changed_bytes = std::fs::read(&latest).unwrap();
        assert_ne!(original_bytes, changed_bytes);
        let view = super::super::read_library(&dir).unwrap();
        assert_eq!(view.saves.len(), 1);
        assert_eq!(view.saves[0].snapshots.len(), 2);
        assert_eq!(
            view.saves[0].snapshots.iter().filter(|s| s.current).count(),
            1
        );
        assert!(view.saves[0]
            .snapshots
            .iter()
            .any(|s| std::fs::read(&s.path).unwrap() == original_bytes));
        assert_eq!(sync_saves(&card, &dir).unwrap().snapshots_added, 0);
        // Relocate identical progress to another block; record provenance, not a new version.
        let mut moved = Ps1Card::create_formatted("card-b");
        moved.set_save_bytes(5, &changed_bytes).unwrap();
        let moved_result = sync_saves(&moved, &dir).unwrap();
        assert_eq!(moved_result.snapshots_added, 0);
        assert_eq!(moved_result.unchanged, 1);
        let view = super::super::read_library(&dir).unwrap();
        assert_eq!(
            view.saves[0]
                .snapshots
                .iter()
                .find(|s| s.current)
                .unwrap()
                .sources
                .len(),
            2
        );
        // Reverting is still possible without creating duplicates or dropping newer history.
        card.raw[BLOCK_SIZE + 300] ^= 1;
        let reverted = sync_saves(&card, &dir).unwrap();
        assert_eq!(reverted.written, 1);
        assert_eq!(reverted.snapshots_added, 0);
        assert_eq!(std::fs::read(&latest).unwrap(), original_bytes);
        assert_eq!(
            super::super::read_library(&dir).unwrap().saves[0]
                .snapshots
                .len(),
            2
        );
        let gme = Ps1Card::open(
            &original.export(super::super::CardFormat::Gme, false),
            "wrapped",
            false,
        )
        .unwrap();
        assert_eq!(gme.image_id(), original.image_id());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn migrates_legacy_backup_and_stops_before_overwrite_if_history_is_blocked() {
        let raw = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../blue.mcr")).unwrap();
        let source = Ps1Card::open(&raw, "blue", false).unwrap();
        let mut card = super::super::compose_new_card(&source, &[13], None).unwrap();
        let dir = test_dir();
        let old = card.get_save_bytes(0);
        let folder = dir.join(component(&old[12..22]));
        std::fs::create_dir(&folder).unwrap();
        let path = folder.join(format!("{}.mcs", component(&old[10..30])));
        std::fs::write(&path, &old).unwrap();
        std::fs::write(folder.join(".snapshots"), b"blocked").unwrap();
        card.raw[BLOCK_SIZE + 300] ^= 1;
        assert!(sync_saves(&card, &dir).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), old);
        std::fs::remove_file(folder.join(".snapshots")).unwrap();
        assert_eq!(sync_saves(&card, &dir).unwrap().snapshots_added, 2);
        let view = super::super::read_library(&dir).unwrap();
        assert!(view.saves[0]
            .snapshots
            .iter()
            .any(|s| s.sources.is_empty() && std::fs::read(&s.path).unwrap() == old));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
