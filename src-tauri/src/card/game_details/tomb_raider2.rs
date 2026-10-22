//! MIT adaptation from game-tools-collection; docs/formats/tomb-raider2-us.md.
use super::{Field, Format, Profile, SaveDetails};
use unicode_normalization::UnicodeNormalization;
const LEVELS: [&str; 18] = [
    "The Great Wall",
    "Venice",
    "Bartoli's Hideout",
    "Opera House",
    "Offshore Rig",
    "Diving Area",
    "40 Fathoms",
    "Wreck of the Maria Doria",
    "Living Quarters",
    "The Deck",
    "Tibetan Foothills",
    "Barkhang Monastery",
    "Catacombs of the Talion",
    "Ice Palace",
    "Temple of Xian",
    "Floating Islands",
    "The Dragon's Lair",
    "Home Sweet Home",
];
pub(super) fn decode(fmt: &Format, code: &str, d: &[u8]) -> Result<SaveDetails, String> {
    let end = d[4..68].iter().position(|&b| b == 0).unwrap_or(64);
    let (title, _, invalid) = encoding_rs::SHIFT_JIS.decode(&d[4..4 + end]);
    if d[2] != 0x11 || invalid || title.nfkc().collect::<String>() != "Tomb Raider II" {
        return Err("Unsupported Tomb Raider II title or icon layout.".into());
    }
    let ticks: u64 = (0..18)
        .map(|i| {
            u32::from_le_bytes(d[0x244 + i * 0x2c..0x248 + i * 0x2c].try_into().unwrap()) as u64
        })
        .sum();
    let seconds = ticks / 30;
    let secrets: u32 = (0..16)
        .map(|i| (d[0x256 + i * 0x2c] & 7).count_ones())
        .sum();
    let level = u16::from_le_bytes(d[0x634..0x636].try_into().unwrap());
    let field = |label: &str, value: String| Field {
        label: label.into(),
        value,
    };
    let mut fields = vec![
        field(
            "Total recorded playtime (hh:mm:ss)",
            format!(
                "{:02}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            ),
        ),
        field("Recorded secrets (first 16 levels)", secrets.to_string()),
    ];
    fields.push(field(
        "Current position",
        match d[0x64c] {
            0 => "Not recorded".into(),
            1 => LEVELS
                .get(level as usize)
                .map_or_else(|| format!("Unknown level ({level})"), |s| s.to_string()),
            v => format!("Unknown marker ({v})"),
        },
    ));
    fields.push(field(
        "Game cleared",
        match d[0x640] {
            0 => "No".into(),
            1 => "Yes".into(),
            v => format!("Unknown ({v})"),
        },
    ));
    Ok(SaveDetails {game:fmt.slug.into(),title:fmt.title.into(),release:format!("NTSC-U · {code}"),checksum_ok:Some(d[0x200..0x1ed0].iter().fold(0u8,|s,&b|s.wrapping_add(b))==0),profiles:vec![Profile {number:1,name:"Saved game".into(),empty:false,status:None,checksum_ok:None,fields,records:vec![]}],notes:vec!["Playtime sums 18 stored level counters at 30 ticks per second. Secrets count only the three documented dragon bits in the first 16 records; current-position statistics are separate. Inventory, health and checkpoint statistics are not decoded. Source/sample validation only; no emulator save/reload validation.".into()]})
}
#[cfg(test)]
mod tests {
    use super::super::{decode, GameDetails};
    fn fixture() -> Vec<u8> {
        let mut d = vec![0; 8192];
        d[..4].copy_from_slice(b"SC\x11\x01");
        d[4..18].copy_from_slice(b"Tomb Raider II");
        d[20] = 0x81;
        d[0x244..0x248].copy_from_slice(&54000u32.to_le_bytes());
        d[0x244 + 17 * 44..0x248 + 17 * 44].copy_from_slice(&54030u32.to_le_bytes());
        d[0x256] = 0xff;
        d[0x256 + 15 * 44] = 7;
        d[0x256 + 16 * 44] = 7;
        d[0x634..0x636].copy_from_slice(&17u16.to_le_bytes());
        d[0x64c] = 1;
        d[0x640] = 1;
        d[0x64b] = d[0x200..0x1ed0].iter().fold(0u8, |s, &v| s.wrapping_sub(v));
        d
    }
    #[test]
    fn tomb2_fields_integrity_unknowns_and_bounds() {
        let d = fixture();
        let before = d.clone();
        let GameDetails::Summary(s) = decode("SLUS-00437", "BA", "TOMB2", &d).unwrap() else {
            panic!()
        };
        assert_eq!(s.checksum_ok, Some(true));
        assert_eq!(
            s.profiles[0]
                .fields
                .iter()
                .map(|f| f.value.as_str())
                .collect::<Vec<_>>(),
            ["01:00:01", "6", "Home Sweet Home", "Yes"]
        );
        assert_eq!(d, before);
        let mut bad = d.clone();
        bad[0x635] = 0xff;
        bad[0x640] = 9;
        let GameDetails::Summary(s) = decode("SLUS-00437", "BA", "TOMB2", &bad).unwrap() else {
            panic!()
        };
        assert_eq!(s.checksum_ok, Some(false));
        assert_eq!(s.profiles[0].fields[2].value, "Unknown level (65297)");
        assert_eq!(s.profiles[0].fields[3].value, "Unknown (9)");
        for (marker, expected) in [(0, "Not recorded"), (9, "Unknown marker (9)")] {
            let mut variant = d.clone();
            variant[0x64c] = marker;
            let GameDetails::Summary(s) = decode("SLUS-00437", "BA", "TOMB2", &variant).unwrap()
            else {
                panic!()
            };
            assert_eq!(s.profiles[0].fields[2].value, expected);
        }
        for n in 0..8192 {
            assert!(decode("SLUS-00437", "BA", "TOMB2", &d[..n]).is_err());
        }
        for (code, region, id) in [
            ("SLES-00718", "BE", "TOMB2"),
            ("SLUS-00437", "BI", "TOMB2"),
            ("SLUS-00437", "BA", "TOMB2x"),
            ("SLUS-00437", "BA", "TOMBRAID"),
        ] {
            assert!(decode(code, region, id, &d).is_err());
        }
        for o in [0, 2, 3, 4] {
            let mut bad = d.clone();
            bad[o] ^= 1;
            assert!(decode("SLUS-00437", "BA", "TOMB2", &bad).is_err());
        }
        let mut bad = d;
        bad.push(0);
        assert!(decode("SLUS-00437", "BA", "TOMB2", &bad).is_err());
    }
    #[test]
    #[ignore = "Requires GAME_TOOLS_COLLECTION at the pinned commit; no saves redistributed"]
    fn tomb2_public_cards() {
        use crate::card::engine::Ps1Card;
        use sha2::{Digest, Sha256};
        let root = std::path::PathBuf::from(std::env::var("GAME_TOOLS_COLLECTION").unwrap())
            .join("tests/saves/tomb-raider-ii-starring-lara-croft/playstation");
        for (file, sha) in [
            (
                "usa.mcr",
                "34ca73e5715684fe8f2e0e618895c1326b0fe362b8fe1948ba2e8ef0f09377e2",
            ),
            (
                "usa-rev1.mcr",
                "5fae5f2b69ce3c04c9c14d66dfc7166fff1bc944d5683a5ed98bfb7db931c6ff",
            ),
            (
                "usa-rev2.mcr",
                "58c422e298de4ece374c726a349591275afec864e48d51ca15e53b53a40e7e8d",
            ),
            (
                "usa-rev3.mcr",
                "684c2f0499f698c51f10a7f713c21b54561112fce56b77f646c50dc29a8fb5e2",
            ),
        ] {
            let d = std::fs::read(root.join(file)).unwrap();
            assert_eq!(format!("{:x}", Sha256::digest(&d)), sha);
            let card = Ps1Card::open(&d, file, false).unwrap();
            let view = card.view();
            assert_eq!(view.saves.len(), 1);
            let Some(GameDetails::Summary(s)) = &view.saves[0].game_details else {
                panic!("{:?}", view.saves[0].game_details_error)
            };
            assert_eq!(s.checksum_ok, Some(true));
            assert_eq!(s.profiles[0].fields[2].value, "Venice");
            assert_eq!(s.profiles[0].fields[0].value, "00:00:00");
            assert_eq!(card.save_raw(false), d);
        }
    }
}
