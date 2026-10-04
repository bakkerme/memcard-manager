use std::{
    io::Write,
    path::{Path, PathBuf},
};

const STEM_MAX: usize = 48;

pub fn backup_dir(documents: impl AsRef<Path>) -> PathBuf {
    documents.as_ref().join("memcard-viewer").join("backups")
}

pub fn sanitize_stem(source_name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in source_name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !out.is_empty() && !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.len() > STEM_MAX {
        out.truncate(STEM_MAX);
        while out.ends_with('-') {
            out.pop();
        }
    }
    out
}

pub fn backup_stem(source_name: &str, usb: bool) -> String {
    let stem = sanitize_stem(source_name);
    if !stem.is_empty() {
        stem
    } else if usb {
        "adaptor".into()
    } else {
        "card".into()
    }
}

pub fn backup_filename(stem: &str, timestamp: &str) -> String {
    format!("{stem}-{timestamp}.mcr")
}

pub fn local_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d-%H%M%S").to_string()
}

pub fn display_path(path: &Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Ok(rest) = path.strip_prefix(Path::new(&home)) {
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}

pub fn write_backup(dir: &Path, filename: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create backups folder ({e})."))?;
    let path = dir.join(filename);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("card");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("mcr");
    for n in 1..=1000 {
        let candidate = if n == 1 {
            path.clone()
        } else {
            dir.join(format!("{stem}-{n}.{ext}"))
        };
        // Reserve the name atomically: never truncate a previous backup or follow a symlink.
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Could not create backup ({e}).")),
        };
        if let Err(e) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(&candidate);
            return Err(format!("Could not write backup ({e})."));
        }
        return Ok(candidate);
    }
    Err("Too many backups share this filename. Try again with a new timestamp.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{backup_bytes, CardFormat, Ps1Card, CARD_SIZE};
    use std::path::Path;

    fn blue() -> Vec<u8> {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../blue.mcr"))
            .expect("blue.mcr next to src-tauri")
    }

    #[test]
    fn sanitizes_adaptor_and_file_stems() {
        assert_eq!(sanitize_stem("blue"), "blue");
        assert_eq!(
            sanitize_stem("PLAYSTATION(R)3 Memorycard Adaptor"),
            "playstation-r-3-memorycard-adaptor"
        );
        assert_eq!(sanitize_stem("  "), "");
        assert_eq!(sanitize_stem("Foo---Bar"), "foo-bar");
        assert_eq!(backup_stem("", true), "adaptor");
        assert_eq!(backup_stem("", false), "card");
        assert_eq!(backup_stem("blue.mcr", false), "blue-mcr");
    }

    #[test]
    fn names_timestamped_raw_mcr() {
        assert_eq!(
            backup_filename("blue", "2026-09-17-062300"),
            "blue-2026-09-17-062300.mcr"
        );
        assert_eq!(
            backup_dir("/Users/me/Documents"),
            PathBuf::from("/Users/me/Documents/memcard-viewer/backups")
        );
    }

    #[test]
    fn write_backup_round_trips_blue() {
        let source = Ps1Card::open(&blue(), "blue.mcr", false).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "memcard-viewer-backup-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let filename = backup_filename("blue", "2026-09-17-062300");
        let path = write_backup(&dir, &filename, &backup_bytes(&source, CardFormat::Raw)).unwrap();
        assert_eq!(path.file_name().unwrap(), filename.as_str());
        let written = std::fs::read(&path).unwrap();
        assert_eq!(written.len(), CARD_SIZE);
        let opened = Ps1Card::open(&written, "round.mcr", false).unwrap().view();
        assert_eq!(opened.saves.len(), 12);
        assert_eq!(opened.format, CardFormat::Raw);

        let again = write_backup(&dir, &filename, b"second").unwrap();
        assert_eq!(again.file_name().unwrap(), "blue-2026-09-17-062300-2.mcr");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn concurrent_backups_never_overwrite_each_other() {
        let dir = std::env::temp_dir().join(format!(
            "memcard-backup-race-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers: Vec<_> = (0..8u8)
            .map(|value| {
                let dir = dir.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    (write_backup(&dir, "same.mcr", &[value]).unwrap(), value)
                })
            })
            .collect();
        for worker in workers {
            let (path, value) = worker.join().unwrap();
            assert_eq!(std::fs::read(path).unwrap(), vec![value]);
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 8);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn display_path_tildes_home() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/me".into());
        let path = PathBuf::from(&home)
            .join("Documents")
            .join("memcard-viewer")
            .join("backups")
            .join("blue-2026-09-17-062300.mcr");
        if std::env::var("HOME").is_ok() {
            assert_eq!(
                display_path(&path),
                "~/Documents/memcard-viewer/backups/blue-2026-09-17-062300.mcr"
            );
        }
    }
}
