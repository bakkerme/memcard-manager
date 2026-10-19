//! Original US BASCUS-94423SYS only. Independently traced numeric format facts;
//! provenance, completion formula and unresolved fields: docs/formats/ape-escape.md.
use super::{Field, Format, Profile, Record, SaveDetails};
use encoding_rs::SHIFT_JIS;
use unicode_normalization::UnicodeNormalization;

const START: usize = 0x180;
const STRIDE: usize = 0x7a0;
const GADGETS: [&str; 8] = [
    "Stun Club",
    "Time Net",
    "Monkey Radar",
    "Slingback Shooter",
    "Super Hoop",
    "Magic Punch",
    "Sky Flyer",
    "RC Car",
];

struct Level {
    index: usize,
    name: &'static str,
    monkeys: usize,
    coins: usize,
    // Bronze, silver, gold: exclusive upper bounds in 60-Hz ticks.
    thresholds: Option<[u32; 3]>,
    rooms: std::ops::Range<u8>,
}

const LEVELS: [Level; 22] = [
    Level {
        index: 1,
        name: "Fossil Field",
        monkeys: 4,
        coins: 1,
        thresholds: Some([2700, 2100, 1800]),
        rooms: 1..2,
    },
    Level {
        index: 2,
        name: "Primordial Ooze",
        monkeys: 6,
        coins: 1,
        thresholds: Some([3600, 2820, 2100]),
        rooms: 2..3,
    },
    Level {
        index: 3,
        name: "Molten Lava",
        monkeys: 7,
        coins: 1,
        thresholds: Some([7800, 6300, 4500]),
        rooms: 3..6,
    },
    Level {
        index: 4,
        name: "Thick Jungle",
        monkeys: 14,
        coins: 4,
        thresholds: Some([9000, 7200, 5400]),
        rooms: 6..11,
    },
    Level {
        index: 5,
        name: "Dark Ruins",
        monkeys: 13,
        coins: 4,
        thresholds: Some([7800, 6300, 4800]),
        rooms: 11..15,
    },
    Level {
        index: 6,
        name: "Cryptic Relics",
        monkeys: 8,
        coins: 1,
        thresholds: Some([6600, 5100, 3900]),
        rooms: 15..19,
    },
    Level {
        index: 7,
        name: "Stadium Attack",
        monkeys: 0,
        coins: 0,
        thresholds: None,
        rooms: 19..20,
    },
    Level {
        index: 8,
        name: "Crabby Beach",
        monkeys: 8,
        coins: 1,
        thresholds: Some([6000, 5100, 4200]),
        rooms: 20..22,
    },
    Level {
        index: 9,
        name: "Coral Cave",
        monkeys: 8,
        coins: 1,
        thresholds: Some([7800, 6300, 5100]),
        rooms: 22..24,
    },
    Level {
        index: 10,
        name: "Dexter's Island",
        monkeys: 11,
        coins: 3,
        thresholds: Some([6600, 5400, 4200]),
        rooms: 24..29,
    },
    Level {
        index: 11,
        name: "Snowy Mammoth",
        monkeys: 6,
        coins: 1,
        thresholds: Some([7200, 5700, 4500]),
        rooms: 29..30,
    },
    Level {
        index: 12,
        name: "Frosty Retreat",
        monkeys: 9,
        coins: 3,
        thresholds: Some([7200, 5400, 4200]),
        rooms: 30..33,
    },
    Level {
        index: 13,
        name: "Hot Springs",
        monkeys: 9,
        coins: 2,
        thresholds: Some([6600, 5400, 4500]),
        rooms: 33..36,
    },
    Level {
        index: 14,
        name: "Gladiator Attack",
        monkeys: 0,
        coins: 0,
        thresholds: None,
        rooms: 36..37,
    },
    Level {
        index: 15,
        name: "Sushi Temple",
        monkeys: 12,
        coins: 3,
        thresholds: Some([5400, 4200, 3300]),
        rooms: 37..40,
    },
    Level {
        index: 16,
        name: "Wabi Sabi Wall",
        monkeys: 10,
        coins: 3,
        thresholds: Some([6000, 4800, 3600]),
        rooms: 40..45,
    },
    Level {
        index: 17,
        name: "Crumbling Castle",
        monkeys: 20,
        coins: 4,
        thresholds: Some([4800, 3600, 2700]),
        rooms: 45..53,
    },
    Level {
        index: 20,
        name: "City Park",
        monkeys: 13,
        coins: 3,
        thresholds: Some([3900, 3000, 2100]),
        rooms: 53..56,
    },
    Level {
        index: 21,
        name: "Specter's Factory",
        monkeys: 10,
        coins: 2,
        thresholds: Some([7500, 5700, 4500]),
        rooms: 56..63,
    },
    Level {
        index: 22,
        name: "TV Tower",
        monkeys: 12,
        coins: 2,
        thresholds: Some([6900, 5100, 3900]),
        rooms: 63..69,
    },
    Level {
        index: 24,
        name: "Monkey Madness",
        monkeys: 24,
        coins: 10,
        thresholds: Some([6000, 4800, 3600]),
        rooms: 69..87,
    },
    Level {
        index: 30,
        name: "Peak Point Matrix",
        monkeys: 0,
        coins: 0,
        thresholds: None,
        rooms: 87..88,
    },
];

fn word(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn field(label: &str, value: impl ToString) -> Field {
    Field {
        label: label.into(),
        value: value.to_string(),
    }
}
fn name(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &b in bytes.iter().take_while(|&&b| b != 0) {
        match b {
            0x10..=0x29 => result.push((b'A' + b - 0x10) as char),
            0x30..=0x49 => result.push((b'a' + b - 0x30) as char),
            0x6a..=0x73 => result.push((b'0' + b - 0x6a) as char),
            0x7f => result.push(' '),
            _ => result.push_str(&format!("[0x{b:02X}]")),
        }
    }
    result
}
fn gadget(id: u8) -> String {
    match id {
        0xff => "None".into(),
        0..=7 => GADGETS[id as usize].into(),
        _ => format!("Unknown (ID 0x{id:02X})"),
    }
}
fn state(value: u8) -> String {
    match value {
        0 => "Not cleared".into(),
        1 => "Cleared".into(),
        2 => "Perfect clear".into(),
        _ => format!("Unknown (0x{value:02X})"),
    }
}
fn time(ticks: u32) -> String {
    if ticks == u32::MAX {
        return "Not recorded".into();
    }
    format!(
        "{:02}:{:02}.{:02}",
        ticks / 3600,
        ticks / 60 % 60,
        ticks % 60 * 100 / 60
    )
}
fn medal(ticks: u32, thresholds: [u32; 3]) -> (&'static str, usize) {
    if ticks == u32::MAX {
        return ("Not recorded", 0);
    }
    if ticks < thresholds[2] {
        ("Gold", 8)
    } else if ticks < thresholds[1] {
        ("Silver", 7)
    } else if ticks < thresholds[0] {
        ("Bronze", 5)
    } else {
        ("No medal", 0)
    }
}
fn remaining(count: usize, total: usize) -> String {
    total
        .checked_sub(count)
        .map(|n| n.to_string())
        .unwrap_or_else(|| "Unknown (stored count exceeds total)".into())
}

/// Dispatch already validates region, exact identifier, size and block count.
pub(super) fn decode(format: &Format, code: &str, data: &[u8]) -> Result<SaveDetails, String> {
    let title_bytes = &data[4..68];
    let end = title_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(title_bytes.len());
    let (title, _, malformed) = SHIFT_JIS.decode(&title_bytes[..end]);
    if data[2] != 0x11
        || malformed
        || title.nfkc().collect::<String>() != "Ape Escape"
        || &data[0x50..0x58] != b"\x01\x00CRD0\x00\x00"
    {
        return Err("This Ape Escape save does not match the verified original US layout.".into());
    }
    let mut notes = vec![
        "Original US adventure save only. Save checksum verification is unavailable.".into(),
        "Completion follows the game's calculation. Time-trial medals use the original US thresholds; times are minutes:seconds.centiseconds.".into(),
        "Individual monkey names, health, playtime, detailed story flags and minigame records are not decoded. Unmapped name glyphs appear as [0xNN].".into(),
    ];
    let mut profiles = Vec::new();
    for i in 0..4 {
        let p = &data[START + i * STRIDE..START + (i + 1) * STRIDE];
        let room = u16::from_le_bytes([p[0], p[1]]);
        let mut profile = Profile {
            number: (i + 1) as u8,
            name: name(&p[0x18..0x28]),
            empty: room == 0xffff,
            status: None,
            checksum_ok: None,
            fields: vec![],
            records: vec![],
        };
        if profile.empty {
            profiles.push(profile);
            continue;
        }
        if room >= 0x8000 {
            return Err(format!(
                "Ape Escape profile {} has an unsupported empty/profile marker (0x{room:04X}).",
                i + 1
            ));
        }
        let warn = |notes: &mut Vec<String>, message: &str| {
            notes.push(format!("Save {}: {message}", i + 1))
        };
        let coins: Vec<_> = p[0x68..0x168]
            .chunks_exact(2)
            .take_while(|c| c[0] != 0xff)
            .collect();
        if coins.len() == 128 {
            warn(
                &mut notes,
                "coin list has no terminator; all 128 records are shown in totals.",
            );
        }
        if coins.len() > 50
            || coins.iter().any(|c| {
                c[1] != 1
                    || !LEVELS
                        .iter()
                        .any(|l| l.coins > 0 && l.rooms.contains(&c[0]))
            })
        {
            warn(
                &mut notes,
                "coin records contain unexpected values; collectible totals may be inconsistent.",
            );
        }
        // The game's total includes the first 25 counters, even unused indices.
        let monkeys: usize = p[0x48..0x61].iter().map(|&n| n as usize).sum();
        let challenges = word(p, 0x668);
        let stadium = challenges & 0x10 != 0;
        let gladiator = challenges & 0x1000 != 0;
        let bonus_coins = 5 * (usize::from(stadium) + usize::from(gladiator));
        let mut points = monkeys
            + coins.len() * 2
            + 15 * (usize::from(stadium) + usize::from(gladiator))
            + 15 * usize::from(p[0x46] == 2)
            + [0x670, 0x671, 0x674].iter().filter(|&&o| p[o] == 5).count() * 8;
        let mut medals = [0usize; 3];
        let mut cleared = 0;
        let mut inconsistent = false;
        for l in &LEVELS {
            let clear = p[0x28 + l.index];
            let ticks = word(p, 0x690 + l.index * 4);
            // Race reward flags, rather than ordinary stage clear bytes, are
            // the verified challenge progress indicator.
            let race = l.index == 7 || l.index == 14;
            let mut fields = if race {
                vec![]
            } else {
                vec![field("Progress", state(clear))]
            };
            if !race && !matches!(clear, 0..=2) {
                inconsistent = true;
            }
            if let Some(thresholds) = l.thresholds {
                let captured = p[0x48 + l.index] as usize;
                let coin_count = coins.iter().filter(|c| l.rooms.contains(&c[0])).count();
                // Use the game's stored counters. Raw capture flags also have
                // unused slots, so their population is research-only.
                if captured > l.monkeys || coin_count > l.coins {
                    inconsistent = true;
                }
                fields.extend([
                    field("Monkeys captured", format!("{captured} / {}", l.monkeys)),
                    field("Monkeys remaining", remaining(captured, l.monkeys)),
                    field("Specter Coins", format!("{coin_count} / {}", l.coins)),
                    field("Coins remaining", remaining(coin_count, l.coins)),
                    field("Best time", time(ticks)),
                ]);
                let (rating, credit) = medal(ticks, thresholds);
                fields.push(field("Time-trial medal", rating));
                // Display the exclusive targets explicitly: equality is not a medal.
                fields.push(field(
                    "Medal targets (under)",
                    format!(
                        "Gold {} · Silver {} · Bronze {}",
                        time(thresholds[2]),
                        time(thresholds[1]),
                        time(thresholds[0])
                    ),
                ));
                points += credit
                    + match clear {
                        1 => 15,
                        2 => 25,
                        _ => 0,
                    };
                cleared += usize::from(matches!(clear, 1 | 2));
                match credit {
                    8 => medals[0] += 1,
                    7 => medals[1] += 1,
                    5 => medals[2] += 1,
                    _ => {}
                }
            } else if l.index == 7 || l.index == 14 {
                fields.push(field("Best race time", time(ticks)));
                let earned = if l.index == 7 { stadium } else { gladiator };
                fields.push(field(
                    "Specter Coin reward",
                    if earned { "5 / 5" } else { "0 / 5" },
                ));
            }
            profile.records.push(Record {
                name: l.name.into(),
                fields,
            });
        }
        if inconsistent || monkeys > 204 || points > 1000 {
            warn(&mut notes, "stored progress or collectible counts are inconsistent; raw counters are retained.");
        }
        let mask = word(p, 4);
        let unlocked: Vec<_> = GADGETS
            .iter()
            .enumerate()
            .filter(|(j, _)| mask & (1 << j) != 0)
            .map(|(_, n)| *n)
            .collect();
        if mask & !0xff != 0 {
            warn(
                &mut notes,
                &format!(
                    "unknown gadget unlock bits 0x{:08X} are present.",
                    mask & !0xff
                ),
            );
        }
        let location = LEVELS
            .iter()
            .find(|l| l.rooms.contains(&(room as u8)) && room < 256)
            .map(|l| l.name.to_string())
            .unwrap_or_else(|| format!("Unknown (room 0x{room:04X})"));
        profile.fields = vec![
            field("Completion", format!("{}.{:01}%", points / 10, points % 10)),
            field("Saved stage", location),
            field("Monkeys captured", format!("{monkeys} / 204")),
            field("Monkeys remaining", remaining(monkeys, 204)),
            field(
                "Specter Coins",
                format!("{} / 60", coins.len() + bonus_coins),
            ),
            field("Coins remaining", remaining(coins.len() + bonus_coins, 60)),
            field("Level coins", format!("{} / 50", coins.len())),
            field("Race reward coins", format!("{bonus_coins} / 10")),
            field("Collecting levels cleared", format!("{cleared} / 19")),
            field(
                "Time-trial medals",
                format!(
                    "{} gold · {} silver · {} bronze",
                    medals[0], medals[1], medals[2]
                ),
            ),
            field("Extra lives", p[2].wrapping_sub(1)),
            field("Energy chips", p[3]),
            field("Guided pellets", p[0x0c]),
            field("Explosive pellets", p[0x0d]),
            field(
                "Unlocked gadgets",
                if unlocked.is_empty() {
                    "None".into()
                } else {
                    unlocked.join(" · ")
                },
            ),
        ];
        for (j, label) in [
            "Triangle gadget",
            "Square gadget",
            "Circle gadget",
            "Cross gadget",
        ]
        .iter()
        .enumerate()
        {
            profile.fields.push(field(label, gadget(p[8 + j])));
        }
        profiles.push(profile);
    }
    Ok(SaveDetails {
        game: format.slug.into(),
        title: format.title.into(),
        release: format!("US · {code}"),
        checksum_ok: None,
        profiles,
        notes,
    })
}
