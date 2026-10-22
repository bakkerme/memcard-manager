//! MIT source adaptation: RyudoSynbios/game-tools-collection; docs/formats/tekken2-us.md.
use super::{Field, Format, Profile, SaveDetails};
use unicode_normalization::UnicodeNormalization;

pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    let end = d[4..68].iter().position(|&b| b == 0).unwrap_or(64);
    let (title, _, invalid) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if d[2] != 0x13 || invalid || title.nfkc().collect::<String>() != "[TEKKEN2]" {
        return Err("Unsupported Tekken 2 title or icon layout.".into());
    }
    let encoded = &d[0x200..0x400];
    let mut data = encoded.to_vec();
    for i in 1..data.len() {
        // Previous ENCODED byte, with modulo-256 arithmetic; never mutate input.
        data[i] = encoded[i].wrapping_sub(encoded[i - 1].wrapping_mul(5).wrapping_add(1));
    }
    let checksum_ok = data.iter().fold(0u8, |s, &v| s.wrapping_add(v)) == 0;
    const NAMES: [&str; 23] = [
        "Kazuya",
        "Paul",
        "Law",
        "Jack-2",
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
        "Devil",
        "Lei",
        "Jun",
        "Bruce",
        "Baek",
        "Roger",
    ];
    let unlocked = NAMES
        .iter()
        .enumerate()
        .filter(|(i, _)| data[1 + i / 8] & (1 << (i % 8)) != 0)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>();
    let seconds = u32::from_le_bytes(data[9..13].try_into().unwrap()) / 60;
    Ok(SaveDetails {
        game: fmt.slug.into(), title: fmt.title.into(), release: format!("NTSC-U · {code}"), checksum_ok: Some(checksum_ok),
        profiles: vec![Profile {number: 1, name: "Game progress".into(), empty: false, status: None, checksum_ok: None,
            fields: vec![Field { label: "Playtime (hh:mm:ss)".into(), value: format!("{:02}:{:02}:{:02}", seconds/3600, seconds/60%60, seconds%60) },
                Field { label: "Unlocked characters".into(), value: if unlocked.is_empty() { "None recorded".into() } else {unlocked.join(", ")} }], records: vec![] }],
        notes: vec!["Time is decoded from a 60-Hz counter for this US release. Time attack, survival, arcade defeat flags and options are not decoded. Source/sample validation only; no emulator save/reload validation.".into()],
    })
}
#[cfg(test)]
mod tests {
    use super::super::{decode, GameDetails};
    fn fixture() -> Vec<u8> {
        let mut d = vec![0; 8192];
        d[..4].copy_from_slice(b"SC\x13\x01");
        d[4..13].copy_from_slice(b"[TEKKEN2]");
        d[15] = 0x81;
        let mut plain = [0u8; 512];
        plain[1] = 1;
        plain[3] = 0x40;
        plain[9..13].copy_from_slice(&216060u32.to_le_bytes());
        plain[0] = plain[1..].iter().fold(0u8, |s, &v| s.wrapping_sub(v));
        // Forward encoding is independent of the production reverse transform.
        d[0x200] = plain[0];
        for i in 1..512 {
            d[0x200 + i] = plain[i].wrapping_add(d[0x200 + i - 1].wrapping_mul(5).wrapping_add(1));
        }
        d
    }
    #[test]
    fn tekken2_values_integrity_and_bounds() {
        let d = fixture();
        let before = d.clone();
        let GameDetails::Summary(s) = decode("SLUS-00213", "BA", "TEKKEN-2", &d).unwrap() else {
            panic!()
        };
        assert_eq!(s.checksum_ok, Some(true));
        assert_eq!(s.profiles[0].fields[0].value, "01:00:01");
        assert_eq!(s.profiles[0].fields[1].value, "Kazuya, Roger");
        assert_eq!(d, before);
        let mut bad = d.clone();
        bad[0x3ff] ^= 1;
        let GameDetails::Summary(s) = decode("SLUS-00213", "BA", "TEKKEN-2", &bad).unwrap() else {
            panic!()
        };
        assert_eq!(s.checksum_ok, Some(false));
        for n in 0..8192 {
            assert!(decode("SLUS-00213", "BA", "TEKKEN-2", &d[..n]).is_err());
        }
        for (code, region, id) in [
            ("SCES-00255", "BE", "TEKKEN-2"),
            ("SLUS-00213", "BI", "TEKKEN-2"),
            ("SLUS-00213", "BA", "TEKKEN-3"),
            ("SLUS-00213", "BA", "TEKKEN-2x"),
        ] {
            assert!(decode(code, region, id, &d).is_err());
        }
        for offset in [0, 2, 3, 4] {
            let mut bad = d.clone();
            bad[offset] ^= 1;
            assert!(decode("SLUS-00213", "BA", "TEKKEN-2", &bad).is_err());
        }
        let mut bad = d;
        bad.push(0);
        assert!(decode("SLUS-00213", "BA", "TEKKEN-2", &bad).is_err());
    }
    #[test]
    #[ignore = "Requires GAME_TOOLS_COLLECTION at the documented source commit; no third-party saves redistributed"]
    fn tekken2_public_card() {
        use crate::card::engine::Ps1Card;
        use sha2::{Digest, Sha256};
        let path = std::path::PathBuf::from(std::env::var("GAME_TOOLS_COLLECTION").unwrap())
            .join("tests/saves/tekken-2/playstation/usa.mcr");
        let d = std::fs::read(path).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&d)),
            "f784bdd4527a01ac88242adceb3ce6c1a175c6719108080b9aac6ff844631d2c"
        );
        let card = Ps1Card::open(&d, "Tekken 2 public sample", false).unwrap();
        let view = card.view();
        assert_eq!(view.saves.len(), 1);
        let Some(GameDetails::Summary(s)) = &view.saves[0].game_details else {
            panic!("{:?}", view.saves[0].game_details_error)
        };
        assert_eq!(s.checksum_ok, Some(true));
        assert_eq!(s.profiles[0].fields[0].value, "00:00:30");
        assert_eq!(
            s.profiles[0].fields[1].value,
            "Paul, Law, Jack-2, Nina, King, Yoshimitsu, Michelle, Heihachi, Lei, Jun"
        );
        assert_eq!(card.save_raw(false), d);
    }
}
