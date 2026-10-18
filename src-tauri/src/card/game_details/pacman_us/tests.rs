use super::super::{decode, GameDetails, SaveDetails};
use crate::card::engine::{CardFormat, Ps1Card, BLOCK_SIZE, HEADER_SIZE, MCS_HEADER_SIZE};
use sha2::{Digest, Sha256};

fn put_i32(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn payload() -> Vec<u8> {
    let mut data = vec![0; BLOCK_SIZE];
    data[..4].copy_from_slice(&[0x43, 0, 0x13, 1]);
    let title = b"PAC-MAN WORLD 20TH ANNIV.";
    data[4..4 + title.len()].copy_from_slice(title);
    data[0x200..0x204].copy_from_slice(&0x784u32.to_le_bytes());
    put_u16(&mut data, 0x208, 2);
    for slot in 0..3 {
        put_i32(&mut data, 0x20c + slot * 0x280 + 0xc, -1);
    }
    data
}

fn summary(data: &[u8]) -> SaveDetails {
    match decode("SLUS-00439", "BA", "PACMAN20", data).unwrap() {
        GameDetails::Summary(summary) => summary,
        _ => panic!("Expected summary"),
    }
}

fn value(summary: &SaveDetails, slot: usize, label: &str) -> String {
    summary.profiles[slot]
        .fields
        .iter()
        .find(|f| f.label == label)
        .unwrap()
        .value
        .clone()
}

fn assert_profile(summary: &SaveDetails, slot: usize, stages: usize, friends: usize, time: &str) {
    assert!(!summary.profiles[slot].empty);
    assert_eq!(summary.profiles[slot].number, (slot + 1) as u8);
    assert_eq!(
        summary.profiles[slot].name,
        format!("Game {} of 3", slot + 1)
    );
    assert_eq!(
        value(summary, slot, "Quest completion"),
        format!("{stages} of 23 quest stages cleared")
    );
    assert_eq!(
        value(summary, slot, "Friends rescued"),
        format!("{friends} of 6")
    );
    assert_eq!(value(summary, slot, "Playtime (hh:mm:ss)"), time);
    assert_eq!(summary.profiles[slot].fields.len(), 3);
    assert!(summary.profiles[slot].records.is_empty());
    assert_eq!(summary.profiles[slot].checksum_ok, None);
}

fn original_values() -> Vec<u8> {
    // Constructed representation of independently observed/code-traced values.
    // Real save bytes remain private and are exercised by the ignored corpus test.
    let mut data = payload();
    put_i32(&mut data, 0x218, 2);
    for id in [236, 230, 232, 234, 222] {
        put_u16(&mut data, 0x20c + 0x54 + 2 * id, 3);
    }
    put_u16(&mut data, 0x20c + 0x5a, 2);
    put_i32(&mut data, 0x20c + 0x24c, (55 * 60 + 9) * 300);
    data
}

fn mcs(data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; MCS_HEADER_SIZE];
    bytes[0] = 0x51;
    bytes[4..8].copy_from_slice(&(BLOCK_SIZE as u32).to_le_bytes());
    bytes[8..10].fill(0xff);
    bytes[10..30].copy_from_slice(b"BASLUS-00439PACMAN20");
    bytes[127] = bytes[..127].iter().fold(0, |a, b| a ^ b);
    bytes.extend_from_slice(data);
    bytes
}

#[test]
fn pacman_us_known_values_and_only_supported_fields() {
    let data = original_values();
    let before = data.clone();
    let s = summary(&data);
    assert_eq!(s.game, "pac-man-world");
    assert_eq!(s.release, "NTSC-U · SLUS-00439");
    assert_eq!(s.checksum_ok, None);
    assert_eq!(s.profiles.len(), 3);
    assert_profile(&s, 0, 5, 1, "0:55:09");
    for slot in [1, 2] {
        assert!(s.profiles[slot].empty);
        assert!(s.profiles[slot].fields.is_empty());
        assert_eq!(s.profiles[slot].number, (slot + 1) as u8);
    }
    let dto = serde_json::to_value(&s).unwrap();
    assert!(dto["checksumOk"].is_null());
    let labels: Vec<_> = dto["profiles"][0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["label"].as_str().unwrap())
        .collect();
    assert_eq!(
        labels,
        ["Quest completion", "Friends rescued", "Playtime (hh:mm:ss)"]
    );
    assert_eq!(data, before);
}

#[test]
fn pacman_us_fixed_profiles_ignore_selected_slot_and_stale_empty_data() {
    let mut data = payload();
    put_u16(&mut data, 0x20a, 2); // Last-selected slot does not hide other profiles.
    data[0x20c..0x48c].fill(0xff);
    put_i32(&mut data, 0x218, i32::MIN);
    for (slot, occupancy) in [(1, 0), (2, i32::MAX)] {
        put_i32(&mut data, 0x20c + slot * 0x280 + 0xc, occupancy);
    }
    let s = summary(&data);
    assert!(s.profiles[0].empty);
    assert!(s.profiles[0].fields.is_empty());
    assert_profile(&s, 1, 0, 0, "0:00:00");
    assert_profile(&s, 2, 0, 0, "0:00:00");
}

#[test]
fn pacman_us_stage_table_masks_and_friend_bit_boundaries() {
    let mut data = payload();
    put_i32(&mut data, 0x218, 0);
    // First/last quest entries count low-two-bit state 3, even with upper bits.
    for (id, state) in [
        (236, 0xfff3),
        (238, 3),
        (230, 2),
        (232, 1),
        (234, 0),
        (235, 3),
        (237, 3),
        (239, 3),
    ] {
        put_u16(&mut data, 0x20c + 0x54 + id * 2, state);
    }
    for (offset, bits) in [
        (0x58, 2),
        (0x5a, 0xfff2),
        (0x5c, 1),
        (0x5e, 4),
        (0x64, 2),
        (0x66, 2),
    ] {
        put_u16(&mut data, 0x20c + offset, bits);
    }
    assert_profile(&summary(&data), 0, 2, 2, "0:00:00");
}

#[test]
fn pacman_us_timer_boundaries_and_negative_values() {
    let mut data = payload();
    put_i32(&mut data, 0x218, 0);
    for (ticks, expected) in [
        (0, "0:00:00"),
        (299, "0:00:00"),
        (300, "0:00:01"),
        (17_999, "0:00:59"),
        (18_000, "0:01:00"),
        (1_079_999, "0:59:59"),
        (1_080_000, "1:00:00"),
        (i32::MAX, "1988:24:38"),
        (-1, "Unknown"),
        (i32::MIN, "Unknown"),
    ] {
        put_i32(&mut data, 0x20c + 0x24c, ticks);
        assert_eq!(value(&summary(&data), 0, "Playtime (hh:mm:ss)"), expected);
    }
}

#[test]
fn pacman_us_rejects_unsupported_layouts_and_keeps_pal_header_gate() {
    let data = original_values();
    for (code, region, id) in [
        ("SCES-00934", "BE", "PACMAN20"),
        ("SLUS-00439", "BE", "PACMAN20"),
        ("SLUS-00439", "BA", "PACMAN21"),
        ("SLUS-00439", "BA", "PACMAN200"),
        ("SLUS-00440", "BA", "PACMAN20"),
    ] {
        assert!(decode(code, region, id, &data).is_err());
    }
    for size in [0, 3, BLOCK_SIZE - 1, BLOCK_SIZE + 1, 2 * BLOCK_SIZE] {
        let mut bad = data.clone();
        bad.resize(size, 0);
        assert!(decode("SLUS-00439", "BA", "PACMAN20", &bad).is_err());
    }
    for header in [
        [b'S', b'C', 0x13, 1],
        [b'S', b'C', 0x11, 1],
        [0x43, 0, 0x13, 2],
        [0x43, 0, 0x12, 1],
        [0x43, 1, 0x13, 1],
    ] {
        let mut bad = data.clone();
        bad[..4].copy_from_slice(&header);
        assert!(decode("SLUS-00439", "BA", "PACMAN20", &bad).is_err());
    }
    for (offset, bytes) in [
        (0x200, 0x785u32.to_le_bytes().to_vec()),
        (0x208, 3u16.to_le_bytes().to_vec()),
    ] {
        let mut bad = data.clone();
        bad[offset..offset + bytes.len()].copy_from_slice(&bytes);
        assert!(decode("SLUS-00439", "BA", "PACMAN20", &bad).is_err());
    }
    let mut pal = data;
    pal[..4].copy_from_slice(&[b'S', b'C', 0x11, 1]);
    assert!(decode("SCES-00934", "BE", "PACMAN20", &pal).is_ok());
}

#[test]
fn pacman_us_card_containers_mcs_reimport_and_chain_rejection() {
    let bytes = mcs(&original_values());
    let before = bytes.clone();
    let mut card = Ps1Card::create_formatted("pacman-us");
    card.set_save_bytes(7, &bytes).unwrap();
    let raw = card.save_raw(false);
    let dto = serde_json::to_value(&card.view().saves[0].game_details).unwrap();
    assert_eq!(
        dto["profiles"][0]["fields"][0]["value"],
        "5 of 23 quest stages cleared"
    );
    for format in [CardFormat::Raw, CardFormat::Gme, CardFormat::Vgs] {
        let encoded = card.export(format, false);
        let opened = Ps1Card::open(&encoded, "pacman-us", false).unwrap();
        let view = opened.view();
        assert_eq!(view.slots.len(), 15);
        assert_eq!(view.saves[0].linked_slots, [7]);
        assert!(view.saves[0].game_details_error.is_none());
        assert_eq!(
            serde_json::to_value(&view.saves[0].game_details).unwrap(),
            dto
        );
        assert_eq!(opened.save_raw(false), raw);
        let mut imported = Ps1Card::create_formatted("reimport");
        imported
            .set_save_bytes(2, &opened.get_save_bytes(7))
            .unwrap();
        assert_eq!(
            serde_json::to_value(&imported.view().saves[0].game_details).unwrap(),
            dto
        );
    }
    // Corrupt either pointer byte or allocation, preserving directory XOR.
    for (offset, byte) in [(8, 0), (9, 0), (5, 0x40), (7, 1)] {
        let mut bad = raw.clone();
        let header = &mut bad[8 * HEADER_SIZE..9 * HEADER_SIZE];
        header[offset] = byte;
        header[127] = header[..127].iter().fold(0, |a, b| a ^ b);
        let opened = Ps1Card::open(&bad, "broken-chain", false).unwrap();
        let view = opened.view();
        assert_eq!(view.saves[0].prod_code, "SLUS-00439");
        assert!(view.saves[0].game_details.is_none());
        assert!(view.saves[0]
            .game_details_error
            .as_ref()
            .unwrap()
            .contains("block chain"));
        assert_eq!(opened.save_raw(false), bad);
    }
    assert_eq!(card.save_raw(false), raw);
    assert_eq!(bytes, before);
}

#[test]
#[ignore = "requires private review-d991f863 outputs; see docs/formats/pac-man-world-us.md"]
fn pacman_us_private_known_value_corpus() {
    let root = std::env::var_os("PS1_PACMAN_US_REVIEW")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../tools/save_research/runs/review-d991f863")
        });
    for (relative, hash, expected) in [
        ("outputs/final/recovery/inputs/cda613a6e26c631ad826f185477ebc2271d536444166628f378368f8afcd7894.mcs",
         "cda613a6e26c631ad826f185477ebc2271d536444166628f378368f8afcd7894",
         vec![(5, 1, "0:55:09")]),
        ("outputs/final/recovery/fixtures/overnight-20261012T213658Z/SLUS-00439/public-wave3prime15-100percent.mcs",
         "f3cc141a4b75897a913cc76d7c611520a52e05f05e6fcea8c02b47ece2dfa374",
         vec![(22, 6, "12:13:22"), (1, 0, "0:01:37"), (1, 0, "0:01:39")]),
    ] {
        let path = root.join(relative);
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), hash);
        let s = summary(&bytes[MCS_HEADER_SIZE..]);
        for (slot, &(stages, friends, time)) in expected.iter().enumerate() {
            assert_profile(&s, slot, stages, friends, time);
        }
        for slot in expected.len()..3 {
            assert!(s.profiles[slot].empty);
            assert!(s.profiles[slot].fields.is_empty());
        }
        let mut card = Ps1Card::create_formatted("private-pacman");
        card.set_save_bytes(7, &bytes).unwrap();
        for format in [CardFormat::Raw, CardFormat::Gme, CardFormat::Vgs] {
            let encoded = card.export(format, false);
            let opened = Ps1Card::open(&encoded, "private-pacman", false).unwrap();
            assert_eq!(serde_json::to_value(&opened.view().saves[0].game_details).unwrap(),
                       serde_json::to_value(&s).unwrap());
        }
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
