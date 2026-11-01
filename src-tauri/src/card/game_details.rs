//! Read-only dispatch for original PS1 saves. Offsets are relative to reconstructed
//! SC payloads, never card positions or MCS/container headers. See docs/save-details.md
//! for pinned layout sources, licenses, supported releases and validation boundaries.
use serde::Serialize;

mod ape_escape;
mod archive_summaries;
mod bomberman_fantasy_race;
mod crash_bandicoot;
mod grandia;
mod legend_of_dragoon;
mod level_summaries;
mod mega_man_legends2;
mod metal_gear_solid;
mod overnight;
mod pacman_us;
mod rayman;
mod resident_evil;
mod summaries;
mod tekken2;
mod thps;
mod tomb_raider2;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum GameDetails {
    Digimon(super::digimon_world2::GameDetails),
    Summary(SaveDetails),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDetails {
    pub game: String,
    pub title: String,
    pub release: String,
    /// None means no verified checksum algorithm, never a passing checksum.
    pub checksum_ok: Option<bool>,
    pub profiles: Vec<Profile>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub number: u8,
    pub name: String,
    pub empty: bool,
    /// Additive tri-state occupancy for formats that can distinguish unknown slots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ProfileStatus>,
    pub checksum_ok: Option<bool>,
    pub fields: Vec<Field>,
    pub records: Vec<Record>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileStatus {
    Occupied,
    Empty,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Record {
    pub name: String,
    pub fields: Vec<Field>,
}

pub(super) struct Format {
    pub slug: &'static str,
    pub title: &'static str,
    pub blocks: usize,
    pub region: &'static str,
    pub identifier_prefix: &'static str,
}

impl Format {
    pub(super) fn accepts_identifier(&self, identifier: &str) -> bool {
        let Some(suffix) = identifier.strip_prefix(self.identifier_prefix) else {
            return false;
        };
        match self.slug {
            "tony-hawks-pro-skater" => matches!(suffix, "THPSGN0" | "THPSGCL"),
            "crash-bandicoot" => matches!(suffix, "00867900" | "01313863"),
            "digimon-world-2" => true, // Existing pilot validates its own layout.
            "azure-dreams" => suffix == "00",
            "yu-gi-oh-forbidden-memories" => suffix.is_empty(),
            "klonoa" => suffix.is_empty(),
            "digimon-digital-card-battle" => suffix == "_A",
            "mega-man-legends" => matches!(suffix, "-DASH00" | "-DASH01"),
            "kula-world" => suffix.is_empty(),
            "tomb-raider" => suffix.is_empty(),
            // The 8-byte suffix is the save id (`aaa63592`, `smd56832`), not a fixed token.
            "rayman" => true,
            // Story files are `drgn` plus a four-digit suffix from 0000 through 0255.
            "legend-of-dragoon" => {
                suffix.len() == 4
                    && suffix.bytes().all(|byte| byte.is_ascii_digit())
                    && suffix.parse::<u8>().is_ok()
            }
            // Story files are `G` plus seven printable bytes. VR training starts with `V`.
            "metal-gear-solid" => {
                identifier.len() == 8
                    && identifier.starts_with('G')
                    && identifier.bytes().all(|byte| (0x20..0x7f).contains(&byte))
            }
            "final-fantasy-vii" | "final-fantasy-viii" => {
                suffix.len() == 2
                    && suffix.bytes().all(|b| b.is_ascii_digit())
                    && matches!(suffix.parse::<u8>(), Ok(1..=15))
            }
            "final-fantasy-ix" | "chrono-cross" | "silent-hill" => {
                suffix.len() == 2
                    && suffix.bytes().all(|b| b.is_ascii_digit())
                    && matches!(suffix.parse::<u8>(), Ok(0..=14))
            }
            "castlevania-symphony-of-the-night" => {
                suffix.len() == 2 && suffix.bytes().all(|b| b.is_ascii_digit())
            }
            "final-fantasy-tactics" => {
                suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'A'..=b'O')
            }
            "mega-man-legends-2" => {
                suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'0'..=b'4')
            }
            _ => suffix.is_empty(),
        }
    }
}

pub(super) fn format(code: &str) -> Option<Format> {
    let (slug, title, blocks, region, identifier_prefix) = match code {
        "SCUS-94491" => (
            "legend-of-dragoon",
            "The Legend of Dragoon",
            1,
            "BA",
            "drgn",
        ),
        "SCUSP94457" => ("grandia", "Grandia", 1, "BA", "GRA-S01"),
        "SLUS-00860" => (
            "tony-hawks-pro-skater",
            "Tony Hawk's Pro Skater",
            1,
            "BA",
            "",
        ),
        "SLUS-00170" => ("resident-evil", "Resident Evil", 1, "BA", "1"),
        "SLUS-01193" => ("digimon-world-2", "Digimon World 2", 2, "BA", ""),
        "SCES-01000" => ("kula-world", "Kula World", 1, "BE", "KULA"),
        "SLUS-00437" => ("tomb-raider-2", "Tomb Raider II", 1, "BA", "TOMB2"),
        "SLES-00024" => ("tomb-raider", "Tomb Raider", 1, "BE", "TOMBRAID"),
        "SLUS-00152" => ("tomb-raider", "Tomb Raider", 1, "BA", "TOMBRAID"),
        // Observed USA directory prefix is BI. BA is also accepted; see rayman::decode.
        "SLUS-00005" => ("rayman", "Rayman", 1, "BI", ""),
        "SLUS-00594" => ("metal-gear-solid", "Metal Gear Solid", 1, "BA", ""),
        "SLES-03900" => (
            "digimon-digital-card-battle",
            "Digimon Digital Card Battle",
            2,
            "BE",
            "",
        ),
        "SCUS-94900" => ("crash-bandicoot", "Crash Bandicoot", 1, "BA", ""),
        "SCUS-94570" => ("crash-bash", "Crash Bash", 1, "BA", ""),
        "SCES-00934" => ("pac-man-world", "Pac-Man World", 1, "BE", "PACMAN20"),
        "SLUS-00439" => ("pac-man-world", "Pac-Man World", 1, "BA", "PACMAN20"),
        "SLUS-00823" => (
            "bomberman-fantasy-race",
            "Bomberman Fantasy Race",
            1,
            "BA",
            "SYSTEM",
        ),
        "SLUS-00603" => ("mega-man-legends", "Mega Man Legends", 1, "BA", ""),
        "SLES-01219" => ("azure-dreams", "Azure Dreams", 3, "BE", ""),
        "SLUS-01411" => (
            "yu-gi-oh-forbidden-memories",
            "Yu-Gi-Oh! Forbidden Memories",
            1,
            "BA",
            "-YUGIOH",
        ),
        "SLUS-00585" => ("klonoa", "Klonoa: Door to Phantomile", 1, "BA", "KLONOA"),
        "SCES-00867" => ("final-fantasy-vii", "Final Fantasy VII", 1, "BE", "FF7-S"),
        "SLUSP00892" => ("final-fantasy-viii", "Final Fantasy VIII", 1, "BA", "0426"),
        "SLES-02965" => ("final-fantasy-ix", "Final Fantasy IX", 1, "BE", "00000-"),
        "SCUS-94221" => (
            "final-fantasy-tactics",
            "Final Fantasy Tactics",
            1,
            "BA",
            "FFT",
        ),
        "SLUSP01041" => ("chrono-cross", "Chrono Cross", 1, "BA", "USCHRO"),
        "SLUS-00067" => (
            "castlevania-symphony-of-the-night",
            "Castlevania: Symphony of the Night",
            1,
            "BA",
            "DRAX",
        ),
        "SCUS-94194" => ("gran-turismo", "Gran Turismo", 5, "BA", "GT"),
        "SCES-02380" => ("gran-turismo-2", "Gran Turismo 2", 4, "BE", "GAME"),
        "SCUS-94426" => (
            "ctr-crash-team-racing",
            "CTR: Crash Team Racing",
            1,
            "BA",
            "-SLOTS",
        ),
        "SCUS-94228" => ("spyro-the-dragon", "Spyro the Dragon", 1, "BA", "SPYRO"),
        "SCUS-94423" => ("ape-escape", "Ape Escape", 1, "BA", "SYS"),
        "SLUS-00213" => ("tekken-2", "Tekken 2", 1, "BA", "TEKKEN-2"),
        "SLUS-00402" => ("tekken-3", "Tekken 3", 1, "BA", "TEKKEN-3"),
        "SLUS-00707" => ("silent-hill", "Silent Hill", 1, "BA", "SILENT"),
        "SLUS-01140" => (
            "mega-man-legends-2",
            "Mega Man Legends 2",
            1,
            "BA",
            "-DASH2",
        ),
        _ => return None,
    };
    Some(Format {
        slug,
        title,
        blocks,
        region,
        identifier_prefix,
    })
}

fn region_accepted(code: &str, region: &str, format: &Format) -> bool {
    if code == "SLUS-00005" {
        // The USA disc and the GameFAQs sample both write BI. BA is accepted and unobserved.
        return region == "BI" || region == "BA";
    }
    region == format.region
}

pub(super) fn decode(
    code: &str,
    region: &str,
    identifier: &str,
    data: &[u8],
) -> Result<GameDetails, String> {
    let format = format(code).ok_or("This release does not have a verified decoder yet.")?;
    if !region_accepted(code, region, &format) {
        return Err("The save region does not match the supported product code. Game details are unavailable for this variant.".into());
    }
    if !format.accepts_identifier(identifier) {
        return Err("This file is not a supported adventure/profile save (it may contain settings or a replay).".into());
    }
    // The exact US Pac-Man writer deliberately emits this non-SC header.
    // Keep the exception confined to its release, after region/identifier checks.
    let header_matches = if code == "SLUS-00439" {
        data.starts_with(&[0x43, 0x00, 0x13, 0x01])
    } else {
        data.starts_with(b"SC") && data.get(3).copied().map(usize::from) == Some(format.blocks)
    };
    if data.len() != format.blocks * 8192 || !header_matches {
        return Err(
            "This save does not have the expected game layout. Try another backup of this save."
                .into(),
        );
    }
    if code == "SLUS-00005" {
        return rayman::decode(&format, code, region, data).map(GameDetails::Summary);
    }
    if code == "SLUS-00594" {
        return metal_gear_solid::decode(&format, code, identifier, data).map(GameDetails::Summary);
    }
    if code == "SLUS-00213" {
        return tekken2::decode(&format, code, data).map(GameDetails::Summary);
    }
    if code == "SCUS-94900" {
        return crash_bandicoot::decode(&format, code, data).map(GameDetails::Summary);
    }
    if code == "SLUS-00437" {
        return tomb_raider2::decode(&format, code, data).map(GameDetails::Summary);
    }
    if code == "SCUS-94491" {
        return legend_of_dragoon::decode(&format, code, identifier, data)
            .map(GameDetails::Summary);
    }
    if code == "SCUSP94457" {
        return grandia::decode(&format, code, data).map(GameDetails::Summary);
    }
    if code == "SLUS-00860" {
        return thps::decode(&format, code, data).map(GameDetails::Summary);
    }
    if code == "SLUS-00170" {
        return resident_evil::decode(&format, code, data).map(GameDetails::Summary);
    }
    if code == "SLUS-00823" {
        return bomberman_fantasy_race::decode(&format, code, data).map(GameDetails::Summary);
    }
    if code == "SLUS-00439" {
        return pacman_us::decode(&format, code, data).map(GameDetails::Summary);
    }
    if format.slug == "digimon-world-2" {
        return super::digimon_world2::decode(data).map(GameDetails::Digimon);
    }
    if format.slug == "mega-man-legends-2" {
        return mega_man_legends2::decode(&format, code, identifier, data)
            .map(GameDetails::Summary);
    }
    if format.slug == "ape-escape" {
        return ape_escape::decode(&format, code, data).map(GameDetails::Summary);
    }
    if matches!(
        format.slug,
        "azure-dreams" | "yu-gi-oh-forbidden-memories" | "klonoa"
    ) {
        return overnight::decode(&format, code, data).map(GameDetails::Summary);
    }
    if matches!(
        format.slug,
        "digimon-digital-card-battle" | "crash-bash" | "pac-man-world" | "mega-man-legends"
    ) {
        return archive_summaries::decode(&format, code, data).map(GameDetails::Summary);
    }
    if matches!(format.slug, "kula-world" | "tomb-raider") {
        return level_summaries::decode(&format, code, data).map(GameDetails::Summary);
    }
    summaries::decode(&format, code, identifier, data).map(GameDetails::Summary)
}

#[cfg(test)]
mod tests;
