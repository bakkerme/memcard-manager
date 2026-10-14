//! Read-only level summaries for the observed PAL Kula World and Tomb Raider layouts.
use super::{Field, Format, Profile, SaveDetails};
use unicode_normalization::UnicodeNormalization;

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
fn i16_at(d: &[u8], o: usize) -> i16 {
    i16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
fn i32_at(d: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn field(label: &str, value: impl ToString) -> Field {
    Field {
        label: label.into(),
        value: value.to_string(),
    }
}
fn profile(n: u8, name: String, fields: Vec<Field>) -> Profile {
    Profile {
        number: n,
        name,
        empty: false,
        checksum_ok: None,
        fields,
        records: vec![],
    }
}
fn title(d: &[u8]) -> Result<String, String> {
    let end = d[4..68].iter().position(|&b| b == 0).unwrap_or(64);
    let (text, _, invalid) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if invalid {
        return Err("Invalid save title encoding.".into());
    }
    Ok(text.nfkc().collect::<String>().trim().into())
}
const TOMB_LEVELS: [&str; 22] = [
    "Lara's Home",
    "Caves",
    "City of Vilcabamba",
    "Lost Valley",
    "Tomb of Qualopec",
    "St. Francis' Folly",
    "Colosseum",
    "Palace Midas",
    "The Cistern",
    "Tomb of Tihocan",
    "City of Khamoon",
    "Obelisk of Khamoon",
    "Sanctuary of the Scion",
    "Natla's Mines",
    "Atlantis",
    "The Great Pyramid",
    "Cut Scene 1",
    "Cut Scene 2",
    "Cut Scene 3",
    "Cut Scene 4",
    "Title",
    "Current Position",
];

pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    if d[2] != 0x11
        || title(d)?
            != if code == "SCES-01000" {
                "KULA WORLD"
            } else {
                "Tomb Raider"
            }
    {
        return Err("Unsupported save title or layout revision.".into());
    }
    let mut s = SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!("PAL · {code}"),
        checksum_ok: None,
        profiles: vec![],
        notes: vec![],
    };
    match code {
        "SCES-01000" => {
            for index in 0..4 {
                let o = 0x378 + index * 0x270;
                let occupied = i16_at(d, o + 0x268);
                let mut p = profile((index + 1) as u8, format!("Slot {}", index + 1), vec![]);
                if occupied == 0 {
                    p.empty = true;
                } else if occupied == 1 {
                    let mode = i16_at(d, o + 0x26a);
                    let (mode_name, unit) = match mode {
                        0 => ("Arcade", "points"),
                        1 => ("Copycat", "seconds"),
                        2 => ("Time trial", "seconds"),
                        _ => ("Unknown", "raw"),
                    };
                    let stage = i16_at(d, o + 0x26e);
                    let world = i16_at(d, o + 0x264);
                    let level = i16_at(d, o + 0x262);
                    let display_level = if (stage == 0 || stage == 1) && world >= 0 && level >= 0 {
                        Some(
                            i32::from(world) * if stage == 0 { 15 } else { 2 }
                                + i32::from(level)
                                + if stage == 0 { 1 } else { 151 },
                        )
                    } else {
                        None
                    };
                    p.fields = vec![
                        field("Mode", mode_name),
                        field("Score", format!("{} {unit}", i32_at(d, o + 0x258))),
                        field(
                            "Level",
                            display_level.map_or_else(
                                || format!("Unknown ({world}:{level})"),
                                |v| v.to_string(),
                            ),
                        ),
                        field("Fruit", u16_at(d, o + 0x260)),
                        field("Elapsed seconds", i32_at(d, o + 0x25c)),
                    ];
                } else {
                    p.fields.push(field("Occupancy marker (raw)", occupied));
                }
                s.profiles.push(p);
            }
            s.notes.push("No game checksum is established. Time trial persistence and some progress flags remain unqualified.".into());
        }
        "SLES-00024" => {
            let sum = d[0x200..0x357]
                .iter()
                .chain(&d[0x358..0xf58])
                .fold(0u8, |a, &v| a.wrapping_add(v));
            s.checksum_ok = Some(d[0x357] == sum.wrapping_neg());
            let level = i16_at(d, 0x33e);
            let current = TOMB_LEVELS
                .get(level as usize)
                .copied()
                .unwrap_or("Unknown level");
            let position = u32_at(d, 0x330) & 0x0100_0000 != 0;
            let mut fields = vec![
                field("Current level", current),
                field(
                    "Current position available",
                    if position { "Yes" } else { "No" },
                ),
            ];
            let available = (1..=15)
                .filter(|&i| u16_at(d, 0x200 + i * 14 + 12) & 0x100 != 0)
                .count();
            fields.push(field("Level starts available", available));
            s.profiles.push(profile(1, "Save".into(), fields));
            for (index, name) in TOMB_LEVELS.iter().enumerate().take(16).skip(1) {
                let o = 0x200 + index * 14;
                let flags = u16_at(d, o + 12);
                if flags & 0x100 == 0 {
                    continue;
                }
                let mut items = vec![
                    field("Small medipacks", d[o + 8]),
                    field("Large medipacks", d[o + 9]),
                    field("Scion pieces", d[o + 10]),
                ];
                let weapons = [
                    ("Pistols", 0x200),
                    ("Magnums", 0x400),
                    ("Uzis", 0x800),
                    ("Shotgun", 0x1000),
                ];
                for (label, bit) in weapons {
                    items.push(field(
                        label,
                        if flags & bit != 0 {
                            "Owned"
                        } else {
                            "Not owned"
                        },
                    ));
                }
                s.profiles[0].records.push(super::Record {
                    name: (*name).into(),
                    fields: items,
                });
            }
            s.notes.push("Inventory records describe level starts, not the current checkpoint. Current health and playtime are not decoded.".into());
        }
        _ => return Err("Unsupported release.".into()),
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use crate::card::game_details;
    use sha2::Digest;

    #[test]
    #[ignore = "Requires the ignored overnight snapshot and PS1_RESEARCH_SNAPSHOT"]
    fn exact_release_research_samples() {
        let root = std::path::PathBuf::from(
            std::env::var("PS1_RESEARCH_SNAPSHOT").expect("snapshot path"),
        );
        for (code, region, identifier, sha) in [
            (
                "SCES-01000",
                "BE",
                "KULA",
                "539ecfd372fff3bc2efc054e01ad69931e8ebe4c3e1aa3e7cd1de5f28a5ec2c1",
            ),
            (
                "SLES-00024",
                "BE",
                "TOMBRAID",
                "a23cdfbc1f4f4eba68343c6c25521975fa679c8e6a4c6c3b9c9dcb531745055e",
            ),
        ] {
            let mcs =
                std::fs::read(root.join("recovery/inputs").join(format!("{sha}.mcs"))).unwrap();
            assert_eq!(format!("{:x}", sha2::Sha256::digest(&mcs)), sha, "{code}");
            assert!(mcs.len() >= 128, "{code}");
            let d = &mcs[128..];
            let game_details::GameDetails::Summary(s) =
                game_details::decode(code, region, identifier, d)
                    .unwrap_or_else(|e| panic!("{code}: {e}"))
            else {
                panic!("expected summary")
            };
            if code == "SCES-01000" {
                assert_eq!(s.checksum_ok, None);
                assert_eq!(s.profiles[0].fields[0].value, "Arcade");
                assert_eq!(s.profiles[0].fields[1].value, "59315 points");
                assert_eq!(s.profiles[0].fields[2].value, "11");
            } else {
                assert_eq!(s.checksum_ok, Some(true));
                assert_eq!(s.profiles[0].fields[0].value, "City of Vilcabamba");
                assert!(!s.profiles[0].records.is_empty());
            }
            assert!(game_details::decode(code, "BA", identifier, d).is_err());
            assert!(game_details::decode(code, region, "BAD", d).is_err());
            assert!(game_details::decode(code, region, identifier, &d[..d.len() - 1]).is_err());
        }
    }
}
