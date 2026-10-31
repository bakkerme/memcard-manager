//! Metal Gear Solid USA (`SLUS-00594`) read-only story summary.
//!
//! Directory saves are `BA` / `SLUS-00594` / an 8-byte `G…` identifier, one
//! 8192-byte SC block. Three `0xa40`-byte banks sit at `0x100`, `0xb40` and
//! `0x1580`. Interpreted bank fields are reported only when those copies are
//! identical, structurally valid and CRC32-valid. The editor writes the first
//! bank's checksum only; this reader does not adopt that. Offsets below are
//! bank-relative unless marked SC or MCS. MCS offsets from MGSEdit include the
//! 128-byte directory header (`MCS = SC + 0x80`, so `B = MCS - 0x180`).
//! See `docs/formats/metal-gear-solid-us.md`.

use super::{Field, Format, Profile, Record, SaveDetails};
use encoding_rs::SHIFT_JIS;
use unicode_normalization::UnicodeNormalization;

const BANK_BASES: [usize; 3] = [0x100, 0xb40, 0x1580];
const BANK_SIZE: usize = 0xa40;
/// Editor weapon order. Native ammo checks used this same order.
const WEAPONS: [&str; 10] = [
    "Socom",
    "FA-MAS",
    "Grenades",
    "Nikita",
    "Stinger",
    "Claymore",
    "C4",
    "Stun grenades",
    "Chaff grenades",
    "PSG-1",
];
/// Editor filename letters `A`..=`_`. Spellings are the editor's, including "Blast Furnance".
const FILENAME_LOCATIONS: [&str; 31] = [
    "Dock",
    "Heliport",
    "Tank Hangar",
    "Cell",
    "Medi room",
    "Armory",
    "Armory Sth",
    "Canyon",
    "Nuke Bldg 1",
    "Nuke Bldg. B1",
    "Cmnder room",
    "Nuke Bldg. B2",
    "Lab",
    "Cave",
    "U.Grnd Pssge",
    "Comm Twr A",
    "Roof/Comm Twr",
    "Comm Twr B",
    "Twr Wall A",
    "Walkway",
    "Snowfield",
    "Blast Furnance",
    "Cargo Elev.",
    "Warehouse",
    "Warehouse Nth",
    "U.Grnd Base 1",
    "U.Grnd Base 2",
    "U.Grnd Base 3",
    "Cmnd room",
    "Spply Rte.",
    "Esc Route",
];

const NOTE_CONFIRMED: &str = concat!(
    "Difficulty, weapons and ammo, rations, medicine, diazepam, ID card level, life, max life, ",
    "stage, playtime, save count and the stored continue count were checked on native USA loads ",
    "(research jobs 847c6930 and c2c8f3aa). Stealth and Bandana were confirmed by separate ",
    "cold-boot edits. Job 377ea84c cold-loaded max rations (menu 4/5 to 4/7), max medicine ",
    "(1/1 to 1/6), max diazepam (1/9 to 1/12) and radar bit 0x0800 (visible, then hidden, then ",
    "visible). Filename hours and minutes, after clearing bit 0x40 on the minute characters, ",
    "and location codes A (Dock), ] (Cmnd room) and ^ (Spply Rte.) matched native save lists ",
    "and the Shift-JIS titles. Playtime is frames at 60 Hz. The save-screen label and the ",
    "filename both truncate that to minutes; job 377ea84c's titles agree with that minute ",
    "floor. Easy and Normal were seen on native lists. Hard and Extreme are the code-traced ",
    "names; their gameplay effects were not separately established."
);

const NOTE_EDITOR: &str = concat!(
    "Item words other than the native counts, weapon names, option bits other than radar, the ",
    "generic-flags word, filename difficulty bits, the filename disc rule, the filename new-game ",
    "marker and the Meryl bit are the MGSEdit map (ShendoXT/memcardrex-plugins commit ",
    "6928e9c50600b2ea2c8c501f3424dcc219901133, MIT). They were not cold-boot re-proven in job ",
    "377ea84c. Meryl is bit O (mask 0x08) of the byte at bank 0x15A in Additional/Metal Gear ",
    "Solid save.txt; mainWindow.cs does not implement it. The editor clear-count integer is ",
    "not shown. The editor writes a checksum for the first bank only; this reader still ",
    "requires all three banks to agree and does not pick a copy."
);

const NOTE_UNSUPPORTED: &str = concat!(
    "No story completion percentage, numeric clear count, Tuxedo or ending is decoded. Only ",
    "s16d (Cmnd room) and s18a (Spply Rte.) have confirmed stage names. Other stage ids, ",
    "including rank, stay raw. Filename letters other than A, ] and ^ use the editor table. ",
    "Letters outside that table stay raw. Other option bits and other torture-byte bits stay raw."
);

const NOTE_INVENTORY: &str = concat!(
    "Weapon ammo 0 is still carried and -1 is absent. Other negative weapon words stay unknown; ",
    "the editor would mask off the sign bit instead. Ration, medicine and diazepam counts of ",
    "zero are not confirmed absence, and negative words stay unknown. Their maximums are ",
    "separate unsigned words, and a count can be higher than the menu capacity. Stealth and ",
    "Bandana use native words 1 and -1; zero stays unknown there, not the editor's absent. ",
    "Other item flags follow the editor: a positive word is available and zero or a negative ",
    "word is absent. ID card level 7 was seen in the item menu; levels 1 through 6 use the same ",
    "word and were not each cold-loaded. The stored continue count can lag the live counter ",
    "immediately after a Codec save; the update boundary is unconfirmed. The editor UI clamps ",
    "life at 1400 and playtime at 99:59:59; stored values are shown without those clamps. Save ",
    "count is the native 16-bit word. The editor text calls that byte an 8-bit counter."
);

const NOTE_CRC_OK: &str = concat!(
    "All three banks match and each CRC32 matches. The checksum covers the 2616 bytes from ",
    "bank+8 through the end of the 0xa40-byte bank. The save title and the directory filename ",
    "are outside that range."
);

const NOTE_CRC_BAD: &str = concat!(
    "All three banks match each other and fail CRC32. Weapon, item, life, stage, time, save ",
    "count, continue, radar, flag and Meryl fields are withheld. Filename fields stay visible."
);

const NOTE_CRC_UNRESOLVED: &str = concat!(
    "The three bank copies do not agree, so there is no single checksum verdict. Weapon, item, ",
    "life, stage, time, save count, continue, radar, flag and Meryl fields are withheld. ",
    "Filename fields stay visible. A valid first bank is not preferred over the other copies."
);

fn field(label: &str, value: impl Into<String>) -> Field {
    Field {
        label: label.into(),
        value: value.into(),
    }
}

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn i16_at(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

/// CRC-32/ISO-HDLC, the same algorithm as `zlib.crc32`.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
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

fn saved_title(data: &[u8]) -> Result<String, String> {
    let raw = &data[4..68];
    let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
    let (title, _, malformed) = SHIFT_JIS.decode(&raw[..end]);
    if malformed {
        return Err("Unsupported Metal Gear Solid USA title.".into());
    }
    Ok(title.nfkc().collect::<String>().trim().to_string())
}

struct BankVerdict {
    structure_ok: bool,
    crc_ok: Option<bool>,
}

fn bank_verdict(bank: &[u8]) -> BankVerdict {
    let structure_ok =
        u32_at(bank, 0) == BANK_SIZE as u32 && u32_at(bank, 8) == 95 && u32_at(bank, 12) == 2048;
    let crc_ok = if structure_ok {
        Some(u32_at(bank, 4) == crc32(&bank[8..]))
    } else {
        None
    };
    BankVerdict {
        structure_ok,
        crc_ok,
    }
}

fn stage_id(bank: &[u8]) -> Option<String> {
    let raw = &bank[0x20..0x30];
    let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
    let bytes = &raw[..end];
    if bytes.is_empty() {
        return None;
    }
    let Ok(id) = std::str::from_utf8(bytes) else {
        return None;
    };
    if !id.is_ascii() {
        return None;
    }
    Some(id.to_string())
}

fn stage_label(id: Option<&str>) -> String {
    match id {
        Some("s16d") => "Cmnd room (s16d)".into(),
        Some("s18a") => "Spply Rte. (s18a)".into(),
        Some(id) => id.to_string(),
        None => "Unknown".into(),
    }
}

fn difficulty_label(code: u16) -> String {
    match code {
        0 => "Easy".into(),
        1 => "Normal".into(),
        2 => "Hard".into(),
        3 => "Extreme".into(),
        other => format!("Unknown ({other})"),
    }
}

fn playtime(ticks: u32) -> String {
    let seconds = ticks / 60;
    format!(
        "{:02}:{:02}:{:02} ({ticks} ticks)",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

fn signed_known(value: i16) -> String {
    if value >= 0 {
        value.to_string()
    } else {
        format!("Unknown ({value})")
    }
}

fn max_life_label(value: i16) -> String {
    if value > 0 {
        value.to_string()
    } else {
        format!("Unknown ({value})")
    }
}

fn id_card_label(level: i16) -> String {
    if (1..=7).contains(&level) {
        format!("Level {level}")
    } else {
        format!("Unknown ({level})")
    }
}

fn availability(value: i16) -> String {
    match value {
        1 => "Available".into(),
        -1 => "Absent".into(),
        other => format!("Unknown ({other})"),
    }
}

fn editor_flag(value: i16) -> &'static str {
    if value > 0 {
        "Available (editor)"
    } else {
        "Absent (editor)"
    }
}

fn radar_label(word: u16) -> &'static str {
    if word & 0x0800 != 0 {
        "Hidden"
    } else {
        "Visible"
    }
}

fn meryl_label(byte: u8) -> &'static str {
    if byte & 0x08 != 0 {
        "Dies (bit O)"
    } else {
        "Bit O clear"
    }
}

/// Editor `GHHMMCLI` suffix. `0x40` on a minute character is the difficulty bit.
struct FilenameRead {
    difficulty: String,
    location: String,
    time: String,
    code: String,
    disc: String,
    clear_marker: String,
}

fn filename_read(identifier: &str) -> FilenameRead {
    let bytes = identifier.as_bytes();
    let printable = bytes.len() == 8
        && bytes[0] == b'G'
        && bytes.iter().all(|byte| (0x20..0x7f).contains(byte));
    if !printable {
        return FilenameRead {
            difficulty: "Unknown".into(),
            location: "Unknown".into(),
            time: "Unknown".into(),
            code: "Unknown".into(),
            disc: "Unknown".into(),
            clear_marker: "Unknown".into(),
        };
    }
    let difficulty = match (bytes[3] & 0x40 != 0, bytes[4] & 0x40 != 0) {
        (false, false) => "Easy (editor)",
        (false, true) => "Normal (editor)",
        (true, false) => "Hard (editor)",
        (true, true) => "Extreme (editor)",
    };
    let code = bytes[5];
    FilenameRead {
        difficulty: difficulty.into(),
        location: filename_location(bytes[6]),
        time: filename_time(bytes),
        code: (code as char).to_string(),
        disc: if code >= 0x48 {
            "Disc 2 (editor)".into()
        } else {
            "Disc 1 (editor)".into()
        },
        clear_marker: clear_marker(code),
    }
}

fn filename_time(identifier: &[u8]) -> String {
    let hours = &identifier[1..3];
    let minutes = [identifier[3] & !0x40, identifier[4] & !0x40];
    if hours.iter().all(|byte| byte.is_ascii_digit())
        && minutes.iter().all(|byte| byte.is_ascii_digit())
        && minutes[0] < b'6'
    {
        format!(
            "{}{}:{}{}",
            hours[0] as char, hours[1] as char, minutes[0] as char, minutes[1] as char
        )
    } else {
        "Unknown".into()
    }
}

fn filename_location(byte: u8) -> String {
    if (0x41..0x60).contains(&byte) {
        let name = FILENAME_LOCATIONS[(byte - 0x41) as usize];
        let letter = byte as char;
        if matches!(byte, b'A' | b']' | b'^') {
            format!("{name} ({letter})")
        } else {
            format!("{name} ({letter}, editor)")
        }
    } else {
        format!("Unknown ('{}')", byte as char)
    }
}

fn clear_marker(byte: u8) -> String {
    if byte == b'@' {
        return "New game (editor)".into();
    }
    // `(byte - 0x40) & 7` is the editor's clear count. `0x40` is divisible by 8,
    // so those low bits are `byte & 7`. The integer itself is not shown.
    if byte & 0x07 != 0 {
        "Cleared (editor, count not shown)".into()
    } else {
        "No clear bits (editor, count not shown)".into()
    }
}

#[derive(Clone, Copy)]
enum ItemKind {
    NativeFlag,
    EditorFlag,
    NativeCount(usize),
    IdCard,
    EditorSeconds,
}

struct ItemSpec {
    name: &'static str,
    kind: ItemKind,
}

/// Editor item order, 24 words at bank `0x8a`. The save.txt prose says 23; the
/// arrays and this name list are 24 and end at max rations (`0xba`).
const ITEMS: [ItemSpec; 24] = [
    ItemSpec {
        name: "Cigarettes",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Magnifying scope",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Cardboard box A",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Cardboard box B",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Cardboard box C",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Night-vision goggles",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Thermal goggles",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Gas mask",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Body armor",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Ketchup",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Stealth",
        kind: ItemKind::NativeFlag,
    },
    ItemSpec {
        name: "Bandana",
        kind: ItemKind::NativeFlag,
    },
    ItemSpec {
        name: "Camera",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Rations",
        kind: ItemKind::NativeCount(0xba),
    },
    ItemSpec {
        name: "Cold medicine",
        kind: ItemKind::NativeCount(0xbc),
    },
    ItemSpec {
        name: "Diazepam",
        kind: ItemKind::NativeCount(0xbe),
    },
    ItemSpec {
        name: "PAL card key",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "ID card",
        kind: ItemKind::IdCard,
    },
    ItemSpec {
        name: "Timer bomb",
        kind: ItemKind::EditorSeconds,
    },
    ItemSpec {
        name: "Mine detector",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "MO disc",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Rope",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Handkerchief",
        kind: ItemKind::EditorFlag,
    },
    ItemSpec {
        name: "Suppressor",
        kind: ItemKind::EditorFlag,
    },
];

fn weapon_record(bank: &[u8], index: usize) -> Record {
    let current = i16_at(bank, 0x62 + 2 * index);
    let capacity = i16_at(bank, 0x76 + 2 * index);
    let carried = if current >= 0 {
        "Yes"
    } else if current == -1 {
        "No"
    } else {
        "Unknown"
    };
    let ammo = if current >= 0 {
        current.to_string()
    } else if current == -1 {
        "Absent".into()
    } else {
        format!("Unknown ({current})")
    };
    let capacity = if capacity >= 0 {
        capacity.to_string()
    } else {
        format!("Unknown ({capacity})")
    };
    Record {
        name: WEAPONS[index].into(),
        fields: vec![
            field("Carried", carried),
            field("Ammo", ammo),
            field("Capacity", capacity),
        ],
    }
}

fn item_record(bank: &[u8], index: usize) -> Record {
    let value = i16_at(bank, 0x8a + 2 * index);
    let spec = &ITEMS[index];
    let fields = match spec.kind {
        ItemKind::NativeFlag => vec![
            field("Status", availability(value)),
            field("Stored", value.to_string()),
        ],
        ItemKind::EditorFlag => vec![
            field("Status", editor_flag(value)),
            field("Stored", value.to_string()),
        ],
        ItemKind::NativeCount(max_at) => vec![
            field("Count", signed_known(value)),
            field("Max", u16_at(bank, max_at).to_string()),
        ],
        ItemKind::IdCard => vec![
            field("Level", id_card_label(value)),
            field("Stored", value.to_string()),
        ],
        ItemKind::EditorSeconds => vec![
            field("Status", editor_flag(value)),
            field("Seconds", signed_known(value)),
        ],
    };
    Record {
        name: spec.name.into(),
        fields,
    }
}

fn filename_fields(read: &FilenameRead) -> Vec<Field> {
    vec![
        field("Filename difficulty (editor)", read.difficulty.clone()),
        field("Filename location", read.location.clone()),
        field("Filename time", read.time.clone()),
        field("Filename code", read.code.clone()),
        field("Disc (editor)", read.disc.clone()),
        field("Clear marker (editor)", read.clear_marker.clone()),
    ]
}

pub(super) fn decode(
    fmt: &Format,
    code: &str,
    identifier: &str,
    data: &[u8],
) -> Result<SaveDetails, String> {
    // Dispatch already checked BA / SLUS-00594 / the G… identifier, one block and SC size.
    if !matches!(data[2], 0x11 | 0x12 | 0x13) {
        return Err("Unsupported Metal Gear Solid USA header or icon layout.".into());
    }
    let title = saved_title(data)?;
    let banks: Vec<BankVerdict> = BANK_BASES
        .iter()
        .map(|&base| bank_verdict(&data[base..base + BANK_SIZE]))
        .collect();
    if !banks.iter().any(|bank| bank.structure_ok) {
        return Err("Unsupported Metal Gear Solid USA story layout.".into());
    }
    let first = &data[BANK_BASES[0]..BANK_BASES[0] + BANK_SIZE];
    let identical = BANK_BASES[1..]
        .iter()
        .all(|&base| &data[base..base + BANK_SIZE] == first);
    let uniform_valid = identical && banks.iter().all(|bank| bank.crc_ok == Some(true));
    let checksum_ok = if uniform_valid {
        Some(true)
    } else if identical
        && banks.iter().all(|bank| bank.structure_ok)
        && banks.iter().all(|bank| bank.crc_ok == Some(false))
    {
        Some(false)
    } else {
        None
    };

    let title_value = if title.is_empty() {
        "(no title)".to_string()
    } else {
        title.clone()
    };
    let profile_name = if title.is_empty() {
        "Metal Gear Solid".to_string()
    } else {
        title
    };

    let filename = filename_read(identifier);
    let mut notes = vec![
        NOTE_CONFIRMED.into(),
        NOTE_EDITOR.into(),
        NOTE_UNSUPPORTED.into(),
    ];
    let (fields, records) = if uniform_valid {
        let life = i16_at(first, 0x56);
        let max_life = i16_at(first, 0x58);
        let options = u16_at(first, 0x44);
        let torture = first[0x15a];
        notes.push(NOTE_INVENTORY.into());
        notes.push(NOTE_CRC_OK.into());
        if life >= 0 && max_life > 0 && life > max_life {
            notes.push(
                "Stored life is above stored max life and is shown as stored. A native load of stored life 1000 was clamped to max life 768.".into(),
            );
        }
        let mut fields = vec![
            field("Saved title", title_value),
            field("Difficulty", difficulty_label(u16_at(first, 0x42))),
            field("Stage", stage_label(stage_id(first).as_deref())),
        ];
        fields.extend(filename_fields(&filename));
        fields.extend([
            field("Playtime", playtime(u32_at(first, 0x10))),
            field("Save count", u16_at(first, 0xf0).to_string()),
            field("Continues", signed_known(i16_at(first, 0xee))),
            field("Life", signed_known(life)),
            field("Max life", max_life_label(max_life)),
            field("Radar", radar_label(options)),
            field("Option flags", format!("0x{options:04X}")),
            field("Generic flags", format!("0x{:08X}", u32_at(first, 0x3c))),
            field("ID card", id_card_label(i16_at(first, 0xac))),
            field("Rations", signed_known(i16_at(first, 0xa4))),
            field("Max rations", u16_at(first, 0xba).to_string()),
            field("Medicine", signed_known(i16_at(first, 0xa6))),
            field("Max medicine", u16_at(first, 0xbc).to_string()),
            field("Diazepam", signed_known(i16_at(first, 0xa8))),
            field("Max diazepam", u16_at(first, 0xbe).to_string()),
            field("Stealth", availability(i16_at(first, 0x9e))),
            field("Bandana", availability(i16_at(first, 0xa0))),
            field("Meryl (editor)", meryl_label(torture)),
            field("Torture flags", format!("0x{torture:02X}")),
        ]);
        let mut records: Vec<Record> = (0..WEAPONS.len())
            .map(|index| weapon_record(first, index))
            .collect();
        records.extend((0..ITEMS.len()).map(|index| item_record(first, index)));
        (fields, records)
    } else {
        notes.push(if checksum_ok == Some(false) {
            NOTE_CRC_BAD.into()
        } else {
            NOTE_CRC_UNRESOLVED.into()
        });
        let withheld = if checksum_ok == Some(false) {
            "Withheld (CRC mismatch)"
        } else {
            "Withheld (no single bank verdict)"
        };
        let mut fields = vec![field("Saved title", title_value)];
        fields.extend(filename_fields(&filename));
        fields.push(field("Bank data", withheld));
        (fields, vec![])
    };

    Ok(SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!("NTSC-U · {code}"),
        checksum_ok,
        profiles: vec![Profile {
            number: 1,
            name: profile_name,
            empty: false,
            status: None,
            checksum_ok,
            fields,
            records,
        }],
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::{crc32, BANK_BASES, BANK_SIZE};
    use crate::card::engine::{Ps1Card, BLOCK_SIZE, MCS_HEADER_SIZE};
    use crate::card::game_details::{self, GameDetails};
    use sha2::{Digest, Sha256};

    /// Shift-JIS for `ＭＧＳ　［ＥＺ］　００：００　Ｄｏｃｋ`, the research decoder's sample title.
    const SAMPLE_TITLE: &[u8] = &[
        0x82, 0x6c, 0x82, 0x66, 0x82, 0x72, 0x81, 0x40, 0x81, 0x6d, 0x82, 0x64, 0x82, 0x79, 0x81,
        0x6e, 0x81, 0x40, 0x82, 0x4f, 0x82, 0x4f, 0x81, 0x46, 0x82, 0x4f, 0x82, 0x4f, 0x81, 0x40,
        0x82, 0x63, 0x82, 0x8f, 0x82, 0x83, 0x82, 0x8b,
    ];

    /// SHA-256 of the research decoder's default `sample()` payload.
    const SAMPLE_SHA256: &str = "545bce6ebee5aeb6be7bdb9ff59ef5aa6734e85814e635303b29f77c3aa3155a";

    fn sample() -> Vec<u8> {
        let mut data = vec![0u8; 8192];
        data[..4].copy_from_slice(b"SC\x11\x01");
        data[4..4 + SAMPLE_TITLE.len()].copy_from_slice(SAMPLE_TITLE);
        let mut bank = vec![0u8; BANK_SIZE];
        bank[..4].copy_from_slice(&(BANK_SIZE as u32).to_le_bytes());
        bank[8..12].copy_from_slice(&95u32.to_le_bytes());
        bank[12..16].copy_from_slice(&2048u32.to_le_bytes());
        bank[0x20..0x24].copy_from_slice(b"zzzz");
        bank[0x56..0x58].copy_from_slice(&256i16.to_le_bytes());
        bank[0x58..0x5a].copy_from_slice(&256i16.to_le_bytes());
        let sum = crc32(&bank[8..]);
        bank[4..8].copy_from_slice(&sum.to_le_bytes());
        for base in BANK_BASES {
            data[base..base + BANK_SIZE].copy_from_slice(&bank);
        }
        data
    }

    fn edit(data: &[u8], offset: usize, bytes: &[u8], banks: &[usize], repair: bool) -> Vec<u8> {
        let mut data = data.to_vec();
        for &base in banks {
            let start = base + offset;
            data[start..start + bytes.len()].copy_from_slice(bytes);
            if repair {
                let sum = crc32(&data[base + 8..base + BANK_SIZE]);
                data[base + 4..base + 8].copy_from_slice(&sum.to_le_bytes());
            }
        }
        data
    }

    fn sha256(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn summary(id: &str, data: &[u8]) -> game_details::SaveDetails {
        match game_details::decode("SLUS-00594", "BA", id, data).unwrap() {
            GameDetails::Summary(details) => details,
            other => panic!("expected summary, got {other:?}"),
        }
    }

    fn pairs(details: &game_details::SaveDetails) -> Vec<(&str, &str)> {
        details.profiles[0]
            .fields
            .iter()
            .map(|field| (field.label.as_str(), field.value.as_str()))
            .collect()
    }

    fn value(details: &game_details::SaveDetails, label: &str) -> String {
        pairs(details)
            .into_iter()
            .find(|(name, _)| *name == label)
            .unwrap()
            .1
            .to_string()
    }

    fn directory_field(label: &str) -> bool {
        label == "Saved title"
            || label == "Bank data"
            || label.starts_with("Filename")
            || label == "Disc (editor)"
            || label == "Clear marker (editor)"
    }

    fn record<'a>(details: &'a game_details::SaveDetails, name: &str) -> Vec<(&'a str, &'a str)> {
        details.profiles[0]
            .records
            .iter()
            .find(|record| record.name == name)
            .unwrap()
            .fields
            .iter()
            .map(|field| (field.label.as_str(), field.value.as_str()))
            .collect()
    }

    fn record_dump(details: &game_details::SaveDetails) -> Vec<(String, Vec<(String, String)>)> {
        details.profiles[0]
            .records
            .iter()
            .map(|record| {
                (
                    record.name.clone(),
                    record
                        .fields
                        .iter()
                        .map(|field| (field.label.clone(), field.value.clone()))
                        .collect(),
                )
            })
            .collect()
    }

    #[test]
    fn crc32_matches_the_iso_hdlc_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
    }

    #[test]
    fn research_sample_hash_and_confirmed_zero_bank() {
        let data = sample();
        assert_eq!(sha256(&data), SAMPLE_SHA256);
        let before = data.clone();
        let details = summary("G0000BAA", &data);
        assert_eq!(data, before);
        assert_eq!(details.game, "metal-gear-solid");
        assert_eq!(details.title, "Metal Gear Solid");
        assert_eq!(details.release, "NTSC-U · SLUS-00594");
        assert_eq!(details.checksum_ok, Some(true));
        assert_eq!(details.profiles[0].checksum_ok, Some(true));
        assert_eq!(details.profiles[0].name, "MGS [EZ] 00:00 Dock");
        assert_eq!(details.profiles[0].number, 1);
        assert!(!details.profiles[0].empty);
        assert_eq!(
            pairs(&details),
            vec![
                ("Saved title", "MGS [EZ] 00:00 Dock"),
                ("Difficulty", "Easy"),
                ("Stage", "zzzz"),
                ("Filename difficulty (editor)", "Easy (editor)"),
                ("Filename location", "Dock (A)"),
                ("Filename time", "00:00"),
                ("Filename code", "B"),
                ("Disc (editor)", "Disc 1 (editor)"),
                ("Clear marker (editor)", "Cleared (editor, count not shown)"),
                ("Playtime", "00:00:00 (0 ticks)"),
                ("Save count", "0"),
                ("Continues", "0"),
                ("Life", "256"),
                ("Max life", "256"),
                ("Radar", "Visible"),
                ("Option flags", "0x0000"),
                ("Generic flags", "0x00000000"),
                ("ID card", "Unknown (0)"),
                ("Rations", "0"),
                ("Max rations", "0"),
                ("Medicine", "0"),
                ("Max medicine", "0"),
                ("Diazepam", "0"),
                ("Max diazepam", "0"),
                ("Stealth", "Unknown (0)"),
                ("Bandana", "Unknown (0)"),
                ("Meryl (editor)", "Bit O clear"),
                ("Torture flags", "0x00"),
            ]
        );
        assert_eq!(details.profiles[0].records.len(), 34);
        assert_eq!(details.profiles[0].records[0].name, "Socom");
        assert_eq!(details.profiles[0].records[9].name, "PSG-1");
        assert_eq!(details.profiles[0].records[10].name, "Cigarettes");
        assert_eq!(details.profiles[0].records[33].name, "Suppressor");
        assert_eq!(
            record(&details, "Socom"),
            vec![("Carried", "Yes"), ("Ammo", "0"), ("Capacity", "0")]
        );
        assert_eq!(
            record(&details, "Claymore"),
            vec![("Carried", "Yes"), ("Ammo", "0"), ("Capacity", "0")]
        );
        assert_eq!(
            record(&details, "Cigarettes"),
            vec![("Status", "Absent (editor)"), ("Stored", "0")]
        );
        assert_eq!(
            record(&details, "Rations"),
            vec![("Count", "0"), ("Max", "0")]
        );
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("No story completion percentage")
                && note.contains("clear count")
                && note.contains("Tuxedo")));
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("847c6930") && note.contains("c2c8f3aa")));
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("377ea84c") && note.contains("first bank")));
        assert!(details.profiles[0]
            .fields
            .iter()
            .all(|field| !field.label.to_ascii_lowercase().contains("completion")));
        assert!(details.profiles[0]
            .fields
            .iter()
            .all(|field| !field.value.contains("times")));
    }

    #[test]
    fn native_anchor_words_and_same_minute_progress_stay_distinct() {
        let mut data = sample();
        data = edit(&data, 0x10, &2_079_543u32.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x20, b"s16d\0", &BANK_BASES, true);
        data = edit(&data, 0x42, &1u16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x56, &570i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x58, &768i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x62, &122i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x76, &217i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x62 + 10, &2i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x76 + 10, &5i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x62 + 14, &14i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x76 + 14, &31i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x62 + 16, &7i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x76 + 16, &29i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0x9e, &1i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0xa0, &(-1i16).to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0xa4, &2i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0xa6, &4i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0xa8, &4i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0xac, &7i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0xee, &59i16.to_le_bytes(), &BANK_BASES, true);
        data = edit(&data, 0xf0, &48u16.to_le_bytes(), &BANK_BASES, true);
        let before = data.clone();
        let details = summary("G0000BAA", &data);
        assert_eq!(data, before);
        assert_eq!(details.checksum_ok, Some(true));
        assert_eq!(value(&details, "Difficulty"), "Normal");
        assert_eq!(value(&details, "Stage"), "Cmnd room (s16d)");
        assert_eq!(value(&details, "Playtime"), "09:37:39 (2079543 ticks)");
        assert_eq!(value(&details, "Save count"), "48");
        assert_eq!(value(&details, "Continues"), "59");
        assert_eq!(value(&details, "Life"), "570");
        assert_eq!(value(&details, "Max life"), "768");
        assert_eq!(value(&details, "ID card"), "Level 7");
        assert_eq!(value(&details, "Rations"), "2");
        assert_eq!(value(&details, "Medicine"), "4");
        assert_eq!(value(&details, "Diazepam"), "4");
        assert_eq!(value(&details, "Stealth"), "Available");
        assert_eq!(value(&details, "Bandana"), "Absent");
        assert_eq!(
            record(&details, "Socom"),
            vec![("Carried", "Yes"), ("Ammo", "122"), ("Capacity", "217")]
        );
        assert_eq!(
            record(&details, "Claymore"),
            vec![("Carried", "Yes"), ("Ammo", "2"), ("Capacity", "5")]
        );
        assert_eq!(
            record(&details, "Stun grenades"),
            vec![("Carried", "Yes"), ("Ammo", "14"), ("Capacity", "31")]
        );
        assert_eq!(
            record(&details, "Chaff grenades"),
            vec![("Carried", "Yes"), ("Ammo", "7"), ("Capacity", "29")]
        );
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("lag the live counter")));

        let zero_socom = edit(&data, 0x62, &0i16.to_le_bytes(), &BANK_BASES, true);
        let later = edit(&data, 0x10, &2_084_646u32.to_le_bytes(), &BANK_BASES, true);
        let later = edit(&later, 0xf0, &49u16.to_le_bytes(), &BANK_BASES, true);
        let zero = summary("G0000BAA", &zero_socom);
        let next = summary("G0000BAA", &later);
        assert_eq!(value(&details, "Saved title"), value(&zero, "Saved title"));
        assert_eq!(value(&details, "Playtime"), value(&zero, "Playtime"));
        assert_eq!(value(&details, "Save count"), value(&zero, "Save count"));
        assert_eq!(pairs(&details), pairs(&zero));
        assert_eq!(record(&zero, "Socom")[0].1, "Yes");
        assert_eq!(record(&zero, "Socom")[1].1, "0");
        assert_ne!(record(&details, "Socom"), record(&zero, "Socom"));
        assert_eq!(value(&next, "Playtime"), "09:39:04 (2084646 ticks)");
        assert_eq!(value(&next, "Save count"), "49");
        assert_ne!(value(&details, "Playtime"), value(&next, "Playtime"));
    }

    #[test]
    fn named_difficulties_stages_boundaries_and_unclaimed_words() {
        for (code, label) in [(0u16, "Easy"), (1, "Normal"), (2, "Hard"), (3, "Extreme")] {
            let data = edit(&sample(), 0x42, &code.to_le_bytes(), &BANK_BASES, true);
            assert_eq!(value(&summary("G0000BAA", &data), "Difficulty"), label);
        }
        let unknown = edit(&sample(), 0x42, &65535u16.to_le_bytes(), &BANK_BASES, true);
        assert_eq!(
            value(&summary("G0000BAA", &unknown), "Difficulty"),
            "Unknown (65535)"
        );

        let supply = edit(&sample(), 0x20, b"s18a\0", &BANK_BASES, true);
        assert_eq!(
            value(&summary("G0000BAA", &supply), "Stage"),
            "Spply Rte. (s18a)"
        );
        let rank = edit(&sample(), 0x20, b"rank\0", &BANK_BASES, true);
        assert_eq!(value(&summary("G0000BAA", &rank), "Stage"), "rank");
        let blank = edit(&sample(), 0x20, &[0; 4], &BANK_BASES, true);
        assert_eq!(value(&summary("G0000BAA", &blank), "Stage"), "Unknown");
        let binary = edit(&sample(), 0x20, &[0xff, 0xfe], &BANK_BASES, true);
        assert_eq!(value(&summary("G0000BAA", &binary), "Stage"), "Unknown");

        for ticks in [0u32, 255, 256, 32767, 32768, 65535, u32::MAX] {
            let data = edit(&sample(), 0x10, &ticks.to_le_bytes(), &BANK_BASES, true);
            let shown = value(&summary("G0000BAA", &data), "Playtime");
            assert!(
                shown.contains(&format!("({ticks} ticks)")),
                "{shown} missing {ticks}"
            );
        }
        let max_ticks = edit(&sample(), 0x10, &u32::MAX.to_le_bytes(), &BANK_BASES, true);
        assert_eq!(
            value(&summary("G0000BAA", &max_ticks), "Playtime"),
            "19884:06:28 (4294967295 ticks)"
        );
        for count in [0u16, 255, 256, 32767, 32768, 65535] {
            let data = edit(&sample(), 0xf0, &count.to_le_bytes(), &BANK_BASES, true);
            assert_eq!(
                value(&summary("G0000BAA", &data), "Save count"),
                count.to_string()
            );
        }
        for continues in [0i16, 59, 60, 321, 32767] {
            let data = edit(&sample(), 0xee, &continues.to_le_bytes(), &BANK_BASES, true);
            assert_eq!(
                value(&summary("G0000BAA", &data), "Continues"),
                continues.to_string()
            );
        }
        let negative = edit(&sample(), 0xee, &(-1i16).to_le_bytes(), &BANK_BASES, true);
        assert_eq!(
            value(&summary("G0000BAA", &negative), "Continues"),
            "Unknown (-1)"
        );

        let over = edit(&sample(), 0x56, &1000i16.to_le_bytes(), &BANK_BASES, true);
        let over = edit(&over, 0x58, &768i16.to_le_bytes(), &BANK_BASES, true);
        let details = summary("G0000BAA", &over);
        assert_eq!(value(&details, "Life"), "1000");
        assert_eq!(value(&details, "Max life"), "768");
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("clamped to max life 768")));
        let dead = edit(&sample(), 0x56, &(-1i16).to_le_bytes(), &BANK_BASES, true);
        assert_eq!(value(&summary("G0000BAA", &dead), "Life"), "Unknown (-1)");
        let no_max = edit(&sample(), 0x58, &0i16.to_le_bytes(), &BANK_BASES, true);
        assert_eq!(
            value(&summary("G0000BAA", &no_max), "Max life"),
            "Unknown (0)"
        );

        let absent = edit(
            &sample(),
            0x62 + 10,
            &(-1i16).to_le_bytes(),
            &BANK_BASES,
            true,
        );
        assert_eq!(
            record(&summary("G0000BAA", &absent), "Claymore"),
            vec![("Carried", "No"), ("Ammo", "Absent"), ("Capacity", "0")]
        );
        let odd = edit(&sample(), 0x62, &(-2i16).to_le_bytes(), &BANK_BASES, true);
        let odd = edit(&odd, 0xa4, &(-2i16).to_le_bytes(), &BANK_BASES, true);
        let details = summary("G0000BAA", &odd);
        assert_eq!(
            record(&details, "Socom"),
            vec![
                ("Carried", "Unknown"),
                ("Ammo", "Unknown (-2)"),
                ("Capacity", "0")
            ]
        );
        assert_eq!(value(&details, "Rations"), "Unknown (-2)");
        for level in 1i16..=7 {
            let data = edit(&sample(), 0xac, &level.to_le_bytes(), &BANK_BASES, true);
            assert_eq!(
                value(&summary("G0000BAA", &data), "ID card"),
                format!("Level {level}")
            );
        }
        let high = edit(&sample(), 0xac, &8i16.to_le_bytes(), &BANK_BASES, true);
        assert_eq!(value(&summary("G0000BAA", &high), "ID card"), "Unknown (8)");
        let both = edit(&sample(), 0x9e, &(-1i16).to_le_bytes(), &BANK_BASES, true);
        let both = edit(&both, 0xa0, &1i16.to_le_bytes(), &BANK_BASES, true);
        let details = summary("G0000BAA", &both);
        assert_eq!(value(&details, "Stealth"), "Absent");
        assert_eq!(value(&details, "Bandana"), "Available");
        let other = edit(&sample(), 0x9e, &2i16.to_le_bytes(), &BANK_BASES, true);
        assert_eq!(
            value(&summary("G0000BAA", &other), "Stealth"),
            "Unknown (2)"
        );

        let unclamped = edit(&sample(), 0x56, &2000i16.to_le_bytes(), &BANK_BASES, true);
        assert_eq!(value(&summary("G0000BAA", &unclamped), "Life"), "2000");

        let base = summary("G0000BAA", &sample());
        let baseline = pairs(&base);
        let base_records = record_dump(&base);
        for offset in [0x40usize, 0x46] {
            let data = edit(&sample(), offset, &3u16.to_le_bytes(), &BANK_BASES, true);
            let details = summary("G0000BAA", &data);
            assert_eq!(
                pairs(&details),
                baseline,
                "offset {offset:#x} changed a displayed field"
            );
            assert_eq!(
                record_dump(&details),
                base_records,
                "offset {offset:#x} changed a record"
            );
        }
        let pal = edit(&sample(), 0xaa, &3u16.to_le_bytes(), &BANK_BASES, true);
        let details = summary("G0000BAA", &pal);
        assert_eq!(pairs(&details), baseline);
        assert_eq!(
            record(&details, "PAL card key"),
            vec![("Status", "Available (editor)"), ("Stored", "3")]
        );
    }

    #[test]
    fn checksum_consensus_withholds_disagreeing_or_broken_banks() {
        let flipped = edit(&sample(), 0x42, &1u16.to_le_bytes(), &[0xb40], true);
        let details = summary("G0000BAA", &flipped);
        assert_eq!(details.checksum_ok, None);
        assert!(details.profiles[0].records.is_empty());
        assert_eq!(value(&details, "Saved title"), "MGS [EZ] 00:00 Dock");
        assert_eq!(
            value(&details, "Bank data"),
            "Withheld (no single bank verdict)"
        );
        assert!(pairs(&details)
            .iter()
            .all(|(label, _)| directory_field(label)));
        assert!(pairs(&details).iter().all(|(label, _)| *label != "Radar"));
        assert_eq!(value(&details, "Filename location"), "Dock (A)");

        let first_bank = edit(&sample(), 0x44, &0x0800u16.to_le_bytes(), &[0x100], true);
        let details = summary("G0000BAA", &first_bank);
        assert_eq!(details.checksum_ok, None);
        assert!(pairs(&details)
            .iter()
            .all(|(label, _)| directory_field(label)));
        assert!(pairs(&details).iter().all(|(label, _)| *label != "Radar"));

        let one_bad = edit(&sample(), 0x56, &1i16.to_le_bytes(), &[0x100], false);
        let details = summary("G0000BAA", &one_bad);
        assert_eq!(details.checksum_ok, None);
        assert_eq!(
            value(&details, "Bank data"),
            "Withheld (no single bank verdict)"
        );

        let all_bad = edit(&sample(), 0x56, &1i16.to_le_bytes(), &BANK_BASES, false);
        let details = summary("G0000BAA", &all_bad);
        assert_eq!(details.checksum_ok, Some(false));
        assert_eq!(details.profiles[0].checksum_ok, Some(false));
        assert_eq!(value(&details, "Bank data"), "Withheld (CRC mismatch)");
        assert!(details.profiles[0].records.is_empty());
        assert!(details.notes.iter().any(|note| note.contains("fail CRC32")));

        let mut mixed = edit(&sample(), 0x56, &1i16.to_le_bytes(), &BANK_BASES, false);
        mixed = edit(&mixed, 0x56, &2i16.to_le_bytes(), &[0xb40], false);
        let details = summary("G0000BAA", &mixed);
        assert_eq!(details.checksum_ok, None);
        assert_eq!(
            value(&details, "Bank data"),
            "Withheld (no single bank verdict)"
        );

        let mut padding = sample();
        padding[0x1fc0] ^= 1;
        let details = summary("G0000BAA", &padding);
        assert_eq!(details.checksum_ok, Some(true));
        assert_eq!(value(&details, "Life"), "256");
    }

    #[test]
    fn rejects_other_releases_identifiers_headers_and_layouts() {
        let data = sample();
        let format = game_details::format("SLUS-00594").unwrap();
        assert_eq!(format.slug, "metal-gear-solid");
        assert_eq!(format.region, "BA");
        assert_eq!(format.blocks, 1);
        assert!(format.accepts_identifier("G0000BAA"));
        assert!(!format.accepts_identifier("V00000@A"));
        assert!(!format.accepts_identifier("G"));
        assert!(!format.accepts_identifier("G000\nBAA"));
        assert!(game_details::format("SLES-01370").is_none());
        assert!(game_details::format("SLUS-00776").is_none());
        assert!(game_details::format("SLUS-00957").is_none());
        assert!(game_details::decode("SLUS-00594", "BI", "G0000BAA", &data).is_err());
        assert!(game_details::decode("SLUS-00594", "BE", "G0000BAA", &data).is_err());
        assert!(game_details::decode("SLUS-00594", "BA", "V00000@A", &data).is_err());

        for icon in [0x11u8, 0x12, 0x13] {
            let mut header = data.clone();
            header[2] = icon;
            assert!(
                game_details::decode("SLUS-00594", "BA", "G0000BAA", &header).is_ok(),
                "icon {icon:#x}"
            );
        }
        let mut bad_icon = data.clone();
        bad_icon[2] = 0x10;
        assert!(game_details::decode("SLUS-00594", "BA", "G0000BAA", &bad_icon).is_err());
        let mut bad_blocks = data.clone();
        bad_blocks[3] = 2;
        assert!(game_details::decode("SLUS-00594", "BA", "G0000BAA", &bad_blocks).is_err());

        let mut bad_title = data.clone();
        bad_title[4..68].fill(0);
        bad_title[4] = 0x81;
        assert!(game_details::decode("SLUS-00594", "BA", "G0000BAA", &bad_title).is_err());

        for offset in [0usize, 8, 12] {
            let broken = edit(&data, offset, &[0; 4], &BANK_BASES, true);
            assert!(
                game_details::decode("SLUS-00594", "BA", "G0000BAA", &broken).is_err(),
                "marker {offset:#x}"
            );
        }
        for length in [0usize, 4, 8191, 8193, 16384] {
            let bytes = if length <= data.len() {
                data[..length].to_vec()
            } else {
                vec![0u8; length]
            };
            assert!(
                game_details::decode("SLUS-00594", "BA", "G0000BAA", &bytes).is_err(),
                "{length}"
            );
        }
    }

    #[test]
    fn card_view_distinguishes_story_saves_and_rejects_vr_and_a_second_block() {
        fn mcs(id: &str, payload: &[u8]) -> Vec<u8> {
            let mut bytes = vec![0u8; MCS_HEADER_SIZE + payload.len()];
            bytes[0] = 0x51;
            bytes[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes[8..10].copy_from_slice(&0xffffu16.to_le_bytes());
            let name = format!("BASLUS-00594{id}");
            assert_eq!(name.len(), 20);
            bytes[10..30].copy_from_slice(name.as_bytes());
            bytes[MCS_HEADER_SIZE..].copy_from_slice(payload);
            bytes
        }

        let easy = sample();
        let normal = edit(&sample(), 0x42, &1u16.to_le_bytes(), &BANK_BASES, true);
        let normal = edit(&normal, 0x20, b"s18a\0", &BANK_BASES, true);
        let normal = edit(&normal, 0xf0, &49u16.to_le_bytes(), &BANK_BASES, true);
        let mut card = Ps1Card::create_formatted("mgs-two-saves");
        card.set_save_bytes(0, &mcs("G0000BAA", &easy)).unwrap();
        card.set_save_bytes(0, &mcs("G0000BAB", &normal)).unwrap();
        let original = card.save_raw(false);
        let view = card.view();
        assert_eq!(view.saves.len(), 2);
        let progress: Vec<_> = view
            .saves
            .iter()
            .map(|save| {
                assert_eq!(save.region_raw, "BA");
                assert_eq!(save.prod_code, "SLUS-00594");
                assert!(
                    save.game_details_error.is_none(),
                    "{:?}",
                    save.game_details_error
                );
                let GameDetails::Summary(details) = save.game_details.as_ref().unwrap() else {
                    panic!("expected summary");
                };
                (
                    save.identifier.as_str(),
                    value(details, "Difficulty").to_string(),
                    value(details, "Filename difficulty (editor)").to_string(),
                    value(details, "Filename location").to_string(),
                    value(details, "Stage").to_string(),
                    value(details, "Save count").to_string(),
                )
            })
            .collect();
        assert_eq!(
            progress,
            vec![
                (
                    "G0000BAA",
                    "Easy".into(),
                    "Easy (editor)".into(),
                    "Dock (A)".into(),
                    "zzzz".into(),
                    "0".into()
                ),
                (
                    "G0000BAB",
                    "Normal".into(),
                    "Easy (editor)".into(),
                    "Dock (A)".into(),
                    "Spply Rte. (s18a)".into(),
                    "49".into()
                ),
            ]
        );
        assert_eq!(card.save_raw(false), original);
        let reopened = Ps1Card::open(&original, "mgs-two-saves.mcr", false).unwrap();
        assert_eq!(reopened.view().saves[1].identifier, "G0000BAB");

        let mut vr = Ps1Card::create_formatted("mgs-vr");
        vr.set_save_bytes(0, &mcs("V00000@A", &easy)).unwrap();
        let save = &vr.view().saves[0];
        assert!(save.game_details.is_none());
        assert!(save.game_details_error.is_some());

        let mut linked = mcs("G0000BAA", &easy);
        linked.extend_from_slice(&easy);
        assert_eq!(linked.len(), MCS_HEADER_SIZE + BLOCK_SIZE * 2);
        let mut card = Ps1Card::create_formatted("mgs-linked");
        card.set_save_bytes(0, &linked).unwrap();
        let save = &card.view().saves[0];
        assert!(save.game_details.is_none());
        assert!(save.game_details_error.is_some());
    }

    #[test]
    fn editor_map_filename_radar_meryl_and_items() {
        let mut menu = sample();
        menu = edit(&menu, 0xa4, &4i16.to_le_bytes(), &BANK_BASES, true);
        menu = edit(&menu, 0xba, &7u16.to_le_bytes(), &BANK_BASES, true);
        menu = edit(&menu, 0xa6, &1i16.to_le_bytes(), &BANK_BASES, true);
        menu = edit(&menu, 0xbc, &6u16.to_le_bytes(), &BANK_BASES, true);
        menu = edit(&menu, 0xa8, &1i16.to_le_bytes(), &BANK_BASES, true);
        menu = edit(&menu, 0xbe, &12u16.to_le_bytes(), &BANK_BASES, true);
        menu = edit(&menu, 0x44, &0x0800u16.to_le_bytes(), &BANK_BASES, true);
        let details = summary("G0936H]A", &menu);
        assert_eq!(value(&details, "Rations"), "4");
        assert_eq!(value(&details, "Max rations"), "7");
        assert_eq!(
            record(&details, "Rations"),
            vec![("Count", "4"), ("Max", "7")]
        );
        assert_eq!(value(&details, "Medicine"), "1");
        assert_eq!(value(&details, "Max medicine"), "6");
        assert_eq!(
            record(&details, "Cold medicine"),
            vec![("Count", "1"), ("Max", "6")]
        );
        assert_eq!(value(&details, "Diazepam"), "1");
        assert_eq!(value(&details, "Max diazepam"), "12");
        assert_eq!(
            record(&details, "Diazepam"),
            vec![("Count", "1"), ("Max", "12")]
        );
        assert_eq!(value(&details, "Radar"), "Hidden");
        assert_eq!(value(&details, "Option flags"), "0x0800");
        assert_eq!(value(&details, "Filename time"), "09:36");
        assert_eq!(value(&details, "Filename location"), "Cmnd room (])");
        assert_eq!(value(&details, "Filename code"), "H");
        assert_eq!(value(&details, "Disc (editor)"), "Disc 2 (editor)");
        assert_eq!(
            value(&details, "Clear marker (editor)"),
            "No clear bits (editor, count not shown)"
        );
        assert_eq!(
            value(&details, "Filename difficulty (editor)"),
            "Easy (editor)"
        );

        for (word, radar, raw) in [
            (0u16, "Visible", "0x0000"),
            (8, "Visible", "0x0008"),
            (0x2000, "Visible", "0x2000"),
            (0x2400, "Visible", "0x2400"),
            (0x0800, "Hidden", "0x0800"),
            (0x2c00, "Hidden", "0x2C00"),
            (0xffff, "Hidden", "0xFFFF"),
        ] {
            let data = edit(&sample(), 0x44, &word.to_le_bytes(), &BANK_BASES, true);
            let details = summary("G0000BAA", &data);
            assert_eq!(value(&details, "Radar"), radar, "{word:#x}");
            assert_eq!(value(&details, "Option flags"), raw, "{word:#x}");
        }

        let flags = edit(
            &sample(),
            0x3c,
            &0xa1b2_c3d4u32.to_le_bytes(),
            &BANK_BASES,
            true,
        );
        let details = summary("G0000BAA", &flags);
        assert_eq!(value(&details, "Generic flags"), "0xA1B2C3D4");
        assert_eq!(value(&details, "Radar"), "Visible");
        assert_eq!(value(&details, "Difficulty"), "Easy");

        for (byte, meryl) in [
            (0u8, "Bit O clear"),
            (0x40, "Bit O clear"),
            (0x41, "Bit O clear"),
            (0x08, "Dies (bit O)"),
            (0x49, "Dies (bit O)"),
        ] {
            let data = edit(&sample(), 0x15a, &[byte], &BANK_BASES, true);
            let details = summary("G0000BAA", &data);
            assert_eq!(value(&details, "Meryl (editor)"), meryl, "{byte:#x}");
            assert_eq!(value(&details, "Torture flags"), format!("0x{byte:02X}"));
        }

        let mut items = sample();
        items = edit(&items, 0x8a, &1i16.to_le_bytes(), &BANK_BASES, true);
        items = edit(&items, 0xae, &45i16.to_le_bytes(), &BANK_BASES, true);
        items = edit(&items, 0xb8, &(-1i16).to_le_bytes(), &BANK_BASES, true);
        items = edit(&items, 0x9e, &1i16.to_le_bytes(), &BANK_BASES, true);
        let details = summary("G0000BAA", &items);
        assert_eq!(
            record(&details, "Cigarettes"),
            vec![("Status", "Available (editor)"), ("Stored", "1")]
        );
        assert_eq!(
            record(&details, "Timer bomb"),
            vec![("Status", "Available (editor)"), ("Seconds", "45")]
        );
        assert_eq!(
            record(&details, "Suppressor"),
            vec![("Status", "Absent (editor)"), ("Stored", "-1")]
        );
        assert_eq!(
            record(&details, "Stealth"),
            vec![("Status", "Available"), ("Stored", "1")]
        );
        assert_eq!(value(&details, "Stealth"), "Available");
        assert_eq!(
            record(&details, "Bandana"),
            vec![("Status", "Unknown (0)"), ("Stored", "0")]
        );

        let names = [
            (
                "G000pCAA",
                "00:00",
                "Normal (editor)",
                "Dock (A)",
                "C",
                "Disc 1 (editor)",
                "Cleared (editor, count not shown)",
            ),
            (
                "G0000BAA",
                "00:00",
                "Easy (editor)",
                "Dock (A)",
                "B",
                "Disc 1 (editor)",
                "Cleared (editor, count not shown)",
            ),
            (
                "G0936H]A",
                "09:36",
                "Easy (editor)",
                "Cmnd room (])",
                "H",
                "Disc 2 (editor)",
                "No clear bits (editor, count not shown)",
            ),
            (
                "G070vH^A",
                "07:06",
                "Normal (editor)",
                "Spply Rte. (^)",
                "H",
                "Disc 2 (editor)",
                "No clear bits (editor, count not shown)",
            ),
            (
                "G0635H^A",
                "06:35",
                "Easy (editor)",
                "Spply Rte. (^)",
                "H",
                "Disc 2 (editor)",
                "No clear bits (editor, count not shown)",
            ),
            (
                "G9959@?Z",
                "99:59",
                "Easy (editor)",
                "Unknown ('?')",
                "@",
                "Disc 1 (editor)",
                "New game (editor)",
            ),
            (
                "G00p0BAA",
                "00:00",
                "Hard (editor)",
                "Dock (A)",
                "B",
                "Disc 1 (editor)",
                "Cleared (editor, count not shown)",
            ),
            (
                "G00pqBAA",
                "00:01",
                "Extreme (editor)",
                "Dock (A)",
                "B",
                "Disc 1 (editor)",
                "Cleared (editor, count not shown)",
            ),
            (
                "G0060@AA",
                "Unknown",
                "Easy (editor)",
                "Dock (A)",
                "@",
                "Disc 1 (editor)",
                "New game (editor)",
            ),
            (
                "G0A00@AA",
                "Unknown",
                "Easy (editor)",
                "Dock (A)",
                "@",
                "Disc 1 (editor)",
                "New game (editor)",
            ),
            (
                "G0000B_A",
                "00:00",
                "Easy (editor)",
                "Esc Route (_, editor)",
                "B",
                "Disc 1 (editor)",
                "Cleared (editor, count not shown)",
            ),
            (
                "G0000BBA",
                "00:00",
                "Easy (editor)",
                "Heliport (B, editor)",
                "B",
                "Disc 1 (editor)",
                "Cleared (editor, count not shown)",
            ),
        ];
        for (id, time, difficulty, location, code, disc, marker) in names {
            let details = summary(id, &sample());
            assert_eq!(value(&details, "Filename time"), time, "{id}");
            assert_eq!(
                value(&details, "Filename difficulty (editor)"),
                difficulty,
                "{id}"
            );
            assert_eq!(value(&details, "Filename location"), location, "{id}");
            assert_eq!(value(&details, "Filename code"), code, "{id}");
            assert_eq!(value(&details, "Disc (editor)"), disc, "{id}");
            assert_eq!(value(&details, "Clear marker (editor)"), marker, "{id}");
            assert_eq!(value(&details, "Difficulty"), "Easy", "{id}");
        }

        for (ticks, hm) in [
            (0u32, "00:00"),
            (2_076_953, "09:36"),
            (1_535_399, "07:06"),
            (1_423_525, "06:35"),
        ] {
            let data = edit(&sample(), 0x10, &ticks.to_le_bytes(), &BANK_BASES, true);
            let shown = value(&summary("G0000BAA", &data), "Playtime");
            assert!(
                shown.starts_with(&format!("{hm}:")),
                "{shown} did not keep minute floor {hm}"
            );
        }
    }

    #[derive(serde::Deserialize)]
    struct JobFixture {
        sha256: String,
        metadata: JobMeta,
        expected_fields: JobExpected,
    }

    #[derive(serde::Deserialize)]
    struct JobMeta {
        identifier: String,
    }

    #[derive(serde::Deserialize)]
    struct JobExpected {
        max_rations: u16,
        radar_hidden: bool,
        max_cold_meds: u16,
        max_diazepam: u16,
    }

    fn promote_fixture(raw: &[u8]) -> Vec<u8> {
        let mut data = raw.to_vec();
        for base in BANK_BASES {
            data[base..base + 4].copy_from_slice(&(BANK_SIZE as u32).to_le_bytes());
            data[base + 8..base + 12].copy_from_slice(&95u32.to_le_bytes());
            data[base + 12..base + 16].copy_from_slice(&2048u32.to_le_bytes());
            let sum = crc32(&data[base + 8..base + BANK_SIZE]);
            data[base + 4..base + 8].copy_from_slice(&sum.to_le_bytes());
        }
        data
    }

    fn assert_job_fixture(raw: &[u8], sidecar: &str) {
        let fixture: JobFixture = serde_json::from_str(sidecar).unwrap();
        assert_eq!(sha256(raw), fixture.sha256);
        let id = fixture
            .metadata
            .identifier
            .strip_prefix("BASLUS-00594")
            .unwrap();
        assert_eq!(id.len(), 8);
        let data = promote_fixture(raw);
        assert_ne!(sha256(raw), sha256(&data));
        let details = summary(id, &data);
        assert_eq!(details.checksum_ok, Some(true));
        assert_eq!(
            value(&details, "Max rations"),
            fixture.expected_fields.max_rations.to_string()
        );
        assert_eq!(
            value(&details, "Max medicine"),
            fixture.expected_fields.max_cold_meds.to_string()
        );
        assert_eq!(
            value(&details, "Max diazepam"),
            fixture.expected_fields.max_diazepam.to_string()
        );
        assert_eq!(
            value(&details, "Radar"),
            if fixture.expected_fields.radar_hidden {
                "Hidden"
            } else {
                "Visible"
            }
        );
        assert_eq!(
            u16::from_le_bytes(data[0x1ba..0x1bc].try_into().unwrap()),
            fixture.expected_fields.max_rations
        );
    }

    #[test]
    fn job377_synthetic_fixtures_keep_their_hashes() {
        assert_job_fixture(
            include_bytes!("testdata/mgs-synthetic-early.sc"),
            include_str!("testdata/mgs-synthetic-early.json"),
        );
        assert_job_fixture(
            include_bytes!("testdata/mgs-synthetic-late.sc"),
            include_str!("testdata/mgs-synthetic-late.json"),
        );
        assert_job_fixture(
            include_bytes!("testdata/mgs-synthetic-boundary.sc"),
            include_str!("testdata/mgs-synthetic-boundary.json"),
        );
    }
}
