//! Narrow read-only summaries from the October 2026 exact-release research.
//! Offsets are reconstructed SC payload offsets. See docs/formats/overnight-readers.md.
use super::{Field, Format, Profile, SaveDetails};

fn word(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn half(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
fn field(label: &str, value: impl ToString) -> Field {
    Field {
        label: label.into(),
        value: value.to_string(),
    }
}
fn profile(number: u8, name: String, fields: Vec<Field>) -> Profile {
    Profile {
        number,
        name,
        empty: false,
        checksum_ok: None,
        fields,
        records: vec![],
    }
}
fn title(d: &[u8]) -> Result<String, String> {
    let end = d[4..68].iter().position(|&b| b == 0).unwrap_or(64);
    let (decoded, _, errors) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if errors {
        return Err("Invalid save title encoding.".into());
    }
    use unicode_normalization::UnicodeNormalization;
    Ok(decoded.nfkc().collect())
}

pub(super) fn decode(f: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    // These exact releases have an observed SC revision byte 0x13. Other
    // revisions must be investigated before their offsets are used.
    if d[2] != 0x13 {
        return Err("Unsupported save layout revision.".into());
    }
    let title_text = title(d)?;
    let marker = match code {
        "SLES-01219" => "Azure Dreams",
        "SLUS-01411" => "Yu-Gi-Oh! Forbidden Memories",
        "SLUS-00585" => "KLONOA",
        _ => return Err("Unsupported release.".into()),
    };
    if !title_text.starts_with(marker) {
        return Err("Unexpected save title for this release.".into());
    }
    let mut result = SaveDetails {
        game: f.slug.into(),
        title: f.title.into(),
        release: format!(
            "{} · {code}",
            if f.region == "BE" { "PAL" } else { "NTSC-U" }
        ),
        checksum_ok: None,
        profiles: vec![],
        notes: vec![],
    };
    match code {
        "SLES-01219" => {
            let mut check = 0u32;
            for o in (0x208..0x6000).step_by(4) {
                check ^= word(d, o);
            }
            result.checksum_ok = Some(word(d, 0x204) == check);
            let seconds = word(d, 0x21c) / 60;
            let money = word(d, 0x224);
            let town_money = word(d, 0x2d5c);
            result.profiles.push(profile(
                1,
                "Saved game".into(),
                vec![
                    field(
                        "Playtime (hh:mm:ss)",
                        format!(
                            "{}:{:02}:{:02}",
                            (seconds / 3600).min(999),
                            seconds / 60 % 60,
                            seconds % 60
                        ),
                    ),
                    field("Money (G)", money),
                    field("Buildings", word(d, 0x228)),
                    field("Game count", word(d, 0x22c).min(999)),
                ],
            ));
            if money != town_money {
                result
                    .notes
                    .push("Preview and town money copies differ; the preview may be stale.".into());
            }
            result.notes.push("Preview counts and elapsed time only; inventory and town progression are not decoded.".into());
        }
        "SLUS-01411" => {
            for (index, base) in [0x200, 0x880].into_iter().enumerate() {
                let deck = (0..40).map(|i| half(d, base + 2 * i)).collect::<Vec<_>>();
                let unknown = deck.iter().filter(|&&id| id > 722).count();
                let deck_count = deck.iter().filter(|&&id| (1..=722).contains(&id)).count();
                let chest_count: u32 = d[base + 0x50..base + 0x322].iter().map(|&v| v as u32).sum();
                let mut p = profile(
                    (index + 1) as u8,
                    if index == 0 {
                        "Primary bank"
                    } else {
                        "Backup bank"
                    }
                    .into(),
                    vec![
                        field("Deck cards", deck_count),
                        field("Cards in chest", chest_count),
                        field("Owned cards", chest_count + deck_count as u32),
                        field("StarChips", word(d, base + 0x5e0)),
                        field("Save sequence (raw)", word(d, base + 0x404)),
                    ],
                );
                if unknown > 0 {
                    p.fields.push(field("Unknown deck IDs", unknown));
                }
                result.profiles.push(p);
            }
            result.notes.push("Banks are shown separately. Retail load tries the primary first, then offers a backup retry; bank integrity is not checked by this summary.".into());
            result
                .notes
                .push("Card names, campaign state and password history are not decoded.".into());
        }
        "SLUS-00585" => {
            for i in 0..3 {
                let base = 0x204 + i * 0xf8;
                let history = half(d, base + 0xe2) as usize;
                let selected = if history < 4 { history } else { 0 };
                let o = base + selected * 0x38;
                let mut sum = 0u32;
                for j in (0..0x34).step_by(4) {
                    sum = sum.wrapping_shl(1).wrapping_add(word(d, o + j));
                }
                let ok = sum == word(d, o + 0x34);
                let stage = half(d, o + 2) as i16;
                let rescued: u32 = d[o + 24..o + 36].iter().map(|&v| v as u32).sum();
                let label = if stage == -1 {
                    "No data".into()
                } else if (0..14).contains(&stage) {
                    let code = [0u8, 1, 3, 4, 6, 7, 9, 10, 12, 13, 15, 16, 20, 21][stage as usize];
                    if d[o + 6] & 1 != 0 {
                        "Vision clear".into()
                    } else if code == 20 {
                        "Final Vision".into()
                    } else if code == 21 {
                        "Vision 8 (stage 21)".into()
                    } else {
                        format!("Vision {}-{}", code / 3 + 1, code % 3 + 1)
                    }
                } else {
                    format!("Unknown stage ({stage})")
                };
                let control = match d[base + 0xf4] {
                    0 => "A",
                    1 => "B",
                    2 => "C",
                    _ => "Unknown",
                };
                let sound = match d[base + 0xf7] {
                    1 => "Mono",
                    2 => "Stereo",
                    _ => "Unknown",
                };
                let mut p = profile(
                    (i + 1) as u8,
                    format!("Profile {}", i + 1),
                    vec![
                        field("Saved stage", label),
                        field("Lives", d[o + 4]),
                        field("Rescued", rescued),
                        field("Controls", control),
                        field("Sound", sound),
                    ],
                );
                p.empty = stage == -1;
                p.checksum_ok = Some(ok);
                result.profiles.push(p);
            }
            result.notes.push("The selected record checksum covers its preview record only; options and other history records are outside this check.".into());
            result
                .notes
                .push("Per-stage flags and full progression are not decoded.".into());
        }
        _ => unreachable!(),
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::game_details;

    fn sample(code: &str, name: &str) -> Vec<u8> {
        let f = game_details::format(code).unwrap();
        let mut d = vec![0; f.blocks * 8192];
        d[..4].copy_from_slice(&[b'S', b'C', 0x13, f.blocks as u8]);
        let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(name);
        assert!(!invalid);
        d[4..4 + encoded.len()].copy_from_slice(&encoded);
        d
    }

    #[test]
    fn azure_bounded_preview_and_integrity() {
        let mut d = sample("SLES-01219", "Azure Dreams");
        d[0x21c..0x220].copy_from_slice(&(60u32 * 3661).to_le_bytes());
        d[0x224..0x228].copy_from_slice(&9470u32.to_le_bytes());
        d[0x2d5c..0x2d60].copy_from_slice(&9470u32.to_le_bytes());
        d[0x228..0x22c].copy_from_slice(&2u32.to_le_bytes());
        let mut check = 0u32;
        for o in (0x208..0x6000).step_by(4) {
            check ^= word(&d, o);
        }
        d[0x204..0x208].copy_from_slice(&check.to_le_bytes());
        let before = d.clone();
        let s = game_details::decode("SLES-01219", "BE", "00", &d).unwrap();
        let game_details::GameDetails::Summary(s) = s else {
            panic!("expected summary")
        };
        assert_eq!(s.checksum_ok, Some(true));
        assert_eq!(s.profiles[0].fields[0].value, "1:01:01");
        assert_eq!(s.profiles[0].fields[1].value, "9470");
        assert_eq!(d, before);
        d[0x2d5c] ^= 1;
        assert_eq!(
            decode(
                &game_details::format("SLES-01219").unwrap(),
                "SLES-01219",
                &d
            )
            .unwrap()
            .checksum_ok,
            Some(false)
        );
        assert!(game_details::decode("SLES-01219", "BA", "00", &d).is_err());
        assert!(game_details::decode("SLES-01219", "BE", "01", &d).is_err());
        d[2] = 0x11;
        assert!(game_details::decode("SLES-01219", "BE", "00", &d).is_err());
    }

    #[test]
    fn ygo_banks_do_not_merge_and_unknown_ids_survive() {
        let mut d = sample("SLUS-01411", "Yu-Gi-Oh! Forbidden Memories");
        d[0x200..0x202].copy_from_slice(&8u16.to_le_bytes());
        d[0x202..0x204].copy_from_slice(&800u16.to_le_bytes());
        d[0x250 + 7] = 2;
        d[0x200 + 0x5e0..0x200 + 0x5e4].copy_from_slice(&1234u32.to_le_bytes());
        d[0x880..0x882].copy_from_slice(&167u16.to_le_bytes());
        let s = decode(
            &game_details::format("SLUS-01411").unwrap(),
            "SLUS-01411",
            &d,
        )
        .unwrap();
        assert_eq!(s.checksum_ok, None);
        assert_eq!(s.profiles[0].fields[0].value, "1");
        assert_eq!(s.profiles[0].fields[2].value, "3");
        assert_eq!(s.profiles[0].fields[3].value, "1234");
        assert_eq!(s.profiles[0].fields.last().unwrap().value, "1");
        assert_eq!(s.profiles[1].fields[0].value, "1");
        assert_eq!(s.profiles[1].fields[2].value, "1");
    }

    #[test]
    fn klonoa_final_stage_labels_use_sparse_codes() {
        let mut d = sample("SLUS-00585", "KLONOA  -Door To Phantomile-");
        for (stage, label) in [(12u16, "Final Vision"), (13, "Vision 8 (stage 21)")] {
            d[0x206..0x208].copy_from_slice(&stage.to_le_bytes());
            let s = decode(
                &game_details::format("SLUS-00585").unwrap(),
                "SLUS-00585",
                &d,
            )
            .unwrap();
            assert_eq!(s.profiles[0].fields[0].value, label);
        }
    }

    #[test]
    fn klonoa_selected_history_record_and_bounds() {
        let mut d = sample("SLUS-00585", "KLONOA  -Door To Phantomile-");
        let base = 0x204;
        d[base + 0xe2..base + 0xe4].copy_from_slice(&1u16.to_le_bytes());
        d[base + 0xf4] = 2;
        d[base + 0xf7] = 1;
        let record = base + 0x38;
        d[record + 2..record + 4].copy_from_slice(&2u16.to_le_bytes());
        d[record + 4] = 27;
        d[record + 24..record + 36].fill(1);
        let mut check = 0u32;
        for j in (0..0x34).step_by(4) {
            check = check.wrapping_shl(1).wrapping_add(word(&d, record + j));
        }
        d[record + 0x34..record + 0x38].copy_from_slice(&check.to_le_bytes());
        let s = decode(
            &game_details::format("SLUS-00585").unwrap(),
            "SLUS-00585",
            &d,
        )
        .unwrap();
        assert_eq!(s.profiles[0].checksum_ok, Some(true));
        assert_eq!(s.profiles[0].fields[0].value, "Vision 2-1");
        assert_eq!(s.profiles[0].fields[2].value, "12");
        assert_eq!(s.profiles[0].fields[3].value, "C");
        assert_eq!(s.profiles[0].fields[4].value, "Mono");
        d[record + 24] ^= 1;
        let s = decode(
            &game_details::format("SLUS-00585").unwrap(),
            "SLUS-00585",
            &d,
        )
        .unwrap();
        assert_eq!(s.profiles[0].checksum_ok, Some(false));
    }
}

#[cfg(test)]
mod local_corpus {
    use super::*;
    use crate::card::game_details;

    #[test]
    #[ignore = "Requires the ignored overnight snapshot and PS1_RESEARCH_SNAPSHOT"]
    fn exact_release_research_samples() {
        let root = std::path::PathBuf::from(
            std::env::var("PS1_RESEARCH_SNAPSHOT").expect("snapshot path"),
        );
        for (code, region, identifier, sha) in [
            (
                "SLES-01219",
                "BE",
                "00",
                "c6089815d0306db87f690b738414ef1f65f29b2b8efe1f243375755b53f49f15",
            ),
            (
                "SLUS-01411",
                "BA",
                "-YUGIOH",
                "5ad949729d95b049e7702d99237900eba0766f627e64d351a59f937575346d8b",
            ),
            (
                "SLUS-00585",
                "BA",
                "KLONOA",
                "3182ac609d86d6b55c2bed2795fcd7c3d1cbc8d34687b4240b21a918b2135c08",
            ),
        ] {
            let path = root.join("recovery/inputs").join(format!("{sha}.mcs"));
            let mcs = std::fs::read(&path).unwrap();
            let d = &mcs[128..];
            let game_details::GameDetails::Summary(s) =
                game_details::decode(code, region, identifier, d).unwrap()
            else {
                panic!("expected summary")
            };
            match code {
                "SLES-01219" => {
                    assert_eq!(s.checksum_ok, Some(true));
                    assert_eq!(s.profiles[0].fields[0].value, "4:34:41");
                    assert_eq!(s.profiles[0].fields[1].value, "13470");
                }
                "SLUS-01411" => {
                    assert_eq!(s.profiles[0].fields[3].value, "1");
                    assert_eq!(half(d, 0x200), 8);
                }
                "SLUS-00585" => {
                    assert_eq!(s.profiles[0].fields[0].value, "Vision 2-1");
                    assert_eq!(s.profiles[0].fields[2].value, "10");
                }
                _ => unreachable!(),
            }
        }
    }
}
