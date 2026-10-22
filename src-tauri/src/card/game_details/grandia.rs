//! MIT adaptation from RyudoSynbios/game-tools-collection; docs/formats/grandia-us.md.
use super::{Field, Format, Profile, SaveDetails};
use unicode_normalization::UnicodeNormalization;
pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    let end = d[4..68].iter().position(|&v| v == 0).unwrap_or(64);
    let (title, _, invalid) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if d[2] != 0x13
        || invalid
        || !title.nfkc().collect::<String>().starts_with("GRANDIA−")
        || &d[0x280..0x28f] != b"GrandiaBupData\0"
    {
        return Err("Unsupported Grandia US title or data marker.".into());
    }
    // US/Japanese editor fields shift by 0x180 relative to PAL. Never use PAL offsets here.
    let seconds = u32::from_le_bytes(d[0x290..0x294].try_into().unwrap()) / 60;
    let gold = u32::from_le_bytes(d[0x330..0x334].try_into().unwrap());
    let location = u16::from_le_bytes(d[0x2f4..0x2f6].try_into().unwrap());
    let disc = match d[0x329] {
        1 => "Disc 1".into(),
        2 => "Disc 2".into(),
        0x82 => "Transition".into(),
        v => format!("Unknown ({v})"),
    };
    let field = |label: &str, value: String| Field {
        label: label.into(),
        value,
    };
    Ok(SaveDetails{game:fmt.slug.into(),title:fmt.title.into(),release:format!("NTSC-U · {code}"),checksum_ok:None,
    profiles:vec![Profile{number:2,name:"Saved game".into(),empty:false,status:None,checksum_ok:None,fields:vec![field("Disc",disc),field("Playtime (hh:mm:ss)",format!("{:02}:{:02}:{:02}",seconds/3600,seconds/60%60,seconds%60)),field("Gold",gold.to_string()),field("Location ID",format!("0x{location:04X}"))],records:vec![]}],notes:vec!["Only the observed GRA-S01 file (upstream slot 2) is supported. P-bearing SCUSP94457 is the save product code. Game integrity is unverified. Party, inventory, spells and named locations are not decoded. Source/sample validation only; no emulator save/reload validation.".into()]})
}
#[cfg(test)]
mod tests {
    use super::super::{decode, GameDetails};
    fn fixture() -> Vec<u8> {
        let mut d = vec![0; 8192];
        d[..4].copy_from_slice(b"SC\x13\x01");
        let (t, _, _) = encoding_rs::SHIFT_JIS.encode("ＧＲＡＮＤＩＡ−２");
        d[4..4 + t.len()].copy_from_slice(&t);
        d[0x280..0x28f].copy_from_slice(b"GrandiaBupData\0");
        d[0x290..0x294].copy_from_slice(&216060u32.to_le_bytes());
        d[0x329] = 0x82;
        d[0x330..0x334].copy_from_slice(&123456u32.to_le_bytes());
        d[0x2f4..0x2f6].copy_from_slice(&0xbeefu16.to_le_bytes());
        d
    }
    #[test]
    fn grandia_fields_unknowns_and_bounds() {
        let d = fixture();
        let before = d.clone();
        let GameDetails::Summary(s) = decode("SCUSP94457", "BA", "GRA-S01", &d).unwrap() else {
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
            ["Transition", "01:00:01", "123456", "0xBEEF"]
        );
        assert_eq!(d, before);
        let mut bad = d.clone();
        bad[0x329] = 0xff;
        let GameDetails::Summary(s) = decode("SCUSP94457", "BA", "GRA-S01", &bad).unwrap() else {
            panic!()
        };
        assert_eq!(s.profiles[0].fields[0].value, "Unknown (255)");
        for n in 0..8192 {
            assert!(decode("SCUSP94457", "BA", "GRA-S01", &d[..n]).is_err());
        }
        for (code, region, id) in [
            ("SCUS-94457", "BA", "GRA-S01"),
            ("SLES-02397", "BE", "GRA-S01"),
            ("SCUSP94457", "BI", "GRA-S01"),
            ("SCUSP94457", "BA", "GRA-S00"),
            ("SCUSP94457", "BA", "GRA-S01x"),
        ] {
            assert!(decode(code, region, id, &d).is_err());
        }
        for o in [0, 2, 3, 4, 0x280] {
            let mut bad = d.clone();
            bad[o] ^= 1;
            assert!(decode("SCUSP94457", "BA", "GRA-S01", &bad).is_err());
        }
        let mut bad = d;
        bad.push(0);
        assert!(decode("SCUSP94457", "BA", "GRA-S01", &bad).is_err());
    }
    #[test]
    #[ignore = "Requires GAME_TOOLS_COLLECTION at pinned commit; no saves redistributed"]
    fn grandia_public_card() {
        use crate::card::engine::Ps1Card;
        use sha2::{Digest, Sha256};
        let p = std::path::PathBuf::from(std::env::var("GAME_TOOLS_COLLECTION").unwrap())
            .join("tests/saves/grandia/playstation/usa.mcr");
        let d = std::fs::read(p).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&d)),
            "2dd2cc33bc05be439045ca86df3f6023c35e0eef9b4d046d21f351525bb44e11"
        );
        let card = Ps1Card::open(&d, "Grandia public sample", false).unwrap();
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
            ["Disc 1", "00:07:50", "200", "0x2C00"]
        );
        assert_eq!(card.save_raw(false), d);
    }
}
