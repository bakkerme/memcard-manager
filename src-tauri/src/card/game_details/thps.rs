//! MIT adaptation from RyudoSynbios/game-tools-collection; docs/formats/thps-us.md.
use super::{Field, Format, Profile, SaveDetails};
use unicode_normalization::UnicodeNormalization;
pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    let end = d[4..68].iter().position(|&v| v == 0).unwrap_or(64);
    let (title, _, invalid) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if d[2] != 0x11
        || invalid
        || !title
            .nfkc()
            .collect::<String>()
            .starts_with("THPS CAREER ー ")
    {
        return Err("Unsupported THPS career title or icon layout.".into());
    }
    const SKATERS: [&str; 11] = [
        "Tony Hawk",
        "Bob Burnquist",
        "Geoff Rowley",
        "Bucky Lasek",
        "Chad Muska",
        "Kareem Campbell",
        "Andrew Reynolds",
        "Rune Glifberg",
        "Jamie Thomas",
        "Elissa Steamer",
        "Officer Dick",
    ];
    let fields = SKATERS
        .iter()
        .enumerate()
        .map(|(i, name)| Field {
            label: format!("{name} · stored tapes"),
            value: d[0x288 + i * 14].to_string(),
        })
        .collect();
    let sum = d[0x280..0xc80]
        .iter()
        .fold(0u16, |s, &v| s.wrapping_add(u16::from(v) + 1));
    let stored = u16::from_le_bytes(d[0x236..0x238].try_into().unwrap());
    Ok(SaveDetails{game:fmt.slug.into(),title:fmt.title.into(),release:format!("NTSC-U · {code}"),checksum_ok:Some(sum==stored),profiles:vec![Profile{number:1,name:"Career".into(),empty:false,status:None,checksum_ok:None,fields,records:vec![]}],notes:vec!["Each career file contains stored tape totals for eleven skaters. Zero is a recorded total, not an empty slot or locked skater. Medal flags, individual goals, scores, options and replay files are not decoded. Source/sample validation only; no emulator save/reload validation.".into()]})
}
#[cfg(test)]
mod tests {
    use super::super::{decode, GameDetails};
    fn fixture() -> Vec<u8> {
        let mut d = vec![0; 8192];
        d[..4].copy_from_slice(b"SC\x11\x01");
        let (t, _, _) = encoding_rs::SHIFT_JIS.encode("ＴＨＰＳ　ＣＡＲＥＥＲ　ー　ＴＥＳＴ");
        d[4..4 + t.len()].copy_from_slice(&t);
        d[0x288] = 30;
        d[0x288 + 10 * 14] = 7;
        let sum = (d[0x280..0xc80].iter().map(|&v| u32::from(v)).sum::<u32>() + 0xa00) as u16;
        d[0x236..0x238].copy_from_slice(&sum.to_le_bytes());
        d
    }
    #[test]
    fn thps_fields_checksum_and_bounds() {
        let d = fixture();
        let before = d.clone();
        for id in ["THPSGN0", "THPSGCL"] {
            let GameDetails::Summary(s) = decode("SLUS-00860", "BA", id, &d).unwrap() else {
                panic!()
            };
            assert_eq!(s.checksum_ok, Some(true));
            assert_eq!(s.profiles[0].fields.len(), 11);
            assert_eq!(s.profiles[0].fields[0].value, "30");
            assert_eq!(s.profiles[0].fields[10].value, "7");
            assert!(!s.profiles[0].empty);
        }
        assert_eq!(d, before);
        let mut bad = d.clone();
        bad[0xc7f] ^= 1;
        let GameDetails::Summary(s) = decode("SLUS-00860", "BA", "THPSGN0", &bad).unwrap() else {
            panic!()
        };
        assert_eq!(s.checksum_ok, Some(false));
        for n in 0..8192 {
            assert!(decode("SLUS-00860", "BA", "THPSGN0", &d[..n]).is_err());
        }
        for (code, region, id) in [
            ("SLES-02055", "BE", "THPSGN0"),
            ("SLUS-00860", "BI", "THPSGN0"),
            ("SLUS-00860", "BA", "THPSRPL"),
            ("SLUS-00860", "BA", "THPSGN0x"),
        ] {
            assert!(decode(code, region, id, &d).is_err());
        }
        for o in [0, 2, 3, 4] {
            let mut bad = d.clone();
            bad[o] ^= 1;
            assert!(decode("SLUS-00860", "BA", "THPSGN0", &bad).is_err());
        }
        let mut bad = d;
        bad.push(0);
        assert!(decode("SLUS-00860", "BA", "THPSGN0", &bad).is_err());
    }
    #[test]
    #[ignore = "Requires GAME_TOOLS_COLLECTION at pinned commit; no saves redistributed"]
    fn thps_public_card() {
        use crate::card::engine::Ps1Card;
        use sha2::{Digest, Sha256};
        let p = std::path::PathBuf::from(std::env::var("GAME_TOOLS_COLLECTION").unwrap())
            .join("tests/saves/tony-hawk-s-skateboarding/playstation/usa.mcr");
        let d = std::fs::read(p).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&d)),
            "db8859ad5ee9635cf81d59484f8539aed18c33dc3f29373969806bf5ed3c371c"
        );
        let card = Ps1Card::open(&d, "THPS public sample", false).unwrap();
        let view = card.view();
        assert_eq!(view.saves.len(), 2);
        for save in &view.saves {
            let Some(GameDetails::Summary(s)) = &save.game_details else {
                panic!("{:?}", save.game_details_error)
            };
            assert_eq!(s.checksum_ok, Some(true));
            assert_eq!(s.profiles[0].fields.len(), 11);
            assert!(s.profiles[0].fields.iter().all(|f| f.value == "0"));
        }
        assert_eq!(card.save_raw(false), d);
    }
}
