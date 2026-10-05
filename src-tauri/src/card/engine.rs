use encoding_rs::SHIFT_JIS;
use serde::Serialize;
use unicode_normalization::UnicodeNormalization;

pub const SLOT_COUNT: usize = 15;
pub const CARD_SIZE: usize = 131_072;
pub const HEADER_SIZE: usize = 128;
pub const BLOCK_SIZE: usize = 8192;
pub const MCS_HEADER_SIZE: usize = 128;
pub const GME_HEADER_SIZE: usize = 3904;
pub const VGS_HEADER_SIZE: usize = 64;
pub const GME_SIZE: usize = GME_HEADER_SIZE + CARD_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardError(pub String);

impl std::fmt::Display for CardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CardError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotType {
    Formatted,
    Initial,
    MiddleLink,
    EndLink,
    DeletedInitial,
    DeletedMiddleLink,
    DeletedEndLink,
    Corrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataKind {
    Save,
    Software,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardFormat {
    Raw,
    Gme,
    Vgs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardSource {
    File,
    Usb,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotInfo {
    pub index: u8,
    #[serde(rename = "type")]
    pub slot_type: SlotType,
    pub next: Option<u8>,
    pub xor_ok: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInfo {
    pub master_slot: u8,
    pub linked_slots: Vec<u8>,
    pub title: String,
    pub region: String,
    pub region_raw: String,
    pub prod_code: String,
    pub identifier: String,
    pub size_kb: u32,
    pub kind: DataKind,
    pub deleted: bool,
    pub frame_count: u8,
    pub frames: Vec<Vec<u8>>,
    pub game_details: Option<super::game_details::GameDetails>,
    pub game_details_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardView {
    pub source_name: String,
    pub image_id: String,
    pub format: CardFormat,
    pub source: CardSource,
    pub slots: Vec<SlotInfo>,
    pub saves: Vec<SaveInfo>,
    pub used_blocks: u8,
}

fn slot_type_from_byte(b: u8) -> SlotType {
    match b {
        0xa0 => SlotType::Formatted,
        0x51 => SlotType::Initial,
        0x52 => SlotType::MiddleLink,
        0x53 => SlotType::EndLink,
        0xa1 => SlotType::DeletedInitial,
        0xa2 => SlotType::DeletedMiddleLink,
        0xa3 => SlotType::DeletedEndLink,
        _ => SlotType::Corrupted,
    }
}

fn is_master_type(t: SlotType) -> bool {
    matches!(t, SlotType::Initial | SlotType::DeletedInitial)
}

fn is_link_type(t: SlotType) -> bool {
    matches!(
        t,
        SlotType::MiddleLink
            | SlotType::EndLink
            | SlotType::DeletedMiddleLink
            | SlotType::DeletedEndLink
    )
}

fn decode_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .copied()
        .filter(|&b| b != 0)
        .map(|b| b as char)
        .collect()
}

fn decode_shift_jis_title(save_block: &[u8]) -> String {
    let mut raw = Vec::with_capacity(64);
    for i in 0..64 {
        let b = save_block[4 + i];
        if i % 2 == 0 && b == 0 {
            break;
        }
        raw.push(b);
    }
    let (cow, _, _) = SHIFT_JIS.decode(&raw);
    cow.nfkc().collect::<String>().replace('\0', "")
}

fn region_name(raw: &str) -> String {
    match raw {
        "BA" => "America".into(),
        "BE" => "Europe".into(),
        "BI" => "Japan".into(),
        other => other.to_string(),
    }
}

fn xor_header(header: &[u8]) -> u8 {
    let mut sum = 0u8;
    for b in header.iter().take(127) {
        sum ^= *b;
    }
    sum
}

fn detect_magic(bytes: &[u8]) -> String {
    let n = bytes.len().min(11);
    let ascii: String = bytes[..n]
        .iter()
        .copied()
        .filter(|&b| b != 0 && b != 1 && b != 0x3f)
        .filter(|&b| (0x20..0x7f).contains(&b))
        .map(|b| b as char)
        .collect();
    if !ascii.is_empty() {
        return ascii;
    }
    bytes
        .iter()
        .take(4.min(bytes.len()))
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn load_palette(block: &[u8]) -> [[u8; 4]; 16] {
    let mut palette = [[0u8; 4]; 16];
    for i in 0..16 {
        let lo = block[96 + i * 2];
        let hi = block[96 + i * 2 + 1];
        let r = (lo & 0x1f) << 3;
        let g = ((hi & 0x3) << 6) | ((lo & 0xe0) >> 2);
        let b = (hi & 0x7c) << 1;
        let black_flag = hi & 0x80;
        if (r | g | b | black_flag) == 0 {
            palette[i] = [0, 0, 0, 0];
        } else {
            palette[i] = [r, g, b, 255];
        }
    }
    palette
}

fn load_frames(block: &[u8], frame_count: u8) -> Vec<Vec<u8>> {
    let palette = load_palette(block);
    let count = frame_count.clamp(0, 3) as usize;
    let mut frames = Vec::with_capacity(count);
    for icon in 0..count {
        let mut rgba = vec![0u8; 16 * 16 * 4];
        let mut byte_count = 128 + 128 * icon;
        for y in 0..16 {
            for x in (0..16).step_by(2) {
                let packed = block[byte_count];
                byte_count += 1;
                let left = palette[(packed & 0xf) as usize];
                let right = palette[(packed >> 4) as usize];
                let i0 = (x + y * 16) * 4;
                rgba[i0..i0 + 4].copy_from_slice(&left);
                rgba[i0 + 4..i0 + 8].copy_from_slice(&right);
            }
        }
        frames.push(rgba);
    }
    frames
}

fn frame_count_from_save(block: &[u8]) -> u8 {
    match block[2] {
        0x11 => 1,
        0x12 => 2,
        0x13 => 3,
        _ => 0,
    }
}

fn strip_extension(name: &str) -> String {
    std::path::Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Untitled")
        .to_string()
}

/// PS1 memory card engine, ported from MemcardRex `ps1card.cs`.
///
/// Open with fix_data=false so XOR / FreePSXBoot cards are not rewritten.
/// Compose packs selected master saves onto a blank card from slot 0.
#[derive(Debug)]
pub struct Ps1Card {
    pub raw: Vec<u8>,
    pub source_name: String,
    pub format: CardFormat,
    pub source: CardSource,
    slot_type: [SlotType; SLOT_COUNT],
    master_slot: [u8; SLOT_COUNT],
    gme_comments: [String; SLOT_COUNT],
}

impl Ps1Card {
    pub fn create_formatted(name: &str) -> Self {
        let mut card = Self {
            raw: vec![0; CARD_SIZE],
            source_name: name.to_string(),
            format: CardFormat::Raw,
            source: CardSource::File,
            slot_type: [SlotType::Formatted; SLOT_COUNT],
            master_slot: core::array::from_fn(|i| i as u8),
            gme_comments: core::array::from_fn(|_| String::new()),
        };
        card.write_authentic_shell();
        card.format_all();
        card
    }

    pub fn open(bytes: &[u8], source_name: &str, fix_data: bool) -> Result<Self, CardError> {
        Self::open_from(bytes, source_name, fix_data, CardSource::File)
    }

    pub fn open_from(
        bytes: &[u8],
        source_name: &str,
        fix_data: bool,
        source: CardSource,
    ) -> Result<Self, CardError> {
        if bytes.len() < 2 {
            return Err(CardError("File is too small to be a Memory Card.".into()));
        }

        let magic = detect_magic(bytes);
        if magic.starts_with("PMV") {
            return Err(CardError(
                "PSP .vmp is not in this slice (saw “PMV”).".into(),
            ));
        }

        let (format, offset) = if magic.starts_with("123-456-STD") {
            (CardFormat::Gme, GME_HEADER_SIZE)
        } else if magic.starts_with("VgsM") {
            (CardFormat::Vgs, VGS_HEADER_SIZE)
        } else if bytes.len() >= 2 && bytes[0] == 0x4d && bytes[1] == 0x43 {
            (CardFormat::Raw, 0usize)
        } else if bytes.len() >= GME_SIZE && bytes[3904] == b'M' && bytes[3905] == b'C' {
            (CardFormat::Gme, GME_HEADER_SIZE)
        } else {
            let shown = if magic.is_empty() {
                format!("{:x} {:x}", bytes[0], bytes[1])
            } else {
                magic
            };
            return Err(CardError(format!(
                "Not a supported Memory Card format (saw “{shown}”)."
            )));
        };

        if bytes.len() < offset + CARD_SIZE {
            return Err(CardError(format!(
                "Raw card is truncated ({} bytes, need {}).",
                bytes.len().saturating_sub(offset),
                CARD_SIZE
            )));
        }

        let mut card = Self {
            raw: bytes[offset..offset + CARD_SIZE].to_vec(),
            source_name: strip_extension(source_name),
            format,
            source,
            slot_type: [SlotType::Formatted; SLOT_COUNT],
            master_slot: core::array::from_fn(|i| i as u8),
            gme_comments: core::array::from_fn(|_| String::new()),
        };

        if format == CardFormat::Gme && bytes.len() >= GME_HEADER_SIZE {
            for slot in 0..SLOT_COUNT {
                let start = 64 + 256 * slot;
                let end = (start + 256).min(bytes.len().min(GME_HEADER_SIZE));
                if start < end {
                    let comment = decode_ascii(&bytes[start..end]);
                    card.gme_comments[slot] = comment.trim_end_matches('\0').to_string();
                }
            }
        }

        if fix_data {
            card.recalculate_xor();
        }
        card.refresh_directory();
        Ok(card)
    }

    fn header(&self, slot: usize) -> &[u8] {
        let start = HEADER_SIZE + slot * HEADER_SIZE;
        &self.raw[start..start + HEADER_SIZE]
    }

    fn header_mut(&mut self, slot: usize) -> &mut [u8] {
        let start = HEADER_SIZE + slot * HEADER_SIZE;
        &mut self.raw[start..start + HEADER_SIZE]
    }

    fn save_block(&self, slot: usize) -> &[u8] {
        let start = BLOCK_SIZE + slot * BLOCK_SIZE;
        &self.raw[start..start + BLOCK_SIZE]
    }

    fn save_block_mut(&mut self, slot: usize) -> &mut [u8] {
        let start = BLOCK_SIZE + slot * BLOCK_SIZE;
        &mut self.raw[start..start + BLOCK_SIZE]
    }

    pub fn find_save_links(&self, initial_slot: usize) -> Vec<usize> {
        let mut links = Vec::new();
        let mut current = initial_slot;
        for _ in 0..SLOT_COUNT {
            links.push(current);
            if self.slot_type[current] == SlotType::Corrupted {
                break;
            }
            let pointer = self.header(current)[8] as usize;
            if pointer == 0xff || pointer >= SLOT_COUNT {
                break;
            }
            if !is_link_type(self.slot_type[pointer]) {
                return links;
            }
            current = pointer;
        }
        links
    }

    pub fn get_save_bytes(&self, slot_number: usize) -> Vec<u8> {
        let links = self.find_save_links(slot_number);
        let mut out = vec![0u8; MCS_HEADER_SIZE + links.len() * BLOCK_SIZE];
        out[..HEADER_SIZE].copy_from_slice(self.header(links[0]));
        for (i, slot) in links.iter().enumerate() {
            let dest = MCS_HEADER_SIZE + i * BLOCK_SIZE;
            out[dest..dest + BLOCK_SIZE].copy_from_slice(self.save_block(*slot));
        }
        out
    }

    pub fn set_save_bytes(
        &mut self,
        slot_number: usize,
        save_bytes: &[u8],
    ) -> Result<usize, usize> {
        let slot_count = (save_bytes.len() - MCS_HEADER_SIZE) / BLOCK_SIZE;
        let free = self.find_free_slots(slot_number, slot_count);
        if free.len() < slot_count {
            return Err(slot_count);
        }

        let number_of_bytes = (slot_count * BLOCK_SIZE) as u32;
        {
            let first = self.header_mut(free[0]);
            let n = HEADER_SIZE.min(save_bytes.len());
            first[..n].copy_from_slice(&save_bytes[..n]);
            first[4] = (number_of_bytes & 0xff) as u8;
            first[5] = ((number_of_bytes >> 8) & 0xff) as u8;
            first[6] = ((number_of_bytes >> 16) & 0xff) as u8;
        }

        for i in 0..slot_count {
            let src_start = MCS_HEADER_SIZE + i * BLOCK_SIZE;
            let src_end = (src_start + BLOCK_SIZE).min(save_bytes.len());
            let dest = self.save_block_mut(free[i]);
            dest.fill(0);
            dest[..src_end - src_start].copy_from_slice(&save_bytes[src_start..src_end]);
        }

        for i in 0..free.len().saturating_sub(1) {
            let next = free[i + 1] as u8;
            let h = self.header_mut(free[i]);
            h[0] = 0x52;
            h[8] = next;
            h[9] = 0x00;
        }

        let last_idx = *free.last().unwrap();
        {
            let last = self.header_mut(last_idx);
            last[0] = 0x53;
            last[8] = 0xff;
            last[9] = 0xff;
        }
        self.header_mut(free[0])[0] = 0x51;

        self.recalculate_xor();
        self.refresh_directory();
        Ok(slot_count)
    }

    pub fn compose_from(
        &mut self,
        source: &Ps1Card,
        master_slots: &[usize],
    ) -> Result<(), CardError> {
        let needed: usize = master_slots
            .iter()
            .map(|slot| source.find_save_links(*slot).len())
            .sum();
        if needed > SLOT_COUNT {
            return Err(CardError(format!(
                "Selection needs {needed} blocks; a card only has {SLOT_COUNT}."
            )));
        }
        self.write_authentic_shell();
        self.format_all();
        for slot in master_slots {
            let payload = source.get_save_bytes(*slot);
            if let Err(req) = self.set_save_bytes(0, &payload) {
                return Err(CardError(format!("Not enough free slots (need {req}).")));
            }
        }
        Ok(())
    }

    pub fn save_raw(&self, fix_data: bool) -> Vec<u8> {
        if !fix_data {
            return self.raw.clone();
        }
        let mut out = vec![0u8; CARD_SIZE];
        out[0] = 0x4d;
        out[1] = 0x43;
        out[127] = 0x0e;
        out[8064] = 0x4d;
        out[8065] = 0x43;
        out[8191] = 0x0e;
        for i in 0..20 {
            let base = 2048 + i * HEADER_SIZE;
            out[base] = 0xff;
            out[base + 1] = 0xff;
            out[base + 2] = 0xff;
            out[base + 3] = 0xff;
            out[base + 8] = 0xff;
            out[base + 9] = 0xff;
        }
        for slot in 0..SLOT_COUNT {
            let h = HEADER_SIZE + slot * HEADER_SIZE;
            out[h..h + HEADER_SIZE].copy_from_slice(self.header(slot));
            let b = BLOCK_SIZE + slot * BLOCK_SIZE;
            out[b..b + BLOCK_SIZE].copy_from_slice(self.save_block(slot));
        }
        out
    }

    fn gme_header(&self, raw: &[u8]) -> Vec<u8> {
        let mut header = vec![0u8; GME_HEADER_SIZE];
        header[0] = b'1';
        header[1] = b'2';
        header[2] = b'3';
        header[3] = b'-';
        header[4] = b'4';
        header[5] = b'5';
        header[6] = b'6';
        header[7] = b'-';
        header[8] = b'S';
        header[9] = b'T';
        header[10] = b'D';
        header[18] = 0x1;
        header[20] = 0x1;
        header[21] = b'M';
        for slot in 0..SLOT_COUNT {
            let slot_header =
                &raw[HEADER_SIZE + slot * HEADER_SIZE..HEADER_SIZE + (slot + 1) * HEADER_SIZE];
            header[22 + slot] = slot_header[0];
            header[38 + slot] = slot_header[8];
            let comment = self.gme_comments[slot].as_bytes();
            let dest = 64 + 256 * slot;
            let n = comment.len().min(255);
            header[dest..dest + n].copy_from_slice(&comment[..n]);
        }
        header
    }

    fn vgs_header() -> Vec<u8> {
        let mut header = vec![0u8; VGS_HEADER_SIZE];
        header[0] = b'V';
        header[1] = b'g';
        header[2] = b's';
        header[3] = b'M';
        header[4] = 0x1;
        header[8] = 0x1;
        header[12] = 0x1;
        header[17] = 0x2;
        header
    }

    pub fn export(&self, format: CardFormat, fix_data: bool) -> Vec<u8> {
        let raw = self.save_raw(fix_data);
        match format {
            CardFormat::Raw => raw,
            CardFormat::Gme => {
                let mut out = self.gme_header(&raw);
                out.extend_from_slice(&raw);
                out
            }
            CardFormat::Vgs => {
                let mut out = Self::vgs_header();
                out.extend_from_slice(&raw);
                out
            }
        }
    }

    /// Content identity of this image, not a physical card serial number.
    pub fn image_id(&self) -> String {
        super::snapshot::image_id(&self.raw)
    }

    pub fn view(&self) -> CardView {
        let mut slots = Vec::with_capacity(SLOT_COUNT);
        let mut saves = Vec::new();
        let mut used_blocks = 0u8;

        for i in 0..SLOT_COUNT {
            let header = self.header(i);
            let pointer = header[8];
            let xor_ok = xor_header(header) == header[127];
            slots.push(SlotInfo {
                index: i as u8,
                slot_type: self.slot_type[i],
                next: if pointer == 0xff || pointer as usize >= SLOT_COUNT {
                    None
                } else {
                    Some(pointer)
                },
                xor_ok,
            });
            if self.slot_type[i] != SlotType::Formatted {
                used_blocks += 1;
            }
        }

        for i in 0..SLOT_COUNT {
            if !is_master_type(self.slot_type[i]) {
                continue;
            }
            let linked = self.find_save_links(i);
            let header = self.header(i);
            let block = self.save_block(i);
            let region_raw = decode_ascii(&header[10..12]);
            let prod_code = decode_ascii(&header[12..22]);
            let identifier = decode_ascii(&header[22..30]);
            let size_bytes =
                u32::from(header[4]) | (u32::from(header[5]) << 8) | (u32::from(header[6]) << 16);
            let frame_count = frame_count_from_save(block);
            let kind = if header[0x10] == 0x50
                && block[0x52] == 0x4d
                && block[0x53] == 0x43
                && block[0x54] == 0x58
                && (block[0x55] == 0x30 || block[0x55] == 0x31)
            {
                DataKind::Software
            } else {
                DataKind::Save
            };
            let mut title = decode_shift_jis_title(block);
            title = title.trim().to_string();
            if title.is_empty() {
                title = if prod_code.is_empty() {
                    format!("Slot {}", i + 1)
                } else {
                    prod_code.clone()
                };
            }
            let (game_details, game_details_error) = if let Some(format) =
                super::game_details::format(&prod_code)
            {
                let chain_valid = linked.len() == format.blocks
                    && header[7] == 0 // A PS1 save size must fit the supported card geometry.
                    && size_bytes as usize == format.blocks * BLOCK_SIZE
                    && linked.iter().enumerate().all(|(index, &slot)| {
                        let h = self.header(slot);
                        let pointer = u16::from_le_bytes([h[8], h[9]]);
                        let deleted = self.slot_type[i] == SlotType::DeletedInitial;
                        let expected_type = if index == 0 {
                            if deleted {
                                0xa1
                            } else {
                                0x51
                            }
                        } else if index + 1 == linked.len() {
                            if deleted {
                                0xa3
                            } else {
                                0x53
                            }
                        } else if deleted {
                            0xa2
                        } else {
                            0x52
                        };
                        h[0] == expected_type
                            && if index + 1 == linked.len() {
                                pointer == 0xffff
                            } else {
                                pointer as usize == linked[index + 1]
                            }
                    })
                    && linked
                        .iter()
                        .enumerate()
                        .all(|(index, slot)| !linked[..index].contains(slot));
                if !format.accepts_identifier(&identifier) {
                    (None, Some("This file is not a supported adventure/profile save (it may contain settings or a replay).".into()))
                } else if !chain_valid {
                    (None, Some("This save has an incomplete or unexpected block chain. Try another backup of this save.".into()))
                } else {
                    let bytes = self.get_save_bytes(i);
                    match super::game_details::decode(
                        &prod_code,
                        &region_raw,
                        &identifier,
                        &bytes[MCS_HEADER_SIZE..],
                    ) {
                        Ok(details) => (Some(details), None),
                        Err(error) => (None, Some(error)),
                    }
                }
            } else {
                (None, None)
            };
            saves.push(SaveInfo {
                master_slot: i as u8,
                linked_slots: linked.iter().map(|s| *s as u8).collect(),
                title,
                region: region_name(&region_raw),
                region_raw,
                prod_code,
                identifier,
                size_kb: if size_bytes == 0 {
                    (linked.len() * 8) as u32
                } else {
                    size_bytes / 1024
                },
                kind,
                deleted: self.slot_type[i] == SlotType::DeletedInitial,
                frame_count,
                frames: load_frames(block, frame_count),
                game_details,
                game_details_error,
            });
        }

        CardView {
            source_name: self.source_name.clone(),
            image_id: self.image_id(),
            format: self.format,
            source: self.source,
            slots,
            saves,
            used_blocks,
        }
    }

    fn write_authentic_shell(&mut self) {
        self.raw = vec![0u8; CARD_SIZE];
        self.raw[0] = 0x4d;
        self.raw[1] = 0x43;
        self.raw[127] = 0x0e;
        self.raw[8064] = 0x4d;
        self.raw[8065] = 0x43;
        self.raw[8191] = 0x0e;
        for i in 0..20 {
            let base = 2048 + i * HEADER_SIZE;
            self.raw[base] = 0xff;
            self.raw[base + 1] = 0xff;
            self.raw[base + 2] = 0xff;
            self.raw[base + 3] = 0xff;
            self.raw[base + 8] = 0xff;
            self.raw[base + 9] = 0xff;
        }
    }

    fn format_slot(&mut self, slot: usize) {
        self.header_mut(slot).fill(0);
        self.save_block_mut(slot).fill(0);
        {
            let header = self.header_mut(slot);
            header[0] = 0xa0;
            header[8] = 0xff;
            header[9] = 0xff;
        }
        self.master_slot[slot] = slot as u8;
        self.gme_comments[slot].clear();
    }

    fn format_all(&mut self) {
        for i in 0..SLOT_COUNT {
            self.format_slot(i);
        }
        self.recalculate_xor();
        self.refresh_directory();
    }

    fn recalculate_xor(&mut self) {
        for i in 0..SLOT_COUNT {
            let x = xor_header(self.header(i));
            self.header_mut(i)[127] = x;
        }
    }

    fn refresh_directory(&mut self) {
        for i in 0..SLOT_COUNT {
            self.slot_type[i] = slot_type_from_byte(self.header(i)[0]);
            self.master_slot[i] = i as u8;
        }
        self.find_broken_links();
        for i in 0..SLOT_COUNT {
            if !is_master_type(self.slot_type[i]) {
                continue;
            }
            for linked in self.find_save_links(i) {
                self.master_slot[linked] = i as u8;
            }
        }
    }

    fn find_broken_links(&mut self) {
        let mut touched = [false; SLOT_COUNT];
        for i in 0..SLOT_COUNT {
            if !is_master_type(self.slot_type[i]) {
                continue;
            }
            for linked in self.find_save_links(i) {
                touched[linked] = true;
            }
        }
        for i in 0..SLOT_COUNT {
            if is_link_type(self.slot_type[i]) && !touched[i] {
                self.slot_type[i] = SlotType::Formatted;
            }
        }
    }

    fn find_free_slots(&self, start: usize, required: usize) -> Vec<usize> {
        let mut found = Vec::new();
        for i in 0..SLOT_COUNT {
            let slot = (i + start) % SLOT_COUNT;
            if self.slot_type[slot] == SlotType::Formatted {
                found.push(slot);
            }
            if found.len() == required {
                break;
            }
        }
        found
    }
}

pub fn compose_new_card(
    source: &Ps1Card,
    master_slots: &[usize],
    name: Option<&str>,
) -> Result<Ps1Card, CardError> {
    let dest_name = name
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}-composed", source.source_name));
    let mut dest = Ps1Card::create_formatted(&dest_name);
    dest.compose_from(source, master_slots)?;
    Ok(dest)
}

pub fn backup_bytes(card: &Ps1Card, format: CardFormat) -> Vec<u8> {
    card.export(format, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn blue() -> Vec<u8> {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../blue.mcr"))
            .expect("blue.mcr next to src-tauri")
    }

    #[test]
    fn open_blue_full_card() {
        let view = Ps1Card::open(&blue(), "blue.mcr", false).unwrap().view();
        assert_eq!(view.format, CardFormat::Raw);
        assert_eq!(view.slots.len(), 15);
        assert_eq!(view.used_blocks, 15);
        assert_eq!(view.saves.len(), 12);

        let by_slot = |n: u8| view.saves.iter().find(|s| s.master_slot == n).unwrap();

        let s0 = by_slot(0);
        assert!(s0.title.contains("PAC-MAN WORLD 20TH"));
        assert_eq!(s0.region, "America");
        assert_eq!(s0.prod_code, "SLUS-00439");
        assert_eq!(s0.linked_slots, vec![0]);
        assert_eq!(s0.size_kb, 8);
        assert_eq!(s0.frame_count, 3);
        assert!(!s0.deleted);

        let s1 = by_slot(1);
        assert_eq!(s1.title, "PAC-MAN WORLD");
        assert_eq!(s1.region, "Europe");
        assert_eq!(s1.linked_slots, vec![1]);

        let s8 = by_slot(8);
        assert_eq!(s8.title, "DigimonWorld2");
        assert_eq!(s8.linked_slots, vec![8, 9]);
        assert_eq!(s8.size_kb, 16);
        assert_eq!(view.slots[9].slot_type, SlotType::EndLink);

        let s11 = by_slot(11);
        assert_eq!(s11.title.trim(), "Worms World Party");
        assert_eq!(
            s11.linked_slots,
            vec![11, 12, 14],
            "non-contiguous Worms chain"
        );
        assert_eq!(s11.size_kb, 24);
        assert_eq!(view.slots[12].slot_type, SlotType::MiddleLink);
        assert_eq!(view.slots[14].slot_type, SlotType::EndLink);

        let s13 = by_slot(13);
        assert!(s13.title.contains("MEGAMAN LEGENDS"));
        assert_eq!(s13.linked_slots, vec![13]);

        assert!(by_slot(7).deleted);
        assert!(by_slot(7).title.contains("CTR"));
        assert!(by_slot(10).deleted);

        let regions: std::collections::HashSet<_> = view
            .saves
            .iter()
            .filter(|s| !s.deleted)
            .map(|s| s.region.as_str())
            .collect();
        assert!(regions.contains("America"));
        assert!(regions.contains("Europe"));

        assert!(view
            .saves
            .iter()
            .any(|s| s.linked_slots.len() == 1 && !s.deleted));
        assert!(view.saves.iter().any(|s| s.linked_slots.len() > 1));
        assert!(view
            .saves
            .iter()
            .all(|s| s.frames.len() == s.frame_count as usize));
        assert_eq!(view.saves[0].frames[0].len(), 16 * 16 * 4);
        assert!(view.slots.iter().all(|s| s.xor_ok));
    }

    #[test]
    fn rejects_unknown_magic() {
        let junk = vec![0x00, 0x01, 0x02, 0x03];
        let err = Ps1Card::open(&junk, "nope.bin", false).unwrap_err();
        assert!(err.0.to_lowercase().contains("saw"));
    }

    #[test]
    fn names_vmp_magic() {
        let mut vmp = vec![0u8; 16];
        vmp[0] = b'P';
        vmp[1] = b'M';
        vmp[2] = b'V';
        let err = Ps1Card::open(&vmp, "card.vmp", false).unwrap_err();
        assert!(err.0.contains("PMV"));
    }

    #[test]
    fn formatted_empty_card() {
        let view = Ps1Card::create_formatted("Untitled").view();
        assert_eq!(view.used_blocks, 0);
        assert!(view.saves.is_empty());
        assert!(view
            .slots
            .iter()
            .all(|s| s.slot_type == SlotType::Formatted && s.xor_ok));
    }

    #[test]
    fn compose_packs_from_slot_zero() {
        let source = Ps1Card::open(&blue(), "blue.mcr", false).unwrap();
        let composed = compose_new_card(&source, &[0, 8, 11], None).unwrap();
        let view = composed.view();
        let raw = composed.save_raw(true);

        assert_eq!(raw.len(), CARD_SIZE);
        assert_eq!(&raw[0..2], b"MC");
        assert_eq!(view.saves.len(), 3);
        assert_eq!(view.saves[0].master_slot, 0);
        assert!(view.saves[0].title.contains("PAC-MAN WORLD 20TH"));
        assert_eq!(view.saves[0].linked_slots, vec![0]);
        assert_eq!(view.saves[1].title, "DigimonWorld2");
        assert_eq!(view.saves[1].linked_slots, vec![1, 2]);
        assert_eq!(view.saves[2].title.trim(), "Worms World Party");
        assert_eq!(view.saves[2].linked_slots, vec![3, 4, 5]);
        assert_eq!(view.used_blocks, 6);
        assert_eq!(view.slots[6].slot_type, SlotType::Formatted);

        let round_trip = Ps1Card::open(&raw, "composed.mcr", false).unwrap().view();
        assert_eq!(
            round_trip
                .saves
                .iter()
                .map(|s| s.title.clone())
                .collect::<Vec<_>>(),
            view.saves
                .iter()
                .map(|s| s.title.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            round_trip
                .saves
                .iter()
                .map(|s| s.linked_slots.len())
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            round_trip
                .saves
                .iter()
                .map(|s| s.region.clone())
                .collect::<Vec<_>>(),
            vec!["America", "America", "America"]
        );
    }

    #[test]
    fn compose_fails_over_fifteen_blocks() {
        let source = Ps1Card::open(&blue(), "blue.mcr", false).unwrap();
        let err = compose_new_card(&source, &[11, 11, 11, 11, 11, 11], None).unwrap_err();
        assert!(err.0.contains("15"));
    }

    #[test]
    fn gme_round_trip() {
        let source = Ps1Card::open(&blue(), "blue.mcr", false).unwrap();
        let gme = source.export(CardFormat::Gme, false);
        assert_eq!(gme.len(), GME_SIZE);
        assert_eq!(&gme[0..11], b"123-456-STD");
        let opened = Ps1Card::open(&gme, "blue.gme", false).unwrap();
        assert_eq!(opened.format, CardFormat::Gme);
        let view = opened.view();
        assert_eq!(view.used_blocks, 15);
        assert_eq!(
            view.saves
                .iter()
                .find(|s| s.master_slot == 11)
                .unwrap()
                .linked_slots,
            vec![11, 12, 14]
        );
    }

    #[test]
    fn vgs_round_trip() {
        let source = Ps1Card::open(&blue(), "blue.mcr", false).unwrap();
        let vgs = source.export(CardFormat::Vgs, false);
        assert_eq!(vgs.len(), VGS_HEADER_SIZE + CARD_SIZE);
        assert_eq!(&vgs[0..4], b"VgsM");
        let opened = Ps1Card::open(&vgs, "blue.vgs", false).unwrap();
        assert_eq!(opened.format, CardFormat::Vgs);
        assert_eq!(opened.view().saves.len(), 12);
    }

    #[test]
    fn truncated_raw_is_named() {
        let err = Ps1Card::open(b"MC", "tiny.mcr", false).unwrap_err();
        assert!(err.0.contains("truncated"));
    }
}
