use super::{
    backup::{backup_filename, backup_stem, display_path, local_timestamp, write_backup},
    backup_bytes, read_library,
    snapshot::{publish, regular_directory},
    CardFormat, CardSource, LibraryView, Ps1Card, CARD_SIZE,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub fn saves_directory(root: &Path) -> PathBuf {
    root.join("saves")
}
pub fn cards_directory(root: &Path) -> PathBuf {
    root.join("card-backups")
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardColor {
    #[default]
    Grey,
    Black,
    White,
    Blue,
    Green,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardBackupMetadata {
    pub name: String,
    pub color: CardColor,
    pub captured_at: Option<String>,
    pub source_name: String,
    pub source: CardSource,
    pub image_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardBackup {
    pub path: String,
    pub filename: String,
    pub name: String,
    pub color: CardColor,
    pub captured_at: Option<String>,
    pub source_name: Option<String>,
    pub source: Option<CardSource>,
    pub image_id: String,
    pub save_count: usize,
    pub used_blocks: u8,
}

fn name_label(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Give the card a name between 1 and 80 characters.".into());
    }
    Ok(name.into())
}

pub fn prepare_collection(root: &Path) -> Result<(), String> {
    if !root.is_dir() {
        return Err("The collection folder is unavailable. Choose a folder again.".into());
    }
    regular_directory(root)?;
    regular_directory(&saves_directory(root))?;
    regular_directory(&cards_directory(root))
}

pub fn configured_collection(config: &Path) -> Result<Option<PathBuf>, String> {
    // A separate setting leaves the previous save-only directory intact for migration.
    super::configured_directory(config)
}

pub fn resolve_collection(
    config: &Path,
    legacy_config: &Path,
    previous_cards: &Path,
) -> Result<Option<PathBuf>, String> {
    if let Some(root) = configured_collection(config)? {
        return Ok(Some(root));
    }
    let Some(root) = super::configured_directory(legacy_config)? else {
        return Ok(None);
    };
    // Reuse the folder the user already chose. Only persist the new setting
    // after copying and verifying the existing saves, history, and card images.
    configure_collection(config, &root, Some(&root), previous_cards)?;
    Ok(Some(root))
}

// Plan every file before writing: a collision never overwrites an existing save or history.
fn plan_copy(
    source: &Path,
    destination: &Path,
    copies: &mut Vec<(PathBuf, PathBuf)>,
    depth: usize,
) -> Result<(), String> {
    if source == destination || !source.exists() {
        return Ok(());
    }
    if depth > 16 {
        return Err(
            "The previous collection has deeply nested folders. Choose a simpler folder structure."
                .into(),
        );
    }
    regular_directory(source)?;
    if destination.exists() {
        regular_directory_existing(destination)?;
    }
    for entry in std::fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name() == ".DS_Store" {
            continue;
        }
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            // The legacy saves folder itself can become the collection root.
            if from == destination
                || from == destination.parent().unwrap().join("card-backups")
                || from == destination.parent().unwrap().join("backups")
            {
                continue;
            }
            plan_copy(&from, &to, copies, depth + 1)?;
        } else if kind.is_file() {
            match std::fs::symlink_metadata(&to) {
                Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
                    if std::fs::read(&from).map_err(|e| e.to_string())?
                        != std::fs::read(&to).map_err(|e| e.to_string())?
                    {
                        return Err(format!("{} already contains different data. Choose another collection folder; existing files were left untouched.", to.display()));
                    }
                }
                Ok(_) => {
                    return Err(format!(
                        "{} is not a regular file. Choose another folder.",
                        to.display()
                    ))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => copies.push((from, to)),
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    Ok(())
}

pub fn configure_collection(
    config: &Path,
    root: &Path,
    previous_saves: Option<&Path>,
    previous_cards: &Path,
) -> Result<(), String> {
    if previous_saves.is_some_and(|source| root != source && root.starts_with(source)) {
        return Err(
            "Choose the previous saves folder itself, or a folder outside it, for the collection."
                .into(),
        );
    }
    prepare_collection(root)?;
    let mut copies = vec![];
    if let Some(source) = previous_saves {
        plan_copy(source, &saves_directory(root), &mut copies, 0)?;
    }
    plan_copy(previous_cards, &cards_directory(root), &mut copies, 0)?;
    // A selected collection may already contain the old app's backups/ folder
    // (for example a Documents folder relocated into iCloud Drive).
    let local_legacy_cards = root.join("backups");
    if local_legacy_cards != previous_cards {
        plan_copy(&local_legacy_cards, &cards_directory(root), &mut copies, 0)?;
    }
    // Atomic exclusive publication also handles a file created after the preflight.
    for (from, to) in copies {
        let relative = to.strip_prefix(root).map_err(|e| e.to_string())?;
        let mut parent = root.to_path_buf();
        for part in relative.parent().unwrap().components() {
            parent.push(part);
            regular_directory(&parent)?;
        }
        let bytes = std::fs::read(&from)
            .map_err(|e| format!("Could not read {} ({e}).", from.display()))?;
        if !publish(&to, &bytes, true)? && std::fs::read(&to).map_err(|e| e.to_string())? != bytes {
            return Err("A destination changed during copying. Previous files are intact; choose another folder.".into());
        }
        if std::fs::read(&to).map_err(|e| e.to_string())? != bytes {
            return Err(
                "A copied file could not be verified. The collection setting was not changed."
                    .into(),
            );
        }
    }
    super::configure_directory(config, root)
}

pub fn read_collection(root: &Path) -> Result<LibraryView, String> {
    // Browsing is read-only; unavailable folders must not be silently recreated.
    regular_directory_existing(root)?;
    regular_directory_existing(&saves_directory(root))?;
    let mut view = read_library(&saves_directory(root))?;
    view.directory = Some(root.to_string_lossy().into_owned());
    view.display_path = Some(display_path(root));
    view.collection_configured = true;
    view.cards = read_card_backups(&cards_directory(root), &mut view.warnings)?;
    Ok(view)
}

fn regular_directory_existing(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => Ok(()),
        _ => Err(format!(
            "Collection folder {} is unavailable. Reconnect it or choose another folder.",
            path.display()
        )),
    }
}

fn read_metadata(path: &Path, image_id: &str) -> Result<Option<CardBackupMetadata>, String> {
    let path = path.with_extension("json");
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(m) if m.is_file() && !m.file_type().is_symlink() && m.len() < 65_536 => {}
        _ => {
            return Err("Card label metadata is unreadable; showing the original filename.".into())
        }
    }
    let metadata: CardBackupMetadata =
        serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if metadata.image_id != image_id {
        return Err(
            "Card label metadata belongs to different contents; showing the original filename."
                .into(),
        );
    }
    Ok(Some(metadata))
}

pub fn read_card_backups(
    directory: &Path,
    warnings: &mut Vec<String>,
) -> Result<Vec<CardBackup>, String> {
    if !directory.exists() {
        return Ok(vec![]);
    }
    regular_directory_existing(directory)?;
    let mut cards = vec![];
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let parsed = (|| -> Result<Option<CardBackup>, String> {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !entry.file_type().map_err(|e| e.to_string())?.is_file()
                || !path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mcr"))
            {
                return Ok(None);
            }
            if entry.metadata().map_err(|e| e.to_string())?.len() != CARD_SIZE as u64 {
                return Err(format!(
                    "{} is not a complete 128 KB card backup.",
                    path.display()
                ));
            }
            let filename = entry.file_name().to_string_lossy().into_owned();
            let card = Ps1Card::open(
                &std::fs::read(&path).map_err(|e| e.to_string())?,
                &filename,
                false,
            )
            .map_err(|e| e.0)?
            .view();
            let meta = match read_metadata(&path, &card.image_id) {
                Ok(meta) => meta,
                Err(e) => {
                    warnings.push(format!("{filename}: {e}"));
                    None
                }
            };
            Ok(Some(CardBackup {
                path: path.to_string_lossy().into_owned(),
                name: meta
                    .as_ref()
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| filename.clone()),
                color: meta.as_ref().map(|m| m.color).unwrap_or_default(),
                captured_at: meta.as_ref().and_then(|m| m.captured_at.clone()),
                source_name: meta.as_ref().map(|m| m.source_name.clone()),
                source: meta.as_ref().map(|m| m.source),
                image_id: card.image_id,
                used_blocks: card.used_blocks,
                save_count: card.saves.iter().filter(|save| !save.deleted).count(),
                filename,
            }))
        })();
        match parsed {
            Ok(Some(card)) => cards.push(card),
            Ok(None) => {}
            Err(e) => warnings.push(e),
        }
    }
    cards.sort_by(|a, b| {
        b.captured_at
            .cmp(&a.captured_at)
            .then(b.filename.cmp(&a.filename))
    });
    Ok(cards)
}

pub fn capture_card(
    root: &Path,
    card: &Ps1Card,
    name: &str,
    color: CardColor,
) -> Result<PathBuf, String> {
    let name = name_label(name)?;
    prepare_collection(root)?;
    let filename = backup_filename(
        &backup_stem(&name, matches!(card.source, CardSource::Usb)),
        &local_timestamp(),
    );
    let path = write_backup(
        &cards_directory(root),
        &filename,
        &backup_bytes(card, CardFormat::Raw),
    )?;
    let metadata = CardBackupMetadata {
        name,
        color,
        captured_at: Some(chrono::Utc::now().to_rfc3339()),
        source_name: card.source_name.clone(),
        source: card.source,
        image_id: card.image_id(),
    };
    if let Err(error) = publish(
        &path.with_extension("json"),
        &serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?,
        true,
    )
    .and_then(|created| {
        if created {
            Ok(())
        } else {
            Err("A label already exists for this filename.".into())
        }
    }) {
        return Err(format!("Card backup was saved to {}, but its label could not be saved: {error}. Refresh Card backups to reopen it.", path.display()));
    }
    Ok(path)
}

pub fn update_card_label(
    root: &Path,
    path: &Path,
    name: &str,
    color: CardColor,
) -> Result<(), String> {
    let name = name_label(name)?;
    let directory = cards_directory(root);
    regular_directory_existing(&directory)?;
    if path.parent() != Some(directory.as_path()) || path.extension().is_none_or(|ext| ext != "mcr")
    {
        return Err("Choose a card from this collection's Card backups.".into());
    }
    let stat = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !stat.is_file() || stat.file_type().is_symlink() || stat.len() != CARD_SIZE as u64 {
        return Err("The card backup is unavailable or incomplete. Refresh Card backups.".into());
    }
    let card = Ps1Card::open(
        &std::fs::read(path).map_err(|e| e.to_string())?,
        "backup",
        false,
    )
    .map_err(|e| e.0)?;
    let mut metadata = read_metadata(path, &card.image_id())?.unwrap_or(CardBackupMetadata {
        name: name.clone(),
        color,
        captured_at: None,
        source_name: path.file_name().unwrap().to_string_lossy().into_owned(),
        source: CardSource::File,
        image_id: card.image_id(),
    });
    metadata.name = name;
    metadata.color = color;
    publish(
        &path.with_extension("json"),
        &serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?,
        false,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            // Concurrent tests must not share a path even when clock readings
            // coincide; another Scratch's Drop would remove this test's files.
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "memcard-collection-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn blue() -> Ps1Card {
        let bytes =
            std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../blue.mcr")).unwrap();
        Ps1Card::open(&bytes, "blue.mcr", false).unwrap()
    }

    #[test]
    fn reuses_the_selected_legacy_folder_and_preserves_backups() {
        let tmp = Scratch::new();
        let root = tmp.0.join("selected");
        let legacy_config = tmp.0.join("settings/local-backups.json");
        let config = tmp.0.join("settings/collection.json");
        let old_cards = tmp.0.join("old-cards");
        let card = blue();
        std::fs::create_dir_all(&root).unwrap();
        super::super::sync_saves(&card, &root).unwrap();
        super::super::configure_directory(&legacy_config, &root).unwrap();
        let original = read_library(&root).unwrap();
        let old_card =
            write_backup(&old_cards, "old.mcr", &backup_bytes(&card, CardFormat::Raw)).unwrap();

        assert_eq!(
            resolve_collection(&config, &legacy_config, &old_cards).unwrap(),
            Some(root.clone())
        );
        assert_eq!(configured_collection(&config).unwrap(), Some(root.clone()));
        let migrated = read_collection(&root).unwrap();
        assert!(migrated.collection_configured);
        assert_eq!(migrated.saves.len(), original.saves.len());
        assert_eq!(migrated.cards.len(), 1);
        for (before, after) in original.saves.iter().zip(migrated.saves.iter()) {
            assert_eq!(
                std::fs::read(&before.path).unwrap(),
                std::fs::read(&after.path).unwrap()
            );
            assert_eq!(before.snapshots.len(), after.snapshots.len());
            assert_eq!(
                before.snapshots[0].content_id,
                after.snapshots[0].content_id
            );
            assert_eq!(before.snapshots[0].sources, after.snapshots[0].sources);
        }
        assert!(old_card.exists());
        let capture = capture_card(&root, &card, "New backup", CardColor::Blue).unwrap();
        assert_eq!(capture.parent(), Some(cards_directory(&root).as_path()));
        assert_eq!(
            resolve_collection(&config, &legacy_config, &old_cards).unwrap(),
            Some(root.clone())
        );
        assert_eq!(read_collection(&root).unwrap().cards.len(), 2);
    }

    #[test]
    fn explicit_collection_wins_and_first_use_does_not_invent_a_folder() {
        let tmp = Scratch::new();
        let config = tmp.0.join("settings/collection.json");
        let legacy_config = tmp.0.join("settings/local-backups.json");
        let old_cards = tmp.0.join("old-cards");
        assert_eq!(
            resolve_collection(&config, &legacy_config, &old_cards).unwrap(),
            None
        );
        assert!(!config.exists());
        let selected = tmp.0.join("selected");
        super::super::configure_directory(&config, &selected).unwrap();
        // An obsolete, unreadable setting cannot displace the newer choice.
        std::fs::write(&legacy_config, b"invalid json").unwrap();
        assert_eq!(
            resolve_collection(&config, &legacy_config, &old_cards).unwrap(),
            Some(selected)
        );
    }

    #[test]
    fn automatic_migration_conflicts_leave_the_setting_and_originals_intact() {
        let tmp = Scratch::new();
        let root = tmp.0.join("selected");
        std::fs::create_dir_all(&root).unwrap();
        prepare_collection(&root).unwrap();
        std::fs::write(root.join("save.mcs"), b"original").unwrap();
        std::fs::write(saves_directory(&root).join("save.mcs"), b"different").unwrap();
        let legacy_config = tmp.0.join("settings/local-backups.json");
        let config = tmp.0.join("settings/collection.json");
        super::super::configure_directory(&legacy_config, &root).unwrap();
        assert!(resolve_collection(&config, &legacy_config, &tmp.0.join("old-cards")).is_err());
        assert!(!config.exists());
        assert_eq!(
            super::super::configured_directory(&legacy_config).unwrap(),
            Some(root.clone())
        );
        assert_eq!(std::fs::read(root.join("save.mcs")).unwrap(), b"original");
        assert_eq!(
            std::fs::read(saves_directory(&root).join("save.mcs")).unwrap(),
            b"different"
        );
        std::fs::remove_dir_all(&root).unwrap();
        assert!(resolve_collection(&config, &legacy_config, &tmp.0.join("old-cards")).is_err());
        assert!(!root.exists());
        assert!(!config.exists());
    }

    #[test]
    fn shared_root_preserves_card_bytes_and_mcs_history() {
        let tmp = Scratch::new();
        prepare_collection(&tmp.0).unwrap();
        let card = blue();
        let first = super::super::sync_saves(&card, &saves_directory(&tmp.0)).unwrap();
        let path = capture_card(&tmp.0, &card, "Blue card", CardColor::Blue).unwrap();
        let raw_before = std::fs::read(&path).unwrap();
        assert_eq!(raw_before, backup_bytes(&card, CardFormat::Raw));
        let view = read_collection(&tmp.0).unwrap();
        assert!(view.collection_configured);
        assert_eq!(view.cards.len(), 1);
        assert_eq!(view.cards[0].name, "Blue card");
        assert!(view.saves.iter().all(|s| !s.snapshots.is_empty()));
        assert_eq!(view.saves.len(), first.written);
        update_card_label(&tmp.0, &path, "RPGs", CardColor::Green).unwrap();
        let after = read_collection(&tmp.0).unwrap();
        assert_eq!(after.cards[0].name, "RPGs");
        assert!(matches!(after.cards[0].color, CardColor::Green));
        assert_eq!(after.cards[0].captured_at, view.cards[0].captured_at);
        assert_eq!(std::fs::read(&path).unwrap(), raw_before);
        let again = super::super::sync_saves(&card, &saves_directory(&tmp.0)).unwrap();
        assert_eq!(again.written, 0);
        assert_eq!(again.snapshots_added, 0);
    }

    #[test]
    fn migrates_legacy_files_and_history_without_moving_originals() {
        let tmp = Scratch::new();
        let saves = tmp.0.join("old-saves");
        let cards = tmp.0.join("old-cards");
        let root = tmp.0.join("new");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::create_dir_all(&cards).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        super::super::sync_saves(&blue(), &saves).unwrap();
        let old = write_backup(
            &cards,
            "original.mcr",
            &backup_bytes(&blue(), CardFormat::Raw),
        )
        .unwrap();
        let config = tmp.0.join("settings/collection.json");
        configure_collection(&config, &root, Some(&saves), &cards).unwrap();
        assert_eq!(configured_collection(&config).unwrap(), Some(root.clone()));
        assert!(old.exists());
        assert_eq!(
            std::fs::read(&old).unwrap(),
            std::fs::read(cards_directory(&root).join("original.mcr")).unwrap()
        );
        let copied = read_collection(&root).unwrap();
        let original = read_library(&saves).unwrap();
        assert_eq!(copied.saves.len(), original.saves.len());
        for (left, right) in copied.saves.iter().zip(original.saves.iter()) {
            assert_eq!(left.snapshots.len(), right.snapshots.len());
            assert_eq!(left.snapshots[0].content_id, right.snapshots[0].content_id);
            assert_eq!(left.snapshots[0].sources, right.snapshots[0].sources);
            assert_eq!(
                std::fs::read(&left.path).unwrap(),
                std::fs::read(&right.path).unwrap()
            );
        }
        // Copying to the same destination is repeatable.
        configure_collection(&config, &root, Some(&saves), &cards).unwrap();
    }

    #[test]
    fn a_conflicting_destination_does_not_overwrite_files_or_change_settings() {
        let tmp = Scratch::new();
        let old = tmp.0.join("old");
        let root = tmp.0.join("new");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(old.join("save.mcs"), b"old data").unwrap();
        prepare_collection(&root).unwrap();
        std::fs::write(saves_directory(&root).join("save.mcs"), b"different data").unwrap();
        let config = tmp.0.join("collection.json");
        super::super::configure_directory(&config, &old).unwrap();
        assert!(
            configure_collection(&config, &root, Some(&old), &tmp.0.join("missing-cards")).is_err()
        );
        assert_eq!(configured_collection(&config).unwrap(), Some(old.clone()));
        assert_eq!(std::fs::read(old.join("save.mcs")).unwrap(), b"old data");
        assert_eq!(
            std::fs::read(saves_directory(&root).join("save.mcs")).unwrap(),
            b"different data"
        );
    }

    #[test]
    fn legacy_saves_folder_can_become_root_without_recursive_copies() {
        let tmp = Scratch::new();
        let card = blue();
        super::super::sync_saves(&card, &tmp.0).unwrap();
        let original_count = read_library(&tmp.0).unwrap().saves.len();
        configure_collection(
            &tmp.0.join("settings/collection.json"),
            &tmp.0,
            Some(&tmp.0),
            &tmp.0.join("old-cards"),
        )
        .unwrap();
        assert_eq!(read_collection(&tmp.0).unwrap().saves.len(), original_count);
        assert!(!saves_directory(&tmp.0).join("saves").exists());
        assert!(!saves_directory(&tmp.0).join("card-backups").exists());
    }

    #[test]
    fn unreadable_card_is_reported_without_hiding_other_backups() {
        let tmp = Scratch::new();
        capture_card(&tmp.0, &blue(), "Blue", CardColor::Blue).unwrap();
        std::fs::write(cards_directory(&tmp.0).join("empty.mcr"), []).unwrap();
        let view = read_collection(&tmp.0).unwrap();
        assert_eq!(view.cards.len(), 1);
        assert_eq!(view.warnings.len(), 1);
        assert!(update_card_label(
            &tmp.0,
            &tmp.0.join("elsewhere.mcr"),
            "Wrong",
            CardColor::Grey
        )
        .is_err());
    }

    #[test]
    fn discovers_old_backups_inside_the_chosen_root() {
        let tmp = Scratch::new();
        let old_cards = tmp.0.join("backups");
        let original = write_backup(
            &old_cards,
            "old.mcr",
            &backup_bytes(&blue(), CardFormat::Raw),
        )
        .unwrap();
        configure_collection(
            &tmp.0.join("settings/collection.json"),
            &tmp.0,
            Some(&tmp.0),
            &tmp.0.join("missing-document-backups"),
        )
        .unwrap();
        let view = read_collection(&tmp.0).unwrap();
        assert_eq!(view.cards.len(), 1);
        assert!(original.exists());
        assert!(!saves_directory(&tmp.0).join("backups").exists());
        assert_eq!(
            std::fs::read(original).unwrap(),
            std::fs::read(&view.cards[0].path).unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn collection_rejects_redirected_subfolders() {
        let tmp = Scratch::new();
        let outside = tmp.0.join("outside");
        let root = tmp.0.join("root");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink(&outside, saves_directory(&root)).unwrap();
        assert!(prepare_collection(&root).is_err());
        assert!(read_collection(&root).is_err());
        assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
    }
}
