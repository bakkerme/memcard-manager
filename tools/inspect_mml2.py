#!/usr/bin/env python3
"""Read-only research decoder for original US Mega Man Legends 2 saves.

Accepts one 8192-byte SC payload or an 8320-byte MCS export. No write/repair API.
See docs/formats/mega-man-legends-2.md for evidence and interpretation limits.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import unicodedata

SEGMENTS = ((0, 0x17C), (0x200, 0x27C), (0x280, 0x3FC), (0x400, 0xBFC))
PREFIX = "BASLUS-01140-DASH2"
LOCATIONS = (
    "Flutter", "Sulphur-Bottom", "Yosyonke Pad", "Yosyonke Gallery",
    "Forbidden Island", "Manda Pad", "Pokte Ruins", "Nino Pad",
    "Nino Platform", "Ruminoa City", "Glyde Base Gate", "Carlbania",
    "Kimotoma City", "Kimotoma Pad", "Kimotoma Ruins", "Saul Kada",
    "Manda Ruins", "Nino Ruins", "Saul Kada Ruins", "Calinca Ruins",
    "Elysium Pad", "Elysium", "Master's Room", "Forbidden Island", "Kimotoma City",
)
DIFFICULTIES = {0: "Easy", 1: "Normal", 3: "Hard", 4: "Very Hard"}


def u16(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def u32(data, offset):
    return struct.unpack_from("<I", data, offset)[0]


def inspect_save(source):
    """Return JSON-compatible observations; unknown names remain unknown."""
    identifier = None
    slot = None
    if len(source) == 8320:
        identifier = source[10:30].split(b"\0", 1)[0].decode("ascii", errors="strict")
        if not (identifier.startswith(PREFIX) and len(identifier) == len(PREFIX) + 1
                and identifier[-1] in "01234"):
            raise ValueError("Not a supported US DASH2 adventure-save identifier")
        if u32(source, 4) != 8192:
            raise ValueError("MCS directory does not declare a one-block save")
        slot = int(identifier[-1]) + 1
        data = source[128:]
    elif len(source) == 8192:
        data = source
    else:
        raise ValueError("Expected an 8192-byte SC payload or an 8320-byte MCS export")
    if data[:4] != b"SC\x11\x01":
        raise ValueError("Expected the observed one-frame, one-block SC header")
    title = data[4:68].split(b"\0", 1)[0].decode("shift_jis", errors="strict").rstrip()
    normalized_title = unicodedata.normalize("NFKC", title)
    if not normalized_title.startswith("MEGAMAN LEGENDS2["):
        raise ValueError("Title does not match the observed US Legends 2 layout")
    checksums = []
    for start, end in SEGMENTS:
        computed = sum(struct.unpack_from(f"<{(end-start)//4}I", data, start)) & 0xFFFFFFFF
        stored = u32(data, end)
        checksums.append({"start": f"0x{start:03x}", "endExclusive": f"0x{end:03x}",
                          "checksumOffset": f"0x{end:03x}", "stored": f"0x{stored:08x}",
                          "computed": f"0x{computed:08x}", "ok": stored == computed})
    frames = u32(data, 0x138)
    seconds = frames // 60
    location_id = data[0x12F]
    return {
        "format": "mega-man-legends-2-us-research-v1",
        "sourceSha256": hashlib.sha256(source).hexdigest(),
        "container": "mcs" if identifier else "sc-payload",
        "identifier": identifier, "slot": slot, "title": normalized_title,
        "checksumOk": all(c["ok"] for c in checksums), "checksums": checksums,
        "summary": {
            "playtime": f"{seconds//3600:02}:{seconds//60%60:02}:{seconds%60:02}",
            "playtimeFrames": frames, "subsecondFrames": frames % 60,
            "secondTimeCounterFrames": u32(data, 0x13C),
            "locationPreviewId": location_id,
            "location": LOCATIONS[location_id - 1] if 1 <= location_id <= len(LOCATIONS) else None,
            "mapId": data[0x128], "mapSecondaryId": data[0x129],
            "difficultyId": data[0x12E],
            "difficulty": DIFFICULTIES.get(data[0x12E]),
            "zenny": u32(data, 0x200),
            "healthRaw": u16(data, 0x290), "maxHealthRaw": u16(data, 0x292),
        },
        "equipment": {
            "helmetId": data[0x2B4], "shoesId": data[0x2B5], "armorId": data[0x2B6],
            "busterPart1Id": data[0x2B8], "busterPart2Id": data[0x2B9],
            "busterPackedBytes": list(data[0x2B8:0x2BC]),
        },
        "research": {
            "playerPositionRaw": list(struct.unpack_from("<3i", data, 0x280)),
            "playerRotationRaw": u16(data, 0x28C),
            "playerInventoryBytes": list(data[0x2BC:0x2E0]),
            "weaponRecords": [{"index": i, "bytes": list(data[0x2E0+i*8:0x2E5+i*8])}
                              for i in range(20)],
            "setFlagIds": [i for i in range(0x120 * 8)
                           if data[0x400+i//8] & (1 << (i % 8))],
        },
        "notes": [
            "Locations use the game's one-based 25-entry save-preview table.",
            "Four difficulty names are checked against labelled saves; internal level 2 remains unnamed. License tests may change the saved level.",
            "Health uses raw game units. Item names, inventory semantics and story-flag names are not decoded.",
            "The second time counter is distinct in RAM; equal sample values do not establish its purpose.",
            "Checksums cover unused bytes inside their ranges; bytes from 0xc00 onward are not checked.",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("saves", type=Path, nargs="+")
    args = parser.parse_args()
    results = []
    for path in args.saves:
        try:
            results.append({"file": path.name, **inspect_save(path.read_bytes())})
        except (ValueError, UnicodeError, OSError) as error:
            parser.exit(1, f"{path.name}: {error}\n")
    print(json.dumps(results, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
