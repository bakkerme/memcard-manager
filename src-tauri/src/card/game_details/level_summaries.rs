//! Read-only level summaries for observed Kula World and Tomb Raider layouts.
//! US Tomb Raider reuses the PAL level word and payload checksum only.
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
        status: None,
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
        "SLES-00024" | "SLUS-00152" => {
            let sum = d[0x200..0x357]
                .iter()
                .chain(&d[0x358..0xf58])
                .fold(0u8, |a, &v| a.wrapping_add(v));
            s.checksum_ok = Some(d[0x357] == sum.wrapping_neg());
            let level = i16_at(d, 0x33e);
            if code == "SLUS-00152" {
                // Same level word and payload checksum as PAL. Inventory, the
                // current-position bit, health and playtime are unverified here.
                s.release = format!("NTSC-U · {code}");
                let current = match usize::try_from(level) {
                    Ok(index) => TOMB_LEVELS.get(index).copied().map(str::to_string),
                    Err(_) => None,
                }
                .unwrap_or_else(|| format!("Unknown level ({level})"));
                s.profiles.push(profile(
                    1,
                    "Save".into(),
                    vec![field("Current level", current)],
                ));
                s.notes.push("US: current level and payload checksum only. Level-start inventory, current-position availability, health and playtime are not verified for this release. No emulator load validation.".into());
            } else {
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

    fn balance_tomb_checksum(d: &mut [u8]) {
        let sum = d[0x200..0x357]
            .iter()
            .chain(&d[0x358..0xf58])
            .fold(0u8, |a, &v| a.wrapping_add(v));
        d[0x357] = sum.wrapping_neg();
    }

    /// 8192-byte SC. Level at 0x33E; checksum covers [0x200, 0xF58) excluding 0x357.
    /// Position and level-start bytes are present so a US reader that leaked PAL fields would fail.
    fn tomb_sc(level: i16) -> Vec<u8> {
        let mut d = vec![0u8; 8192];
        d[..4].copy_from_slice(b"SC\x11\x01");
        let (title, _, invalid) = encoding_rs::SHIFT_JIS.encode("Ｔｏｍｂ　Ｒａｉｄｅｒ");
        assert!(
            !invalid,
            "full-width Tomb Raider title must encode as Shift-JIS"
        );
        d[4..4 + title.len()].copy_from_slice(&title);
        d[0x33e..0x340].copy_from_slice(&level.to_le_bytes());
        d[0x330..0x334].copy_from_slice(&0x0100_0000u32.to_le_bytes());
        let record = 0x200 + 14;
        d[record + 8] = 2;
        d[record + 9] = 1;
        d[record + 10] = 1;
        d[record + 12..record + 14].copy_from_slice(&0x1300u16.to_le_bytes());
        d[0xf58] = 0x5a;
        balance_tomb_checksum(&mut d);
        d
    }

    fn us_summary(d: &[u8]) -> game_details::SaveDetails {
        match game_details::decode("SLUS-00152", "BA", "TOMBRAID", d).unwrap() {
            game_details::GameDetails::Summary(s) => s,
            other => panic!("expected summary, got {other:?}"),
        }
    }

    #[test]
    fn tomb_raider_us_level_checksum_and_rejections() {
        let format = game_details::format("SLUS-00152").unwrap();
        assert!(format.accepts_identifier("TOMBRAID"));
        assert!(!format.accepts_identifier("TOMB2"));
        assert!(!format.accepts_identifier("TOMBRAIDx"));
        assert_eq!(format.region, "BA");

        let pyramid = tomb_sc(15);
        let before = pyramid.clone();
        let s = us_summary(&pyramid);
        assert_eq!(pyramid, before);
        assert_eq!(s.game, "tomb-raider");
        assert_eq!(s.title, "Tomb Raider");
        assert_eq!(s.release, "NTSC-U · SLUS-00152");
        assert_eq!(s.checksum_ok, Some(true));
        assert_eq!(s.profiles.len(), 1);
        assert!(s.profiles[0].records.is_empty());
        assert_eq!(
            s.profiles[0]
                .fields
                .iter()
                .map(|f| (f.label.as_str(), f.value.as_str()))
                .collect::<Vec<_>>(),
            vec![("Current level", "The Great Pyramid")]
        );
        assert_eq!(
            s.notes,
            vec!["US: current level and payload checksum only. Level-start inventory, current-position availability, health and playtime are not verified for this release. No emulator load validation.".to_string()]
        );
        for label in [
            "Current position available",
            "Level starts available",
            "Small medipacks",
            "Health",
            "Playtime",
        ] {
            assert!(
                s.profiles[0].fields.iter().all(|f| f.label != label),
                "{label}"
            );
        }

        assert_eq!(
            us_summary(&tomb_sc(0)).profiles[0].fields[0].value,
            "Lara's Home"
        );
        assert_eq!(
            us_summary(&tomb_sc(-1)).profiles[0].fields[0].value,
            "Unknown level (-1)"
        );
        assert_eq!(
            us_summary(&tomb_sc(22)).profiles[0].fields[0].value,
            "Unknown level (22)"
        );

        for offset in [0x200usize, 0x356, 0x357, 0x358, 0xf57] {
            let mut bad = pyramid.clone();
            bad[offset] ^= 1;
            assert_eq!(
                us_summary(&bad).checksum_ok,
                Some(false),
                "offset {offset:#x} is inside the checksum"
            );
        }
        for offset in [0xf58usize, 0x1fff] {
            let mut outside = pyramid.clone();
            outside[offset] ^= 1;
            let s = us_summary(&outside);
            assert_eq!(s.checksum_ok, Some(true), "offset {offset:#x}");
            assert_eq!(s.profiles[0].fields[0].value, "The Great Pyramid");
        }

        let pal = match game_details::decode("SLES-00024", "BE", "TOMBRAID", &pyramid).unwrap() {
            game_details::GameDetails::Summary(s) => s,
            other => panic!("expected summary, got {other:?}"),
        };
        assert_eq!(pal.release, "PAL · SLES-00024");
        assert_eq!(pal.checksum_ok, Some(true));
        assert_eq!(pal.profiles[0].fields[0].value, "The Great Pyramid");
        assert_eq!(pal.profiles[0].fields[1].value, "Yes");
        assert!(!pal.profiles[0].records.is_empty());
        assert_eq!(pal.profiles[0].records[0].name, "Caves");
        let pal_unknown =
            match game_details::decode("SLES-00024", "BE", "TOMBRAID", &tomb_sc(22)).unwrap() {
                game_details::GameDetails::Summary(s) => s,
                other => panic!("expected summary, got {other:?}"),
            };
        assert_eq!(pal_unknown.profiles[0].fields[0].value, "Unknown level");

        assert!(game_details::decode("SLUS-00152", "BE", "TOMBRAID", &pyramid).is_err());
        assert!(game_details::decode("SLUS-00152", "BA", "TOMB2", &pyramid).is_err());
        assert!(game_details::decode("SLUS-00152", "BA", "NOTRAID", &pyramid).is_err());
        for offset in [2usize, 3] {
            let mut bad = pyramid.clone();
            bad[offset] ^= 1;
            assert!(
                game_details::decode("SLUS-00152", "BA", "TOMBRAID", &bad).is_err(),
                "header byte {offset}"
            );
        }
        let mut bad_title = pyramid.clone();
        bad_title[4] ^= 1;
        assert!(game_details::decode("SLUS-00152", "BA", "TOMBRAID", &bad_title).is_err());
        for n in [0usize, 128, 8191] {
            assert!(game_details::decode("SLUS-00152", "BA", "TOMBRAID", &pyramid[..n]).is_err());
        }
        let mut long = pyramid.clone();
        long.push(0);
        assert_eq!(long.len(), 8193);
        assert!(game_details::decode("SLUS-00152", "BA", "TOMBRAID", &long).is_err());
    }

    #[test]
    #[ignore = "Private Tomb Raider US saves; set TOMB_RAIDER_US_MCS, TOMB_RAIDER_US_SC, TOMB_RAIDER_US_MCR and TOMB_RAIDER_US_GME"]
    fn tomb_raider_us_real_samples() {
        use crate::card::engine::Ps1Card;

        fn read_hashed(var: &str, sha: &str) -> Vec<u8> {
            let path = std::env::var(var).unwrap_or_else(|_| panic!("set {var}"));
            let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{var}: {error}"));
            assert_eq!(format!("{:x}", sha2::Sha256::digest(&bytes)), sha, "{var}");
            bytes
        }
        fn assert_pyramid(s: &game_details::SaveDetails) {
            assert_eq!(s.checksum_ok, Some(true));
            assert_eq!(s.release, "NTSC-U · SLUS-00152");
            assert_eq!(s.profiles[0].fields[0].value, "The Great Pyramid");
            assert!(s.profiles[0].records.is_empty());
            assert!(s.profiles[0]
                .fields
                .iter()
                .all(|field| field.label == "Current level"));
        }
        fn summary_from_sc(d: &[u8]) -> game_details::SaveDetails {
            match game_details::decode("SLUS-00152", "BA", "TOMBRAID", d).unwrap() {
                game_details::GameDetails::Summary(s) => s,
                other => panic!("expected summary, got {other:?}"),
            }
        }

        let mcs = read_hashed(
            "TOMB_RAIDER_US_MCS",
            "ec49622a97e1175e201783cb6372e97e1a502902a30fb263842e4d95b70c3df5",
        );
        assert!(mcs.len() >= 128 + 8192);
        assert_eq!(
            format!("{:x}", sha2::Sha256::digest(&mcs[128..128 + 8192])),
            "5699244b0e6ca09b64c71fd9790c37d00a352d3af60fbc936038976791cc06eb"
        );
        assert_pyramid(&summary_from_sc(&mcs[128..]));

        let sc = read_hashed(
            "TOMB_RAIDER_US_SC",
            "5699244b0e6ca09b64c71fd9790c37d00a352d3af60fbc936038976791cc06eb",
        );
        assert_pyramid(&summary_from_sc(&sc));

        let mcr = read_hashed(
            "TOMB_RAIDER_US_MCR",
            "7439dd491a2244c1a6012189c66c1d03c7edc117159fb58a8291b91a5fa8f3c4",
        );
        let card = Ps1Card::open(&mcr, "tomb-raider-us.mcr", false).unwrap();
        let view = card.view();
        let game_details::GameDetails::Summary(s) = view.saves[0]
            .game_details
            .clone()
            .unwrap_or_else(|| panic!("{:?}", view.saves[0].game_details_error))
        else {
            panic!("expected summary")
        };
        assert_eq!(view.saves[0].prod_code, "SLUS-00152");
        assert_eq!(view.saves[0].region_raw, "BA");
        assert_eq!(view.saves[0].identifier, "TOMBRAID");
        assert_pyramid(&s);
        assert_eq!(card.save_raw(false), mcr);

        let gme = read_hashed(
            "TOMB_RAIDER_US_GME",
            "273fedac22c71c0bbc1b621f49a1d822b02698d24cc9fbebed7515ea66a7c73f",
        );
        let card = Ps1Card::open(&gme, "tomb-raider-1996.381.gme", false).unwrap();
        let view = card.view();
        let game_details::GameDetails::Summary(s) = view.saves[0]
            .game_details
            .clone()
            .unwrap_or_else(|| panic!("{:?}", view.saves[0].game_details_error))
        else {
            panic!("expected summary")
        };
        assert_eq!(view.saves[0].prod_code, "SLUS-00152");
        assert_pyramid(&s);
    }
}
