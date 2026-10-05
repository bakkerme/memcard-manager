use super::*;
use crate::card::engine::{CardFormat, Ps1Card, BLOCK_SIZE, HEADER_SIZE, MCS_HEADER_SIZE};
use sha2::{Digest, Sha256};

const RELEASES: [(&str, &str, &str); 12] = [
    ("SCES-00867", "BE", "FF7-S01"),
    ("SLUSP00892", "BA", "042610"),
    ("SLES-02965", "BE", "00000-00"),
    ("SCUS-94221", "BA", "FFTA"),
    ("SLUSP01041", "BA", "USCHRO00"),
    ("SLUS-00067", "BA", "DRAX01"),
    ("SCUS-94194", "BA", "GT"),
    ("SCES-02380", "BE", "GAME"),
    ("SCUS-94426", "BA", "-SLOTS"),
    ("SCUS-94228", "BA", "SPYRO"),
    ("SLUS-00402", "BA", "TEKKEN-3"),
    ("SLUS-00707", "BA", "SILENT00"),
];

fn payload(code: &str) -> Vec<u8> {
    let f = format(code).unwrap();
    let mut d = vec![0; f.blocks * BLOCK_SIZE];
    d[..2].copy_from_slice(b"SC");
    d[2] = 0x11;
    d[3] = f.blocks as u8;
    if f.slug == "final-fantasy-vii" {
        d[0x10a4] = 2;
    }
    if f.slug == "final-fantasy-viii" {
        d[0xfac] = 4;
    }
    if f.slug == "final-fantasy-ix" {
        d[0x104] = 8;
    }
    d
}

fn summary(code: &str, region: &str, id: &str, d: &[u8]) -> SaveDetails {
    match decode(code, region, id, d).unwrap() {
        GameDetails::Summary(s) => s,
        _ => panic!("Expected summary"),
    }
}

fn value<'a>(fields: &'a [Field], label: &str) -> &'a str {
    &fields.iter().find(|f| f.label == label).unwrap().value
}

#[test]
fn proposal_has_twelve_distinct_decoders_with_serializable_profiles() {
    let mut games = std::collections::HashSet::new();
    for (code, region, id) in RELEASES {
        let d = payload(code);
        let before = d.clone();
        let s = summary(code, region, id, &d);
        games.insert(s.game.clone());
        let dto = serde_json::to_value(&s).unwrap();
        assert_eq!(dto["game"], s.game);
        assert!(dto.get("checksumOk").is_some());
        assert!(!s.profiles.is_empty());
        assert_eq!(d, before);
    }
    assert_eq!(games.len(), 12);
}

#[test]
fn rejects_truncated_wrong_region_header_and_save_types() {
    for (code, region, id) in RELEASES {
        let d = payload(code);
        for length in [0, 1, 3, 128, d.len() - 1] {
            assert!(
                decode(code, region, id, &d[..length]).is_err(),
                "{code}, {length}"
            );
        }
        assert!(decode(code, "BI", id, &d).is_err());
        assert!(decode(code, region, "REPLAY", &d).is_err());
        assert!(decode(code, region, &format!("{id}X"), &d).is_err());
        let mut bad = d.clone();
        bad[3] += 1;
        assert!(decode(code, region, id, &bad).is_err());
        bad[3] = d[3];
        bad[0] = b'X';
        assert!(decode(code, region, id, &bad).is_err());
    }
    assert!(format("SLUS-01251").is_none()); // Unvalidated NTSC FFIX.
    assert!(format("SCUS-94455").is_none()); // Unvalidated NTSC GT2.
    assert!(decode("SCES-00867", "BE", "FF7-S99", &payload("SCES-00867")).is_err());
}

#[test]
fn arbitrary_bytes_never_panic_or_change_input() {
    for (code, region, id) in RELEASES {
        let mut seed = 0x12345678u32;
        for _ in 0..16 {
            let mut d = payload(code);
            for b in &mut d[4..] {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                *b = (seed >> 24) as u8;
            }
            let before = d.clone();
            let _ = decode(code, region, id, &d);
            assert_eq!(d, before);
        }
    }
}

#[test]
fn pal_seconds_and_frames_are_not_interchangeable() {
    let mut ff7 = payload("SCES-00867");
    ff7[0xd80..0xd84].copy_from_slice(&3600u32.to_le_bytes());
    assert_eq!(
        value(
            &summary("SCES-00867", "BE", "FF7-S01", &ff7).profiles[0].fields,
            "Playtime (hh:mm)"
        ),
        "01:00"
    );
    let mut ff9 = payload("SLES-02965");
    ff9[0x12c..0x130].copy_from_slice(&(50u32 * 3600).to_le_bytes());
    assert_eq!(
        value(
            &summary("SLES-02965", "BE", "00000-00", &ff9).profiles[0].fields,
            "Playtime (hh:mm)"
        ),
        "01:00"
    );
    let mut ff8 = payload("SLUSP00892");
    ff8[0xe50..0xe54].copy_from_slice(&3600u32.to_le_bytes());
    let s = summary("SLUSP00892", "BA", "042610", &ff8);
    assert_eq!(value(&s.profiles[0].fields, "Playtime (hh:mm)"), "01:00");
    assert_eq!(value(&s.profiles[0].fields, "Disc"), "4");
}

#[test]
fn profiles_retain_numbering_and_empty_states() {
    let mut ctr = payload("SCUS-94426");
    for i in 0..4 {
        ctr[0x104 + i * 0x50 + 0x2b] = 0xff;
    }
    ctr[0x104 + 2 * 0x50 + 0x2b] = 0;
    let s = summary("SCUS-94426", "BA", "-SLOTS", &ctr);
    assert_eq!(
        s.profiles
            .iter()
            .map(|p| (p.number, p.empty))
            .collect::<Vec<_>>(),
        vec![(1, true), (2, true), (3, false), (4, true)]
    );
    let mut spyro = payload("SCUS-94228");
    for i in 0..3 {
        spyro[0x200 + i * 0x600] = 10;
        spyro[0x204 + i * 0x600] = 0x52;
    }
    spyro[0x804] = 2;
    let s = summary("SCUS-94228", "BA", "SPYRO", &spyro);
    assert_eq!(
        s.profiles.iter().map(|p| p.empty).collect::<Vec<_>>(),
        vec![true, false, true]
    );
    let sh = summary("SLUS-00707", "BA", "SILENT00", &payload("SLUS-00707"));
    assert_eq!(sh.profiles.len(), 11);
    assert!(sh.profiles.iter().all(|p| p.empty));
    let sotn = summary("SLUS-00067", "BA", "DRAX01", &payload("SLUS-00067"));
    assert_eq!(sotn.checksum_ok, None);
}

fn mcs(code: &str, region: &str, id: &str) -> Vec<u8> {
    let d = payload(code);
    let mut bytes = vec![0; MCS_HEADER_SIZE];
    bytes[0] = 0x51;
    bytes[4..8].copy_from_slice(&(d.len() as u32).to_le_bytes());
    bytes[8..10].copy_from_slice(&0xffffu16.to_le_bytes());
    let name = format!("{region}{code}{id}");
    bytes[10..10 + name.len()].copy_from_slice(name.as_bytes());
    bytes.extend(d);
    bytes
}

#[test]
fn noncontiguous_chain_decodes_and_malformed_chain_stays_browseable() {
    let mut card = Ps1Card::create_formatted("test");
    card.set_save_bytes(0, &mcs("SCES-02380", "BE", "GAME"))
        .unwrap();
    let expected = serde_json::to_value(card.view().saves[0].game_details.clone()).unwrap();
    let original = card.save_raw(false);
    let mut raw = original.clone();
    // Relocate the second block to physical block 10, leaving its bytes intact.
    raw[HEADER_SIZE + 9 * HEADER_SIZE..HEADER_SIZE + 10 * HEADER_SIZE]
        .copy_from_slice(&original[2 * HEADER_SIZE..3 * HEADER_SIZE]);
    raw[10 * BLOCK_SIZE..11 * BLOCK_SIZE]
        .copy_from_slice(&original[2 * BLOCK_SIZE..3 * BLOCK_SIZE]);
    raw[HEADER_SIZE + 8] = 9;
    raw[2 * HEADER_SIZE] = 0xa0;
    raw[2 * HEADER_SIZE + 8..2 * HEADER_SIZE + 10].fill(0xff);
    let card = Ps1Card::open(&raw, "test", false).unwrap();
    assert_eq!(card.view().saves[0].linked_slots, vec![0, 9, 2, 3]);
    assert_eq!(
        serde_json::to_value(&card.view().saves[0].game_details).unwrap(),
        expected
    );
    assert_eq!(card.save_raw(false), raw);
    for (offset, value) in [
        (HEADER_SIZE + 7, 1),
        (HEADER_SIZE + 9, 1),
        (HEADER_SIZE + 9 * HEADER_SIZE, 0x53),
        (HEADER_SIZE + 8, 0),
    ] {
        let mut bad = raw.clone();
        bad[offset] = value;
        let card = Ps1Card::open(&bad, "bad", false).unwrap();
        let view = card.view();
        assert_eq!(view.slots.len(), 15);
        assert!(view.saves[0].game_details.is_none());
        assert!(view.saves[0].game_details_error.is_some());
        assert_eq!(card.save_raw(false), bad);
    }
}

/// Public saves have no redistribution license. Acquire the manifest files locally.
/// This test is explicit so running ignored USB tests is never necessary.
#[test]
#[ignore = "Requires local saves; see tests/fixtures/ps1/README.md"]
fn shared_completion_corpus() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/ps1");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let mut tested = std::collections::HashSet::new();
    for fixture in manifest["fixtures"].as_array().unwrap() {
        let filename = fixture["filename"].as_str().unwrap();
        let path = root.join("local").join(filename);
        if fixture["private"] == true && !path.exists() {
            continue;
        }
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|_| panic!("Missing {filename}: acquire the manifest file first"));
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            fixture["sha256"].as_str().unwrap(),
            "{filename}"
        );
        let card = if filename.ends_with("mcs") {
            let mut card = Ps1Card::create_formatted(filename);
            card.set_save_bytes(0, &bytes).unwrap();
            card
        } else {
            Ps1Card::open(&bytes, filename, false).unwrap()
        };
        let raw = card.save_raw(false);
        let view = card.view();
        assert_eq!(view.slots.len(), 15);
        let save = &view.saves[0];
        assert_eq!(save.prod_code, fixture["code"]);
        assert!(
            save.game_details_error.is_none(),
            "{filename}: {:?}",
            save.game_details_error
        );
        let details = save.game_details.as_ref().unwrap();
        let dto = serde_json::to_value(details).unwrap();
        tested.insert(dto["game"].as_str().unwrap().to_string());
        assert_eq!(dto["checksumOk"], fixture["checksumOk"], "{filename}");
        assert_eq!(
            dto["profiles"].as_array().unwrap().len(),
            fixture["profileCount"].as_u64().unwrap() as usize
        );
        for check in fixture["checks"].as_array().unwrap() {
            let p = &dto["profiles"][check["profile"].as_u64().unwrap() as usize];
            if let Some(name) = check["name"].as_str() {
                assert_eq!(p["name"], name);
            }
            if let Some(empty) = check["empty"].as_bool() {
                assert_eq!(p["empty"], empty);
            }
            let fields = if let Some(record) = check["record"].as_str() {
                {
                    let r = &p["records"][check["recordIndex"].as_u64().unwrap() as usize];
                    assert_eq!(r["name"], record);
                    &r["fields"]
                }
            } else {
                &p["fields"]
            };
            if let Some(label) = check["label"].as_str() {
                let field = fields
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|f| f["label"] == label)
                    .unwrap();
                assert_eq!(field["value"], check["value"], "{filename}: {label}");
            }
        }
        for f in [CardFormat::Raw, CardFormat::Gme, CardFormat::Vgs] {
            let exported = card.export(f, false);
            let reopened = Ps1Card::open(&exported, filename, false).unwrap();
            assert_eq!(
                serde_json::to_value(&reopened.view().saves[0].game_details).unwrap(),
                dto
            );
        }
        let dump = card.get_save_bytes(save.master_slot as usize);
        if dto["checksumOk"] == true {
            let mut damaged = dump[MCS_HEADER_SIZE..].to_vec();
            // A covered data byte, away from identity/layout gates. Tekken encrypts
            // this range; Spyro's first profile covers it; SH checks shared options.
            let offset = if save.prod_code == "SLUS-00402" {
                0x201
            } else {
                0x300
            };
            damaged[offset] ^= 1;
            let changed = decode(
                &save.prod_code,
                &save.region_raw,
                &save.identifier,
                &damaged,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(changed).unwrap()["checksumOk"],
                false,
                "{filename}"
            );
        }
        let mut imported = Ps1Card::create_formatted("mcs");
        imported.set_save_bytes(0, &dump).unwrap();
        assert_eq!(
            serde_json::to_value(&imported.view().saves[0].game_details).unwrap(),
            dto
        );
        assert_eq!(card.save_raw(false), raw);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    assert!(tested.len() >= 11, "Missing a public game's real save");
}
