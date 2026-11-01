//! The Legend of Dragoon USA (`SCUS-94491`) read-only story summary.
//!
//! Directory saves are `BA` / `SCUS-94491` / `drgn` plus four digits, one
//! 8192-byte SC block. Offsets below are SC-relative. LODEdit's published
//! offsets include the 128-byte MCS header (`SC = LODEdit - 0x80`). Its
//! `currentSaveSlot * 0x600` stride is unused in the pinned UI and is not
//! applied. Dragoon-Editor offsets are already relative to a raw card data
//! block. See `docs/formats/legend-of-dragoon-us.md`.

use super::{Field, Format, Profile, Record, SaveDetails};
use encoding_rs::SHIFT_JIS;
use unicode_normalization::UnicodeNormalization;

/// Equipment and consumable IDs. Shared equipment IDs use Dragoon-Editor's
/// display spelling. Consumable IDs are LODEdit's names with underscores
/// written as spaces. Gaps in either list stay unknown.
const ITEMS: &[(u8, &str)] = &[
    (0, "Broad Sword"),
    (1, "Bastard Sword"),
    (2, "Heat Blade"),
    (3, "Falchion"),
    (4, "Mind Crush"),
    (5, "Fairy Sword"),
    (6, "Claymore"),
    (7, "Soul Eater"),
    (8, "Axe"),
    (9, "Tomahawk"),
    (10, "Battle Axe"),
    (11, "Great Axe"),
    (12, "Indora's Axe"),
    (13, "Rapier"),
    (14, "Shadow Cutter"),
    (15, "Dancing Dagger"),
    (16, "Flamberge"),
    (17, "Gladius"),
    (18, "Dragon Buster"),
    (19, "Demon Stiletto"),
    (20, "Spear"),
    (21, "Lance"),
    (22, "Glaive"),
    (23, "Spear Of Terror"),
    (24, "Partisan"),
    (25, "Halberd"),
    (26, "Twister Glaive"),
    (27, "Short Bow"),
    (28, "Sparkle Arrow"),
    (29, "Long Bow"),
    (30, "Bemusing Arrow"),
    (31, "Virulent Arrow"),
    (32, "Detonate Arrow"),
    (33, "Arrow Of Force"),
    (34, "Mace"),
    (35, "Morning Star"),
    (36, "War Hammer"),
    (37, "Heavy Mace"),
    (38, "Basher"),
    (39, "Pretty Hammer"),
    (40, "Iron Knuckle"),
    (41, "Beast Fang"),
    (42, "Diamond Claw"),
    (43, "Thunder Fist"),
    (44, "Destroyer Mace"),
    (45, "Brass Knuckle"),
    (46, "Leather Armor"),
    (47, "Scale Armor"),
    (48, "Chain Mail"),
    (49, "Plate Mail"),
    (50, "Saint Armor"),
    (51, "Red DG Armor"),
    (52, "Jade DG Armor"),
    (53, "Lion Fur"),
    (54, "Breast Plate"),
    (55, "Giganto Armor"),
    (56, "Gold DG Armor"),
    (57, "Disciple Vest"),
    (58, "Warrior Dress"),
    (59, "Master's Vest"),
    (60, "Energy Girdle"),
    (61, "Violet DG Armor"),
    (62, "Clothes"),
    (63, "Leather Jacket"),
    (64, "Silver Vest"),
    (65, "Sparkle Dress"),
    (66, "Robe"),
    (67, "Silver DG Armor"),
    (68, "Dark DG Armor"),
    (69, "Blue DG Armor"),
    (70, "Armor Of Yore"),
    (71, "Satori Vest"),
    (72, "Rainbow Dress"),
    (73, "Angel Robe"),
    (74, "Armor Of Legend"),
    (76, "Bandana"),
    (77, "Sallet"),
    (78, "Armet"),
    (79, "Knight Helm"),
    (80, "Giganto Helm"),
    (81, "Soul Headband"),
    (82, "Felt Hat"),
    (83, "Cape"),
    (84, "Tiara"),
    (85, "Jeweled Crown"),
    (86, "Rose's Hair Band"),
    (88, "Phoenix Plume"),
    (89, "Legend Casque"),
    (90, "Dragon Helm"),
    (91, "Magical Hat"),
    (93, "Leather Boots"),
    (94, "Iron Kneepiece"),
    (95, "Combat Shoes"),
    (96, "Leather Shoes"),
    (97, "Soft Boots"),
    (98, "Stardust Boots"),
    (99, "Magical Greaves"),
    (100, "Dancer's Shoes"),
    (101, "Bandit's Shoes"),
    (103, "Poison Guard"),
    (104, "Active Ring"),
    (105, "Protector"),
    (106, "Panic Guard"),
    (107, "Stun Guard"),
    (108, "Bravery Amulet"),
    (109, "Magic Ego Bell"),
    (110, "Destone Amulet"),
    (111, "Power Wrist"),
    (112, "Knight Shield"),
    (113, "Magical Ring"),
    (114, "Spiritual Ring"),
    (115, "Attack Badge"),
    (116, "Guard Badge"),
    (117, "Giganto Ring"),
    (118, "Elude Cloak"),
    (119, "Spirit Cloak"),
    (120, "Sage's Cloak"),
    (121, "Physical Ring"),
    (122, "Amulet"),
    (123, "Wargod's Sash"),
    (124, "Spirit Ring"),
    (125, "Therapy Ring"),
    (126, "Mage Ring"),
    (127, "Wargod's Amulet"),
    (128, "Talisman"),
    (130, "Holy Ankh"),
    (131, "Dancer's Ring"),
    (133, "Bandit's Ring"),
    (134, "Red-Eye Stone"),
    (135, "Jade Stone"),
    (136, "Silver Stone"),
    (137, "Darkness Stone"),
    (138, "Blue Sea Stone"),
    (139, "Violet Stone"),
    (140, "Golden Stone"),
    (142, "Ruby Ring"),
    (143, "Sapphire Pin"),
    (144, "Rainbow Earring"),
    (146, "Emerald Earring"),
    (148, "Platinum Collar"),
    (149, "Phantom Shield"),
    (150, "Dragon Shield"),
    (151, "Angel Scarf"),
    (152, "Bracelet"),
    (153, "Fake Power Wrist"),
    (154, "Fake Shield"),
    (156, "Wargod Calling"),
    (157, "Ultimate Wargod"),
    (193, "Detonate Rock"),
    (194, "Spark Net"),
    (195, "Burn Out"),
    (197, "Pellet"),
    (198, "Spear Frost"),
    (199, "Spinning Gale"),
    (200, "Attack Ball"),
    (201, "Trans Light"),
    (202, "Dark Mist"),
    (203, "Healing Potion"),
    (204, "Depetrifier"),
    (205, "Mind Purifier"),
    (206, "Body Purifier"),
    (207, "Thunderbolt"),
    (208, "Meteor Fall"),
    (209, "Gushing Magma"),
    (210, "Dancing Ray"),
    (211, "Spirit Potion"),
    (212, "Panic Bell"),
    (214, "Fatal Blizzard"),
    (215, "Stunning Hammer"),
    (216, "Black Rain"),
    (217, "Poison Needle"),
    (218, "Midnight Terror"),
    (220, "Rave Twister"),
    (221, "Total Vanishing"),
    (222, "Angels Prayer"),
    (223, "Charm Potion"),
    (224, "Pandemonium"),
    (225, "Recovery Ball"),
    (227, "Magic Shield"),
    (228, "Material Shield"),
    (229, "Sun Rhapsody"),
    (230, "Smoke Ball"),
    (231, "Healing Fog"),
    (232, "Magic Sig Stone"),
    (233, "Healing Rain"),
    (234, "Moon Serenade"),
    (235, "Power Up"),
    (236, "Power Down"),
    (237, "Speed Up"),
    (238, "Speed Down"),
    (240, "Sachet"),
    (241, "Psyche Bomb"),
    (242, "Burning Wave"),
    (243, "Frozen Jet"),
    (244, "Down Burst"),
    (245, "Gravity Grabber"),
    (246, "Spectral Flash"),
    (247, "Night Raid"),
    (248, "Flash Hall"),
    (249, "Healing Breeze"),
    (250, "Psyche Bomb X"),
];

/// LODEdit goods bits that have a name. Temporary bits are omitted on purpose.
const GOODS: &[(usize, u8, &str)] = &[
    (0, 0x01, "War Bulletin"),
    (0, 0x02, "Fathers Stone"),
    (0, 0x04, "Prison Key"),
    (0, 0x08, "Axe From The Shack"),
    (0, 0x10, "Good Spirits"),
    (0, 0x20, "Shiny Bag"),
    (0, 0x40, "Water Bottle"),
    (0, 0x80, "Life Water"),
    (1, 0x01, "Magic Oil"),
    (1, 0x02, "Yellow Stone"),
    (1, 0x04, "Blue Stone"),
    (1, 0x08, "Red Stone"),
    (1, 0x10, "Letter From Lynn"),
    (1, 0x20, "Pass For Valley"),
    (1, 0x40, "Kates Bouquet"),
    (1, 0x80, "Key To Ship"),
    (2, 0x01, "Boat License"),
    (2, 0x02, "Dragon Blocker"),
    (2, 0x04, "Moon Gem"),
    (2, 0x08, "Moon Dagger"),
    (2, 0x10, "Moon Mirror"),
    (2, 0x20, "Omega Bomb"),
    (2, 0x40, "Omega Master"),
    (2, 0x80, "Law Maker"),
    (3, 0x01, "Law Output"),
    (3, 0x02, "Gold Dragon DS"),
    (3, 0x04, "Magic Shiny Bag"),
    (3, 0x08, "Vanishing Stone"),
    (3, 0x10, "Lavitzs Picture"),
];

const SPIRITS: [&str; 8] = [
    "Red Eyed", "Blue Sea", "Jade", "Golden", "Violet", "Silver", "Darkness", "Divine",
];

/// Character record order is the shared ID order, not Dragoon-Editor's tab order.
const CHARACTERS: [(&str, usize); 9] = [
    ("Dart", 0x52c),
    ("Lavitz", 0x558),
    ("Shana", 0x584),
    ("Rose", 0x5b0),
    ("Haschel", 0x5dc),
    ("Albert", 0x608),
    ("Meru", 0x634),
    ("Kongol", 0x660),
    ("Miranda", 0x68c),
];

const EQUIPMENT_SLOTS: usize = 255;
const ITEM_SLOTS: usize = 32;
const EQUIPMENT_BASE: usize = 0x3e8;
const ITEM_BASE: usize = 0x4e9;

const NOTE_LAYOUT: &str = concat!(
    "Editor-backed USA story summary for BA / SCUS-94491 / drgn and four digits, one block. ",
    "Offsets are SC-relative. LODEdit offsets include the 128-byte MCS header and are 0x80 higher; ",
    "its unused slot stride is not applied. Dragoon-Editor offsets match this SC block. ",
    "Disc serials SCUS-94584, SCUS-94585 and SCUS-94586 are not accepted. No emulator save/reload validation."
);

const NOTE_AGREED: &str = concat!(
    "Party IDs, the gold offset, and each character's experience, HP, MP, SP, level, Dragoon level ",
    "and five equipment bytes agree in both editors. Gold is the 32-bit little-endian word ",
    "Dragoon-Editor reads at that offset. LODEdit reads and writes only its low 16 bits. Stored ",
    "values are shown in full, including amounts above 65535 and above 99999999."
);

const NOTE_LODEDIT: &str = concat!(
    "Playtime, Dragoon spirits, named key items, the equipment bag and the consumable list are ",
    "LODEdit fields. Playtime is a 60 Hz frame count from the game copy, divided by 60. ",
    "Dragoon-Editor does not define playtime. Temporary goods bits are not named. Item 255 is None ",
    "and 254 is Invalid. Other unnamed IDs stay numeric. Shared equipment names use Dragoon-Editor's ",
    "spelling, including Satori Vest where LODEdit's enum says Sator Vest. Consumable names are LODEdit's."
);

const NOTE_WITHHELD: &str = concat!(
    "The character word at +14 is not shown: LODEdit calls it Dragoon experience and Dragoon-Editor ",
    "calls it max SP. Additions, stardust, warps, treasures and story progress are not decoded. ",
    "Neither editor documents a checksum, so checksumOk stays unverified. That is not a passing check. ",
    "Displayed party, gold, playtime and Dragoon spirits are the game copy. Character stats are the ",
    "character record. Dart's max HP is only the LODEdit save-file preview."
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

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn playtime(frames: u32) -> String {
    let seconds = frames / 60;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

fn character_name(id: u32) -> String {
    match id {
        0 => "Dart",
        1 => "Lavitz",
        2 => "Shana",
        3 => "Rose",
        4 => "Haschel",
        5 => "Albert",
        6 => "Meru",
        7 => "Kongol",
        8 => "Miranda",
        0xffff_ffff => "Empty",
        other => return format!("Unknown ({other})"),
    }
    .into()
}

fn item_name(id: u8) -> String {
    match id {
        255 => "None".into(),
        254 => "Invalid".into(),
        _ => ITEMS
            .binary_search_by_key(&id, |row| row.0)
            .ok()
            .map(|index| ITEMS[index].1.to_string())
            .unwrap_or_else(|| format!("Unknown ({id})")),
    }
}

fn slot_list(data: &[u8], start: usize, count: usize, cap: usize) -> String {
    if count == 0 {
        return "None".into();
    }
    (0..count.min(cap))
        .map(|index| item_name(data[start + index]))
        .collect::<Vec<_>>()
        .join(", ")
}

fn spirits(byte: u8) -> String {
    let names = SPIRITS
        .iter()
        .enumerate()
        .filter(|(index, _)| byte & (1 << index) != 0)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>();
    let list = if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    };
    format!("{list} (0x{byte:02X})")
}

fn key_items(goods: &[u8]) -> String {
    let names = GOODS
        .iter()
        .filter(|(index, mask, _)| goods[*index] & mask != 0)
        .map(|(_, _, name)| *name)
        .collect::<Vec<_>>();
    if names.is_empty() {
        "None".into()
    } else {
        names.join(", ")
    }
}

fn goods_bytes(goods: &[u8]) -> String {
    goods
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn saved_title(data: &[u8]) -> Result<String, String> {
    let raw = &data[4..68];
    let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
    let (title, _, malformed) = SHIFT_JIS.decode(&raw[..end]);
    if malformed {
        return Err("Unsupported Legend of Dragoon USA title.".into());
    }
    let title = title.nfkc().collect::<String>().trim().to_string();
    Ok(if title.is_empty() {
        "Saved game".into()
    } else {
        title
    })
}

fn character_record(data: &[u8], name: &str, base: usize) -> Record {
    let mut fields = vec![
        field("Level", data[base + 18].to_string()),
        field("Dragoon level", data[base + 19].to_string()),
        field("HP", u16_at(data, base + 8).to_string()),
        field("MP", u16_at(data, base + 10).to_string()),
        field("SP", u16_at(data, base + 12).to_string()),
        field("Experience", u32_at(data, base).to_string()),
        field("Weapon", item_name(data[base + 20])),
        field("Helmet", item_name(data[base + 21])),
        field("Armor", item_name(data[base + 22])),
        field("Boots", item_name(data[base + 23])),
        field("Accessory", item_name(data[base + 24])),
    ];
    if name == "Dart" {
        fields.insert(
            3,
            field("Max HP (preview)", u16_at(data, 0x194 + 4).to_string()),
        );
    }
    Record {
        name: name.into(),
        fields,
    }
}

fn copy_note(data: &[u8]) -> Option<String> {
    let mut differ = Vec::new();
    if u32_at(data, 0x294) != u32_at(data, 0x19c) {
        differ.push("gold");
    }
    if u32_at(data, 0x2a0) != u32_at(data, 0x1a0) {
        differ.push("playtime");
    }
    if u32_at(data, 0x288) != u32_at(data, 0x188) {
        differ.push("party 1");
    }
    if u32_at(data, 0x28c) != u32_at(data, 0x18c) {
        differ.push("party 2");
    }
    if u32_at(data, 0x290) != u32_at(data, 0x190) {
        differ.push("party 3");
    }
    if data[0x39c] != data[0x1a4] {
        differ.push("Dragoon spirits");
    }
    let dart = CHARACTERS[0].1;
    let preview_differs = data[0x194] != data[dart + 18]
        || data[0x194 + 1] != data[dart + 19]
        || u16_at(data, 0x194 + 2) != u16_at(data, dart + 8);
    if differ.is_empty() && !preview_differs {
        return None;
    }
    let mut text = String::new();
    if !differ.is_empty() {
        text.push_str(&format!(
            "The save-file preview differs from the game copy for {}. Displayed values are the game copy.",
            differ.join(", ")
        ));
    }
    if preview_differs {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(
            "Dart's save-file preview level, Dragoon level, or HP does not match the character record. Displayed character stats are the character record.",
        );
    }
    Some(text)
}

pub(super) fn decode(
    fmt: &Format,
    code: &str,
    identifier: &str,
    data: &[u8],
) -> Result<SaveDetails, String> {
    let name = saved_title(data)?;
    let number = identifier[4..]
        .parse::<u8>()
        .map_err(|_| "Unsupported Legend of Dragoon USA save identifier.".to_string())?;
    let frames = u32_at(data, 0x2a0);
    let gold = u32_at(data, 0x294);
    let spirits_byte = data[0x39c];
    let goods = &data[0x39d..0x3a4];
    let equipment_count = usize::from(data[0x3e4]);
    let item_count = usize::from(data[0x3e6]);
    let mut notes = vec![
        NOTE_LAYOUT.into(),
        NOTE_AGREED.into(),
        NOTE_LODEDIT.into(),
        NOTE_WITHHELD.into(),
    ];
    if item_count > ITEM_SLOTS {
        notes.push(format!(
            "Item count {item_count} is above the 32-slot list. The first 32 slots are shown."
        ));
    }
    if let Some(note) = copy_note(data) {
        notes.push(note);
    }
    let fields = vec![
        field("Playtime (hh:mm:ss)", playtime(frames)),
        field("Gold", gold.to_string()),
        field("Party 1", character_name(u32_at(data, 0x288))),
        field("Party 2", character_name(u32_at(data, 0x28c))),
        field("Party 3", character_name(u32_at(data, 0x290))),
        field("Dragoon spirits", spirits(spirits_byte)),
        field("Key items", key_items(goods)),
        field("Goods bytes", goods_bytes(goods)),
        field("Equipment count", equipment_count.to_string()),
        field(
            "Equipment",
            slot_list(data, EQUIPMENT_BASE, equipment_count, EQUIPMENT_SLOTS),
        ),
        field("Item count", item_count.to_string()),
        field("Items", slot_list(data, ITEM_BASE, item_count, ITEM_SLOTS)),
    ];
    let records = CHARACTERS
        .iter()
        .map(|(name, base)| character_record(data, name, *base))
        .collect();
    Ok(SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!("NTSC-U · {code}"),
        checksum_ok: None,
        profiles: vec![Profile {
            number,
            name,
            empty: false,
            status: None,
            checksum_ok: None,
            fields,
            records,
        }],
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{decode, format, GameDetails};
    use super::{CHARACTERS, ITEMS};
    use crate::card::engine::{Ps1Card, BLOCK_SIZE, MCS_HEADER_SIZE};

    fn put_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn story() -> Vec<u8> {
        let mut data = vec![0; 8192];
        data[..4].copy_from_slice(b"SC\x11\x01");
        data[4..9].copy_from_slice(b"Seles");
        put_u32(&mut data, 0x288, 0);
        put_u32(&mut data, 0x188, 0);
        put_u32(&mut data, 0x28c, 3);
        put_u32(&mut data, 0x18c, 3);
        put_u32(&mut data, 0x290, 0xffff_ffff);
        put_u32(&mut data, 0x190, 0xffff_ffff);
        put_u32(&mut data, 0x294, 123_456);
        put_u32(&mut data, 0x19c, 123_456);
        put_u32(&mut data, 0x2a0, 216_060);
        put_u32(&mut data, 0x1a0, 216_060);
        data[0x39c] = 0x05;
        data[0x1a4] = 0x05;
        data[0x39d] = 0x81;
        data[0x3a0] = 0x20;
        data[0x3e4] = 3;
        data[0x3e8] = 7;
        data[0x3e9] = 255;
        data[0x3ea] = 71;
        data[0x3e6] = 2;
        data[0x4e9] = 203;
        data[0x4ea] = 222;
        let dart = CHARACTERS[0].1;
        put_u32(&mut data, dart, 1000);
        put_u16(&mut data, dart + 8, 120);
        put_u16(&mut data, dart + 10, 30);
        put_u16(&mut data, dart + 12, 50);
        data[dart + 14] = 0xab;
        data[dart + 15] = 0xcd;
        data[dart + 18] = 10;
        data[dart + 19] = 2;
        data[dart + 20] = 12;
        data[dart + 21] = 76;
        data[dart + 22] = 46;
        data[dart + 23] = 93;
        data[dart + 24] = 255;
        data[0x194] = 10;
        data[0x195] = 2;
        put_u16(&mut data, 0x196, 120);
        put_u16(&mut data, 0x198, 200);
        data[CHARACTERS[1].1 + 18] = 11;
        put_u16(&mut data, CHARACTERS[1].1 + 8, 20);
        data[CHARACTERS[2].1 + 18] = 12;
        data[dart + 0x22] = 77;
        data
    }

    fn summary(id: &str, data: &[u8]) -> super::super::SaveDetails {
        match decode("SCUS-94491", "BA", id, data).unwrap() {
            GameDetails::Summary(details) => details,
            other => panic!("expected summary, got {other:?}"),
        }
    }

    fn value<'a>(details: &'a super::super::SaveDetails, label: &str) -> &'a str {
        details.profiles[0]
            .fields
            .iter()
            .find(|field| field.label == label)
            .unwrap_or_else(|| panic!("missing {label}"))
            .value
            .as_str()
    }

    fn record<'a>(details: &'a super::super::SaveDetails, name: &str) -> &'a super::super::Record {
        details.profiles[0]
            .records
            .iter()
            .find(|record| record.name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }

    fn record_value<'a>(record: &'a super::super::Record, label: &str) -> &'a str {
        record
            .fields
            .iter()
            .find(|field| field.label == label)
            .unwrap_or_else(|| panic!("missing {label}"))
            .value
            .as_str()
    }

    #[test]
    fn reads_editor_fields_without_claiming_a_checksum() {
        let data = story();
        let before = data.clone();
        let details = summary("drgn0001", &data);
        assert_eq!(details.game, "legend-of-dragoon");
        assert_eq!(details.title, "The Legend of Dragoon");
        assert_eq!(details.release, "NTSC-U · SCUS-94491");
        assert_eq!(details.checksum_ok, None);
        assert_eq!(details.profiles.len(), 1);
        let profile = &details.profiles[0];
        assert_eq!(profile.number, 1);
        assert_eq!(profile.name, "Seles");
        assert!(!profile.empty);
        assert_eq!(profile.checksum_ok, None);
        assert_eq!(value(&details, "Playtime (hh:mm:ss)"), "01:00:01");
        assert_eq!(value(&details, "Gold"), "123456");
        assert_eq!(value(&details, "Party 1"), "Dart");
        assert_eq!(value(&details, "Party 2"), "Rose");
        assert_eq!(value(&details, "Party 3"), "Empty");
        assert_eq!(value(&details, "Dragoon spirits"), "Red Eyed, Jade (0x05)");
        assert_eq!(value(&details, "Key items"), "War Bulletin, Life Water");
        assert_eq!(value(&details, "Goods bytes"), "81 00 00 20 00 00 00");
        assert_eq!(value(&details, "Equipment count"), "3");
        assert_eq!(
            value(&details, "Equipment"),
            "Soul Eater, None, Satori Vest"
        );
        assert_eq!(value(&details, "Item count"), "2");
        assert_eq!(value(&details, "Items"), "Healing Potion, Angels Prayer");
        assert_eq!(profile.records.len(), 9);
        let dart = record(&details, "Dart");
        assert_eq!(record_value(dart, "Level"), "10");
        assert_eq!(record_value(dart, "Dragoon level"), "2");
        assert_eq!(record_value(dart, "HP"), "120");
        assert_eq!(record_value(dart, "Max HP (preview)"), "200");
        assert_eq!(record_value(dart, "MP"), "30");
        assert_eq!(record_value(dart, "SP"), "50");
        assert_eq!(record_value(dart, "Experience"), "1000");
        assert_eq!(record_value(dart, "Weapon"), "Indora's Axe");
        assert_eq!(record_value(dart, "Helmet"), "Bandana");
        assert_eq!(record_value(dart, "Armor"), "Leather Armor");
        assert_eq!(record_value(dart, "Boots"), "Leather Boots");
        assert_eq!(record_value(dart, "Accessory"), "None");
        assert!(dart.fields.iter().all(|field| field.label != "Max SP"));
        assert_eq!(record_value(record(&details, "Lavitz"), "Level"), "11");
        assert_eq!(record_value(record(&details, "Lavitz"), "HP"), "20");
        assert_eq!(record_value(record(&details, "Shana"), "Level"), "12");
        assert!(record(&details, "Lavitz")
            .fields
            .iter()
            .all(|field| field.label != "Max HP (preview)"));
        assert!(details.notes.iter().any(|note| note.contains("unverified")));
        assert!(!details
            .notes
            .iter()
            .any(|note| note.contains("save-file preview differs")));
        assert_eq!(data, before);
    }

    #[test]
    fn gold_is_32_bit_and_playtime_uses_integer_60_hz_frames() {
        let mut data = story();
        put_u32(&mut data, 0x294, 0x0001_0000);
        put_u32(&mut data, 0x19c, 0x0001_0000);
        put_u32(&mut data, 0x2a0, 59);
        put_u32(&mut data, 0x1a0, 59);
        let details = summary("drgn0000", &data);
        assert_eq!(details.profiles[0].number, 0);
        assert_eq!(value(&details, "Gold"), "65536");
        assert_eq!(value(&details, "Playtime (hh:mm:ss)"), "00:00:00");
        put_u32(&mut data, 0x294, 0x05f5_e0ff);
        put_u32(&mut data, 0x19c, 0x05f5_e0ff);
        put_u32(&mut data, 0x2a0, 60);
        put_u32(&mut data, 0x1a0, 60);
        let details = summary("drgn0255", &data);
        assert_eq!(details.profiles[0].number, 255);
        assert_eq!(value(&details, "Gold"), "99999999");
        assert_eq!(value(&details, "Playtime (hh:mm:ss)"), "00:00:01");
        put_u32(&mut data, 0x2a0, 100 * 3600 * 60);
        put_u32(&mut data, 0x1a0, 100 * 3600 * 60);
        assert_eq!(
            value(&summary("drgn0001", &data), "Playtime (hh:mm:ss)"),
            "100:00:00"
        );
    }

    #[test]
    fn unknown_ids_level_zero_and_disagreed_bytes_stay_explicit() {
        let mut data = story();
        put_u32(&mut data, 0x288, 9);
        put_u32(&mut data, 0x188, 9);
        data[0x3e8] = 75;
        data[0x3e9] = 254;
        data[0x4e9] = 196;
        data[CHARACTERS[0].1 + 18] = 0;
        data[0x194] = 0;
        data[CHARACTERS[0].1 + 20] = 23;
        let details = summary("drgn0002", &data);
        assert_eq!(value(&details, "Party 1"), "Unknown (9)");
        assert_eq!(
            value(&details, "Equipment"),
            "Unknown (75), Invalid, Satori Vest"
        );
        assert_eq!(value(&details, "Items"), "Unknown (196), Angels Prayer");
        assert_eq!(record_value(record(&details, "Dart"), "Level"), "0");
        assert_eq!(
            record_value(record(&details, "Dart"), "Weapon"),
            "Spear Of Terror"
        );
        let mut zero = vec![0; 8192];
        zero[..4].copy_from_slice(b"SC\x11\x01");
        let zeroed = summary("drgn0000", &zero);
        assert_eq!(zeroed.checksum_ok, None);
        assert_eq!(value(&zeroed, "Party 1"), "Dart");
        assert_eq!(value(&zeroed, "Gold"), "0");
        assert_eq!(value(&zeroed, "Equipment"), "None");
        assert_eq!(value(&zeroed, "Items"), "None");
        assert_eq!(value(&zeroed, "Dragoon spirits"), "None (0x00)");
        assert_eq!(record_value(record(&zeroed, "Dart"), "Level"), "0");
        assert_eq!(
            record_value(record(&zeroed, "Dart"), "Weapon"),
            "Broad Sword"
        );
        let before = summary("drgn0002", &data);
        for offset in [
            CHARACTERS[0].1 + 4,
            CHARACTERS[0].1 + 14,
            CHARACTERS[0].1 + 16,
            CHARACTERS[0].1 + 0x22,
            0x4e7,
            0x4e8,
            0x29c,
        ] {
            let mut changed = data.clone();
            changed[offset] ^= 0xff;
            let details = summary("drgn0002", &changed);
            let shown = |details: &super::super::SaveDetails| {
                (
                    details.profiles[0]
                        .fields
                        .iter()
                        .map(|field| (field.label.clone(), field.value.clone()))
                        .collect::<Vec<_>>(),
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
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            };
            assert_eq!(
                shown(&details),
                shown(&before),
                "offset {offset:#x} changed a reported field"
            );
        }
    }

    #[test]
    fn preview_disagreements_and_long_item_counts_are_notes() {
        let mut data = story();
        put_u32(&mut data, 0x19c, 1);
        data[0x1a4] = 0;
        data[0x194 + 2] = 1;
        data[0x3e6] = 40;
        let details = summary("drgn0003", &data);
        assert_eq!(value(&details, "Gold"), "123456");
        assert_eq!(value(&details, "Dragoon spirits"), "Red Eyed, Jade (0x05)");
        assert_eq!(record_value(record(&details, "Dart"), "HP"), "120");
        assert_eq!(value(&details, "Item count"), "40");
        let mismatch = details
            .notes
            .iter()
            .find(|note| note.contains("save-file preview differs"))
            .expect("missing preview note");
        assert!(mismatch.contains("gold"));
        assert!(mismatch.contains("Dragoon spirits"));
        assert!(mismatch.contains("Dart's save-file preview"));
        assert!(!mismatch.contains("playtime"));
        assert!(!mismatch.contains("party"));
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("first 32 slots")));
        assert_eq!(value(&details, "Items").split(", ").count(), 32);
    }

    #[test]
    fn item_table_is_sorted_and_leaves_editor_gaps_unnamed() {
        assert!(ITEMS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for id in (0..=74)
            .chain(76..=86)
            .chain(88..=91)
            .chain(93..=101)
            .chain(103..=128)
            .chain(130..=131)
            .chain(133..=140)
            .chain(142..=144)
            .chain([146])
            .chain(148..=154)
            .chain(156..=157)
        {
            let name = super::item_name(id);
            assert!(!name.starts_with("Unknown"), "{id} -> {name}");
        }
        for id in [
            75, 87, 92, 102, 129, 132, 141, 145, 147, 155, 158, 192, 196, 213, 219, 226, 239, 251,
        ] {
            assert_eq!(super::item_name(id), format!("Unknown ({id})"));
        }
        assert_eq!(super::item_name(255), "None");
        assert_eq!(super::item_name(254), "Invalid");
        assert_eq!(super::item_name(0), "Broad Sword");
    }

    #[test]
    fn rejects_other_releases_identifiers_and_layouts() {
        let data = story();
        let supported = format("SCUS-94491").unwrap();
        assert_eq!(supported.slug, "legend-of-dragoon");
        assert_eq!(supported.region, "BA");
        assert_eq!(supported.blocks, 1);
        assert!(supported.accepts_identifier("drgn0000"));
        assert!(supported.accepts_identifier("drgn0255"));
        assert!(!supported.accepts_identifier("drgn0256"));
        assert!(!supported.accepts_identifier("drgn000"));
        assert!(!supported.accepts_identifier("drgn00000"));
        assert!(!supported.accepts_identifier("DRGN0000"));
        assert!(!supported.accepts_identifier(""));
        assert!(format("SCUS-94584").is_none());
        assert!(format("SCUS-94585").is_none());
        assert!(format("SCUS-94586").is_none());
        assert!(format("SCES-03047").is_none());
        assert!(format("SCESP03047").is_none());
        for (code, region, id) in [
            ("SCUS-94491", "BI", "drgn0001"),
            ("SCUS-94491", "BE", "drgn0001"),
            ("SCUS-94491", "BA", "SETT0001"),
            ("SCUS-94491", "BA", "drgn000a"),
        ] {
            assert!(
                decode(code, region, id, &data).is_err(),
                "{code} {region} {id}"
            );
        }
        for length in [0usize, 4, 128, 8191, 8193, 16384] {
            let bytes = if length <= data.len() {
                data[..length].to_vec()
            } else {
                vec![0u8; length]
            };
            assert!(
                decode("SCUS-94491", "BA", "drgn0001", &bytes).is_err(),
                "{length}"
            );
        }
        let mut extra = data.clone();
        extra.push(0);
        assert!(decode("SCUS-94491", "BA", "drgn0001", &extra).is_err());
        let mut bad_blocks = data.clone();
        bad_blocks[3] = 2;
        assert!(decode("SCUS-94491", "BA", "drgn0001", &bad_blocks).is_err());
        let mut not_sc = data.clone();
        not_sc[0] = b'X';
        assert!(decode("SCUS-94491", "BA", "drgn0001", &not_sc).is_err());
        for icon in [0x00u8, 0x11, 0x13] {
            let mut header = data.clone();
            header[2] = icon;
            assert!(
                decode("SCUS-94491", "BA", "drgn0001", &header).is_ok(),
                "icon {icon:#x}"
            );
        }
        let mut bad_title = data.clone();
        bad_title[4..68].fill(0);
        bad_title[4] = 0x81;
        assert!(decode("SCUS-94491", "BA", "drgn0001", &bad_title).is_err());
        let mut blank = data.clone();
        blank[4..68].fill(0);
        let details = summary("drgn0001", &blank);
        assert_eq!(details.profiles[0].name, "Saved game");
        assert_eq!(data, story());
    }

    #[test]
    fn card_view_reads_a_story_save_and_rejects_other_files() {
        fn mcs(id: &str, payload: &[u8]) -> Vec<u8> {
            let mut bytes = vec![0u8; MCS_HEADER_SIZE + payload.len()];
            bytes[0] = 0x51;
            bytes[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes[8..10].copy_from_slice(&0xffffu16.to_le_bytes());
            let name = format!("BASCUS-94491{id}");
            assert_eq!(name.len(), 20);
            bytes[10..30].copy_from_slice(name.as_bytes());
            bytes[MCS_HEADER_SIZE..].copy_from_slice(payload);
            bytes
        }

        let mut card = Ps1Card::create_formatted("lod");
        card.set_save_bytes(0, &mcs("drgn0000", &story())).unwrap();
        let original = card.save_raw(false);
        let save = &card.view().saves[0];
        assert_eq!(save.region_raw, "BA");
        assert_eq!(save.prod_code, "SCUS-94491");
        assert_eq!(save.identifier, "drgn0000");
        assert!(
            save.game_details_error.is_none(),
            "{:?}",
            save.game_details_error
        );
        let GameDetails::Summary(details) = save.game_details.as_ref().unwrap() else {
            panic!("expected summary");
        };
        assert_eq!(details.checksum_ok, None);
        assert_eq!(value(details, "Gold"), "123456");
        assert_eq!(card.save_raw(false), original);
        let reopened = Ps1Card::open(&original, "lod.mcr", false).unwrap();
        assert_eq!(reopened.view().saves[0].identifier, "drgn0000");

        let mut other = Ps1Card::create_formatted("lod-other");
        other.set_save_bytes(0, &mcs("SETT0001", &story())).unwrap();
        let save = &other.view().saves[0];
        assert!(save.game_details.is_none());
        assert!(save.game_details_error.is_some());

        let mut linked = mcs("drgn0000", &story());
        linked.extend_from_slice(&story());
        assert_eq!(linked.len(), MCS_HEADER_SIZE + BLOCK_SIZE * 2);
        let mut card = Ps1Card::create_formatted("lod-linked");
        card.set_save_bytes(0, &linked).unwrap();
        let save = &card.view().saves[0];
        assert!(save.game_details.is_none());
        assert!(save.game_details_error.is_some());
    }
}
