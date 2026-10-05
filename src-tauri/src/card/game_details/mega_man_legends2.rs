//! Independently derived original-US layout; evidence and limits are recorded in
//! docs/formats/mega-man-legends-2.md. Offsets are relative to the SC payload.
use super::{Field, Format, Profile, SaveDetails};
use encoding_rs::SHIFT_JIS;
use unicode_normalization::UnicodeNormalization;

const SEGMENTS: [(usize, usize); 4] = [(0, 0x17c), (0x200, 0x27c), (0x280, 0x3fc), (0x400, 0xbfc)];

// One-based save-preview IDs from the US game's 25-entry Shift-JIS table.
// Duplicate place names represent different map/secondary-map pairs. ID 0 has
// no named entry. See the spec for table and save-writer addresses.
const LOCATIONS: [&str; 25] = [
    "Flutter",
    "Sulphur-Bottom",
    "Yosyonke Pad",
    "Yosyonke Gallery",
    "Forbidden Island",
    "Manda Pad",
    "Pokte Ruins",
    "Nino Pad",
    "Nino Platform",
    "Ruminoa City",
    "Glyde Base Gate",
    "Carlbania",
    "Kimotoma City",
    "Kimotoma Pad",
    "Kimotoma Ruins",
    "Saul Kada",
    "Manda Ruins",
    "Nino Ruins",
    "Saul Kada Ruins",
    "Calinca Ruins",
    "Elysium Pad",
    "Elysium",
    "Master's Room",
    "Forbidden Island",
    "Kimotoma City",
];

fn location_name(id: u8) -> String {
    id.checked_sub(1)
        .and_then(|index| LOCATIONS.get(index as usize))
        .map(|name| (*name).into())
        .unwrap_or_else(|| unknown_id(id))
}

fn difficulty_name(id: u8) -> String {
    // Cross-checked against explicitly labelled starting-mode saves. The game
    // accepts five internal levels; ID 2 is not a selectable starting mode and
    // remains unnamed until its license-test semantics are independently traced.
    match id {
        0 => "Easy".into(),
        1 => "Normal".into(),
        3 => "Hard".into(),
        4 => "Very Hard".into(),
        _ => unknown_id(id),
    }
}

// Numeric ID/name facts from Skatr11718's published modifier tables, cross-checked
// against the game's Equipment screen for the first private save. See the format
// spec for provenance. These are explicit IDs, not alphabetical/walkthrough order.
const HELMETS: [&str; 3] = ["None", "Normal Helmet", "Padded Helmet"];
const SHOES: [&str; 6] = [
    "None",
    "Jet Skates",
    "Hydrojets",
    "Asbestos Shoes",
    "Cleated Shoes",
    "Hover Shoes",
];
const ARMOR: [&str; 8] = [
    "None",
    "Normal Armor",
    "Padded Armor",
    "Padded Armor Omega",
    "Link Armor",
    "Link Armor Omega",
    "Kevlar Armor",
    "Kevlar Armor Omega",
];
const BUSTER_PARTS: [&str; 32] = [
    "None",
    "Power Raiser",
    "Power Raiser Alpha",
    "Power Raiser Omega",
    "Turbo Charger",
    "Turbo Charger Alpha",
    "Turbo Charger Omega",
    "Range Booster",
    "Range Booster Alpha",
    "Range Booster Omega",
    "Rapid Fire",
    "Rapid Fire Alpha",
    "Rapid Fire Omega",
    "Blaster Unit",
    "Buster Unit",
    "Power Blaster",
    "Sniper Unit",
    "Autofire Unit",
    "Blaster Unit Omega",
    "Buster Unit Omega",
    "Power Blaster Omega",
    "Sniper Unit Omega",
    "Autofire Unit Omega",
    "Upgrade Pack",
    "Booster Pack",
    "Energizer Pack",
    "Upgrade Pack Omega",
    "Booster Pack Omega",
    "Energizer Pack Omega",
    "Accessory Pack",
    "Accessory Pack Alpha",
    "Accessory Pack Omega",
];

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn field(label: &str, value: impl Into<String>) -> Field {
    Field {
        label: label.into(),
        value: value.into(),
    }
}

fn unknown_id(id: u8) -> String {
    format!("Unknown (ID 0x{id:02X})")
}

fn equipment_name(names: &[&str], id: u8) -> String {
    names
        .get(id as usize)
        .map(|name| (*name).into())
        .unwrap_or_else(|| unknown_id(id))
}

/// Dispatch has already checked the release, identifier, SC size and block count.
pub(super) fn decode(
    format: &Format,
    code: &str,
    identifier: &str,
    data: &[u8],
) -> Result<SaveDetails, String> {
    // Keep support narrow to the observed icon mode and US title marker. A
    // checksum mismatch alone still allows viewing with the shared warning.
    let title_bytes = &data[4..68];
    let title_end = title_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(title_bytes.len());
    let (title, _, malformed) = SHIFT_JIS.decode(&title_bytes[..title_end]);
    if data[2] != 0x11
        || malformed
        || !title
            .nfkc()
            .collect::<String>()
            .starts_with("MEGAMAN LEGENDS2[")
    {
        return Err("This save does not match the supported US Mega Man Legends 2 layout.".into());
    }

    let checksum_ok = SEGMENTS.iter().all(|&(start, end)| {
        let sum = data[start..end].chunks_exact(4).fold(0u32, |sum, word| {
            sum.wrapping_add(u32::from_le_bytes(word.try_into().unwrap()))
        });
        sum == u32_at(data, end)
    });
    // The game's display counter ticks at 60 Hz, regardless of render FPS.
    // The separate counter at 0x13c is not a duplicate validation marker.
    let seconds = u32_at(data, 0x138) / 60;
    let playtime = format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    );
    let number = identifier.as_bytes()[identifier.len() - 1] - b'0' + 1;
    Ok(SaveDetails {
        game: format.slug.into(),
        title: format.title.into(),
        release: format!("US · {code}"),
        checksum_ok: Some(checksum_ok),
        profiles: vec![Profile {
            number,
            name: format!("Save {number}"),
            empty: false,
            checksum_ok: Some(checksum_ok),
            fields: vec![
                field("Saved location", location_name(data[0x12f])),
                field("Playtime (hh:mm:ss)", playtime),
                field("Zenny", u32_at(data, 0x200).to_string()),
                field("Health (game units)", format!("{} / {}", u16_at(data, 0x290), u16_at(data, 0x292))),
                field("Difficulty", difficulty_name(data[0x12e])),
                field("Equipped helmet", equipment_name(&HELMETS, data[0x2b4])),
                field("Equipped shoes", equipment_name(&SHOES, data[0x2b5])),
                field("Equipped armor", equipment_name(&ARMOR, data[0x2b6])),
                field("Buster part 1", equipment_name(&BUSTER_PARTS, data[0x2b8])),
                field("Buster part 2", equipment_name(&BUSTER_PARTS, data[0x2b9])),
                field("Buster part 3", equipment_name(&BUSTER_PARTS, data[0x2ba])),
            ],
            records: vec![],
        }],
        notes: vec![
            "Location names follow the game's save preview. Difficulty reflects the saved level and may change after license tests; unknown levels retain their stored ID. Equipment names use published ID mappings.".into(),
            "Health uses the game's stored units. Inventory, key items, weapon upgrades and story progress are not decoded yet.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipment_tables_cover_documented_ranges_and_preserve_other_ids() {
        for table in [&HELMETS[..], &SHOES[..], &ARMOR[..], &BUSTER_PARTS[..]] {
            assert_eq!(equipment_name(table, 0), "None");
            for id in 1..table.len() {
                let name = equipment_name(table, id as u8);
                assert!(!name.is_empty());
                assert!(!name.starts_with("Unknown"));
            }
            for id in table.len()..=255 {
                assert_eq!(equipment_name(table, id as u8), unknown_id(id as u8));
            }
        }
        assert_eq!(equipment_name(&HELMETS, 2), "Padded Helmet");
        assert_eq!(equipment_name(&SHOES, 2), "Hydrojets");
        assert_eq!(equipment_name(&SHOES, 5), "Hover Shoes");
        assert_eq!(equipment_name(&ARMOR, 3), "Padded Armor Omega");
        assert_eq!(equipment_name(&ARMOR, 7), "Kevlar Armor Omega");
        for (id, name) in [
            (1, "Power Raiser"),
            (6, "Turbo Charger Omega"),
            (0x0d, "Blaster Unit"),
            (0x0e, "Buster Unit"),
            (0x12, "Blaster Unit Omega"),
            (0x1d, "Accessory Pack"),
            (0x1f, "Accessory Pack Omega"),
        ] {
            assert_eq!(equipment_name(&BUSTER_PARTS, id), name);
        }
    }
}
