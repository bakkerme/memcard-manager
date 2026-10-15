//! Additional exact-release read-only archive summaries. See docs/formats/overnight-readers.md.
use super::{Field, Format, Profile, SaveDetails};

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn i32_at(d: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn f(label: &str, value: impl ToString) -> Field {
    Field {
        label: label.into(),
        value: value.to_string(),
    }
}
fn p(number: u8, name: String, fields: Vec<Field>) -> Profile {
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
    use unicode_normalization::UnicodeNormalization;
    let end = d[4..68].iter().position(|&b| b == 0).unwrap_or(64);
    let (decoded, _, errors) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if errors {
        return Err("Invalid save title encoding.".into());
    }
    Ok(decoded.nfkc().collect::<String>().trim().into())
}
fn clock(seconds: u32) -> String {
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    let (revision, title_prefix) = match code {
        "SLES-03900" => (0x13, "DCB["),
        "SCUS-94570" => (0x12, "CRASH BASH"),
        "SCES-00934" => (0x11, "PAC-MAN WORLD"),
        "SLUS-00603" => (0x11, "MEGAMAN LEGENDS["),
        _ => return Err("Unsupported release.".into()),
    };
    if d[2] != revision || !title(d)?.starts_with(title_prefix) {
        return Err("Unsupported save title or layout revision.".into());
    }
    let mut s = SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!(
            "{} · {code}",
            if fmt.region == "BE" { "PAL" } else { "NTSC-U" }
        ),
        checksum_ok: None,
        profiles: vec![],
        notes: vec![],
    };
    match code {
        "SLES-03900" => {
            if u16_at(d, 0x216) != 0x2774 {
                return Err("Unknown Digimon profile length.".into());
            }
            let bytes = &d[0x200..0x2974];
            let xor = bytes.iter().fold(0u8, |a, &b| a ^ b);
            let sum = bytes.iter().fold(0u8, |a, &b| a.wrapping_add(b));
            s.checksum_ok = Some(d[0x2974] == xor && d[0x2975] == sum);
            let ticks = i32_at(d, 0x224);
            let time = if ticks >= 0 {
                clock((ticks as u32) / 60)
            } else {
                format!("Unknown ({ticks} ticks)")
            };
            let area = [
                "Beginner City",
                "Flame City",
                "Jungle City",
                "Igloo City",
                "Junk City",
                "Dark City",
                "Desert Island",
                "Pyramid City",
                "Sky City",
                "Steep Road",
                "Wiseman Tower",
                "Infinity Tower",
                "Mega Area",
                "Giga Area",
                "Giga Area",
                "Tera Area",
            ];
            let area_name = area
                .get(d[0x20e] as usize)
                .map(|v| (*v).to_string())
                .unwrap_or_else(|| format!("Unknown area ({})", d[0x20e]));
            let seen = u16_at(d, 0x212);
            let computed_seen = d[0x16b2..0x17df].iter().filter(|&&v| v & 0x40 != 0).count();
            let owned: u32 = d[0x16b2..0x17df].iter().map(|&v| u32::from(v & 7)).sum();
            let player = String::from_utf8_lossy(&d[0x200..0x20d])
                .trim_end_matches('\0')
                .trim()
                .to_string();
            s.profiles.push(p(
                1,
                if player.is_empty() {
                    "Player".into()
                } else {
                    player
                },
                vec![
                    f("Playtime (hh:mm:ss)", time),
                    f("Area", area_name),
                    f("Cards seen", format!("{seen} / 301")),
                    f("Owned copies", owned),
                    f(
                        "Game completion (preview)",
                        format!(
                            "{}.{:01}%",
                            u32::from(u16_at(d, 0x214)) * 1000 / 166 / 10,
                            u32::from(u16_at(d, 0x214)) * 1000 / 166 % 10
                        ),
                    ),
                    f("Save count", d[0x20d]),
                ],
            ));
            if computed_seen != usize::from(seen) {
                s.notes
                    .push("Stored and computed seen-card counts differ.".into());
            }
            s.notes
                .push("Card names, deck and partner layout are not decoded.".into());
        }
        "SCUS-94570" => {
            let check = (0x200..0x850)
                .step_by(4)
                .fold(0u32, |a, o| a.wrapping_add(u32_at(d, o)));
            s.checksum_ok = Some(check == u32_at(d, 0x85c));
            for index in 0..4 {
                let o = 0x200 + index * 0x194;
                let state = o + 8;
                let raw_name = &d[o + 0x20..o + 0x28];
                let name = if raw_name.iter().all(u8::is_ascii) {
                    String::from_utf8_lossy(raw_name)
                        .trim_end_matches([' ', '\0'])
                        .to_string()
                } else {
                    String::new()
                };
                let trophy = &d[state + 0xe4..state + 0xe4 + 33];
                let relic = &d[state + 0x105..state + 0x105 + 33];
                let gem = &d[state + 0x126..state + 0x126 + 33];
                let crystal = &d[state + 0x147..state + 0x147 + 33];
                let bonus = &d[state + 0x168..state + 0x168 + 33];
                let count = |a: &[u8]| a.iter().filter(|&&v| v != 0).count();
                let percent = count(trophy)
                    + count(relic)
                    + count(gem)
                    + count(crystal)
                    + 2 * relic.iter().filter(|&&v| v >= 2).count()
                    + 8 * count(bonus);
                let mut item = p(
                    (index + 1) as u8,
                    if name.is_empty() {
                        format!("Profile {}", index + 1)
                    } else {
                        name
                    },
                    vec![
                        f("Completion (preview)", format!("{percent}%")),
                        f(
                            "Trophies",
                            trophy.iter().map(|&v| u32::from(v)).sum::<u32>(),
                        ),
                        f("Relic groups", count(relic)),
                        f("Gem groups", count(gem)),
                        f("Crystal groups", count(crystal)),
                    ],
                );
                item.empty =
                    d[o..o + 8].iter().all(|&v| v == 0) && raw_name.iter().all(|&v| v == 0);
                s.profiles.push(item);
            }
            s.notes.push("Completion follows the game preview formula; individual award names and unusual byte domains remain unknown.".into());
        }
        "SCES-00934" => {
            if u32_at(d, 0x200) != 0x784 {
                return Err("Unknown Pac-Man profile length.".into());
            }
            let check: u32 = d[0x208..0x98c].iter().map(|&v| u32::from(v)).sum();
            s.checksum_ok = Some(check == u32_at(d, 0x204));
            for index in 0..3 {
                let o = 0x20c + index * 0x280;
                let lives = i32_at(d, o + 0xc);
                let ticks = i32_at(d, o + 0x24c);
                let mut item = p((index + 1) as u8, format!("Profile {}", index + 1), vec![]);
                item.empty = lives < 0;
                if !item.empty {
                    item.fields = vec![
                        f("Lives", lives.wrapping_add(1)),
                        f(
                            "Playtime (hh:mm:ss)",
                            if ticks >= 0 {
                                clock(ticks as u32 / 300)
                            } else {
                                "Unknown".into()
                            },
                        ),
                    ];
                    let friends = (0..6)
                        .filter(|&j| u16_at(d, o + 0x5a + j * 2) & 2 != 0)
                        .count();
                    item.fields.push(f("Friends rescued", friends));
                    const SCORE_INDICES: [usize; 23] = [
                        236, 230, 232, 234, 222, 224, 228, 30, 32, 34, 36, 38, 40, 42, 44, 52, 54,
                        56, 58, 46, 48, 50, 238,
                    ];
                    let score: u32 = SCORE_INDICES
                        .iter()
                        .map(|&j| u32::from(u16_at(d, o + 0x56 + j * 2)) * 5)
                        .sum();
                    item.fields.push(f("Score", score));
                }
                s.profiles.push(item);
            }
            s.notes.push("Percentage and level names are not decoded; timer values are from the PAL load preview.".into());
        }
        "SLUS-00603" => {
            let title = title(d)?;
            let clock_part = title.rsplit_once(']').map(|(_, v)| v).unwrap_or("");
            if clock_part.len() != 8
                || !clock_part.bytes().enumerate().all(|(i, b)| {
                    if i == 2 || i == 5 {
                        b == b':'
                    } else {
                        b.is_ascii_digit()
                    }
                })
            {
                return Err("Unknown Mega Man Legends title clock.".into());
            }
            let ticks = u32_at(d, 0x180);
            let owned = (1..=32)
                .filter(|&id| {
                    let flag = 0x4ff + id;
                    d[0x380 + flag / 8] & (0x80 >> (flag % 8)) != 0
                })
                .count();
            let slots = if d[0x380 + 0x526 / 8] & (0x80 >> (0x526 % 8)) != 0 {
                3
            } else {
                2
            };
            let equipped = d[0x208..0x208 + slots]
                .iter()
                .filter(|&&id| (1..=32).contains(&id))
                .count();
            s.profiles.push(p(
                1,
                "Saved game".into(),
                vec![
                    f(
                        "Playtime (hh:mm:ss)",
                        format!(
                            "{:02}:{:02}:{:02}",
                            ticks / 108000 % 100,
                            ticks / 1800 % 60,
                            ticks / 30 % 60
                        ),
                    ),
                    f("Zenny", u64::from(u32_at(d, 0x184)) * 10),
                    f("Owned buster parts", owned),
                    f("Equipped buster parts", equipped),
                    f("Stage ID", format!("0x{:02X}", d[0x194])),
                    f("Area ID", format!("0x{:02X}", d[0x195])),
                ],
            ));
            s.notes.push("Location and remaining equipment names are unknown. Title clock and gameplay timer may differ; gameplay timer is shown.".into());
            s.notes
                .push("Game integrity is not checked by this summary.".into());
        }
        _ => unreachable!(),
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
                "SLES-03900",
                "BE",
                "_A",
                "afec70c02641f15e24356b0552b3edc0711a2a2de2ede37e66369e476c685793",
            ),
            (
                "SCUS-94570",
                "BA",
                "",
                "6e1d6c13f634ef876b4b10611363d5f7f9f0bdfb0024685dd460b00097876643",
            ),
            (
                "SCES-00934",
                "BE",
                "PACMAN20",
                "26ecfcf92a30a6b3bb7848f2f1a7231604733b8ec5f5b30267d990efa9485044",
            ),
            (
                "SLUS-00603",
                "BA",
                "-DASH00",
                "c63b487fba568fe5f8bfbe91e5fca8184f47ee598fc0cd5b8c6c7463ff52e1d1",
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
            match code {
                "SLES-03900" => {
                    assert_eq!(s.checksum_ok, Some(true));
                    assert_eq!(s.profiles[0].fields[0].value, "8:25:45");
                    assert_eq!(s.profiles[0].fields[1].value, "Junk City");
                    assert_eq!(s.profiles[0].fields[2].value, "83 / 301");
                }
                "SCUS-94570" => {
                    assert_eq!(s.checksum_ok, Some(true));
                    assert_eq!(s.profiles[0].name, "BLB");
                    assert_eq!(s.profiles[0].fields[0].value, "4%");
                    assert_eq!(s.profiles[0].fields[1].value, "4");
                }
                "SCES-00934" => {
                    assert_eq!(s.checksum_ok, Some(true));
                    assert_eq!(s.profiles[0].fields[0].value, "6");
                    assert_eq!(s.profiles[0].fields[1].value, "1:21:01");
                    assert_eq!(s.profiles[0].fields[2].value, "2");
                }
                "SLUS-00603" => {
                    assert_eq!(s.checksum_ok, None);
                    assert_eq!(s.profiles[0].fields[0].value, "07:02:34");
                    assert_eq!(s.profiles[0].fields[1].value, "8730");
                }
                _ => unreachable!(),
            }
            assert!(game_details::decode(code, "BI", identifier, d).is_err());
            assert!(game_details::decode(code, region, "BAD", d).is_err());
            assert!(game_details::decode(code, region, identifier, &d[..d.len() - 1]).is_err());
        }
    }
}
