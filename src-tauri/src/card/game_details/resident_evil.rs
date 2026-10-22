//! Original US release only; MIT adaptation, docs/formats/resident-evil-us.md.
use super::{Field, Format, Profile, SaveDetails};
use unicode_normalization::UnicodeNormalization;
pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    let end = d[4..68].iter().position(|&v| v == 0).unwrap_or(64);
    let (title, _, invalid) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if d[2] != 0x13
        || invalid
        || !title
            .nfkc()
            .collect::<String>()
            .starts_with("RESIDENT EVIL ")
    {
        return Err("Unsupported Resident Evil title or icon layout.".into());
    }
    let seconds = u32::from_le_bytes(d[0x224..0x228].try_into().unwrap()) / 60;
    let health = u16::from_le_bytes(d[0x21e..0x220].try_into().unwrap());
    let location = u16::from_be_bytes(d[0x200..0x202].try_into().unwrap());
    let field = |label: &str, value: String| Field {
        label: label.into(),
        value,
    };
    Ok(SaveDetails{game:fmt.slug.into(),title:fmt.title.into(),release:format!("NTSC-U · {code}"),checksum_ok:None,profiles:vec![Profile{number:2,name:"Saved game".into(),empty:false,status:None,checksum_ok:None,fields:vec![field("Character",if d[0x22b]&1==0{"Chris Redfield".into()}else{"Jill Valentine".into()}),field("Playtime (hh:mm:ss)",format!("{:02}:{:02}:{:02}",seconds/3600,seconds/60%60,seconds%60)),field("Health (stored units)",health.to_string()),field("Ink ribbons used",d[0x228].to_string()),field("Location ID",format!("0x{location:04X}"))],records:vec![]}],notes:vec!["Only original US SLUS-00170 identifier 1 (upstream slot 2) is supported; Director's Cut is separate. Integrity is unverified. Health stays in stored units; no condition thresholds are inferred. Inventory, item box, story flags and named rooms are not decoded. Source/sample validation only; no emulator save/reload validation.".into()]})
}
#[cfg(test)]
mod tests {
    use super::super::{decode, GameDetails};
    fn fixture() -> Vec<u8> {
        let mut d = vec![0; 8192];
        d[..4].copy_from_slice(b"SC\x13\x01");
        let title = b"RESIDENT EVIL TEST  ";
        d[4..4 + title.len()].copy_from_slice(title);
        d[0x224..0x228].copy_from_slice(&216060u32.to_le_bytes());
        d[0x21e..0x220].copy_from_slice(&140u16.to_le_bytes());
        d[0x228] = 9;
        d[0x22b] = 0x81;
        d[0x200..0x202].copy_from_slice(&0xbeefu16.to_be_bytes());
        d
    }
    #[test]
    fn resident_evil_fields_bounds_and_unknown_integrity() {
        let d = fixture();
        let before = d.clone();
        let GameDetails::Summary(s) = decode("SLUS-00170", "BA", "1", &d).unwrap() else {
            panic!()
        };
        assert_eq!(s.checksum_ok, None);
        assert_eq!(s.profiles[0].number, 2);
        assert_eq!(
            s.profiles[0]
                .fields
                .iter()
                .map(|f| f.value.as_str())
                .collect::<Vec<_>>(),
            ["Jill Valentine", "01:00:01", "140", "9", "0xBEEF"]
        );
        assert_eq!(d, before);
        let mut variant = d.clone();
        variant[0x22b] = 0xfe;
        variant[0x21e..0x220].copy_from_slice(&65535u16.to_le_bytes());
        let GameDetails::Summary(s) = decode("SLUS-00170", "BA", "1", &variant).unwrap() else {
            panic!()
        };
        assert_eq!(s.checksum_ok, None);
        assert_eq!(s.profiles[0].fields[0].value, "Chris Redfield");
        assert_eq!(s.profiles[0].fields[2].value, "65535");
        for n in 0..8192 {
            assert!(decode("SLUS-00170", "BA", "1", &d[..n]).is_err());
        }
        for (code, region, id) in [
            ("SLUS-00551", "BA", "1"),
            ("SLES-00200", "BE", "1"),
            ("SLUS-00170", "BI", "1"),
            ("SLUS-00170", "BA", "0"),
            ("SLUS-00170", "BA", "1x"),
        ] {
            assert!(decode(code, region, id, &d).is_err());
        }
        for o in [0, 2, 3, 4] {
            let mut bad = d.clone();
            bad[o] ^= 1;
            assert!(decode("SLUS-00170", "BA", "1", &bad).is_err());
        }
        let mut bad = d;
        bad.push(0);
        assert!(decode("SLUS-00170", "BA", "1", &bad).is_err());
    }
    #[test]
    #[ignore = "Requires GAME_TOOLS_COLLECTION at pinned commit; no saves redistributed"]
    fn resident_evil_public_card() {
        use crate::card::engine::Ps1Card;
        use sha2::{Digest, Sha256};
        let p = std::path::PathBuf::from(std::env::var("GAME_TOOLS_COLLECTION").unwrap())
            .join("tests/saves/resident-evil/playstation/usa.mcr");
        let d = std::fs::read(p).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&d)),
            "519ab9dee658dc3bdb927f9775b903f605cb8eb94df807591a36b30bdd4316cb"
        );
        let card = Ps1Card::open(&d, "Resident Evil public sample", false).unwrap();
        let view = card.view();
        assert_eq!(view.saves.len(), 1);
        let Some(GameDetails::Summary(s)) = &view.saves[0].game_details else {
            panic!("{:?}", view.saves[0].game_details_error)
        };
        assert_eq!(s.checksum_ok, None);
        assert_eq!(
            s.profiles[0]
                .fields
                .iter()
                .map(|f| f.value.as_str())
                .collect::<Vec<_>>(),
            ["Chris Redfield", "00:04:26", "140", "1", "0x0006"]
        );
        assert_eq!(card.save_raw(false), d);
    }
}
