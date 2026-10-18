//! Exact-release read-only SLUS-00439 summary. Independently implemented from
//! verified numeric layout facts; see docs/formats/pac-man-world-us.md.
use super::{Field, Format, Profile, SaveDetails};

// Exact US executable's quest table at 0x8010d078, including special entries.
const QUEST_STAGE_IDS: [usize; 23] = [
    236, 230, 232, 234, 222, 224, 228, 30, 32, 34, 36, 38, 40, 42, 44, 52, 54, 56, 58, 46, 48, 50,
    238,
];

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn i32_at(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn field(label: &str, value: String) -> Field {
    Field {
        label: label.into(),
        value,
    }
}

pub(super) fn decode(fmt: &Format, code: &str, data: &[u8]) -> Result<SaveDetails, String> {
    // Dispatch has already checked release, region, identifier, header and size.
    if u32::from_le_bytes(data[0x200..0x204].try_into().unwrap()) != 0x784
        || u16_at(data, 0x208) != 2
    {
        return Err("Unsupported Pac-Man World US body length or version.".into());
    }
    let profiles = (0..3)
        .map(|index| {
            let start = 0x20c + index * 0x280;
            let empty = i32_at(data, start + 0x0c) < 0;
            let mut fields = vec![];
            if !empty {
                let cleared = QUEST_STAGE_IDS
                    .iter()
                    .filter(|&&id| u16_at(data, start + 0x54 + 2 * id) & 3 == 3)
                    .count();
                let friends = (0..6)
                    .filter(|&id| u16_at(data, start + 0x5a + 2 * id) & 2 != 0)
                    .count();
                let ticks = i32_at(data, start + 0x24c);
                let time = if ticks < 0 {
                    "Unknown".into()
                } else {
                    let seconds = ticks as u32 / 300;
                    format!(
                        "{}:{:02}:{:02}",
                        seconds / 3600,
                        seconds / 60 % 60,
                        seconds % 60
                    )
                };
                fields = vec![
                    field(
                        "Quest completion",
                        format!("{cleared} of 23 quest stages cleared"),
                    ),
                    field("Friends rescued", format!("{friends} of 6")),
                    field("Playtime (hh:mm:ss)", time),
                ];
            }
            Profile {
                number: (index + 1) as u8,
                name: format!("Game {} of 3", index + 1),
                empty,
                checksum_ok: None,
                fields,
                records: vec![],
            }
        })
        .collect();
    Ok(SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!("NTSC-U · {code}"),
        checksum_ok: None,
        profiles,
        notes: vec![
            "Quest completion counts 23 quest-table entries, including special stages; it is not the game's percentage.".into(),
            "Saved location, lives, score and native percentage are not decoded. Game integrity is unverified.".into(),
            "Playtime uses the saved display counter; pause behavior, overflow and new-save persistence are unqualified.".into(),
        ],
    })
}

#[cfg(test)]
mod tests;
