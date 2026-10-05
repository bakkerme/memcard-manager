"""Synthetic regressions for the research reader; optional local real-save corpus."""
import hashlib
import os
from pathlib import Path
import struct
import unittest

from inspect_mml2 import inspect_save, PREFIX, SEGMENTS


def payload():
    data = bytearray(8192)
    data[:4] = b"SC\x11\x01"
    title = "ＭＥＧＡＭＡＮ　ＬＥＧＥＮＤＳ２［１］０３：４１：２６".encode("shift_jis")
    data[4:4+len(title)] = title
    struct.pack_into("<I", data, 0x138, 797203)
    struct.pack_into("<I", data, 0x200, 11950)
    struct.pack_into("<HH", data, 0x290, 128, 128)
    data[0x12F] = 8
    return seal(data)


def seal(data):
    for start, end in SEGMENTS:
        value = sum(struct.unpack_from(f"<{(end-start)//4}I", data, start)) & 0xFFFFFFFF
        struct.pack_into("<I", data, end, value)
    return data


def mcs(data, identifier=PREFIX+"0"):
    header = bytearray(128)
    struct.pack_into("<I", header, 4, 8192)
    header[10:10+len(identifier)] = identifier.encode("ascii")
    return header + data


class InspectionTests(unittest.TestCase):
    def test_frames_and_containers_are_read_only(self):
        data = payload()
        before = bytes(data)
        raw = inspect_save(data)
        wrapped = inspect_save(mcs(data))
        self.assertEqual(raw["summary"], wrapped["summary"])
        self.assertEqual(wrapped["slot"], 1)
        self.assertEqual(raw["summary"]["playtime"], "03:41:26")
        self.assertEqual(raw["summary"]["subsecondFrames"], 43)
        self.assertEqual(raw["summary"]["zenny"], 11950)
        self.assertEqual(raw["summary"]["healthRaw"], 128)
        self.assertTrue(raw["checksumOk"])
        self.assertEqual(bytes(data), before)
        struct.pack_into("<I", data, 0x138, 216000)
        self.assertEqual(inspect_save(seal(data))["summary"]["playtime"], "01:00:00")

    def test_each_checksum_and_unchecked_gaps(self):
        for index, (start, end) in enumerate(SEGMENTS):
            data = payload()
            data[max(start, 0x100)] ^= 1  # Preserve SC and title validation.
            report = inspect_save(data)
            self.assertFalse(report["checksumOk"])
            self.assertEqual([c["ok"] for c in report["checksums"]],
                             [i != index for i in range(4)])
            data = payload()
            data[end] ^= 1
            self.assertFalse(inspect_save(data)["checksumOk"])
        for offset in [0x180, 0x1FF, 0xC00, 0x1FFF]:
            data = payload()
            data[offset] ^= 1
            self.assertTrue(inspect_save(data)["checksumOk"])
        data = payload()
        data[0x400:0xBFC] = b"\xff" * (0xBFC-0x400)
        report = inspect_save(seal(data))
        self.assertTrue(report["checksumOk"])
        self.assertEqual(report["checksums"][3]["computed"], "0xfffffe01")

    def test_unknown_values_and_bit_order_stay_explicit(self):
        data = payload()
        data[0x12F] = 99
        data[0x12E] = 254
        data[0x400] = 0x81
        data[0x401] = 2
        report = inspect_save(seal(data))
        self.assertIsNone(report["summary"]["location"])
        self.assertEqual(report["summary"]["difficultyId"], 254)
        self.assertIsNone(report["summary"]["difficulty"])
        self.assertEqual(report["research"]["setFlagIds"], [0, 7, 9])

    def test_rejects_other_layouts(self):
        for size in [0, 1, 128, 3072, 8191, 8193, 8321]:
            with self.assertRaises(ValueError):
                inspect_save(bytes(size))
        for identifier in [PREFIX+"5", "BESLUS-01140-DASH20", PREFIX+"00", "OTHER"]:
            with self.assertRaises(ValueError):
                inspect_save(mcs(payload(), identifier))
        data = payload()
        data[3] = 2
        with self.assertRaises(ValueError):
            inspect_save(data)
        data = payload()
        data[4:68] = b"\0" * 64
        with self.assertRaises(ValueError):
            inspect_save(data)
        data = mcs(payload())
        struct.pack_into("<I", data, 4, 16384)
        with self.assertRaises(ValueError):
            inspect_save(data)

    @unittest.skipUnless(os.environ.get("MML2_LOCAL_FIXTURES"), "Private fixtures are optional")
    def test_two_real_saves(self):
        root = Path(os.environ["MML2_LOCAL_FIXTURES"])
        cases = [
            ("0", "13210887c585cbaed345e51ce1763aad2b368e1e658ad7c20354086591c65ce6", "03:41:26", 11950, 1, 2),
            ("1", "630f4ed27e0b579c155f04b0057f4cbf5df7c125d45d8fd3e4c421a368b5440c", "04:47:52", 31700, 2, 3),
        ]
        for suffix, digest, time, zenny, shoes, armor in cases:
            source = (root / (PREFIX + suffix + ".mcs")).read_bytes()
            self.assertEqual(hashlib.sha256(source).hexdigest(), digest)
            report = inspect_save(source)
            self.assertTrue(report["checksumOk"])
            self.assertEqual(report["summary"]["playtime"], time)
            self.assertEqual(report["summary"]["zenny"], zenny)
            self.assertEqual(report["summary"]["location"], "Nino Pad")
            self.assertEqual(report["summary"]["difficulty"], "Normal")
            self.assertEqual(report["equipment"]["shoesId"], shoes)
            self.assertEqual(report["equipment"]["armorId"], armor)


if __name__ == "__main__":
    unittest.main()
