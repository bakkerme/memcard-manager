//! Read-only US Digimon World 2 save decoder (SLUS-01193).
//!
//! Layout, checksum, text encoding and name tables adapted from acemon33/DW2-TT,
//! licensed GPL-3.0, revision 6325d60a628db4b0576d67162d059e653d59ea74:
//! https://github.com/acemon33/DW2-TT under dw2_exp_multiplier/: Entity/SaveFile.cs,
//! DigimonWorld2Tool/TextConversion.cs and Resources/Vanilla/{data,config}.xml.
//! This module never rewrites a save.
use super::digimon_world2_names::{location_name, species_name, technique_name};
use serde::Serialize;

const SAVE_SIZE: usize = 0x4000;
const PROFILE_START: usize = 0x200;
const PROFILE_SIZE: usize = 0x1058;

// USA save-menu conversion, confirmed by Wyrelade/Digimon-World-2-Decomp (CC0):
// cff2114139f0d93dd8fa94fa09600462ed9a35a2/src/stag1100/stag1100_301C.c,
// Stg11_CardMenuDraw. The VSync counter uses 216000 ticks/hour and 3600 ticks/minute;
// the menu clamps to 0x14996ff (99:59:59 plus 59 ticks) before formatting HH:MM.
fn playtime_minutes(ticks: u32) -> u32 {
    ticks.min(0x14996ff) / 3600
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "game", rename_all = "kebab-case")]
pub enum GameDetails {
    #[serde(rename = "digimon-world-2")]
    DigimonWorld2 {
        #[serde(rename = "checksumOk")]
        checksum_ok: bool,
        profiles: Vec<Profile>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub number: u8,
    pub empty: bool,
    pub tamer: String,
    pub rank: String,
    pub location: String,
    pub bits: i32,
    pub beetle: String,
    pub playtime_minutes: Option<u32>,
    pub digimon: Vec<Digimon>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Digimon {
    pub roster_slot: u8,
    pub status: u8,
    pub species: String,
    pub name: String,
    pub level: u8,
    pub max_level: u8,
    pub dp: u8,
    pub experience: u32,
    pub hp: u16,
    pub max_hp: u16,
    pub mp: u16,
    pub max_mp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
    pub techniques: Vec<String>,
    pub inherited_techniques: Vec<String>,
}

fn word(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn checksum(data: &[u8]) -> u16 {
    let mut value = 0u16;
    for offset in (0..0x3ffc).step_by(4) {
        value = (value ^ word(data, offset)).wrapping_add(word(data, offset + 2));
    }
    value ^ word(data, 0x3ffc)
}

// Player-entered names use single-byte characters, FF terminates, FD is a space.
// Unknown characters remain visible as replacement marks, never guessed.
fn name(data: &[u8]) -> String {
    let mut result = String::new();
    let mut bytes = data.iter().copied();
    while let Some(byte) = bytes.next() {
        let ch = match byte {
            0xff => break,
            0x00..=0x09 => char::from(b'0' + byte),
            0x0a..=0x23 => char::from(b'A' + byte - 0x0a),
            0x24..=0x3d => char::from(b'a' + byte - 0x24),
            0x3e => '○',
            0x3f => '×',
            0x40 => '△',
            0x41 => '□',
            0x42 => '&',
            0x44 => '?',
            0x45 => '!',
            0x46 => '/',
            0x47 => '♪',
            0x49 => '-',
            0x4f => 'Ω',
            0x54 => ',',
            0x55 => '.',
            0x56 => '\'',
            0x57 => '"',
            0x58 => ';',
            0x59 => ':',
            0x5a => '%',
            0x5b => '+',
            0x5d => '#',
            0x5e => 'õ',
            0x5f => 'ô',
            0x60 => 'ó',
            0x61 => 'í',
            0x62 => 'é',
            0x63 => 'ê',
            0x64 => 'á',
            0x65 => 'à',
            0x66 => 'â',
            0x67 => 'ã',
            0x68 => 'ú',
            0x69 => 'ç',
            0xfd => ' ',
            // Dictionary tokens are used in story text, not player-entered names.
            0xf0 => {
                bytes.next();
                '\u{fffd}'
            }
            _ => '\u{fffd}',
        };
        result.push(ch);
    }
    result.trim().to_string()
}

fn techniques(data: &[u8]) -> Vec<String> {
    let mut result = Vec::new();
    for &id in data.iter().filter(|&&id| id != 0) {
        let label = technique_name(id)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Unknown technique (0x{id:02X})"));
        if !result.contains(&label) {
            result.push(label);
        }
    }
    result
}

pub(super) fn decode(data: &[u8]) -> Result<GameDetails, String> {
    if data.len() != SAVE_SIZE || &data[..2] != b"SC" || data[3] != 2 {
        return Err("This save does not have the expected two-block US Digimon World 2 layout. Try another backup of this save.".into());
    }
    let mut profiles = Vec::new();
    for i in 0..3 {
        let start = PROFILE_START + i * PROFILE_SIZE;
        let p = &data[start..start + PROFILE_SIZE];
        let empty = p.iter().all(|&b| b == 0);
        // An erased record must not appear as a populated profile with invented values.
        if p.iter().all(|&b| b == 0xff) {
            return Err(
                "An in-game profile is erased or unreadable. Try another backup of this save."
                    .into(),
            );
        }
        let mut digimon = Vec::new();
        if !empty {
            for n in 0..36 {
                let d = &p[0xf0 + n * 0x5c..0xf0 + (n + 1) * 0x5c];
                if d[0] == 0 {
                    continue;
                }
                digimon.push(Digimon {
                    roster_slot: (n + 1) as u8,
                    status: d[0],
                    species: species_name(d[1])
                        .filter(|s| *s != "?????")
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("Unknown Digimon (0x{:02X})", d[1])),
                    name: name(&d[76..90]),
                    level: d[13],
                    dp: d[14],
                    max_level: d[15],
                    experience: u32::from_le_bytes(d[16..20].try_into().unwrap()),
                    max_hp: word(d, 20),
                    hp: word(d, 22),
                    max_mp: word(d, 24),
                    mp: word(d, 26),
                    attack: word(d, 28),
                    defense: word(d, 30),
                    speed: word(d, 32),
                    techniques: techniques(&d[34..46]),
                    inherited_techniques: techniques(&d[46..68]),
                });
            }
        }
        profiles.push(Profile {
            number: (i + 1) as u8,
            empty,
            tamer: if empty {
                String::new()
            } else {
                name(&p[0x20..0x26])
            },
            beetle: if empty {
                String::new()
            } else {
                name(&p[0xdd..0xe5])
            },
            rank: [
                "Cadet",
                "Beginner",
                "Amateur",
                "Rookie",
                "Normal",
                "Pro",
                "Expert",
                "Elite",
                "Commander",
                "Chief",
                "Master",
            ]
            .get(p[0x1e] as usize)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Unknown rank (0x{:02X})", p[0x1e])),
            location: location_name(p[0], p[0x0d])
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Unknown location (0x{:02X}/0x{:02X})", p[0], p[0x0d])),
            bits: i32::from_le_bytes(p[0x14..0x18].try_into().unwrap()),
            playtime_minutes: if empty {
                None
            } else {
                Some(playtime_minutes(u32::from_le_bytes(
                    p[0x10..0x14].try_into().unwrap(),
                )))
            },
            digimon,
        });
    }
    Ok(GameDetails::DigimonWorld2 {
        checksum_ok: checksum(data) == word(data, 0x3ffe),
        profiles,
    })
}

#[cfg(test)]
mod tests {
    use super::super::engine::{compose_new_card, Ps1Card};
    use super::*;

    fn card() -> Ps1Card {
        Ps1Card::open(include_bytes!("../../../blue.mcr"), "blue.mcr", false).unwrap()
    }
    fn payload() -> Vec<u8> {
        card().get_save_bytes(8)[128..].to_vec()
    }

    #[test]
    fn real_us_save_has_three_profiles_and_one_agumon() {
        let view = card().view();
        let save = view.saves.iter().find(|s| s.master_slot == 8).unwrap();
        let GameDetails::DigimonWorld2 {
            checksum_ok,
            profiles,
        } = match save.game_details.as_ref().unwrap() {
            super::super::game_details::GameDetails::Digimon(details) => details,
            _ => panic!("expected Digimon World 2 details"),
        };
        assert!(*checksum_ok);
        let dto = serde_json::to_value(save).unwrap();
        assert_eq!(dto["gameDetails"]["game"], "digimon-world-2");
        assert_eq!(dto["gameDetails"]["checksumOk"], true);
        assert_eq!(dto["gameDetails"]["profiles"][0]["digimon"][0]["maxHp"], 61);
        assert!(save.game_details_error.is_none());
        assert_eq!(profiles.len(), 3);
        assert!(profiles[1].empty && profiles[2].empty);
        let p = &profiles[0];
        assert_eq!(
            (&*p.tamer, &*p.beetle, &*p.rank, &*p.location),
            ("Akira", "Gunner", "Beginner", "Digital City - Main Gate")
        );
        assert_eq!(p.bits, 0);
        // User confirmed the game's save menu displays 01:00 for this fixture.
        assert_eq!(p.playtime_minutes, Some(60));
        assert_eq!(dto["gameDetails"]["profiles"][0]["playtimeMinutes"], 60);
        assert!(profiles[1..].iter().all(|p| p.playtime_minutes.is_none()));
        assert_eq!(p.digimon.len(), 1);
        let d = &p.digimon[0];
        assert_eq!(
            (&*d.name, &*d.species, d.level, d.status),
            ("Argoota", "Agumon", 4, 3)
        );
        assert_eq!(
            (
                d.hp,
                d.max_hp,
                d.mp,
                d.max_mp,
                d.attack,
                d.defense,
                d.speed,
                d.experience
            ),
            (61, 61, 57, 57, 37, 40, 20, 50)
        );
        assert_eq!(d.techniques, vec!["Pepper Breath"]);
        assert!(view
            .saves
            .iter()
            .filter(|s| super::super::game_details::format(&s.prod_code).is_none())
            .all(|s| s.game_details.is_none() && s.game_details_error.is_none()));
    }

    #[test]
    fn changed_data_warns_without_repairing_bytes() {
        let mut data = payload();
        data[0x214] = 7;
        let before = data.clone();
        let GameDetails::DigimonWorld2 {
            checksum_ok,
            profiles,
        } = decode(&data).unwrap();
        assert!(!checksum_ok);
        assert_eq!(profiles[0].bits, 7);
        assert_eq!(data, before);
    }

    #[test]
    fn playtime_matches_game_minute_boundaries_and_display_cap() {
        for (ticks, minutes) in [
            (0, 0),
            (3599, 0),
            (3600, 1),
            (215999, 59),
            (216000, 60),
            (217430, 60),
            (219599, 60),
            (219600, 61),
            (0x14996ff, 5999),
            (21600000, 5999),
            (u32::MAX, 5999),
        ] {
            assert_eq!(playtime_minutes(ticks), minutes, "counter {ticks}");
        }
    }

    #[test]
    fn playtime_is_read_from_each_profile_in_little_endian_order() {
        let mut data = payload();
        let first = data[PROFILE_START..PROFILE_START + PROFILE_SIZE].to_vec();
        for (i, ticks) in [217430u32, 435600, 0].into_iter().enumerate() {
            let start = PROFILE_START + i * PROFILE_SIZE;
            data[start..start + PROFILE_SIZE].copy_from_slice(&first);
            data[start + 0x10..start + 0x14].copy_from_slice(&ticks.to_le_bytes());
        }
        let before = data.clone();
        let GameDetails::DigimonWorld2 { profiles, .. } = decode(&data).unwrap();
        assert_eq!(
            profiles
                .iter()
                .map(|p| p.playtime_minutes)
                .collect::<Vec<_>>(),
            vec![Some(60), Some(121), Some(0)]
        );
        assert_eq!(data, before);
    }

    #[test]
    fn all_profiles_and_unknown_values_are_preserved() {
        let mut data = payload();
        let first = data[0x200..0x1258].to_vec();
        data[0x1258..0x22b0].copy_from_slice(&first);
        data[0x22b0..0x3308].copy_from_slice(&first);
        data[0x220] = 0x70;
        data[0x2f1] = 0xff;
        data[0x2f0] = 9;
        let GameDetails::DigimonWorld2 { profiles, .. } = decode(&data).unwrap();
        assert!(profiles.iter().all(|p| !p.empty));
        assert!(profiles[0].tamer.starts_with('\u{fffd}'));
        assert_eq!(profiles[0].digimon[0].species, "Unknown Digimon (0xFF)");
        assert_eq!(profiles[0].digimon[0].status, 9);
        assert_eq!(name(&[0, 0xfd, 0x0a, 0xff, 0x0b]), "0 A");
    }

    #[test]
    fn malformed_layout_is_rejected() {
        for length in [0, 1, 8192, 16383, 16385] {
            assert!(decode(&vec![0; length]).is_err());
        }
        let mut data = payload();
        data[0] = 0;
        assert!(decode(&data).is_err());
        data = payload();
        data[3] = 1;
        assert!(decode(&data).is_err());
        data = payload();
        data[0x1258..0x22b0].fill(0xff);
        assert!(decode(&data).is_err());
    }

    #[test]
    fn malformed_chain_and_other_releases_do_not_decode() {
        let original = include_bytes!("../../../blue.mcr");
        let mut raw = original.to_vec();
        raw[128 + 8 * 128 + 8] = 0xff;
        let view = Ps1Card::open(&raw, "broken.mcr", false).unwrap().view();
        let save = view.saves.iter().find(|s| s.master_slot == 8).unwrap();
        assert!(save.game_details.is_none());
        assert!(save
            .game_details_error
            .as_ref()
            .unwrap()
            .contains("block chain"));
        raw = original.to_vec();
        raw[128 + 8 * 128 + 11] = b'E';
        let view = Ps1Card::open(&raw, "other-region.mcr", false)
            .unwrap()
            .view();
        let save = view.saves.iter().find(|s| s.master_slot == 8).unwrap();
        assert!(save.game_details.is_none() && save.game_details_error.is_some());
    }

    #[test]
    fn exported_mcs_details_match_card_details() {
        let source = card();
        let bytes = source.get_save_bytes(8);
        let mut imported = Ps1Card::create_formatted("backup");
        imported.set_save_bytes(0, &bytes).unwrap();
        let original = source
            .view()
            .saves
            .into_iter()
            .find(|s| s.master_slot == 8)
            .unwrap();
        let restored = imported.view().saves.remove(0);
        assert_eq!(
            serde_json::to_value(original.game_details).unwrap(),
            serde_json::to_value(restored.game_details).unwrap()
        );
    }

    #[test]
    fn details_survive_relocation_and_non_contiguous_chain() {
        let source = card();
        let composed = compose_new_card(&source, &[8], None).unwrap();
        let mut raw = composed.save_raw(true);
        // Relocate the continuation block from slot 1 to slot 5.
        let continuation = raw[2 * 8192..3 * 8192].to_vec();
        raw[6 * 8192..7 * 8192].copy_from_slice(&continuation);
        let header = raw[256..384].to_vec();
        raw[768..896].copy_from_slice(&header);
        raw[256] = 0xa0;
        raw[136] = 5;
        let moved = Ps1Card::open(&raw, "moved.mcr", false).unwrap();
        let view = moved.view();
        assert_eq!(view.saves[0].linked_slots, vec![0, 5]);
        let source_view = source.view();
        let original = source_view
            .saves
            .iter()
            .find(|s| s.master_slot == 8)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&original.game_details).unwrap(),
            serde_json::to_value(&view.saves[0].game_details).unwrap()
        );
    }
}
