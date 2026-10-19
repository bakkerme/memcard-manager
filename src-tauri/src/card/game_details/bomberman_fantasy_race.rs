//! Exact-US SYSTEM profile summary from verified layout facts, not imported game
//! code. See docs/formats/bomberman-fantasy-race-us.md for evidence and limits.
use super::{Field, Format, Profile, ProfileStatus, SaveDetails};

const PROFILE_START: usize = 0xd08;
const PROFILE_STRIDE: usize = 0x44;
const PROFILE_COUNT: usize = 50;

fn check(data: &[u8], start: usize, size: usize) -> bool {
    let body = &data[start..start + size];
    let xor = body.iter().fold(0u8, |a, &b| a ^ b);
    let sum = body.iter().fold(0u8, |a, &b| a.wrapping_add(b));
    data[start + size..start + size + 2] == [xor, sum]
}

fn name(glyphs: &[u8]) -> String {
    if glyphs.iter().any(|&v| !matches!(v, 0x0a..=0x3d | 0x49)) {
        return format!(
            "Unknown (glyph IDs {})",
            glyphs
                .iter()
                .map(|v| format!("{v:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    let name: String = glyphs
        .iter()
        .map(|&v| match v {
            0x0a..=0x23 => char::from(b'A' + v - 0x0a),
            0x24..=0x3d => char::from(b'a' + v - 0x24),
            _ => ' ',
        })
        .collect();
    let name = name.trim_end();
    if name.is_empty() {
        "Unnamed profile".into()
    } else {
        name.into()
    }
}

pub(super) fn decode(fmt: &Format, code: &str, data: &[u8]) -> Result<SaveDetails, String> {
    // Dispatch enforces the exact release/region/identifier and payload size.
    if !data.starts_with(&[b'S', b'C', 0x11, 1]) {
        return Err("Unsupported Bomberman Fantasy Race US layout.".into());
    }
    let global_ok = check(data, 0x200, 0xb04);
    let record_checks: Vec<bool> = (0..PROFILE_COUNT)
        .map(|i| check(data, PROFILE_START + i * PROFILE_STRIDE, 0x40))
        .collect();
    let capacity = usize::from(data[0xd03]).min(PROFILE_COUNT);
    let mut profiles = vec![];
    for (i, &record_ok) in record_checks.iter().enumerate().take(capacity) {
        let start = PROFILE_START + i * PROFILE_STRIDE;
        let status = if !global_ok || !record_ok {
            ProfileStatus::Unknown
        } else {
            match data[start] {
                0 => ProfileStatus::Empty,
                0xff => ProfileStatus::Occupied,
                _ => ProfileStatus::Unknown,
            }
        };
        let mut fields = vec![];
        let mut identity = format!("Profile {:02}", i + 1);
        if status == ProfileStatus::Occupied {
            let player = name(&data[start + 1..start + 6]);
            // Keep the fixed slot identity when the name cannot be decoded.
            if !player.starts_with("Unknown (") {
                identity = player.clone();
            }
            fields = vec![
                Field {
                    label: "Player".into(),
                    value: player,
                },
                Field {
                    label: "Normal course tickets".into(),
                    value: format!("{} of 8", data[start + 6].count_ones()),
                },
                Field {
                    label: "Mirrored course tickets".into(),
                    value: format!("{} of 8", data[start + 7].count_ones()),
                },
            ];
        }
        profiles.push(Profile {
            number: (i + 1) as u8,
            name: identity,
            empty: status == ProfileStatus::Empty,
            status: Some(status),
            checksum_ok: Some(global_ok && record_ok),
            fields,
            records: vec![],
        });
    }
    let mut notes = vec![
        "Course tickets unlock Time Attack courses; they are not race wins or all tickets offered for purchase in 1P.".into(),
        "Course names, race-win flags, currency, animals, rankings and overall completion are not decoded.".into(),
        "Checksum agreement does not certify loadability. Deleted entries may be invisible in the game's Load menu; no repair is performed.".into(),
    ];
    if !matches!(data[0xd03], 5 | 50) {
        notes.push(format!(
            "Unusual stored profile capacity ({}); showing at most 50 slots.",
            data[0xd03]
        ));
    }
    if profiles.is_empty() {
        notes.push("No profile slots are declared by this save; this does not establish that the card has no recoverable profiles.".into());
    }
    Ok(SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!("NTSC-U · {code}"),
        checksum_ok: Some(global_ok && record_checks.iter().all(|&v| v)),
        profiles,
        notes,
    })
}

#[cfg(test)]
mod tests;
