//! Local bundle preparation only. Reuses the MemcardRex Core parser port by
//! Shendo; GPL-3.0-or-later. No Rust build is needed on the research server.
#![allow(dead_code, unused_imports)]
#[path = "../src/card/mod.rs"]
mod card;
use std::{fs, io::Write, path::PathBuf};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("Usage: export_research_save CARD SLOT_1_TO_15 NEW_OUTPUT.mcs".into());
    }
    let slot: usize = args[1].parse().map_err(|_| "Invalid slot")?;
    if !(1..=15).contains(&slot) {
        return Err("Slot must be 1..15".into());
    }
    let bytes = fs::read(&args[0]).map_err(|e| e.to_string())?;
    let parsed = card::Ps1Card::open(&bytes, &args[0], false).map_err(|e| e.to_string())?;
    let view = parsed.view();
    let save = view
        .saves
        .iter()
        .find(|s| s.master_slot as usize == slot - 1)
        .ok_or("Slot is not a master save")?;
    let raw = parsed.save_raw(false);
    let mut seen = Vec::new();
    let deleted = save.deleted;
    for (index, &block) in save.linked_slots.iter().enumerate() {
        if seen.contains(&block) {
            return Err("Duplicate block link".into());
        }
        seen.push(block);
        let start = 128 + block as usize * 128;
        let header = &raw[start..start + 128];
        let last = index + 1 == save.linked_slots.len();
        let expected_type = if index == 0 {
            if deleted {
                0xa1
            } else {
                0x51
            }
        } else if last {
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
        let pointer = u16::from_le_bytes([header[8], header[9]]);
        if header[0] != expected_type
            || (last && pointer != 0xffff)
            || (!last && pointer as u8 != save.linked_slots[index + 1])
            || (!last && pointer > 14)
        {
            return Err("Incomplete or malformed block chain".into());
        }
    }
    let header = &raw[slot * 128..slot * 128 + 128];
    if u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize
        != save.linked_slots.len() * 8192
    {
        return Err("Directory size disagrees with the block chain".into());
    }
    let output = parsed.get_save_bytes(slot - 1);
    let target = PathBuf::from(&args[2]);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| e.to_string())?;
    file.write_all(&output).map_err(|e| e.to_string())?;
    println!(
        "Exported {} block(s) from slot {}",
        save.linked_slots.len(),
        slot
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
