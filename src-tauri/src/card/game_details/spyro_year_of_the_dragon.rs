//! Spyro: Year of the Dragon USA (`SCUS-94467`) read-only summary.
//!
//! Traced from TheMobyCollective/spyro-3 (CC0) commit
//! `e97cd3387ac1bb7b7c9c958f7b7bdc591a2ae273`, the matching NTSC-U rev 0
//! decomp. `include/memcard.h` is the `SaveFile` layout. The title overlay
//! writes one directory file, `BASCUS-94467SPY3_1`, copies a `SC 13 01`
//! header, and stores three 0x900-byte slots at SC offsets `0x280`, `0xB80`
//! and `0x1480`. See `docs/formats/spyro-year-of-the-dragon-us.md`.
//!
//! Not checked on a console or emulator save. Fields below are what those
//! matching routines read and write. Skill points, story flags, speedway
//! times and individual gem bits are left undecoded.

use super::{Field, Format, Profile, Record, SaveDetails};

const SLOT_BASE: usize = 0x280;
const SLOT_SIZE: usize = 0x900;
const CHECKSUM_LEN: usize = 0x8FC;
/// Super Bonus Round. The percentage routine counts it in the numerator and
/// leaves it out of the denominator.
const BONUS_LEVEL: usize = 0x24;

/// Shift-JIS fullwidth `ＳＰＹＲＯ３`, copied from the title-screen header template.
const TITLE_PREFIX: [u8; 12] = [
    0x82, 0x72, 0x82, 0x6F, 0x82, 0x78, 0x82, 0x71, 0x82, 0x6E, 0x82, 0x52,
];

/// Level-name table `D_80069CF8`, indices 0..=36. Indices 37..=39 are the
/// decomp's trailing "Empty" strings and are not level names.
const LEVELS: [&str; 37] = [
    "Sunrise Spring Home",
    "Sunny Villa",
    "Cloud Spires",
    "Molten Crater",
    "Seashell Shore",
    "Mushroom Speedway",
    "Sheila's Alp",
    "Buzz's Dungeon",
    "Crawdad Farm",
    "Midday Garden Home",
    "Icy Peak",
    "Enchanted Towers",
    "Spooky Swamp",
    "Bamboo Terrace",
    "Country Speedway",
    "Sgt. Byrd's Base",
    "Spike's Arena",
    "Spider Town",
    "Evening Lake Home",
    "Frozen Altars",
    "Lost Fleet",
    "Fireworks Factory",
    "Charmed Ridge",
    "Honey Speedway",
    "Bentley's Outpost",
    "Scorch's Pit",
    "Starfish Reef",
    "Midnight Mountain Home",
    "Crystal Islands",
    "Desert Ruins",
    "Haunted Tomb",
    "Dino Mines",
    "Harbor Speedway",
    "Agent 9's Lab",
    "Sorceress' Lair",
    "Bugbot Factory",
    "Super Bonus Round",
];

/// `D_80066F54` multiplied by 100. The atlas comment calls that table
/// "gems per level / 100", and both the loader cap and the percentage
/// denominator multiply by 100.
const GEM_MAX: [u16; 40] = [
    400, 400, 400, 400, 400, 400, 400, 0, 200, 400, 500, 500, 500, 500, 400, 500, 0, 200, 400, 600,
    600, 600, 600, 400, 600, 0, 200, 400, 700, 700, 700, 700, 400, 700, 0, 200, 5000, 0, 0, 0,
];

/// `D_80066F7C`, eggs per level. The 40 entries sum to 149.
const EGG_MAX: [u8; 40] = [
    5, 6, 6, 6, 6, 3, 3, 1, 1, 5, 6, 6, 6, 6, 3, 3, 1, 1, 5, 6, 6, 6, 6, 3, 3, 1, 1, 5, 6, 6, 6, 6,
    3, 3, 1, 1, 1, 0, 0, 0,
];

const EGG_TABLE_TOTAL: u32 = 149;

fn field(label: &str, value: impl ToString) -> Field {
    Field {
        label: label.into(),
        value: value.to_string(),
    }
}

fn gem_stored(slot: &[u8], level: usize) -> u32 {
    u32::from(u16::from_le_bytes([
        slot[0x530 + level * 2],
        slot[0x531 + level * 2],
    ]))
}

fn egg_bits(slot: &[u8], level: usize) -> u32 {
    u32::from(slot[0x580 + level].count_ones())
}

/// Sum copied into the HUD gem counter on load. Levels 0..=38 are capped to
/// `GEM_MAX`. Index 39 is added raw; the decomp comment calls it gems
/// retrieved from Moneybags.
fn loader_gems(slot: &[u8]) -> u32 {
    (0..40)
        .map(|level| {
            let stored = gem_stored(slot, level);
            if level < 0x27 {
                stored.min(u32::from(GEM_MAX[level]))
            } else {
                stored
            }
        })
        .sum()
}

/// Set bits copied into the HUD egg counter on load, all 40 bytes.
fn loader_eggs(slot: &[u8]) -> u32 {
    (0..40).map(|level| egg_bits(slot, level)).sum()
}

/// `func_title_8007928C`. `None` is the routine's -1: visited level 0 is clear.
fn completion(slot: &[u8]) -> Option<u32> {
    if slot[8] == 0 {
        return None;
    }
    let mut numer = 0u32;
    let mut denom = 0u32;
    for level in 0..0x27 {
        numer += gem_stored(slot, level) + egg_bits(slot, level) * 100;
        if level != BONUS_LEVEL {
            denom += u32::from(GEM_MAX[level]) + u32::from(EGG_MAX[level]) * 100;
        }
    }
    let mut pct = numer.saturating_mul(100) / denom;
    if numer != 0 && pct == 0 {
        pct = 1;
    }
    Some(pct)
}

fn checksum_matches(slot: &[u8]) -> bool {
    let sum = slot[..CHECKSUM_LEN]
        .iter()
        .fold(0u32, |sum, &byte| sum.wrapping_add(u32::from(byte)));
    sum == u32::from_le_bytes(slot[CHECKSUM_LEN..CHECKSUM_LEN + 4].try_into().unwrap())
}

fn location(id: u8) -> String {
    LEVELS
        .get(usize::from(id))
        .copied()
        .map(str::to_string)
        .unwrap_or_else(|| format!("unknown ({id})"))
}

pub(super) fn decode(fmt: &Format, code: &str, data: &[u8]) -> Result<SaveDetails, String> {
    if data[2] != 0x13 || data.get(4..4 + TITLE_PREFIX.len()) != Some(TITLE_PREFIX.as_slice()) {
        return Err("Unsupported Spyro: Year of the Dragon save title or icon layout.".into());
    }
    let mut profiles = Vec::with_capacity(3);
    for index in 0..3 {
        let start = SLOT_BASE + index * SLOT_SIZE;
        let slot = &data[start..start + SLOT_SIZE];
        let empty = slot[8] == 0;
        let mut profile = Profile {
            number: index as u8 + 1,
            name: format!("Adventure {}", index + 1),
            empty,
            status: None,
            checksum_ok: Some(checksum_matches(slot)),
            fields: Vec::new(),
            records: Vec::new(),
        };
        if !empty {
            profile.fields = vec![
                field("Saved location", location(slot[5])),
                field("Lives", slot[6]),
                field("Gems", loader_gems(slot)),
                field("Eggs", format!("{} / {EGG_TABLE_TOTAL}", loader_eggs(slot))),
                field("Completion", format!("{}%", completion(slot).unwrap())),
            ];
            profile.records = LEVELS
                .iter()
                .enumerate()
                .map(|(level, name)| Record {
                    name: (*name).to_string(),
                    fields: vec![
                        field("Gems", gem_stored(slot, level)),
                        field(
                            "Eggs",
                            format!("{} / {}", egg_bits(slot, level), EGG_MAX[level]),
                        ),
                    ],
                })
                .collect();
        }
        profiles.push(profile);
    }
    Ok(SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!("NTSC-U · {code}"),
        checksum_ok: Some(profiles.iter().all(|profile| profile.checksum_ok == Some(true))),
        profiles,
        notes: vec![
            "Three adventures live in one SPY3_1 file. Completion is the title-screen percentage: stored gems plus 100 per set egg bit, divided by the level tables, with Super Bonus Round left out of the denominator. A full table is 117%. An adventure is empty when Sunrise Spring Home is not marked visited. Code-traced from the NTSC-U rev 0 decomp; not checked on a console save.".into(),
            "Gems are the loader's capped counter, including stored total 39. Eggs count every set bit across 40 bytes; the egg table sums to 149. Lives are the stored byte. The loader changes lives below 4 to 4 and lives of 100 or more to 0. There is no Spyro 1 dragon tally. Skill points, story flags, speedway times and individual gem bits are not decoded.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::{
        checksum_matches, completion, loader_eggs, loader_gems, EGG_MAX, EGG_TABLE_TOTAL, GEM_MAX,
        LEVELS, SLOT_BASE, SLOT_SIZE, TITLE_PREFIX,
    };
    use crate::card::engine::{CardFormat, Ps1Card, BLOCK_SIZE, MCS_HEADER_SIZE};
    use crate::card::game_details::{decode, GameDetails};

    fn stamp(slot: &mut [u8]) {
        let sum = slot[..0x8FC]
            .iter()
            .fold(0u32, |sum, &byte| sum.wrapping_add(u32::from(byte)));
        slot[0x8FC..0x900].copy_from_slice(&sum.to_le_bytes());
    }

    fn payload() -> Vec<u8> {
        let mut data = vec![0; BLOCK_SIZE];
        data[..4].copy_from_slice(b"SC\x13\x01");
        data[4..4 + TITLE_PREFIX.len()].copy_from_slice(&TITLE_PREFIX);
        for index in 0..3 {
            let start = SLOT_BASE + index * SLOT_SIZE;
            stamp(&mut data[start..start + SLOT_SIZE]);
        }
        data
    }

    fn summary(data: &[u8]) -> crate::card::game_details::SaveDetails {
        match decode("SCUS-94467", "BA", "SPY3_1", data).unwrap() {
            GameDetails::Summary(details) => details,
            other => panic!("expected summary, got {other:?}"),
        }
    }

    fn value(details: &crate::card::game_details::SaveDetails, label: &str) -> String {
        details.profiles[0]
            .fields
            .iter()
            .find(|field| field.label == label)
            .unwrap()
            .value
            .clone()
    }

    #[test]
    fn tables_match_the_decomp_totals() {
        assert_eq!(LEVELS.len(), 37);
        assert_eq!(
            GEM_MAX.iter().map(|gems| u32::from(*gems)).sum::<u32>(),
            20_000
        );
        assert_eq!(
            EGG_MAX.iter().map(|eggs| u32::from(*eggs)).sum::<u32>(),
            EGG_TABLE_TOTAL
        );
        assert_eq!(GEM_MAX[0x24], 5000);
        assert_eq!(EGG_MAX[0x24], 1);
    }

    #[test]
    fn fresh_file_is_three_empty_checksummed_adventures() {
        let data = payload();
        let before = data.clone();
        let details = summary(&data);
        assert_eq!(details.game, "spyro-year-of-the-dragon");
        assert_eq!(details.release, "NTSC-U · SCUS-94467");
        assert_eq!(details.checksum_ok, Some(true));
        assert_eq!(details.profiles.len(), 3);
        assert!(details.profiles.iter().all(|profile| profile.empty));
        assert!(details
            .profiles
            .iter()
            .all(|profile| profile.checksum_ok == Some(true) && profile.fields.is_empty()));
        assert_eq!(data, before);
    }

    #[test]
    fn partial_progress_uses_the_title_screen_percentage() {
        let mut data = payload();
        let slot = &mut data[SLOT_BASE..SLOT_BASE + SLOT_SIZE];
        slot[5] = 1;
        slot[6] = 3;
        slot[8] = 1;
        slot[0x530] = 1; // one Sunrise Spring gem
        slot[0x581] = 0x01; // one Sunny Villa egg
        stamp(slot);
        let details = summary(&data);
        assert!(!details.profiles[0].empty);
        assert_eq!(details.profiles[0].checksum_ok, Some(true));
        assert_eq!(value(&details, "Saved location"), "Sunny Villa");
        assert_eq!(value(&details, "Lives"), "3");
        assert_eq!(value(&details, "Gems"), "1");
        assert_eq!(value(&details, "Eggs"), "1 / 149");
        assert_eq!(value(&details, "Completion"), "1%");
        assert_eq!(details.profiles[0].records[1].name, "Sunny Villa");
        assert_eq!(details.profiles[0].records[1].fields[1].value, "1 / 6");
        assert!(details.profiles[1].empty && details.profiles[2].empty);
        assert_eq!(details.checksum_ok, Some(true));
    }

    #[test]
    fn full_table_is_117_percent_and_the_loader_caps_separately() {
        let mut data = payload();
        let slot = &mut data[SLOT_BASE..SLOT_BASE + SLOT_SIZE];
        slot[5] = 36;
        slot[6] = 4;
        slot[8] = 1;
        for level in 0..37 {
            let gems = GEM_MAX[level].to_le_bytes();
            slot[0x530 + level * 2] = gems[0];
            slot[0x531 + level * 2] = gems[1];
            if EGG_MAX[level] > 0 {
                slot[0x580 + level] = (1u16 << EGG_MAX[level]) as u8 - 1;
            }
        }
        stamp(slot);
        assert_eq!(loader_gems(&data[SLOT_BASE..SLOT_BASE + SLOT_SIZE]), 20_000);
        assert_eq!(loader_eggs(&data[SLOT_BASE..SLOT_BASE + SLOT_SIZE]), 149);
        assert_eq!(
            completion(&data[SLOT_BASE..SLOT_BASE + SLOT_SIZE]),
            Some(117)
        );
        let details = summary(&data);
        assert_eq!(value(&details, "Saved location"), "Super Bonus Round");
        assert_eq!(value(&details, "Gems"), "20000");
        assert_eq!(value(&details, "Eggs"), "149 / 149");
        assert_eq!(value(&details, "Completion"), "117%");
        assert_eq!(details.profiles[0].records.len(), 37);

        let mut over = data.clone();
        let slot = &mut over[SLOT_BASE..SLOT_BASE + SLOT_SIZE];
        slot[0x530..0x532].copy_from_slice(&9999u16.to_le_bytes());
        slot[0x581] = 0xff;
        slot[0x57E..0x580].copy_from_slice(&500u16.to_le_bytes());
        stamp(slot);
        let slot = &over[SLOT_BASE..SLOT_BASE + SLOT_SIZE];
        assert!(checksum_matches(slot));
        // Stored 9999 is capped to 400 for the loader total. Index 39 is added
        // raw there and is not part of the percentage numerator.
        assert_eq!(loader_gems(slot), 20_500);
        let numer = 20_000 - 400 + 9999 + (149 - 6 + 8) * 100;
        assert_eq!(completion(slot), Some(numer * 100 / 29_800));
        let details = summary(&over);
        assert_eq!(value(&details, "Gems"), "20500");
        assert_eq!(value(&details, "Eggs"), "151 / 149");
        assert_eq!(details.profiles[0].records[0].fields[0].value, "9999");
        assert_eq!(details.profiles[0].records[1].fields[1].value, "8 / 6");
    }

    #[test]
    fn visited_home_with_only_total_39_is_zero_percent() {
        let mut data = payload();
        let slot = &mut data[SLOT_BASE..SLOT_BASE + SLOT_SIZE];
        slot[8] = 1;
        slot[0x57E..0x580].copy_from_slice(&500u16.to_le_bytes());
        stamp(slot);
        let details = summary(&data);
        assert!(!details.profiles[0].empty);
        assert_eq!(value(&details, "Gems"), "500");
        assert_eq!(value(&details, "Completion"), "0%");
    }

    #[test]
    fn clear_visited_home_stays_empty_even_with_other_bytes() {
        let mut data = payload();
        let slot = &mut data[SLOT_BASE..SLOT_BASE + SLOT_SIZE];
        slot[5] = 4;
        slot[6] = 9;
        slot[0x530] = 50;
        stamp(slot);
        let details = summary(&data);
        assert!(details.profiles[0].empty);
        assert!(details.profiles[0].fields.is_empty());
        assert_eq!(details.profiles[0].checksum_ok, Some(true));
    }

    #[test]
    fn checksum_mismatch_keeps_fields_and_fails_the_file() {
        let mut data = payload();
        let slot = &mut data[SLOT_BASE..SLOT_BASE + SLOT_SIZE];
        slot[6] = 4;
        slot[8] = 1;
        stamp(slot);
        data[SLOT_BASE + 6] ^= 1;
        let details = summary(&data);
        assert_eq!(details.checksum_ok, Some(false));
        assert_eq!(details.profiles[0].checksum_ok, Some(false));
        assert_eq!(value(&details, "Lives"), "5");
        assert_eq!(details.profiles[1].checksum_ok, Some(true));
    }

    #[test]
    fn rejects_other_releases_identifiers_and_headers() {
        let data = payload();
        for (code, region, id) in [
            ("SCES-02835", "BE", "SPY3_1"),
            ("SCUS-94467", "BI", "SPY3_1"),
            ("SCUS-94467", "BA", "SPYRO"),
            ("SCUS-94467", "BA", "SPY3_2"),
            ("SCUS-94467", "BA", "SPY3_1X"),
            ("SCUS-94467", "BA", ""),
        ] {
            assert!(
                decode(code, region, id, &data).is_err(),
                "{code} {region} {id}"
            );
        }
        for length in 0..BLOCK_SIZE {
            assert!(decode("SCUS-94467", "BA", "SPY3_1", &data[..length]).is_err());
        }
        let mut bad = data.clone();
        bad.push(0);
        assert!(decode("SCUS-94467", "BA", "SPY3_1", &bad).is_err());
        for offset in [0, 2, 3, 4] {
            let mut bad = data.clone();
            bad[offset] ^= 1;
            assert!(decode("SCUS-94467", "BA", "SPY3_1", &bad).is_err());
        }
    }

    #[test]
    fn card_roundtrip_keeps_three_adventures() {
        let data = payload();
        let mut bytes = vec![0; MCS_HEADER_SIZE];
        bytes[0] = 0x51;
        bytes[4..8].copy_from_slice(&(data.len() as u32).to_le_bytes());
        bytes[8..10].copy_from_slice(&0xffffu16.to_le_bytes());
        bytes[10..28].copy_from_slice(b"BASCUS-94467SPY3_1");
        bytes.extend(data);
        let mut card = Ps1Card::create_formatted("spyro 3");
        card.set_save_bytes(1, &bytes).unwrap();
        let view = card.view();
        assert_eq!(view.saves[0].prod_code, "SCUS-94467");
        assert_eq!(view.saves[0].identifier, "SPY3_1");
        assert!(view.saves[0].game_details_error.is_none());
        let dto = serde_json::to_value(&view.saves[0].game_details).unwrap();
        assert_eq!(dto["game"], "spyro-year-of-the-dragon");
        assert_eq!(dto["profiles"].as_array().unwrap().len(), 3);
        assert_eq!(dto["checksumOk"], true);
        for format in [CardFormat::Raw, CardFormat::Gme, CardFormat::Vgs] {
            let encoded = card.export(format, false);
            let opened = Ps1Card::open(&encoded, "spyro 3", false).unwrap();
            assert_eq!(
                serde_json::to_value(&opened.view().saves[0].game_details).unwrap(),
                dto
            );
        }
    }
}
