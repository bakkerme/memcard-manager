"""Constructed-data behavior tests; opt-in real saves remain private."""
import hashlib
import os
from pathlib import Path
import struct
import unittest

from inspect_ape_escape import inspect_save, IDENTIFIER, LEVELS, medal, clock


def payload():
    d = bytearray(8192)
    d[:4] = b'SC\x11\x01'
    title = 'Ａｐｅ　Ｅｓｃａｐｅ'.encode('shift_jis')
    d[4:4+len(title)] = title
    d[0x50:0x58] = b'\x01\x00CRD0\x00\x00'
    for i in range(4):
        struct.pack_into('<H', d, 0x180+i*0x7a0, 0xffff)
    return d


def wrapped(d):
    h = bytearray(128)
    h[0] = 0x51
    struct.pack_into('<IH', h, 4, 8192, 0xffff)
    h[10:10+len(IDENTIFIER)] = IDENTIFIER.encode('ascii')
    for b in h[:127]:
        h[127] ^= b
    return h+d


class ApeInspectionTests(unittest.TestCase):
    def test_empty_profiles_validation_and_read_only(self):
        d = payload()
        original = bytes(d)
        result = inspect_save(d, IDENTIFIER)
        self.assertEqual(len(result['profiles']), 4)
        self.assertTrue(all(p['empty'] for p in result['profiles']))
        self.assertEqual(inspect_save(wrapped(d))['profiles'], result['profiles'])
        self.assertIsNone(result['checksumOk'])
        self.assertEqual(bytes(d), original)
        for damaged in [d[:-1], wrapped(d)[:-1]]:
            with self.assertRaises(ValueError):
                inspect_save(damaged, IDENTIFIER)
        with self.assertRaises(ValueError):
            inspect_save(d)
        for offset in (2, 3, 4, 0x50, 0x52):
            bad = d.copy(); bad[offset] ^= 1
            with self.assertRaises((ValueError, UnicodeError)):
                inspect_save(bad, IDENTIFIER)
        bad = wrapped(d); bad[127] ^= 1
        with self.assertRaises(ValueError):
            inspect_save(bad)

    def test_complete_profile_formula_and_strict_medals(self):
        d = payload()
        p = memoryview(d)[0x180:0x920]
        struct.pack_into('<H', p, 0, 87)
        p[0x18:0x1e] = bytes([0x15,0x38,0x3b,0x34,0x6b,0])
        p[0x690:0x710] = b'\xff'*128
        coins = []
        for idx, _, monkeys, count, thresholds, start, end in LEVELS:
            p[0x28+idx] = 2
            p[0x48+idx] = monkeys
            if monkeys:
                # Populate captured flags independently, spilling over rooms.
                p[0x2f4+start*8:0x2f4+start*8+monkeys] = b'\x02'*monkeys
                struct.pack_into('<I', p, 0x690+idx*4, thresholds[2]-1)
            coins.extend([(start,1)]*count)
        for i,(room,flag) in enumerate(coins):
            p[0x68+i*2:0x6a+i*2] = bytes([room,flag])
        p[0x68+len(coins)*2] = 0xff
        struct.pack_into('<I',p,0x668,0x1010)
        for offset in (0x670,0x671,0x674):
            p[offset] = 5
        before = bytes(d)
        r = inspect_save(d,IDENTIFIER)['profiles'][0]
        self.assertEqual(r['name'],'File1')
        self.assertEqual(r['completionTenths'],1000)
        self.assertEqual(r['monkeys'],204)
        self.assertEqual(r['specterCoins'],60)
        self.assertEqual(r['anomalies'],[])
        self.assertEqual(bytes(d),before)
        self.assertEqual(medal(1800,(2700,2100,1800)),('Silver',7))
        self.assertEqual(medal(1799,(2700,2100,1800)),('Gold',8))
        self.assertEqual(clock(3659),'01:00.98')
        self.assertIsNone(clock(0xffffffff))

    @unittest.skipUnless(os.environ.get('APE_ESCAPE_CORPUS'), 'Set APE_ESCAPE_CORPUS to the ignored research run directory')
    def test_private_real_corpus(self):
        root = Path(os.environ['APE_ESCAPE_CORPUS'])
        sources = [
            ('source.sc', [102,None,None,None]),
            ('ape-escape.164.sc',[1000,833,None,None]),
            ('ape-escape.4583.sc',[1000,1000,892,985]),
        ]
        # Full wrapper/card hashes are pinned in tests/fixtures/ps1/manifest.json.
        for filename, expected in sources:
            d=(root/filename).read_bytes(); before=hashlib.sha256(d).hexdigest()
            r=inspect_save(d,IDENTIFIER)
            self.assertEqual([p.get('completionTenths') for p in r['profiles']],expected)
            self.assertEqual(hashlib.sha256((root/filename).read_bytes()).hexdigest(),before)


if __name__ == '__main__':
    unittest.main()
