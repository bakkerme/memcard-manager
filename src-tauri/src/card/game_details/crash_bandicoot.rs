//! Adapted from RyudoSynbios/game-tools-collection (MIT); see docs/formats/crash-bandicoot-us.md.
use super::{Field, Format, Profile, SaveDetails};
use unicode_normalization::UnicodeNormalization;

pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    // The dispatch bounds this to one reconstructed block. Three icon frames
    // move upstream's 0x100-relative data forward by two 0x80-byte frames.
    let end = d[4..68].iter().position(|&b| b == 0).unwrap_or(64);
    let (title, _, invalid) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    let title = title.nfkc().collect::<String>();
    let title = title.trim_end_matches('\0');
    let percent = title
        .strip_prefix("Crash (")
        .and_then(|s| s.strip_suffix("%)"));
    if d[2] != 0x13
        || invalid
        || !percent.is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("Unsupported Crash save title or icon layout.".into());
    }
    let checksum = d[0x200..0x280]
        .iter()
        .enumerate()
        .fold(0x1234_5678u32, |sum, (i, &v)| {
            sum.wrapping_add(if i < 0x7c { u32::from(v) } else { 0 })
                .rotate_left(3)
        });
    let stored = u32::from_le_bytes(d[0x27c..0x280].try_into().unwrap());
    let preview = u16::from_le_bytes(d[0x200..0x202].try_into().unwrap());
    let level = u32::from_le_bytes(d[0x204..0x208].try_into().unwrap());
    let field = |label: &str, value: String| Field {
        label: label.into(),
        value,
    };
    Ok(SaveDetails {
        game: fmt.slug.into(), title: fmt.title.into(), release: format!("NTSC-U · {code}"),
        checksum_ok: Some(checksum == stored),
        profiles: vec![Profile {
            number: 1, name: "Saved adventure".into(), empty: false, status: None, checksum_ok: None,
            fields: vec![
                field("Completion (save preview)", format!("{}%", percent.unwrap())),
                field("Current level (raw)", level.to_string()),
                field("Gems (save preview)", ((preview >> 5) & 31).to_string()),
                field("Keys (save preview)", ((preview >> 10) & 3).to_string()),
            ], records: vec![],
        }],
        notes: vec!["One adventure per card file. Completion and collectible counts are stored previews, not recomputed progress. Level names, individual gems, lives and playtime are not decoded. Source/sample validation only; no emulator save/reload validation.".into()],
    })
}

#[cfg(test)]
mod tests {
    use super::super::{decode, GameDetails};
    fn fixture() -> Vec<u8> {
        let mut d = vec![0; 8192];
        d[..4].copy_from_slice(b"SC\x13\x01");
        d[4..16].copy_from_slice(b"Crash (100%)");
        // Bytes after the title terminator may be non-text padding.
        d[17] = 0x81;
        d[0x200..0x202].copy_from_slice(&(29u16 | (26 << 5) | (2 << 10)).to_le_bytes());
        d[0x204..0x208].copy_from_slice(&29u32.to_le_bytes());
        // Independent word-by-word checksum vector; zero checksum bytes still rotate.
        let mut c = 0x12345678u32;
        for i in 0x200..0x280 {
            let v = if i < 0x27c { d[i] } else { 0 };
            c = c.wrapping_add(v.into());
            c = (c << 3) | (c >> 29);
        }
        d[0x27c..0x280].copy_from_slice(&c.to_le_bytes());
        d
    }
    #[test]
    fn crash_fields_checksum_and_immutable_input() {
        let d = fixture();
        let before = d.clone();
        for id in ["00867900", "01313863"] {
            let GameDetails::Summary(s) = decode("SCUS-94900", "BA", id, &d).unwrap() else {
                panic!()
            };
            assert_eq!(s.checksum_ok, Some(true));
            assert_eq!(
                s.profiles[0]
                    .fields
                    .iter()
                    .map(|f| f.value.as_str())
                    .collect::<Vec<_>>(),
                ["100%", "29", "26", "2"]
            );
        }
        assert_eq!(d, before);
        let mut bad = d;
        bad[0x204] ^= 1;
        let GameDetails::Summary(s) = decode("SCUS-94900", "BA", "00867900", &bad).unwrap() else {
            panic!()
        };
        assert_eq!(s.checksum_ok, Some(false));
    }
    #[test]
    fn crash_rejects_unmatched_and_malformed_layouts() {
        let d = fixture();
        for n in 0..8192 {
            assert!(decode("SCUS-94900", "BA", "00867900", &d[..n]).is_err());
        }
        for (code, region, id) in [
            ("SCES-00344", "BE", "00867900"),
            ("SCUS-94900", "BI", "00867900"),
            ("SCUS-94900", "BA", "00867900X"),
            ("SCUS-94900", "BA", ""),
        ] {
            assert!(decode(code, region, id, &d).is_err());
        }
        for o in [0, 2, 3, 4] {
            let mut bad = d.clone();
            bad[o] ^= 1;
            assert!(decode("SCUS-94900", "BA", "00867900", &bad).is_err());
        }
        let mut bad = d.clone();
        bad.push(0);
        assert!(decode("SCUS-94900", "BA", "00867900", &bad).is_err());
    }
    #[test]
    #[ignore = "Requires GAME_TOOLS_COLLECTION pointing to the pinned public source checkout; no saves redistributed"]
    fn crash_public_card() {
        use crate::card::engine::Ps1Card;
        use sha2::{Digest, Sha256};
        let path = std::path::PathBuf::from(std::env::var("GAME_TOOLS_COLLECTION").unwrap())
            .join("tests/saves/crash-bandicoot/playstation/usa.mcr");
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            "adeeaa6d1f10909cb765767f676346eedcb566e1788b818b255570412c8ba964"
        );
        let card = Ps1Card::open(&bytes, "Crash public source sample", false).unwrap();
        let view = card.view();
        assert_eq!(view.saves.len(), 2);
        for (save, expected) in view.saves.iter().zip(["1%", "2%"]) {
            let Some(GameDetails::Summary(s)) = &save.game_details else {
                panic!("{:?}", save.game_details_error)
            };
            assert_eq!(s.checksum_ok, Some(true));
            assert_eq!(s.profiles[0].fields[0].value, expected);
        }
        assert_eq!(card.save_raw(false), bytes);
    }
}
