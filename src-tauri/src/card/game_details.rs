//! Read-only dispatch for original PS1 saves. Offsets are relative to reconstructed
//! SC payloads, never card positions or MCS/container headers. See docs/save-details.md
//! for pinned layout sources, licenses, supported releases and validation boundaries.
use serde::Serialize;

mod summaries;

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
    pub checksum_ok: Option<bool>,
    pub fields: Vec<Field>,
    pub records: Vec<Record>,
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
            "digimon-world-2" => true, // Existing pilot validates its own layout.
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
            _ => suffix.is_empty(),
        }
    }
}

pub(super) fn format(code: &str) -> Option<Format> {
    let (slug, title, blocks, region, identifier_prefix) = match code {
        "SLUS-01193" => ("digimon-world-2", "Digimon World 2", 2, "BA", ""),
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
        "SLUS-00402" => ("tekken-3", "Tekken 3", 1, "BA", "TEKKEN-3"),
        "SLUS-00707" => ("silent-hill", "Silent Hill", 1, "BA", "SILENT"),
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

pub(super) fn decode(
    code: &str,
    region: &str,
    identifier: &str,
    data: &[u8],
) -> Result<GameDetails, String> {
    let format = format(code).ok_or("This release does not have a verified decoder yet.")?;
    if region != format.region {
        return Err("The save region does not match the supported product code. Game details are unavailable for this variant.".into());
    }
    if !format.accepts_identifier(identifier) {
        return Err("This file is not a supported adventure/profile save (it may contain settings or a replay).".into());
    }
    if data.len() != format.blocks * 8192
        || !data.starts_with(b"SC")
        || data[3] as usize != format.blocks
    {
        return Err(
            "This save does not have the expected game layout. Try another backup of this save."
                .into(),
        );
    }
    if format.slug == "digimon-world-2" {
        return super::digimon_world2::decode(data).map(GameDetails::Digimon);
    }
    summaries::decode(&format, code, identifier, data).map(GameDetails::Summary)
}

#[cfg(test)]
mod tests;
