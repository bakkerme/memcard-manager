use super::*;
use crate::card::engine::{CardFormat, Ps1Card, BLOCK_SIZE, HEADER_SIZE, MCS_HEADER_SIZE};
use sha2::{Digest, Sha256};

fn repair(data: &mut [u8], offset: usize, size: usize) {
    data[offset + size] = data[offset..offset + size].iter().fold(0, |a, b| a ^ b);
    data[offset + size + 1] = data[offset..offset + size]
        .iter()
        .fold(0u8, |a, b| a.wrapping_add(*b));
}
fn checks(data: &mut [u8]) {
    repair(data, 0x200, 0xb04);
    for i in 0..50 {
        repair(data, 0xd08 + i * 0x44, 0x40);
    }
}
fn payload() -> Vec<u8> {
    let mut d = vec![0; BLOCK_SIZE];
    d[..4].copy_from_slice(b"SC\x11\x01");
    d[0xd03] = 5;
    d[0xd08] = 0xff;
    d[0xd09..0xd0e].copy_from_slice(&[0x0b, 0x2f, 0x25, 0x49, 0x49]); // Blb
    d[0xd0e] = 1;
    checks(&mut d);
    d
}
fn summary(d: &[u8]) -> SaveDetails {
    match super::super::decode("SLUS-00823", "BA", "SYSTEM", d).unwrap() {
        super::super::GameDetails::Summary(s) => s,
        _ => panic!("Expected summary"),
    }
}
fn value(s: &SaveDetails, slot: usize, label: &str) -> String {
    s.profiles[slot]
        .fields
        .iter()
        .find(|f| f.label == label)
        .unwrap()
        .value
        .clone()
}
fn mcs(d: &[u8]) -> Vec<u8> {
    let mut b = vec![0; MCS_HEADER_SIZE];
    b[0] = 0x51;
    b[4..8].copy_from_slice(&8192u32.to_le_bytes());
    b[8..10].fill(0xff);
    b[10..28].copy_from_slice(b"BASLUS-00823SYSTEM");
    b[127] = b[..127].iter().fold(0, |a, b| a ^ b);
    b.extend(d);
    b
}
#[test]
fn known_values_only_supported_fields_and_serialized_states() {
    let d = payload();
    let before = d.clone();
    let s = summary(&d);
    assert_eq!(s.checksum_ok, Some(true));
    assert_eq!(s.profiles.len(), 5);
    assert_eq!(s.profiles[0].status, Some(ProfileStatus::Occupied));
    assert_eq!(value(&s, 0, "Player"), "Blb");
    assert_eq!(value(&s, 0, "Normal course tickets"), "1 of 8");
    assert_eq!(value(&s, 0, "Mirrored course tickets"), "0 of 8");
    assert_eq!(s.profiles[0].fields.len(), 3);
    assert!(s.profiles[0].records.is_empty());
    assert!(s.profiles[1..]
        .iter()
        .all(|p| p.empty && p.status == Some(ProfileStatus::Empty) && p.fields.is_empty()));
    let dto = serde_json::to_value(&s).unwrap();
    assert_eq!(dto["profiles"][0]["status"], "occupied");
    assert_eq!(dto["profiles"][1]["status"], "empty");
    assert_eq!(d, before);
}
#[test]
fn integrity_and_unfamiliar_markers_are_unknown_never_empty() {
    for marker in [1, 0x55, 0x7f, 0xfe] {
        let mut d = payload();
        d[0xd08] = marker;
        checks(&mut d);
        let s = summary(&d);
        assert_eq!(s.profiles[0].status, Some(ProfileStatus::Unknown));
        assert!(!s.profiles[0].empty);
        assert!(s.profiles[0].fields.is_empty());
        assert_eq!(s.profiles[0].checksum_ok, Some(true));
        assert_eq!(
            serde_json::to_value(&s).unwrap()["profiles"][0]["status"],
            "unknown"
        );
    }
    let mut d = payload();
    d[0xd09] ^= 1;
    let s = summary(&d);
    assert_eq!(s.checksum_ok, Some(false));
    assert_eq!(s.profiles[0].status, Some(ProfileStatus::Unknown));
    assert!(s.profiles[0].fields.is_empty());
    assert!(s.profiles[1].empty);
    let mut d = payload();
    d[0xd08 + 0x44 + 1] = 99; // Damaged record marked empty.
    let s = summary(&d);
    assert_eq!(s.profiles[1].status, Some(ProfileStatus::Unknown));
    assert!(!s.profiles[1].empty);
    assert!(s.profiles[1].fields.is_empty());
    let mut d = payload();
    d[0x200] ^= 1;
    let s = summary(&d);
    assert!(s
        .profiles
        .iter()
        .all(|p| p.status == Some(ProfileStatus::Unknown) && !p.empty && p.fields.is_empty()));
}
#[test]
fn glyph_boundaries_unknown_ids_and_ticket_masks() {
    let mut d = payload();
    for (glyphs, expected) in [([10, 35, 36, 61, 73], "AZaz"), ([73; 5], "Unnamed profile")] {
        d[0xd09..0xd0e].copy_from_slice(&glyphs);
        checks(&mut d);
        assert_eq!(value(&summary(&d), 0, "Player"), expected);
    }
    d[0xd09] = 0x3e;
    checks(&mut d);
    let s = summary(&d);
    assert!(value(&s, 0, "Player").contains("3E"));
    assert_eq!(s.profiles[0].name, "Profile 01");
    for (normal, mirrored, n, m) in [
        (0, 0, 0, 0),
        (3, 129, 2, 2),
        (128, 128, 1, 1),
        (255, 255, 8, 8),
    ] {
        d[0xd0e] = normal;
        d[0xd0f] = mirrored;
        d[0xd12] = 0x5c;
        checks(&mut d);
        let s = summary(&d);
        assert_eq!(value(&s, 0, "Normal course tickets"), format!("{n} of 8"));
        assert_eq!(value(&s, 0, "Mirrored course tickets"), format!("{m} of 8"));
    }
}
#[test]
fn capacity_bounds_last_profile_and_inactive_record_integrity() {
    let mut d = payload();
    for (raw, expected) in [(0, 0), (5, 5), (50, 50), (51, 50), (255, 50)] {
        d[0xd03] = raw;
        checks(&mut d);
        let s = summary(&d);
        assert_eq!(s.profiles.len(), expected);
        if !matches!(raw, 5 | 50) {
            assert!(s.notes.iter().any(|n| n.contains("Unusual")));
        }
    }
    let last = 0xd08 + 49 * 0x44;
    d[last] = 255;
    d[last + 1..last + 6].copy_from_slice(&[10, 11, 12, 13, 14]);
    d[last + 6] = 255;
    d[last + 7] = 128;
    checks(&mut d);
    let s = summary(&d);
    assert_eq!(s.profiles[49].number, 50);
    assert_eq!(value(&s, 49, "Player"), "ABCDE");
    assert_eq!(value(&s, 49, "Normal course tickets"), "8 of 8");
    d[0xd03] = 5;
    checks(&mut d);
    d[last + 1] ^= 1;
    let s = summary(&d);
    assert_eq!(s.checksum_ok, Some(false));
    assert_eq!(s.profiles[0].status, Some(ProfileStatus::Occupied));
}
#[test]
fn rejects_unsupported_release_region_identifier_header_and_size() {
    let d = payload();
    for (code, region, id) in [
        ("SLUS-00823", "BE", "SYSTEM"),
        ("SLUS-00823", "BA", "SYSTEM1"),
        ("SLUS-00823", "BA", "GHOST"),
        ("SLUS-00824", "BA", "SYSTEM"),
    ] {
        assert!(super::super::decode(code, region, id, &d).is_err());
    }
    for offset in 0..4 {
        let mut bad = d.clone();
        bad[offset] ^= 1;
        assert!(super::super::decode("SLUS-00823", "BA", "SYSTEM", &bad).is_err());
    }
    for len in [0, 3, 8191, 8193, 16384] {
        let mut bad = d.clone();
        bad.resize(len, 0);
        assert!(super::super::decode("SLUS-00823", "BA", "SYSTEM", &bad).is_err());
    }
}
#[test]
fn whole_card_containers_deleted_metadata_mcs_reimport_and_bad_chain() {
    let bytes = mcs(&payload());
    let before = bytes.clone();
    let mut card = Ps1Card::create_formatted("bfr");
    card.set_save_bytes(10, &bytes).unwrap();
    let expected = serde_json::to_value(summary(&payload())).unwrap();
    let raw = card.save_raw(false);
    for format in [CardFormat::Raw, CardFormat::Gme, CardFormat::Vgs] {
        let b = card.export(format, false);
        let opened = Ps1Card::open(&b, "bfr", false).unwrap();
        let view = opened.view();
        assert_eq!(view.slots.len(), 15);
        assert_eq!(view.saves[0].linked_slots, [10]);
        assert_eq!(
            serde_json::to_value(&view.saves[0].game_details).unwrap(),
            expected
        );
        let mut imported = Ps1Card::create_formatted("reimport");
        imported
            .set_save_bytes(0, &opened.get_save_bytes(10))
            .unwrap();
        assert_eq!(
            serde_json::to_value(&imported.view().saves[0].game_details).unwrap(),
            expected
        );
        assert_eq!(opened.save_raw(false), raw);
    }
    for (offset, v) in [(8, 0), (9, 0), (5, 0x40), (7, 1)] {
        let mut bad = raw.clone();
        let h = &mut bad[11 * HEADER_SIZE..12 * HEADER_SIZE];
        h[offset] = v;
        h[127] = h[..127].iter().fold(0, |a, b| a ^ b);
        let c = Ps1Card::open(&bad, "bad", false).unwrap();
        assert!(c.view().saves[0].game_details.is_none());
        assert!(c.view().saves[0].game_details_error.is_some());
        assert_eq!(c.save_raw(false), bad);
    }
    let mut deleted = raw.clone();
    let h = &mut deleted[11 * HEADER_SIZE..12 * HEADER_SIZE];
    h[0] = 0xa1;
    h[127] = h[..127].iter().fold(0, |a, b| a ^ b);
    let c = Ps1Card::open(&deleted, "deleted", false).unwrap();
    assert!(c.view().saves[0].deleted);
    assert_eq!(
        serde_json::to_value(&c.view().saves[0].game_details).unwrap(),
        expected
    );
    assert_eq!(c.save_raw(false), deleted);
    assert_eq!(bytes, before);
}
#[test]
#[ignore = "requires private Bomberman review outputs; see docs/formats/bomberman-fantasy-race-us.md"]
fn private_known_value_corpus() {
    let root =
        std::path::PathBuf::from(std::env::var_os("PS1_BFR_REVIEW").expect("Set PS1_BFR_REVIEW"));
    for (rel,hash,slots) in [
        ("outputs/final/recovery/inputs/72965dcd7fc508a9b4cf104b339f09e44c05b2bcfb72259a0e1ff7111775930b.mcs","72965dcd7fc508a9b4cf104b339f09e44c05b2bcfb72259a0e1ff7111775930b",1),
        ("outputs/research/name-entry-001/saved-system.sc","14d458a0b8e0dd4e2c39ed084602c1b64f41470c4ac8d7063e806773fef9059f",2),
        ("outputs/final/recovery/fixtures/continuation-20261019/SLUS-00823/summary-cases-002/controlled-tickets.sc","ce27e8a5bc6ec1ed281bb91efd9d4f254b85d9da13a23945929a561499914b84",1),
    ] {
        let path=root.join(rel);let b=std::fs::read(&path).unwrap();assert_eq!(format!("{:x}",Sha256::digest(&b)),hash);
        let d=if b.len()==8320 {&b[128..]} else {&b[..]};let s=summary(d);
        assert_eq!(s.checksum_ok,Some(true));assert_eq!(value(&s,0,"Player"),"Blb");
        if slots==2 {assert_eq!(value(&s,1,"Player"),"a");assert_eq!(value(&s,1,"Normal course tickets"),"1 of 8");}
        let controlled=rel.contains("controlled-tickets");
        assert_eq!(value(&s,0,"Normal course tickets"),if controlled {"2 of 8"} else {"1 of 8"});
        assert_eq!(value(&s,0,"Mirrored course tickets"),if controlled {"2 of 8"} else {"0 of 8"});
        assert!(s.profiles[slots..].iter().all(|p|p.status==Some(ProfileStatus::Empty)));
        let mut c=Ps1Card::create_formatted("corpus");c.set_save_bytes(10,&mcs(d)).unwrap();
        for f in [CardFormat::Raw,CardFormat::Gme,CardFormat::Vgs] {let encoded=c.export(f,false);let opened=Ps1Card::open(&encoded,"corpus",false).unwrap();assert_eq!(serde_json::to_value(&opened.view().saves[0].game_details).unwrap(),serde_json::to_value(&s).unwrap());}
        assert_eq!(std::fs::read(path).unwrap(),b);
    }
}
