//! Small, useful field sets from pinned save maps, not full editor support.
//! RyudoSynbios/game-tools-collection (MIT) cc2bc1044986903833547369b007f57fef2216b9;
//! Shendo's SpyroEdit (MIT). Credits and license notices travel in docs.
use super::{Field, Format, Profile, Record, SaveDetails};

fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
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
fn time(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 3600, seconds / 60 % 60)
}
fn ascii(d: &[u8]) -> String {
    d.iter()
        .take_while(|&&b| b != 0 && b != 0xff)
        .map(|&b| {
            if (32..127).contains(&b) {
                b as char
            } else {
                '\u{fffd}'
            }
        })
        .collect::<String>()
        .trim()
        .into()
}
fn ff7_text(d: &[u8]) -> String {
    d.iter()
        .take_while(|&&b| b != 0xff)
        .map(|&b| match b {
            0 => ' ',
            0x10..=0x19 => (b + 0x20) as char,
            0x21..=0x3a | 0x41..=0x5a => (b + 0x20) as char,
            0x0d => '-',
            0x0e => '.',
            _ => '\u{fffd}',
        })
        .collect::<String>()
        .trim()
        .into()
}
fn fft_text(d: &[u8]) -> String {
    d.iter()
        .take_while(|&&b| b != 0xfe)
        .map(|&b| match b {
            0..=9 => (b + b'0') as char,
            0xa..=0x23 => (b - 0xa + b'A') as char,
            0x24..=0x3d => (b - 0x24 + b'a') as char,
            0xfa => ' ',
            0x3e => '!',
            0x40 => '?',
            0x5f => '.',
            0x93 => '\'',
            _ => '\u{fffd}',
        })
        .collect::<String>()
        .trim()
        .into()
}
fn ff9_text(d: &[u8]) -> String {
    d.iter()
        .take_while(|&&b| b != 0xff)
        .map(|&b| match b {
            0..=9 => (b + b'0') as char,
            0x10..=0x29 => (b - 0x10 + b'A') as char,
            0x30..=0x49 => (b - 0x30 + b'a') as char,
            0x0f => ' ',
            0x0b => '-',
            0x2a => '(',
            0x4a => ')',
            0x2b => '!',
            0x2c => '?',
            0x2e => ':',
            0x2f => '.',
            0x4b => ',',
            0x4c => '/',
            _ => '\u{fffd}',
        })
        .collect::<String>()
        .trim()
        .into()
}
fn character_name(saved: String, default: &str) -> String {
    if saved.is_empty() {
        default.into()
    } else if saved == default {
        saved
    } else {
        format!("{saved} ({default})")
    }
}
fn crc_ccitt(d: &[u8], inverted: bool) -> u16 {
    let mut crc = 0xffffu16;
    for &byte in d {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    if inverted {
        !crc
    } else {
        crc
    }
}
// FFVIII's original table has entry 0xff = 0 rather than the normal 0x1ef0.
// Keep this game quirk separate from FFVII's standard CRC. Two independent
// shared PS1 saves match this result (see the fixture manifest).
fn ff8_checksum(d: &[u8]) -> u16 {
    let mut crc = 0xffffu16;
    for &byte in d {
        let index = byte ^ (crc >> 8) as u8;
        let mut entry = (index as u16) << 8;
        for _ in 0..8 {
            entry = if entry & 0x8000 != 0 {
                (entry << 1) ^ 0x1021
            } else {
                entry << 1
            };
        }
        if index == 0xff {
            entry = 0;
        }
        crc = entry ^ (crc << 8);
    }
    !crc
}
fn crc_reflected(d: &[u8]) -> u16 {
    let mut crc = 0xffffu16;
    for &byte in d {
        crc ^= byte as u16;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0x8408
            } else {
                crc >> 1
            };
        }
    }
    crc
}
fn crc32(d: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in d {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
fn unknown_id(id: u32) -> String {
    format!("Unknown (0x{id:X})")
}
fn lookup(names: &[&str], id: u8) -> String {
    names
        .get(id as usize)
        .map(|s| s.to_string())
        .unwrap_or_else(|| unknown_id(id as u32))
}

pub(super) fn decode(
    f: &Format,
    code: &str,
    _identifier: &str,
    d: &[u8],
) -> Result<SaveDetails, String> {
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
    let mut p = profile(1, "Saved game".into(), vec![]);
    match f.slug {
        "final-fantasy-vii" => {
            if !(1..=3).contains(&d[0x10a4]) {
                return Err("Unknown FFVII disc/layout revision.".into());
            }
            result.checksum_ok = Some(u16le(d, 0x200) == crc_ccitt(&d[0x204..0x12f4], true));
            p.name = ff7_text(&d[0x208..0x211]);
            p.fields = vec![
                field("Disc", d[0x10a4]),
                field("Playtime (hh:mm)", time(u32le(d, 0xd80) as u64)),
                field("Gil", u32le(d, 0xd7c)),
                field("GP", u16le(d, 0xeee)),
                field("Saved location", ff7_text(&d[0x228..0x23c])),
                field("Story progress value", u16le(d, 0xda4)),
            ];
            for (i, name) in [
                "Cloud",
                "Barret",
                "Tifa",
                "Aeris",
                "Red XIII",
                "Yuffie",
                "Cait Sith",
                "Vincent",
                "Cid",
            ]
            .iter()
            .enumerate()
            {
                let o = 0x254 + i * 0x84;
                if d[o + 1] == 0 {
                    continue;
                }
                p.records.push(Record {
                    name: character_name(ff7_text(&d[o + 0x10..o + 0x19]), name),
                    fields: vec![
                        field("Level", d[o + 1]),
                        field("HP", u16le(d, o + 0x2c)),
                        field("MP", u16le(d, o + 0x30)),
                        field("Experience", u32le(d, o + 0x3c)),
                        field(
                            "In party",
                            if d[0x6f8..0x6fb].contains(&(i as u8)) {
                                "Yes"
                            } else {
                                "No"
                            },
                        ),
                    ],
                });
            }
        }
        "final-fantasy-viii" => {
            if !(1..=4).contains(&d[0xfac]) {
                return Err("Unknown FFVIII disc/layout revision.".into());
            }
            let checksum = ff8_checksum(&d[0x1d0..0x1520]);
            result.checksum_ok = Some(u16le(d, 0x180) == checksum && u16le(d, 0x1520) == checksum);
            p.fields = vec![
                field("Disc", d[0xfac]),
                field("Playtime (hh:mm)", time(u32le(d, 0xe50) as u64)),
                field("Gil (Squall)", u32le(d, 0xc8c)),
                field("Gil (Laguna)", u32le(d, 0xc90)),
                field("Location ID", format!("0x{:04X}", u16le(d, 0xec2))),
            ];
            for (i, name) in [
                "Squall", "Zell", "Irvine", "Quistis", "Rinoa", "Selphie", "Seifer", "Edea",
            ]
            .iter()
            .enumerate()
            {
                let o = 0x610 + i * 0x98;
                let xp = u32le(d, o + 4);
                p.records.push(Record {
                    name: name.to_string(),
                    fields: vec![
                        field("Level", (xp / 1000 + 1).min(100)),
                        field("Experience", xp),
                        field("HP", u16le(d, o)),
                    ],
                });
            }
            let mut obtained = 0;
            for (i, name) in [
                "Quezacotl",
                "Shiva",
                "Ifrit",
                "Siren",
                "Brothers",
                "Diablos",
                "Carbuncle",
                "Leviathan",
                "Pandemona",
                "Cerberus",
                "Alexander",
                "Doomtrain",
                "Bahamut",
                "Cactuar",
                "Tonberry",
                "Eden",
            ]
            .iter()
            .enumerate()
            {
                let o = 0x1d0 + i * 0x44;
                if d[o + 0x11] == 0 {
                    continue;
                }
                obtained += 1;
                p.records.push(Record {
                    name: format!("GF: {name}"),
                    fields: vec![field("Experience", u32le(d, o + 0xc))],
                });
            }
            p.fields
                .push(field("Guardian Forces", format!("{obtained} / 16")));
        }
        "final-fantasy-ix" => {
            let disc = match d[0x104] {
                1 => 1,
                2 => 2,
                4 => 3,
                8 => 4,
                _ => return Err("Unknown FFIX disc/layout revision.".into()),
            };
            p.name = ff9_text(&d[0x9d0..0x9db]);
            result.checksum_ok = Some(u16le(d, 0x13fe) == crc_reflected(&d[..0x13fe]));
            p.fields = vec![
                field("Disc", disc),
                field("Saved location", ff9_text(&d[0x110..0x12c])),
                field(
                    "Playtime (hh:mm)",
                    time((u32le(d, 0x12c) / if f.region == "BE" { 50 } else { 60 }) as u64),
                ),
                field("Gil", u32le(d, 0xee8)),
                field("Save count", u32le(d, 0x178)),
                field("Location ID", format!("0x{:04X}", u16le(d, 0x1a0))),
            ];
            for (i, name) in [
                "Zidane", "Vivi", "Garnet", "Steiner", "Freya", "Quina", "Eiko", "Amarant",
                "Beatrix",
            ]
            .iter()
            .enumerate()
            {
                let o = 0x9d0 + i * 0x90;
                if d[o + 0xb] == 0 {
                    continue;
                }
                p.records.push(Record {
                    name: character_name(ff9_text(&d[o..o + 0xb]), name),
                    fields: vec![field("Level", d[o + 0xb])],
                });
            }
        }
        "final-fantasy-tactics" => {
            p.name = fft_text(&d[0x101..0x10f]);
            p.fields = vec![
                field("Playtime (hh:mm)", time(u32le(d, 0x120) as u64)),
                field("War funds", u32le(d, 0x1934)),
                field("Location ID", format!("0x{:02X}", d[0x1948])),
            ];
            // Sixty parity bits over 128-byte groups, including the stored checksum.
            let mut checksum = [0xffu8; 8];
            for (i, group) in d[..60 * 128].chunks_exact(128).enumerate() {
                let odd = group.iter().map(|b| b.count_ones()).sum::<u32>() & 1;
                let mask = 1 << (7 - i % 8);
                checksum[i / 8] = (checksum[i / 8] & !mask) | ((odd as u8) << (7 - i % 8));
            }
            result.checksum_ok = Some(d[0x118..0x120] == checksum);
            for i in 0..20 {
                let o = 0x480 + i * 0xe0;
                let level = d[o + 0x1a];
                if level == 0 || d[o + 5] == 0xff {
                    continue;
                }
                p.records.push(Record {
                    name: fft_text(&d[o + 0xc2..o + 0xd0]),
                    fields: vec![
                        field("Level", level),
                        field("Job ID", format!("0x{:02X}", d[o + 6])),
                    ],
                });
            }
        }
        "chrono-cross" => {
            result.checksum_ok =
                Some(u32le(d, 0x1ffc) == d[..0x1ff8].iter().map(|&v| v as u32).sum::<u32>());
            p.fields = vec![
                field("Playtime (hh:mm)", time((u32le(d, 0x588) / 60) as u64)),
                field("Gold", u32le(d, 0x58c)),
                field("Stars", d[0x5a4]),
                field("Current stars", d[0x5a5]),
                field("Location ID", format!("0x{:04X}", u16le(d, 0x18e))),
            ];
            result.notes.push(
                "Compressed character, equipment and element records are not decoded yet.".into(),
            );
        }
        "castlevania-symphony-of-the-night" => {
            // US layout is +0x100 relative to the PAL source definitions.
            p.name = ascii(&d[0x464..0x46c]);
            let hours = u32le(d, 0x504);
            let minutes = u32le(d, 0x508);
            if hours > 99 || minutes > 59 || u32le(d, 0x4bc) > 99 {
                return Err("Unknown or malformed Symphony of the Night layout.".into());
            }
            p.fields = vec![
                field("Character", lookup(&["Alucard", "Richter"], d[0x230])),
                field("Level", u32le(d, 0x4bc)),
                field(
                    "Playtime (hh:mm)",
                    time(hours as u64 * 3600 + minutes as u64 * 60),
                ),
                field("Gold", u32le(d, 0x4c4)),
                field(
                    "Map discovery",
                    format!("{:.1}%", u16le(d, 0x22a) as f64 / 9.42),
                ),
                field("Kills", u32le(d, 0x4c8)),
            ];
            result
                .notes
                .push("No game checksum has been verified for this format.".into());
        }
        "gran-turismo" => {
            let mut sum = 0xaaaau32;
            let mut crc = 0x3770u32;
            for &byte in &d[0x200..0x6da4] {
                sum = sum.wrapping_add(byte as u32) ^ ((byte as u32) << 8);
                let mut v = byte as u32;
                for _ in 0..8 {
                    crc <<= 1;
                    if crc & 0x10000 != 0 {
                        crc ^= 0x11021;
                    }
                    crc |= (v >> 7) & 1;
                    v <<= 1;
                }
            }
            result.checksum_ok = Some(u32le(d, 0x6da4) == (sum << 16 | (crc & 0xffff)));
            p.fields = vec![
                field("Playtime (hh:mm)", time(u32le(d, 0x200) as u64)),
                field("Credits", u32le(d, 0x204)),
                field("Days", u32le(d, 0x208)),
                field("Garage cars", u16le(d, 0x21a)),
            ];
            for (i, name) in ["B", "A", "IA"].iter().enumerate() {
                let medals = &d[0x2d54 + i * 8..0x2d54 + (i + 1) * 8];
                p.records.push(Record {
                    name: format!("{name} licence"),
                    fields: vec![
                        field(
                            "Passed",
                            format!(
                                "{} / 8",
                                medals.iter().filter(|&&m| (1..=3).contains(&m)).count()
                            ),
                        ),
                        field(
                            "Gold",
                            format!("{} / 8", medals.iter().filter(|&&m| m == 3).count()),
                        ),
                    ],
                });
            }
        }
        "gran-turismo-2" => {
            // GT2SaveEditor's publicly documented raw-layout constants. No unlicensed
            // editor code or car database is incorporated.
            let r = &d[0x200..];
            result.checksum_ok = Some(u32le(r, 31900) == crc32(&d[..0x200 + 31900]));
            if r[15476] > 100 || r[0] > 6 {
                return Err("Unknown or malformed Gran Turismo 2 layout.".into());
            }
            let mut score = 0f64;
            for &b in &r[280..404] {
                for n in [b & 15, b >> 4] {
                    if (1..=6).contains(&n) {
                        score += 1.0 / n as f64;
                    }
                }
            }
            p.fields = vec![
                field("Credits", u32le(r, 31880)),
                field("Days", u32le(r, 248)),
                field("Races", u32le(r, 256)),
                field("Wins", u32le(r, 260)),
                field("Garage cars", r[15476]),
                field(
                    "Career progress (219 events)",
                    format!("{:.2}%", score * 100.0 / 219.0),
                ),
            ];
            for (name, o) in [
                ("B", 13345),
                ("A", 11705),
                ("IC", 10065),
                ("IB", 8425),
                ("IA", 6785),
                ("S", 5145),
            ] {
                let gold = (0..10).filter(|i| r[o + i * 164] == 4).count();
                let passed = (0..10)
                    .filter(|i| (2..=4).contains(&r[o + i * 164]))
                    .count();
                p.records.push(Record {
                    name: format!("{name} licence"),
                    fields: vec![
                        field("Passed", format!("{passed} / 10")),
                        field("Gold", format!("{gold} / 10")),
                    ],
                });
            }
            result.notes.push("Career progress uses 219 events. Disc revision and mods cannot be identified from the product code alone; some releases display a different percentage.".into());
        }
        "ctr-crash-team-racing" => {
            let mut crc = 0u32;
            for &byte in &d[0x100..0x1780] {
                for bit in (0..8).rev() {
                    crc = crc << 1 | ((byte as u32 >> bit) & 1);
                    if crc & 0x10000 != 0 {
                        crc ^= 0x11021;
                    }
                }
            }
            result.checksum_ok = Some(crc as u16 == 0);
            for i in 0..4 {
                let o = 0x104 + i * 0x50;
                let mut p = profile(i as u8 + 1, ascii(&d[o + 0x18..o + 0x20]), vec![]);
                p.empty = d[o + 0x2b] == 0xff;
                if !p.empty {
                    let trophies = ((u32le(d, o) & 0x3fffc0) >> 6).count_ones();
                    let relics = (u32le(d, o + 2) & 0xffffc0).count_ones();
                    let gold = (u32le(d, o + 5) & 0x3ffff).count_ones();
                    let tokens = (u32le(d, o + 9) & 0xffff0).count_ones();
                    let keys = (u16le(d, o + 0xb) & 0x3c0).count_ones();
                    let oxide = (u16le(d, o + 0xc) & 4).count_ones();
                    let oxide_final = (u16le(d, o + 0xc) & 8).count_ones();
                    let gems = (d[o + 0xd] & 0x7c).count_ones();
                    let bonus = (u16le(d, o + 0xd) & 0x780).count_ones();
                    let completion = trophies * 2
                        + relics * 2
                        + u32::from(gold == 18)
                        + tokens
                        + keys
                        + oxide * 2
                        + oxide_final
                        + gems
                        + bonus;
                    p.fields = vec![
                        field("Completion", format!("{completion}%")),
                        field("Trophies", format!("{trophies} / 16")),
                        field("Relics", format!("{relics} / 18")),
                        field("Race CTR tokens", format!("{tokens} / 16")),
                        field("Boss keys", format!("{keys} / 4")),
                        field("Gems", format!("{gems} / 5")),
                    ];
                }
                result.profiles.push(p);
            }
        }
        "spyro-the-dragon" => {
            const LEVELS: [&str; 35] = [
                "Artisans",
                "Stone Hill",
                "Dark Hollow",
                "Town Square",
                "Toasty",
                "Sunny Flight",
                "Peace Keepers",
                "Dry Canyon",
                "Cliff Town",
                "Ice Cavern",
                "Doctor Shemp",
                "Night Flight",
                "Magic Crafters",
                "Alpine Ridge",
                "High Caves",
                "Wizard Peak",
                "Blowhard",
                "Crystal Flight",
                "Beast Makers",
                "Terrace Village",
                "Misty Bog",
                "Tree Tops",
                "Metalhead",
                "Wild Flight",
                "Dream Weavers",
                "Dark Passage",
                "Lofty Castle",
                "Haunted Towers",
                "Jacques",
                "Icy Flight",
                "Gnorc Gnexus",
                "Gnorc Cove",
                "Twilight Harbor",
                "Gnasty Gnorc",
                "Gnasty's Loot",
            ];
            for i in 0..3 {
                // SpyroEdit offsets include the 128-byte MCS directory header.
                let o = 0x200 + i * 0x600;
                let mut p = profile(i as u8 + 1, format!("Adventure {}", i + 1), vec![]);
                p.empty = d[o] == 0 || d[o + 4] == 0x52;
                if !p.empty {
                    p.checksum_ok = Some(
                        u32le(d, o + 0x58c)
                            == d[o..o + 0x58c].iter().map(|&b| b as u32).sum::<u32>(),
                    );
                    let dragons: u32 = d[o + 0x88..o + 0x88 + 35].iter().map(|&b| b as u32).sum();
                    let gems: u32 = (0..35).map(|j| u16le(d, o + 0xac + j * 2) as u32).sum();
                    let eggs: u32 = d[o + 0xf4..o + 0xf4 + 16].iter().map(|&b| b as u32).sum();
                    let level = d[o];
                    let idx = if level >= 10 {
                        ((level - 10) / 10) as usize * 6 + ((level - 10) % 10) as usize
                    } else {
                        usize::MAX
                    };
                    p.fields = vec![
                        field(
                            "Saved location",
                            LEVELS
                                .get(idx)
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| unknown_id(level as u32)),
                        ),
                        field("Lives", d[o + 0xb]),
                        field("Dragons", format!("{dragons} / 80")),
                        field("Gems", format!("{gems} / 14000")),
                        field("Eggs", format!("{eggs} / 12")),
                    ];
                    for (j, name) in LEVELS.iter().enumerate() {
                        p.records.push(Record {
                            name: name.to_string(),
                            fields: vec![
                                field("Dragons", d[o + 0x88 + j]),
                                field("Gems", u16le(d, o + 0xac + j * 2)),
                            ],
                        });
                    }
                }
                result.profiles.push(p);
            }
            result.checksum_ok = Some(
                result
                    .profiles
                    .iter()
                    .filter(|p| !p.empty)
                    .all(|p| p.checksum_ok == Some(true)),
            );
            result.notes.push("A completion percentage is not inferred: SpyroEdit leaves 10% of its formula unresolved.".into());
        }
        "tekken-3" => {
            let mut data = d[0x200..0x400].to_vec();
            let mut previous = data[0];
            for value in &mut data[1..] {
                let encoded = *value;
                *value = value.wrapping_sub(previous.wrapping_mul(5).wrapping_add(1));
                previous = encoded;
            }
            let checksum = data[1..]
                .iter()
                .fold(0u8, |s, &b| s.wrapping_add(b))
                .wrapping_neg();
            result.checksum_ok = Some(data[0] == checksum);
            const NAMES: [&str; 21] = [
                "Paul",
                "Law",
                "Lei",
                "King",
                "Yoshimitsu",
                "Nina",
                "Hwoarang",
                "Xiaoyu",
                "Eddy",
                "Jin",
                "Julia",
                "Kuma",
                "Bryan",
                "Heihachi",
                "Ogre",
                "Mokujin",
                "Gun Jack",
                "Gon",
                "Anna",
                "Doctor B.",
                "True Ogre",
            ];
            let unlocked: Vec<&str> = NAMES
                .iter()
                .enumerate()
                .filter(|(i, _)| data[1 + i / 8] & (1 << (i % 8)) != 0)
                .map(|(_, n)| *n)
                .collect();
            p.fields = vec![
                field("Playtime (hh:mm)", time((u32le(&data, 0x11) / 60) as u64)),
                field("Unlocked characters", unlocked.join(", ")),
                field(
                    "Tekken Ball",
                    if data[0x37] & 1 != 0 {
                        "Unlocked"
                    } else {
                        "Locked"
                    },
                ),
                field(
                    "Theatre",
                    if data[0x38] & 1 != 0 {
                        "Unlocked"
                    } else {
                        "Locked"
                    },
                ),
            ];
            for (i, name) in NAMES.iter().enumerate() {
                let o = 0x149 + i * 8;
                p.records.push(Record {
                    name: name.to_string(),
                    fields: vec![
                        field("Arcade usage", u16le(&data, o)),
                        field("Versus wins", u16le(&data, o + 2)),
                        field("Versus losses", u16le(&data, o + 4)),
                    ],
                });
            }
        }
        "silent-hill" => {
            let endings = ["Good+", "Good", "Bad+", "Bad", "UFO"]
                .iter()
                .enumerate()
                .filter(|(i, _)| d[0x327] & (1 << i) != 0)
                .map(|(_, s)| *s)
                .collect::<Vec<_>>()
                .join(", ");
            let xor_ok = |start: usize, end: usize| {
                let xor = d[start..end].iter().fold(0u8, |s, &b| s ^ b);
                u16le(d, end) == u16::from(xor) * 0x101
            };
            let system_ok = xor_ok(0x204, 0x2fc) && xor_ok(0x300, 0x37c);
            for i in 0..11 {
                let o = 0x380 + i * 0x280;
                let mut p = profile(i as u8 + 1, format!("Save {}", i + 1), vec![]);
                p.empty = u32le(d, 0x204 + i * 12) == 0;
                if p.empty {
                    result.profiles.push(p);
                    continue;
                }
                let xor = d[o..o + 0x27c].iter().fold(0u8, |s, &b| s ^ b);
                p.checksum_ok = Some(u16le(d, o + 0x27c) == u16::from(xor) * 0x101);
                let seconds = u32le(d, o + 0x250) as u64 / 4096;
                let hours_extra = ((d[o + 0x25c] >> 1) & 3) as u64 * 290;
                p.fields = vec![
                    field("Playtime (hh:mm)", time(seconds + hours_extra * 3600)),
                    field(
                        "Difficulty",
                        match d[o + 0x263] {
                            0 => "Normal".into(),
                            0x10 => "Hard".into(),
                            0xf0 => "Easy".into(),
                            v => unknown_id(v as u32),
                        },
                    ),
                    field("Saves", u16le(d, o + 0xa6)),
                    field("Health (0–6)", d[o + 0x242]),
                    field(
                        "Unlocked endings",
                        if endings.is_empty() { "None" } else { &endings },
                    ),
                    field(
                        "Mode",
                        if d[o + 0x25c] & 1 != 0 {
                            "Next Fear"
                        } else {
                            "Normal"
                        },
                    ),
                    field(
                        "Location ID",
                        format!("0x{:04X}", u16::from_be_bytes([d[o + 0xa4], d[o + 0xa5]])),
                    ),
                ];
                result.profiles.push(p);
            }
            result.checksum_ok = Some(
                system_ok
                    && result
                        .profiles
                        .iter()
                        .filter(|p| !p.empty)
                        .all(|p| p.checksum_ok == Some(true)),
            );
            result
                .notes
                .push("The overall checksum also checks save previews and shared options.".into());
        }
        _ => return Err("No decoder is available for this release.".into()),
    }
    if result.profiles.is_empty() {
        result.profiles.push(p);
    }
    let remaining = match f.slug {
        "final-fantasy-vii"=>"Materia, inventory, chocobos and detailed story flags are not decoded yet.",
        "final-fantasy-viii"=>"Guardian Force levels, junctions, abilities, Triple Triad and inventory are not decoded yet.",
        "final-fantasy-ix"=>"Equipment, abilities, Tetra Master and side quests are not decoded yet.",
        "final-fantasy-tactics"=>"Abilities, inventory, propositions and map progress are not decoded yet.",
        "gran-turismo" | "gran-turismo-2"=>"Individual cars, tuning and race records are not decoded yet.",
        "ctr-crash-team-racing"=>"Time-trial ghosts and race records are not decoded yet.",
        "tekken-3"=>"Time attack, survival and Tekken Force records are not decoded yet.",
        "silent-hill"=>"Inventory and detailed result statistics are not decoded yet.",
        "castlevania-symphony-of-the-night"=>"Inventory, relics, familiars and bestiary are not decoded yet.",
        _=>"",
    };
    if !remaining.is_empty() {
        result.notes.push(remaining.into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_vectors_keep_ff8_quirk_separate() {
        assert_eq!(crc_ccitt(b"123456789", false), 0x29b1);
        assert_eq!(crc_reflected(b"123456789"), 0x6f91);
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
        assert_eq!(ff8_checksum(&[0]), 0x00ff);
        assert_eq!(crc_ccitt(&[0], true), 0x1e0f);
    }

    #[test]
    fn text_terminators_spaces_and_unknown_characters_are_explicit() {
        assert_eq!(
            ff7_text(&[0x23, 0x4c, 0x4f, 0x55, 0x44, 0xff, 0x3a]),
            "Cloud"
        );
        assert_eq!(
            fft_text(&[0x1b, 0x24, 0x30, 0x3d, 0x24, 0xfa, 0x19, 0xfe]),
            "Ramza P"
        );
        assert_eq!(
            ff9_text(&[0x29, 0x38, 0x33, 0x30, 0x3d, 0x34, 0xff]),
            "Zidane"
        );
        assert_eq!(ff7_text(&[0x80, 0xff]), "�");
    }
}
