use super::engine::{BLOCK_SIZE, MCS_HEADER_SIZE, SLOT_COUNT};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSource {
    pub image_id: String,
    pub source_name: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMetadata {
    pub content_id: String,
    pub captured_at: String,
    pub sources: Vec<SnapshotSource>,
}

pub fn image_id(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn save_content_id(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() < MCS_HEADER_SIZE + BLOCK_SIZE
        || bytes.len() > MCS_HEADER_SIZE + SLOT_COUNT * BLOCK_SIZE
        || (bytes.len() - MCS_HEADER_SIZE) % BLOCK_SIZE != 0
    {
        return Err("Existing save is not a complete MCS file; it was left untouched.".into());
    }
    let mut hash = Sha256::new();
    hash.update(b"memcard-viewer-save-v1\0");
    hash.update(&bytes[10..30]);
    // Exclude physical directory linkage/checksums: moving a save must not invent new progress.
    hash.update(&bytes[MCS_HEADER_SIZE..]);
    Ok(format!("{:x}", hash.finalize()))
}

pub fn snapshot_directory(latest: &Path) -> PathBuf {
    latest
        .parent()
        .unwrap_or(Path::new("."))
        .join(".snapshots")
        .join(latest.file_stem().unwrap_or_default())
}

pub fn regular_directory(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(format!("{} must be a regular directory.", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir(path)
            .map_err(|e| format!("Could not create {} ({e}).", path.display())),
        Err(e) => Err(format!("Could not inspect {} ({e}).", path.display())),
    }
}

pub fn read_metadata(path: &Path) -> Result<SnapshotMetadata, String> {
    if std::fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Snapshot metadata must not be a symlink.".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Could not read snapshot metadata ({e})."))
}

// Complete temporary file + exclusive hard-link publication for immutable snapshots.
// The mutable latest copy and provenance JSON use atomic rename.
pub fn publish(path: &Path, bytes: &[u8], immutable: bool) -> Result<bool, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temp = path.with_file_name(format!(".memcard-{}-{stamp}.tmp", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("Could not prepare {} ({e}).", path.display()))?;
    let outcome = (|| -> Result<bool, String> {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if immutable {
            match std::fs::hard_link(&temp, path) {
                Ok(()) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
                Err(e) => Err(e.to_string()),
            }
        } else {
            std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
            Ok(true)
        }
    })();
    let _ = std::fs::remove_file(&temp);
    outcome.map_err(|e| {
        format!(
            "Could not publish {} ({e}). Retry Sync; previous snapshots are retained.",
            path.display()
        )
    })
}

pub fn capture(
    latest: &Path,
    bytes: &[u8],
    source: Option<SnapshotSource>,
) -> Result<bool, String> {
    let id = save_content_id(bytes)?;
    let dir = snapshot_directory(latest);
    regular_directory(dir.parent().unwrap())?;
    regular_directory(&dir)?;
    let path = dir.join(format!("{id}.mcs"));
    let created = publish(&path, bytes, true)?;
    let existing_meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !existing_meta.is_file()
        || existing_meta.file_type().is_symlink()
        || save_content_id(&std::fs::read(&path).map_err(|e| e.to_string())?)? != id
    {
        return Err(format!(
            "Snapshot {} does not match its content ID; existing files were left untouched.",
            path.display()
        ));
    }
    let metadata_path = path.with_extension("json");
    let mut metadata = match std::fs::symlink_metadata(&metadata_path) {
        Ok(_) => read_metadata(&metadata_path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SnapshotMetadata {
            content_id: id.clone(),
            captured_at: chrono::Utc::now().to_rfc3339(),
            sources: vec![],
        },
        Err(e) => return Err(e.to_string()),
    };
    if metadata.content_id != id {
        return Err("Snapshot metadata has a different content ID; Sync stopped.".into());
    }
    if let Some(source) = source {
        if !metadata.sources.contains(&source) {
            metadata.sources.push(source);
        }
    }
    publish(
        &metadata_path,
        &serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?,
        false,
    )?;
    Ok(created)
}
