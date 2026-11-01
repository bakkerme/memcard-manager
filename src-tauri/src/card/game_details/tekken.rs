//! Original Tekken read-only summary.
//!
//! MIT source adaptation: RyudoSynbios/game-tools-collection commit
//! `15d71340c3f1717745139c2e0774510cd37dfbdd`,
//! `src/lib/templates/tekken/saveEditor/template.ts` and `utils.ts`.
//! See `docs/formats/tekken.md`.
use super::{Field, Format, Profile, Record, SaveDetails};
use unicode_normalization::UnicodeNormalization;

const NAMES: [&str; 17] = [
    "Kazuya",
    "Paul",
    "Law",
    "Jack",
    "Nina",
    "King",
    "Yoshimitsu",
    "Michelle",
    "Wang",
    "Heihachi",
    "Lee",
    "Prototype Jack",
    "Armor King",
    "Anna",
    "Ganryu",
    "Kuma",
    "Kunimitsu",
];

pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    if d[2] != 0x13 || title(d)? != expected_title(code) {
        return Err("Unsupported Tekken title or icon layout.".into());
    }
    // Template default is /50. utils.ts replaces that with /60 when the
    // selected region is not the first validator entry (Europe).
    let time_divisor = if code == "SCES-00005" { 50.0 } else { 60.0 };
    let region = match code {
        "SCES-00005" => "PAL",
        "SLPS-00040" => "NTSC-J",
        _ => "NTSC-U",
    };
    let mut records = Vec::new();
    for place in 0..16 {
        let offset = 0x228 + place * 12;
        if d[offset..offset + 12].iter().all(|&byte| byte == 0) {
            continue;
        }
        records.push(Record {
            name: format!("Time {} place", ordinal(place + 1)),
            fields: vec![
                field("Time", record_time(u32_at(d, offset), time_divisor)),
                field("Character", character_name(u16_at(d, offset + 4))),
                field("Fight count", u16_at(d, offset + 6).to_string()),
                field("Name", record_name(&d[offset + 8..offset + 11])),
            ],
        });
    }
    for place in 0..16 {
        let offset = 0x2e8 + place * 12;
        if d[offset..offset + 12].iter().all(|&byte| byte == 0) {
            continue;
        }
        records.push(Record {
            name: format!("2P {} place", ordinal(place + 1)),
            fields: vec![
                field("Wins", u32_at(d, offset).to_string()),
                field("Character", character_name(u16_at(d, offset + 4))),
            ],
        });
    }
    Ok(SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!("{region} · {code}"),
        checksum_ok: None,
        profiles: vec![Profile {
            number: 1,
            name: "Game progress".into(),
            empty: false,
            status: None,
            checksum_ok: None,
            fields: vec![
                field("Unlocked characters", flag_list(d, 0x20c)),
                field("Arcade completed", flag_list(d, 0x210)),
                field("Kazuya costume", costume(d[0x214])),
                field("Character totals", character_totals(d)),
            ],
            records,
        }],
        notes: vec!["Seventeen character unlocks, the one mapped Kazuya costume bit, arcade completion, sixteen time places and sixteen versus-win places follow the MIT editor. Europe divides the stored time count by 50; USA and Japan divide by 60. The minutes:seconds.milliseconds reading is that conversion. Character totals are the editor's unnamed per-character counts. Options, button config, disabled costume placeholders and the hidden versus words are not decoded. No checksum is defined. Ported from the MIT editor and checked against its public samples; no emulator save/reload validation.".into()],
    })
}

fn expected_title(code: &str) -> &'static str {
    if code == "SLPS-00040" {
        "真の格闘ゲームファンを熱くさせる![鉄拳]"
    } else {
        "True fighters love TEKKEN !"
    }
}

fn title(d: &[u8]) -> Result<String, String> {
    let end = d[4..68].iter().position(|&byte| byte == 0).unwrap_or(64);
    let (text, _, invalid) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if invalid {
        return Err("Unsupported Tekken title or icon layout.".into());
    }
    Ok(text.nfkc().collect())
}

fn u16_at(d: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(d[offset..offset + 2].try_into().unwrap())
}

fn u32_at(d: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(d[offset..offset + 4].try_into().unwrap())
}

fn field(label: &str, value: impl Into<String>) -> Field {
    Field {
        label: label.into(),
        value: value.into(),
    }
}

fn ordinal(place: usize) -> String {
    let suffix = match place % 100 {
        11..=13 => "th",
        _ => match place % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{place}{suffix}")
}

fn character_name(id: u16) -> String {
    NAMES
        .get(usize::from(id))
        .copied()
        .map(str::to_string)
        .unwrap_or_else(|| format!("Unknown ({id})"))
}

fn record_name(bytes: &[u8]) -> String {
    let allowed = |byte: u8| {
        byte == b' '
            || byte == b'.'
            || byte == b'?'
            || byte.is_ascii_digit()
            || byte.is_ascii_uppercase()
    };
    if bytes.iter().copied().all(allowed) {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        format!(
            "Unknown name ({:02X}{:02X}{:02X})",
            bytes[0], bytes[1], bytes[2]
        )
    }
}

/// Editor chrono group: divide by the regional rate, shift seconds into the
/// hours bucket (`* 60` then hours), and read the seconds and milliseconds
/// components of the unshifted value. The hours bucket is the minute count.
fn record_time(raw: u32, divisor: f64) -> String {
    let seconds = f64::from(raw) / divisor;
    let millis = seconds * 1000.0;
    let minutes = (seconds * 60.0 * 1000.0 / 3_600_000.0).floor() as u64;
    let secs = (millis / 1000.0).floor() as u64 % 60;
    let fractional = (millis % 1000.0).round() as u64;
    format!("{minutes:02}:{secs:02}.{fractional:03}")
}

fn flag_list(data: &[u8], offset: usize) -> String {
    let names = NAMES
        .iter()
        .enumerate()
        .filter(|(index, _)| data[offset + index / 8] & (1 << (index % 8)) != 0)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>();
    let unknown = data[offset + 2] & !1;
    match (names.is_empty(), unknown) {
        (true, 0) => "None recorded".into(),
        (true, mask) => format!("Unknown bits 0x{mask:02X}"),
        (false, 0) => names.join(", "),
        (false, mask) => format!("{}; unknown bits 0x{mask:02X}", names.join(", ")),
    }
}

fn costume(byte: u8) -> String {
    let state = if byte & 1 == 0 { "Locked" } else { "Unlocked" };
    let unknown = byte & !1;
    if unknown == 0 {
        state.into()
    } else {
        format!("{state}; unknown bits 0x{unknown:02X}")
    }
}

fn character_totals(data: &[u8]) -> String {
    let totals = NAMES
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            let total = u32_at(data, 0x3a8 + index * 4);
            (total != 0).then(|| format!("{name} {total}"))
        })
        .collect::<Vec<_>>();
    if totals.is_empty() {
        "None recorded".into()
    } else {
        totals.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::super::{decode, GameDetails};

    const USA_TITLE: &[u8] = &[
        0x82, 0x73, 0x82, 0x92, 0x82, 0x95, 0x82, 0x85, 0x81, 0x40, 0x82, 0x86, 0x82, 0x89, 0x82,
        0x87, 0x82, 0x88, 0x82, 0x94, 0x82, 0x85, 0x82, 0x92, 0x82, 0x93, 0x81, 0x40, 0x82, 0x8C,
        0x82, 0x8F, 0x82, 0x96, 0x82, 0x85, 0x81, 0x40, 0x82, 0x73, 0x82, 0x64, 0x82, 0x6A, 0x82,
        0x6A, 0x82, 0x64, 0x82, 0x6D, 0x81, 0x40, 0x81, 0x49,
    ];
    const JAPAN_TITLE: &[u8] = &[
        0x90, 0x5E, 0x82, 0xCC, 0x8A, 0x69, 0x93, 0xAC, 0x83, 0x51, 0x81, 0x5B, 0x83, 0x80, 0x83,
        0x74, 0x83, 0x40, 0x83, 0x93, 0x82, 0xF0, 0x94, 0x4D, 0x82, 0xAD, 0x82, 0xB3, 0x82, 0xB9,
        0x82, 0xE9, 0x81, 0x49, 0x81, 0x6D, 0x93, 0x53, 0x8C, 0x9D, 0x81, 0x6E,
    ];

    fn payload(code: &str) -> Vec<u8> {
        let mut data = vec![0; 8192];
        data[..4].copy_from_slice(b"SC\x13\x01");
        let title = if code == "SLPS-00040" {
            JAPAN_TITLE
        } else {
            USA_TITLE
        };
        data[4..4 + title.len()].copy_from_slice(title);
        data
    }

    fn summary(code: &str, region: &str, data: &[u8]) -> super::super::SaveDetails {
        match decode(code, region, "<TEKKEN>", data).unwrap() {
            GameDetails::Summary(details) => details,
            other => panic!("expected summary, got {other:?}"),
        }
    }

    fn value<'a>(details: &'a super::super::SaveDetails, label: &str) -> &'a str {
        &details.profiles[0]
            .fields
            .iter()
            .find(|field| field.label == label)
            .unwrap()
            .value
    }

    #[test]
    fn tekken_progress_times_and_unclamped_totals() {
        let mut usa = payload("SLUS-00006");
        usa[0x20c] = 0x01;
        usa[0x20e] = 0x03;
        usa[0x210] = 0x02;
        usa[0x214] = 0x05;
        usa[0x228..0x22c].copy_from_slice(&1234567u32.to_le_bytes());
        usa[0x22c..0x22e].copy_from_slice(&0x10u16.to_le_bytes());
        usa[0x22e..0x230].copy_from_slice(&65535u16.to_le_bytes());
        usa[0x230..0x233].copy_from_slice(b"A?Z");
        usa[0x2e8..0x2ec].copy_from_slice(&99u32.to_le_bytes());
        usa[0x2ec..0x2ee].copy_from_slice(&99u16.to_le_bytes());
        usa[0x3e8..0x3ec].copy_from_slice(&100_000u32.to_le_bytes());
        let before = usa.clone();
        let details = summary("SLUS-00006", "BA", &usa);
        assert_eq!(details.game, "tekken");
        assert_eq!(details.release, "NTSC-U · SLUS-00006");
        assert_eq!(details.checksum_ok, None);
        assert_eq!(
            value(&details, "Unlocked characters"),
            "Kazuya, Kunimitsu; unknown bits 0x02"
        );
        assert_eq!(value(&details, "Arcade completed"), "Paul");
        assert_eq!(
            value(&details, "Kazuya costume"),
            "Unlocked; unknown bits 0x04"
        );
        assert_eq!(value(&details, "Character totals"), "Kunimitsu 100000");
        assert_eq!(details.profiles[0].records.len(), 2);
        assert_eq!(details.profiles[0].records[0].name, "Time 1st place");
        assert_eq!(
            details.profiles[0].records[0]
                .fields
                .iter()
                .map(|field| field.value.as_str())
                .collect::<Vec<_>>(),
            ["342:56.117", "Kunimitsu", "65535", "A?Z"]
        );
        assert_eq!(details.profiles[0].records[1].name, "2P 1st place");
        assert_eq!(
            details.profiles[0].records[1]
                .fields
                .iter()
                .map(|field| field.value.as_str())
                .collect::<Vec<_>>(),
            ["99", "Unknown (99)"]
        );
        assert_eq!(usa, before);

        let mut europe = payload("SCES-00005");
        europe[0x228..0x22c].copy_from_slice(&1234567u32.to_le_bytes());
        let europe = summary("SCES-00005", "BE", &europe);
        assert_eq!(europe.release, "PAL · SCES-00005");
        assert_eq!(europe.profiles[0].records[0].fields[0].value, "411:31.340");

        let mut minutes = payload("SLUS-00006");
        minutes[0x228..0x22c].copy_from_slice(&180000u32.to_le_bytes());
        assert_eq!(
            summary("SLUS-00006", "BA", &minutes).profiles[0].records[0].fields[0].value,
            "50:00.000"
        );
        let mut pal_minutes = payload("SCES-00005");
        pal_minutes[0x228..0x22c].copy_from_slice(&150000u32.to_le_bytes());
        assert_eq!(
            summary("SCES-00005", "BE", &pal_minutes).profiles[0].records[0].fields[0].value,
            "50:00.000"
        );

        let japan = summary("SLPS-00040", "BI", &payload("SLPS-00040"));
        assert_eq!(japan.release, "NTSC-J · SLPS-00040");
        assert_eq!(value(&japan, "Unlocked characters"), "None recorded");
        assert!(japan.profiles[0].records.is_empty());

        let mut bad_name = payload("SLUS-00006");
        bad_name[0x228] = 1;
        bad_name[0x230..0x233].copy_from_slice(&[0x01, 0x02, 0x03]);
        let bad_name = summary("SLUS-00006", "BA", &bad_name);
        assert_eq!(
            bad_name.profiles[0].records[0].fields[3].value,
            "Unknown name (010203)"
        );
    }

    #[test]
    fn tekken_rejects_other_identity_layout_and_truncation() {
        let usa = payload("SLUS-00006");
        let japan = payload("SLPS-00040");
        for (code, region, id, data) in [
            ("SCES-00005", "BA", "<TEKKEN>", usa.as_slice()),
            ("SLUS-00006", "BI", "<TEKKEN>", usa.as_slice()),
            ("SLUS-00006", "BE", "<TEKKEN>", usa.as_slice()),
            ("SLPS-00040", "BA", "<TEKKEN>", japan.as_slice()),
            ("SLUS-00006", "BA", "TEKKEN", usa.as_slice()),
            ("SLUS-00006", "BA", "<TEKKEN>X", usa.as_slice()),
            ("SLUS-00006", "BA", "<tekken>", usa.as_slice()),
            ("SLUS-00213", "BA", "TEKKEN-2", usa.as_slice()),
            ("SLUS-00006", "BA", "<TEKKEN>", japan.as_slice()),
            ("SLPS-00040", "BI", "<TEKKEN>", usa.as_slice()),
        ] {
            assert!(
                decode(code, region, id, data).is_err(),
                "{code} {region} {id}"
            );
        }
        for offset in [0, 2, 3, 4] {
            let mut bad = usa.clone();
            bad[offset] ^= 1;
            assert!(decode("SLUS-00006", "BA", "<TEKKEN>", &bad).is_err());
        }
        for length in 0..8192 {
            assert!(decode("SLUS-00006", "BA", "<TEKKEN>", &usa[..length]).is_err());
        }
        let mut extra = usa;
        extra.push(0);
        assert!(decode("SLUS-00006", "BA", "<TEKKEN>", &extra).is_err());
    }

    #[test]
    fn tekken_card_view_uses_the_directory_identifier() {
        use crate::card::engine::{Ps1Card, MCS_HEADER_SIZE};
        let payload = payload("SLUS-00006");
        let mut bytes = vec![0u8; MCS_HEADER_SIZE + payload.len()];
        bytes[0] = 0x51;
        bytes[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes[8..10].copy_from_slice(&0xffffu16.to_le_bytes());
        let name = "BASLUS-00006<TEKKEN>";
        assert_eq!(name.len(), 20);
        bytes[10..30].copy_from_slice(name.as_bytes());
        bytes[MCS_HEADER_SIZE..].copy_from_slice(&payload);
        let mut card = Ps1Card::create_formatted("tekken");
        card.set_save_bytes(0, &bytes).unwrap();
        let view = card.view();
        assert_eq!(view.saves.len(), 1);
        assert_eq!(view.saves[0].identifier, "<TEKKEN>");
        assert!(view.saves[0].game_details_error.is_none());
        let GameDetails::Summary(details) = view.saves[0].game_details.as_ref().unwrap() else {
            panic!("summary");
        };
        assert_eq!(details.release, "NTSC-U · SLUS-00006");
        assert_eq!(value(details, "Kazuya costume"), "Locked");
    }

    #[test]
    #[ignore = "Requires GAME_TOOLS_COLLECTION at the documented source commit; no third-party saves redistributed"]
    fn tekken_public_cards() {
        use crate::card::engine::Ps1Card;
        use sha2::{Digest, Sha256};

        let root = std::path::PathBuf::from(std::env::var("GAME_TOOLS_COLLECTION").unwrap())
            .join("tests/saves/tekken/playstation");
        for (file, sha, code, region) in [
            (
                "usa.mcr",
                "c03248bf8b5d9234c3ad96c517337f1fcee4c672e4a58bdca59780d27fdcece0",
                "SLUS-00006",
                "BA",
            ),
            (
                "europe.mcr",
                "e2670d27d3e240314218587f763e0bb4c015f15a966fe634053f2a02a165645d",
                "SCES-00005",
                "BE",
            ),
            (
                "japan.mcr",
                "d0dba92f9a33ac91cdf9335a30d93f8eb3755cdd2f92795dcb9942287116b02f",
                "SLPS-00040",
                "BI",
            ),
            (
                "japan-rev1.mcr",
                "9e22e6a430e60ed40dfb15faafcabc4c7186cae0814953c4630eaf9420bca006",
                "SLPS-00040",
                "BI",
            ),
        ] {
            let bytes = std::fs::read(root.join(file)).unwrap();
            assert_eq!(format!("{:x}", Sha256::digest(&bytes)), sha, "{file}");
            let card = Ps1Card::open(&bytes, file, false).unwrap();
            let view = card.view();
            assert_eq!(view.saves.len(), 1, "{file}");
            assert_public(&view.saves[0], code, region);
            assert_eq!(card.save_raw(false), bytes);
        }

        let filled = std::fs::read(root.join("filled.mcr")).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&filled)),
            "b696b0a47725c2cfea58796e3e39357f93d7e52d7a94880dbc661536c627291e"
        );
        let card = Ps1Card::open(&filled, "filled.mcr", false).unwrap();
        let view = card.view();
        let tekken: Vec<_> = view
            .saves
            .iter()
            .filter(|save| save.prod_code == "SCES-00005" || save.prod_code == "SLPS-00040")
            .collect();
        assert_eq!(tekken.len(), 2);
        assert_public(tekken[0], "SCES-00005", "BE");
        assert_public(tekken[1], "SLPS-00040", "BI");
        assert_eq!(
            view.saves
                .iter()
                .filter(|save| save.game_details.is_some())
                .count(),
            2
        );
        assert_eq!(card.save_raw(false), filled);
    }

    fn assert_public(save: &crate::card::engine::SaveInfo, code: &str, region: &str) {
        assert_eq!(save.region_raw, region);
        assert_eq!(save.prod_code, code);
        assert_eq!(save.identifier, "<TEKKEN>");
        assert!(
            save.game_details_error.is_none(),
            "{:?}",
            save.game_details_error
        );
        let GameDetails::Summary(details) = save.game_details.as_ref().unwrap() else {
            panic!("summary");
        };
        assert_eq!(details.checksum_ok, None);
        assert_eq!(
            value(details, "Unlocked characters"),
            "Kazuya, Paul, Law, Jack, Nina, King, Yoshimitsu, Michelle"
        );
        assert_eq!(value(details, "Arcade completed"), "None recorded");
        assert_eq!(value(details, "Kazuya costume"), "Locked");
        assert_eq!(value(details, "Character totals"), "None recorded");
        const TIME: &[(&str, &str, &str, &str)] = &[
            ("Time 1st place", "50:00.000", "Kazuya", "PAS"),
            ("Time 2nd place", "51:00.000", "Paul", "ABE"),
            ("Time 3rd place", "52:00.000", "Law", "ARA"),
            ("Time 4th place", "53:00.000", "Jack", "TAK"),
            ("Time 5th place", "54:00.000", "Nina", "BES"),
            ("Time 6th place", "55:00.000", "King", "KUN"),
            ("Time 7th place", "56:00.000", "Yoshimitsu", "CHA"),
            ("Time 8th place", "57:00.000", "Michelle", "K.M"),
            ("Time 9th place", "58:00.000", "Kazuya", "SAM"),
            ("Time 10th place", "59:00.000", "Paul", "K.N"),
            ("Time 11th place", "60:00.000", "Law", "K.K"),
            ("Time 12th place", "61:00.000", "Jack", "KIN"),
            ("Time 13th place", "62:00.000", "Nina", "KMT"),
            ("Time 14th place", "63:00.000", "King", "NAK"),
            ("Time 15th place", "64:00.000", "Yoshimitsu", "KIM"),
            ("Time 16th place", "65:00.000", "Michelle", "PIX"),
        ];
        const VERSUS: &[(&str, &str, &str)] = &[
            ("2P 1st place", "4", "Michelle"),
            ("2P 2nd place", "4", "Yoshimitsu"),
            ("2P 3rd place", "4", "King"),
            ("2P 4th place", "4", "Nina"),
            ("2P 5th place", "3", "Jack"),
            ("2P 6th place", "3", "Law"),
            ("2P 7th place", "3", "Paul"),
            ("2P 8th place", "3", "Kazuya"),
            ("2P 9th place", "2", "Michelle"),
            ("2P 10th place", "2", "Yoshimitsu"),
            ("2P 11th place", "2", "King"),
            ("2P 12th place", "2", "Nina"),
            ("2P 13th place", "1", "Jack"),
            ("2P 14th place", "1", "Law"),
            ("2P 15th place", "1", "Paul"),
            ("2P 16th place", "1", "Kazuya"),
        ];
        assert_eq!(details.profiles[0].records.len(), TIME.len() + VERSUS.len());
        for (name, time, character, initials) in TIME {
            let record = details.profiles[0]
                .records
                .iter()
                .find(|record| record.name == *name)
                .unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(
                record
                    .fields
                    .iter()
                    .map(|field| field.value.as_str())
                    .collect::<Vec<_>>(),
                [*time, *character, "2", *initials]
            );
        }
        for (name, wins, character) in VERSUS {
            let record = details.profiles[0]
                .records
                .iter()
                .find(|record| record.name == *name)
                .unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(
                record
                    .fields
                    .iter()
                    .map(|field| field.value.as_str())
                    .collect::<Vec<_>>(),
                [*wins, *character]
            );
        }
    }
}
