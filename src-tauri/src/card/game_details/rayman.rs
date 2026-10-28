//! Rayman USA (`SLUS-00005`) read-only summary.
//!
//! Confirmed on a fresh native USA load, and stored the same way in the
//! GameFAQs sample: header `SC 13 01`, ASCII title prefix `Rayman`, three
//! filename bytes at `0xb`, three ASCII completion digits at `0xf`, continues
//! at `0x200`, and lives at `0x400`. Both saves use directory prefix `BI`
//! (`BISLUS-00005`). `BA` is accepted and was not observed.
//!
//! Editor-derived and not reliable: the 102 cage bits, power bits at `0x300` /
//! `0x301`, and current level at `0x1080`. They match the MIT editor
//! `template.ts` at RyudoSynbios/game-tools-collection commit
//! `c4c80f40fd0e102e2dac7257851f43167b2c350f` and were not confirmed on a
//! progressed native save. A GameFAQs file labeled 100% complete reads as
//! level 0, 0/102 cages and no powers on that map while other ranges are
//! populated, so those readings are not progress.
//!
//! Checked against that template (not copied from it):
//! - Its validator expects payload byte 0 to be the ASCII product code
//!   (`SLUS-00005`, and `SLES-00049` / `SLPS-00026` for other regions). Both
//!   real USA saves start with `SC 13 01`. The product code is the directory
//!   filename. Confirmed offsets share that SC origin, so the unconfirmed
//!   fields are not merely shifted by a missing product-code header.
//! - The template models three `0x2000`-byte slots. Observed USA saves are one
//!   block; this reader does not choose a bank.
//! - Its letter resource maps `0x20` to space and `0x61`..=`0x7a` to A..=Z.
//!   Only `0x61` → A was seen in-game (`aaa` shown as AAA). Stored bytes are
//!   shown as stored. The GameFAQs name is `smd`, which that map would display
//!   as SMD.
//! - Health (`0x480`, `0x409`) and tins (`0x406`) are not decoded.
//! - The template defines no checksum. None is validated.
//!
//! See `docs/formats/rayman-us.md`.

use super::{Field, Format, Profile, SaveDetails};

/// Editor level labels 0..=23. Not confirmed against a progressed save.
const LEVELS: [&str; 24] = [
    "Pink Plant Woods",
    "Anguish Lagoon",
    "The Swamps of Forgetfulness",
    "Moskito's Nest",
    "Bongo Hills",
    "Allegro Presto",
    "Gong Heights",
    "Mr Sax's Hullaballoo",
    "Twilight Gulch",
    "The Hard Rocks",
    "Mr Stone's Peaks",
    "Eraser Plains",
    "Pencil Pentathlon",
    "Space Mama's Crater",
    "Crystal Palace",
    "Eat At Joe's",
    "Mr Skops' Stalactites",
    "Mr Dark's Dare",
    "The Dream Forest (Save)",
    "Band Land (Save)",
    "Blue Mountains (Save)",
    "Picture City (Save)",
    "The Caves of Skops (Save)",
    "Candy Chateau (Save)",
];

/// `(level, offset, bit)` from the editor map. 17 levels × 6 bits = 102.
/// Not confirmed; the labeled 100% sample has every bit clear.
const CAGES: [(u8, u16, u8); 102] = [
    (0, 0x521, 6),
    (0, 0x529, 2),
    (0, 0x529, 4),
    (0, 0x529, 5),
    (0, 0x56a, 5),
    (0, 0x56a, 7),
    (1, 0x583, 0),
    (1, 0x584, 3),
    (1, 0x584, 5),
    (1, 0x584, 6),
    (1, 0x584, 7),
    (1, 0x5c3, 6),
    (2, 0x600, 4),
    (2, 0x600, 5),
    (2, 0x626, 1),
    (2, 0x627, 7),
    (2, 0x648, 1),
    (2, 0x648, 2),
    (3, 0x667, 2),
    (3, 0x667, 3),
    (3, 0x667, 6),
    (3, 0x689, 0),
    (3, 0x689, 1),
    (3, 0x689, 3),
    (4, 0x7aa, 0),
    (4, 0x7d3, 5),
    (4, 0x7e8, 3),
    (4, 0x810, 3),
    (4, 0x832, 0),
    (4, 0x832, 1),
    (5, 0x860, 2),
    (5, 0x860, 4),
    (5, 0x860, 5),
    (5, 0x88f, 7),
    (5, 0x8a1, 0),
    (5, 0x8a1, 2),
    (6, 0x900, 2),
    (6, 0x900, 4),
    (6, 0x900, 5),
    (6, 0x900, 7),
    (6, 0x920, 3),
    (6, 0x920, 6),
    (7, 0x954, 2),
    (7, 0x954, 3),
    (7, 0x954, 4),
    (7, 0x954, 5),
    (7, 0x954, 6),
    (7, 0x954, 7),
    (8, 0x9ea, 1),
    (8, 0x9ea, 2),
    (8, 0x9eb, 4),
    (8, 0x9eb, 5),
    (8, 0x9eb, 6),
    (8, 0x9eb, 7),
    (9, 0xa37, 6),
    (9, 0xa44, 5),
    (9, 0xa44, 6),
    (9, 0xa69, 0),
    (9, 0xa69, 1),
    (9, 0xa6a, 6),
    (10, 0xa90, 0),
    (10, 0xa90, 1),
    (10, 0xaa5, 7),
    (10, 0xac6, 1),
    (10, 0xaf3, 0),
    (10, 0xaf4, 7),
    (11, 0xb93, 3),
    (11, 0xb93, 4),
    (11, 0xbb2, 3),
    (11, 0xbcf, 1),
    (11, 0xbcf, 2),
    (11, 0xbd0, 7),
    (12, 0xc17, 0),
    (12, 0xc18, 7),
    (12, 0xc2a, 0),
    (12, 0xc2a, 2),
    (12, 0xc4b, 1),
    (12, 0xc4c, 7),
    (13, 0xc68, 2),
    (13, 0xc68, 3),
    (13, 0xc81, 0),
    (13, 0xc81, 2),
    (13, 0xcb3, 4),
    (13, 0xcb3, 5),
    (14, 0xd2c, 1),
    (14, 0xd2c, 4),
    (14, 0xd2c, 5),
    (14, 0xd4b, 0),
    (14, 0xd4c, 6),
    (14, 0xd4c, 7),
    (15, 0xd87, 3),
    (15, 0xdb1, 1),
    (15, 0xdb1, 3),
    (15, 0xdb1, 4),
    (15, 0xde0, 6),
    (15, 0xe07, 4),
    (16, 0xe20, 1),
    (16, 0xe20, 2),
    (16, 0xe20, 3),
    (16, 0xe20, 4),
    (16, 0xe20, 5),
    (16, 0xe20, 6),
];

const NAMED_POWERS: [(&str, usize, u8); 5] = [
    ("Punch", 0x300, 0),
    ("Hang", 0x300, 1),
    ("Helicopter", 0x300, 2),
    ("Grab", 0x300, 7),
    ("Run", 0x301, 0),
];

const NOTE_CONFIRMED: &str = concat!(
    "Filename, completion text, continues and lives were checked on a fresh native USA load ",
    "(stored aaa, menu AAA, 000%, continues 5, lives 3). Nonzero completion was not seen in-game. ",
    "Directory prefix BI is confirmed on that save and on a second USA save (BISLUS-00005smd56832)."
);

const NOTE_UNCONFIRMED: &str = concat!(
    "Cages (102 bits), power bits at 0x300/0x301 and current level at 0x1080 are editor-derived ",
    "and were not confirmed on a progressed native save. A GameFAQs file labeled 100% complete ",
    "reads as level 0, 0/102 cages and no powers on that map, while other ranges in the same file ",
    "are populated. Those readings are not reliable progress."
);

const NOTE_CHECKSUM: &str = "No game checksum is known, so none is validated.";

const NOTE_BA: &str = concat!(
    "Directory prefix BA is accepted but was not observed on the USA disc or the GameFAQs sample. ",
    "Both of those saves use BI."
);

fn field(label: &str, value: impl Into<String>) -> Field {
    Field {
        label: label.into(),
        value: value.into(),
    }
}

fn filename(data: &[u8]) -> String {
    let raw = &data[0x0b..0x0e];
    if raw.iter().all(|b| (0x20..0x7f).contains(b)) {
        String::from_utf8_lossy(raw).into_owned()
    } else {
        raw.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Three ASCII digits at `0xf`, only when they encode 0..=100.
/// `000%` is the only value checked in-game. Other encodings stay raw.
fn completion(data: &[u8]) -> String {
    let raw = &data[0x0f..0x12];
    if let Ok(text) = std::str::from_utf8(raw) {
        if text.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(value) = text.parse::<u16>() {
                if value <= 100 {
                    return format!("{text}%");
                }
            }
        }
    }
    format!(
        "Unknown ({})",
        raw.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

fn current_level(data: &[u8]) -> String {
    let id = data[0x1080];
    match LEVELS.get(usize::from(id)) {
        Some(name) => format!("editor: {name} (raw {id})"),
        None => format!("editor: unknown (raw {id})"),
    }
}

fn cages(data: &[u8]) -> String {
    let freed = CAGES
        .iter()
        .filter(|(_, offset, bit)| data[usize::from(*offset)] & (1 << bit) != 0)
        .count();
    format!("editor: {freed}/102")
}

fn powers(data: &[u8]) -> String {
    let named: Vec<&str> = NAMED_POWERS
        .iter()
        .filter(|(_, offset, bit)| data[*offset] & (1 << bit) != 0)
        .map(|(label, _, _)| *label)
        .collect();
    let list = if named.is_empty() {
        "none".to_string()
    } else {
        named.join(", ")
    };
    let unknown_300 = data[0x300] & 0x78;
    let unknown_301 = data[0x301] & 0xfe;
    let raw = format!("{:02x}{:02x}", data[0x300], data[0x301]);
    if unknown_300 == 0 && unknown_301 == 0 {
        format!("editor: {list}; raw {raw}")
    } else {
        format!(
            "editor: {list}; raw {raw}; unknown bits 0x300=0x{unknown_300:02x}, 0x301=0x{unknown_301:02x}"
        )
    }
}

pub(super) fn decode(
    fmt: &Format,
    code: &str,
    region: &str,
    data: &[u8],
) -> Result<SaveDetails, String> {
    // Dispatch already checked the product, BI/BA prefix, one SC block and size.
    if data[2] != 0x13 || data.get(4..10) != Some(b"Rayman".as_slice()) {
        return Err("Unsupported Rayman US title or layout revision.".into());
    }
    let mut notes = vec![
        NOTE_CONFIRMED.into(),
        NOTE_UNCONFIRMED.into(),
        NOTE_CHECKSUM.into(),
    ];
    if region == "BA" {
        notes.push(NOTE_BA.into());
    }
    // The editor UI clamps these at 99. That limit was not checked in-game.
    if data[0x400] > 99 {
        notes.push("Lives byte is above the editor UI limit of 99. The raw byte is shown.".into());
    }
    if data[0x200] > 99 {
        notes.push(
            "Continues byte is above the editor UI limit of 99. The raw byte is shown.".into(),
        );
    }
    Ok(SaveDetails {
        game: fmt.slug.into(),
        title: fmt.title.into(),
        release: format!("NTSC-U · {code}"),
        checksum_ok: None,
        profiles: vec![Profile {
            number: 1,
            name: filename(data),
            empty: false,
            status: None,
            checksum_ok: None,
            fields: vec![
                field("Filename", filename(data)),
                field("Completion", completion(data)),
                field("Lives", data[0x400].to_string()),
                field("Continues", data[0x200].to_string()),
                field("Current level (unconfirmed)", current_level(data)),
                field("Cages (unconfirmed)", cages(data)),
                field("Powers (unconfirmed)", powers(data)),
            ],
            records: vec![],
        }],
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::CAGES;
    use crate::card::engine::{Ps1Card, BLOCK_SIZE, GME_HEADER_SIZE, MCS_HEADER_SIZE};
    use crate::card::game_details::{self, GameDetails};
    use sha2::{Digest, Sha256};

    const GAMEFAQS_GME: &[u8] = include_bytes!("testdata/rayman-gamefaqs-100.gme");
    const GAMEFAQS_SHA256: &str =
        "b5e0ddabdf2a7ff9dc031a1922a14c9122ca91f39329c43f6a326e7b71132e78";

    /// Same blank slot as the decoder kit's `make_suite.py`.
    fn slot() -> Vec<u8> {
        let mut data = vec![0u8; 8192];
        data[..10].copy_from_slice(b"SC\x13\x01Rayman");
        data[15..18].copy_from_slice(b"00\0");
        data
    }

    fn sha256(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn summary(region: &str, id: &str, data: &[u8]) -> game_details::SaveDetails {
        match game_details::decode("SLUS-00005", region, id, data).unwrap() {
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

    #[test]
    fn kit_synthetic_slots_match_published_hashes_and_confirmed_fields() {
        let zero = slot();
        assert_eq!(
            sha256(&zero),
            "7ccad2fac3b8a31eddbc1fa8fc12d7792ca9d3940db9d6dc41e3517b46285629"
        );
        let before = zero.clone();
        let details = summary("BI", "aaa63592", &zero);
        assert_eq!(zero, before);
        assert_eq!(details.checksum_ok, None);
        assert!(details.profiles[0].checksum_ok.is_none());
        assert!(details.profiles[0].records.is_empty());
        assert_eq!(
            pairs(&details),
            vec![
                ("Filename", "000000"),
                ("Completion", "Unknown (303000)"),
                ("Lives", "0"),
                ("Continues", "0"),
                (
                    "Current level (unconfirmed)",
                    "editor: Pink Plant Woods (raw 0)"
                ),
                ("Cages (unconfirmed)", "editor: 0/102"),
                ("Powers (unconfirmed)", "editor: none; raw 0000"),
            ]
        );
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("not reliable")));
        assert!(!details.notes.iter().any(|note| note.contains("prefix BA")));

        let mut upper = slot();
        upper[15..18].copy_from_slice(b"100");
        upper[0x400] = 99;
        upper[0x200] = 99;
        upper[0x1080] = 23;
        upper[0x300] = 0xff;
        upper[0x301] = 0xff;
        assert_eq!(
            sha256(&upper),
            "5ea8cd87a5b2be6ed1a01d16c2ca3e41811b40f7fe442f66dd832e72913e78a4"
        );
        let details = summary("BA", "", &upper);
        assert_eq!(
            pairs(&details),
            vec![
                ("Filename", "000000"),
                ("Completion", "100%"),
                ("Lives", "99"),
                ("Continues", "99"),
                (
                    "Current level (unconfirmed)",
                    "editor: Candy Chateau (Save) (raw 23)"
                ),
                ("Cages (unconfirmed)", "editor: 0/102"),
                (
                    "Powers (unconfirmed)",
                    "editor: Punch, Hang, Helicopter, Grab, Run; raw ffff; unknown bits 0x300=0x78, 0x301=0xfe"
                ),
            ]
        );
        assert!(details.notes.iter().any(|note| note.contains("prefix BA")));

        let mut unknown = slot();
        unknown[0x400] = 255;
        unknown[0x200] = 100;
        unknown[0x1080] = 255;
        unknown[15..18].copy_from_slice(b"\xff?%");
        assert_eq!(
            sha256(&unknown),
            "c35a340adc899090c7112ad8eef476b3e04a72b92c58aa4368d803af2c15e944"
        );
        let details = summary("BI", "aaa63592", &unknown);
        assert_eq!(pairs(&details)[1].1, "Unknown (ff3f25)");
        assert_eq!(pairs(&details)[2].1, "255");
        assert_eq!(pairs(&details)[3].1, "100");
        assert_eq!(pairs(&details)[4].1, "editor: unknown (raw 255)");
        assert!(details.notes.iter().any(|note| note.contains("Lives byte")));
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("Continues byte")));

        for (text, expected) in [
            (b"000".as_slice(), "000%"),
            (b"001", "001%"),
            (b"099", "099%"),
            (b"100", "100%"),
            (b"101", "Unknown (313031)"),
            (b" 01", "Unknown (203031)"),
            (b"1\x000", "Unknown (310030)"),
        ] {
            let mut data = slot();
            data[15..18].copy_from_slice(text);
            assert_eq!(pairs(&summary("BI", "aaa63592", &data))[1].1, expected);
        }

        let mut named = slot();
        named[0x0b..0x0e].copy_from_slice(b"aaa");
        named[0x0f..0x12].copy_from_slice(b"000");
        named[0x200] = 5;
        named[0x400] = 3;
        let details = summary("BI", "aaa63592", &named);
        assert_eq!(pairs(&details)[0].1, "aaa");
        assert_eq!(pairs(&details)[1].1, "000%");
        assert_eq!(pairs(&details)[2].1, "3");
        assert_eq!(pairs(&details)[3].1, "5");
        assert_eq!(details.checksum_ok, None);
    }

    #[test]
    fn editor_cage_and_power_bits_change_the_unconfirmed_reading_only() {
        assert_eq!(CAGES.len(), 102);
        for (index, (_, offset, bit)) in CAGES.iter().enumerate() {
            let mut data = slot();
            data[usize::from(*offset)] |= 1 << bit;
            let details = summary("BI", "cage", &data);
            assert_eq!(pairs(&details)[5].1, "editor: 1/102", "cage {index}");
            assert_eq!(pairs(&details)[1].1, "Unknown (303000)");
            assert_eq!(details.checksum_ok, None);
            if index == 0 {
                assert_eq!(
                    sha256(&data),
                    "d3be188c683c1c117e115d270a6fba5013def08b6d181b566fa9e1a3798df457"
                );
            }
        }
        let mut shared = slot();
        shared[0x529] |= (1 << 2) | (1 << 4) | (1 << 5);
        assert_eq!(pairs(&summary("BI", "cage", &shared))[5].1, "editor: 3/102");

        let mut all = slot();
        for (_, offset, bit) in CAGES {
            all[usize::from(offset)] |= 1 << bit;
        }
        assert_eq!(
            sha256(&all),
            "3a83213f77c542c80c52db347d445d100a12f2e223c315d92c9863295766568d"
        );
        assert_eq!(pairs(&summary("BI", "cage", &all))[5].1, "editor: 102/102");

        let mut punch = slot();
        punch[0x300] |= 1;
        assert_eq!(
            sha256(&punch),
            "796841579bdb83ce9677bde083f5d8939414aa8cdae90510f3e2b46cc9c622f9"
        );
        assert_eq!(
            pairs(&summary("BI", "power", &punch))[6].1,
            "editor: Punch; raw 0100"
        );
        let mut grab = slot();
        grab[0x300] |= 1 << 7;
        assert_eq!(
            sha256(&grab),
            "26103834d0047279c2697fb54c3c66ff88fa14803c59426bfe8daaecf8986448"
        );
        assert_eq!(
            pairs(&summary("BI", "power", &grab))[6].1,
            "editor: Grab; raw 8000"
        );
        let mut run = slot();
        run[0x301] |= 1;
        assert_eq!(
            sha256(&run),
            "14123f98259aae76c61bdc01b0610c7a716b9dbb9cb3c6cc88e05e391f4322d3"
        );
        assert_eq!(
            pairs(&summary("BI", "power", &run))[6].1,
            "editor: Run; raw 0001"
        );
        let mut unknown_bit = slot();
        unknown_bit[0x300] |= 1 << 3;
        assert_eq!(
            pairs(&summary("BI", "power", &unknown_bit))[6].1,
            "editor: none; raw 0800; unknown bits 0x300=0x08, 0x301=0x00"
        );
    }

    #[test]
    fn rejects_other_regions_products_headers_and_sizes() {
        let data = slot();
        let format = game_details::format("SLUS-00005").unwrap();
        assert_eq!(format.region, "BI");
        assert_eq!(format.blocks, 1);
        assert!(format.accepts_identifier("aaa63592"));
        assert!(format.accepts_identifier("smd56832"));
        assert!(format.accepts_identifier(""));
        assert!(game_details::format("SLES-00049").is_none());
        assert!(game_details::format("SLPS-00026").is_none());
        assert!(game_details::decode("SLUS-00005", "BE", "aaa63592", &data).is_err());
        assert!(game_details::decode("SLUS-00005", "BA", "TOMBRAID", &data).is_ok());

        let mut header = data.clone();
        header[..10].copy_from_slice(b"SLES-00049");
        assert!(game_details::decode("SLUS-00005", "BI", "aaa63592", &header).is_err());
        for offset in [2usize, 4] {
            let mut bad = data.clone();
            bad[offset] ^= 1;
            assert!(game_details::decode("SLUS-00005", "BI", "aaa63592", &bad).is_err());
        }
        for length in [0usize, 10, 8191, 8193, 16384] {
            let bytes = if length <= data.len() {
                data[..length].to_vec()
            } else {
                vec![0u8; length]
            };
            assert!(
                game_details::decode("SLUS-00005", "BI", "aaa63592", &bytes).is_err(),
                "{length}"
            );
        }
    }

    #[test]
    fn gamefaqs_trimmed_gme_parses_header_name_and_product_code() {
        assert_eq!(sha256(GAMEFAQS_GME), GAMEFAQS_SHA256);
        assert!(GAMEFAQS_GME.starts_with(b"123-456-STD"));
        assert_eq!(
            GAMEFAQS_GME.len(),
            GME_HEADER_SIZE + BLOCK_SIZE + BLOCK_SIZE
        );
        let save_at = GME_HEADER_SIZE + BLOCK_SIZE;
        assert_eq!(save_at, 0x2f40);
        let directory = &GAMEFAQS_GME[GME_HEADER_SIZE + 128..GME_HEADER_SIZE + 256];
        assert_eq!(directory[0], 0x51);
        assert_eq!(
            u32::from_le_bytes(directory[4..8].try_into().unwrap()),
            8192
        );
        assert_eq!(&directory[8..10], &[0xff, 0xff]);
        assert_eq!(&directory[10..30], b"BISLUS-00005smd56832");
        let payload = &GAMEFAQS_GME[save_at..save_at + BLOCK_SIZE];
        assert_eq!(&payload[..4], b"SC\x13\x01");
        assert_eq!(&payload[4..10], b"Rayman");
        assert_eq!(&payload[0x0b..0x0e], b"smd");
        assert_eq!(&payload[0x0f..0x12], b"000");
        // Populated bytes the editor map does not explain. Do not treat 0x2D0 as completion.
        assert_eq!(payload[0x2d0], 0x64);
        assert_eq!(payload[0x200], 5);
        assert_eq!(payload[0x400], 3);
        assert_eq!(payload[0x1080], 0);
        assert_eq!(&payload[0x300..0x302], &[0, 0]);

        let before = payload.to_vec();
        let details = summary("BI", "smd56832", payload);
        assert_eq!(payload, &before[..]);
        assert_eq!(details.game, "rayman");
        assert_eq!(details.title, "Rayman");
        assert_eq!(details.release, "NTSC-U · SLUS-00005");
        assert_eq!(details.checksum_ok, None);
        assert_eq!(details.profiles[0].name, "smd");
        assert!(details.profiles[0].records.is_empty());
        assert_eq!(
            pairs(&details),
            vec![
                ("Filename", "smd"),
                ("Completion", "000%"),
                ("Lives", "3"),
                ("Continues", "5"),
                (
                    "Current level (unconfirmed)",
                    "editor: Pink Plant Woods (raw 0)"
                ),
                ("Cages (unconfirmed)", "editor: 0/102"),
                ("Powers (unconfirmed)", "editor: none; raw 0000"),
            ]
        );
        let confirmed = ["Filename", "Completion", "Lives", "Continues"];
        assert!(details.profiles[0]
            .fields
            .iter()
            .filter(|field| confirmed.contains(&field.label.as_str()))
            .all(|field| !field.label.contains("unconfirmed") && !field.value.contains("editor:")));
        assert!(details.notes.iter().any(|note| {
            note.contains("labeled 100% complete") && note.contains("not reliable progress")
        }));
        assert!(details
            .notes
            .iter()
            .any(|note| note.contains("BISLUS-00005smd56832")));
        assert!(!details.notes.iter().any(|note| note.contains("prefix BA")));

        let mut mcs = directory.to_vec();
        mcs.extend_from_slice(payload);
        assert_eq!(mcs.len(), MCS_HEADER_SIZE + BLOCK_SIZE);
        let mut card = Ps1Card::create_formatted("rayman-gamefaqs-100.gme");
        card.set_save_bytes(0, &mcs).unwrap();
        let raw = card.save_raw(false);
        let view = card.view();
        assert_eq!(view.saves.len(), 1);
        let save = &view.saves[0];
        assert_eq!(save.region_raw, "BI");
        assert_eq!(save.prod_code, "SLUS-00005");
        assert_eq!(save.identifier, "smd56832");
        assert!(
            save.game_details_error.is_none(),
            "{:?}",
            save.game_details_error
        );
        let GameDetails::Summary(card_details) = save.game_details.as_ref().unwrap() else {
            panic!("expected summary");
        };
        assert_eq!(pairs(card_details), pairs(&details));
        let reopened = Ps1Card::open(&raw, "rayman-gamefaqs-100.gme", false).unwrap();
        assert_eq!(reopened.view().saves[0].identifier, "smd56832");
        let reopened_raw = reopened.save_raw(false);
        assert_eq!(&reopened_raw[138..158], b"BISLUS-00005smd56832");
    }

    #[test]
    fn card_view_distinguishes_saves_and_rejects_a_second_block() {
        fn mcs(id: &str, payload: &[u8]) -> Vec<u8> {
            let mut bytes = vec![0u8; MCS_HEADER_SIZE + payload.len()];
            bytes[0] = 0x51;
            bytes[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes[8..10].copy_from_slice(&0xffffu16.to_le_bytes());
            let name = format!("BISLUS-00005{id}");
            bytes[10..10 + name.len()].copy_from_slice(name.as_bytes());
            bytes[MCS_HEADER_SIZE..].copy_from_slice(payload);
            bytes
        }
        let mut aaa = slot();
        aaa[0x0b..0x0e].copy_from_slice(b"aaa");
        aaa[0x0f..0x12].copy_from_slice(b"000");
        aaa[0x200] = 5;
        aaa[0x400] = 3;
        let mut smd = slot();
        smd[0x0b..0x0e].copy_from_slice(b"smd");
        smd[0x0f..0x12].copy_from_slice(b"050");
        smd[0x200] = 1;
        smd[0x400] = 4;
        let mut card = Ps1Card::create_formatted("two-saves");
        card.set_save_bytes(0, &mcs("aaa63592", &aaa)).unwrap();
        card.set_save_bytes(0, &mcs("smd56832", &smd)).unwrap();
        let original = card.save_raw(false);
        let view = card.view();
        assert_eq!(view.saves.len(), 2);
        let progress: Vec<_> = view
            .saves
            .iter()
            .map(|save| {
                let GameDetails::Summary(details) = save.game_details.as_ref().unwrap() else {
                    panic!("{:?}", save.game_details_error);
                };
                (
                    save.identifier.as_str(),
                    pairs(details)[0].1.to_string(),
                    pairs(details)[1].1.to_string(),
                    pairs(details)[2].1.to_string(),
                    pairs(details)[3].1.to_string(),
                )
            })
            .collect();
        assert_eq!(
            progress,
            vec![
                (
                    "aaa63592",
                    "aaa".to_string(),
                    "000%".to_string(),
                    "3".to_string(),
                    "5".to_string(),
                ),
                (
                    "smd56832",
                    "smd".to_string(),
                    "050%".to_string(),
                    "4".to_string(),
                    "1".to_string(),
                ),
            ]
        );
        assert_eq!(card.save_raw(false), original);

        let mut linked = mcs("aaa63592", &aaa);
        linked.extend_from_slice(&aaa);
        let mut card = Ps1Card::create_formatted("linked");
        card.set_save_bytes(0, &linked).unwrap();
        let view = card.view();
        assert!(view.saves[0].game_details.is_none());
        assert!(view.saves[0].game_details_error.is_some());
    }
}
