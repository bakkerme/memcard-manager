#!/usr/bin/env python3
"""Read-only original-US Ape Escape research decoder. See docs/formats/ape-escape.md.

Accepts BASCUS-94423SYS MCS exports, or an 8192-byte reconstructed SC payload
with --identifier BASCUS-94423SYS. Does not extract whole cards or edit saves.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import unicodedata

IDENTIFIER = 'BASCUS-94423SYS'
PROFILE_START = 0x180
PROFILE_SIZE = 0x7A0
# Numeric facts from US executable/archive member 0x16. Thresholds are exclusive.
# index, name, monkeys, coins, bronze/silver/gold ticks, first room, end room
LEVELS = (
    (1, 'Fossil Field', 4, 1, (2700, 2100, 1800), 1, 2),
    (2, 'Primordial Ooze', 6, 1, (3600, 2820, 2100), 2, 3),
    (3, 'Molten Lava', 7, 1, (7800, 6300, 4500), 3, 6),
    (4, 'Thick Jungle', 14, 4, (9000, 7200, 5400), 6, 11),
    (5, 'Dark Ruins', 13, 4, (7800, 6300, 4800), 11, 15),
    (6, 'Cryptic Relics', 8, 1, (6600, 5100, 3900), 15, 19),
    (7, 'Stadium Attack', 0, 0, None, 19, 20),
    (8, 'Crabby Beach', 8, 1, (6000, 5100, 4200), 20, 22),
    (9, 'Coral Cave', 8, 1, (7800, 6300, 5100), 22, 24),
    (10, "Dexter's Island", 11, 3, (6600, 5400, 4200), 24, 29),
    (11, 'Snowy Mammoth', 6, 1, (7200, 5700, 4500), 29, 30),
    (12, 'Frosty Retreat', 9, 3, (7200, 5400, 4200), 30, 33),
    (13, 'Hot Springs', 9, 2, (6600, 5400, 4500), 33, 36),
    (14, 'Gladiator Attack', 0, 0, None, 36, 37),
    (15, 'Sushi Temple', 12, 3, (5400, 4200, 3300), 37, 40),
    (16, 'Wabi Sabi Wall', 10, 3, (6000, 4800, 3600), 40, 45),
    (17, 'Crumbling Castle', 20, 4, (4800, 3600, 2700), 45, 53),
    (20, 'City Park', 13, 3, (3900, 3000, 2100), 53, 56),
    (21, "Specter's Factory", 10, 2, (7500, 5700, 4500), 56, 63),
    (22, 'TV Tower', 12, 2, (6900, 5100, 3900), 63, 69),
    (24, 'Monkey Madness', 24, 10, (6000, 4800, 3600), 69, 87),
    (30, 'Peak Point Matrix', 0, 0, None, 87, 88),
)
GADGETS = ('Stun Club', 'Time Net', 'Monkey Radar', 'Slingback Shooter',
           'Super Hoop', 'Magic Punch', 'Sky Flyer', 'RC Car')


def u16(d, o):
    return struct.unpack_from('<H', d, o)[0]


def u32(d, o):
    return struct.unpack_from('<I', d, o)[0]


def name_text(d):
    # Proven letter/digit ranges; leave other glyphs explicit.
    out = []
    for b in d:
        if b == 0:
            break
        if 0x10 <= b <= 0x29:
            out.append(chr(ord('A') + b - 0x10))
        elif 0x30 <= b <= 0x49:
            out.append(chr(ord('a') + b - 0x30))
        elif 0x6A <= b <= 0x73:
            out.append(chr(ord('0') + b - 0x6A))
        elif b == 0x7F:
            out.append(' ')
        else:
            out.append(f'[0x{b:02X}]')
    return ''.join(out)


def medal(ticks, thresholds):
    if ticks == 0xFFFFFFFF:
        return 'Not recorded', 0
    for name, points, threshold in zip(('Gold', 'Silver', 'Bronze'), (8, 7, 5), reversed(thresholds)):
        if ticks < threshold:
            return name, points
    return 'No medal', 0


def clock(ticks):
    if ticks == 0xFFFFFFFF:
        return None
    # Game's timer uses 60 ticks, centiseconds are floor(remainder*100/60).
    return f'{ticks//3600:02}:{ticks//60%60:02}.{ticks%60*100//60:02}'


def inspect_save(source, identifier=None):
    container = 'sc-payload'
    if len(source) == 8320:
        container = 'mcs'
        raw = source[10:30].split(b'\0', 1)[0]
        identifier = raw.decode('ascii', errors='strict')
        if source[0] != 0x51 or u32(source, 4) != 8192 or u16(source, 8) != 0xFFFF:
            raise ValueError('Expected an active, terminated, one-block MCS directory')
        xor = 0
        for b in source[:127]:
            xor ^= b
        if xor != source[127]:
            raise ValueError('MCS directory XOR does not match')
        d = source[128:]
    elif len(source) == 8192:
        d = source
    else:
        raise ValueError('Expected an 8320-byte MCS or 8192-byte reconstructed SC payload')
    if identifier != IDENTIFIER:
        raise ValueError('Only the original US BASCUS-94423SYS layout is supported; raw SC requires its explicit identifier')
    if d[:4] != b'SC\x11\x01' or d[0x50:0x58] != b'\x01\x00CRD0\x00\x00':
        raise ValueError('Unsupported Ape Escape header/layout')
    title = unicodedata.normalize('NFKC', d[4:68].split(b'\0', 1)[0].decode('shift_jis', errors='strict'))
    if title != 'Ape Escape':
        raise ValueError('Unsupported Ape Escape title/release marker')
    profiles = []
    for i in range(4):
        p = d[PROFILE_START+i*PROFILE_SIZE:PROFILE_START+(i+1)*PROFILE_SIZE]
        saved_room = u16(p, 0)
        item = {'number': i+1, 'name': name_text(p[0x18:0x28]),
                'empty': saved_room == 0xFFFF, 'savedRoomId': saved_room}
        if item['empty']:
            profiles.append(item)
            continue
        if saved_room >= 0x8000:
            raise ValueError(f'Unknown negative profile marker in profile {i+1}')
        coins = []
        for j in range(128):
            room, flag = p[0x68+j*2:0x6A+j*2]
            if room == 255:
                break
            coins.append({'roomId': room, 'rawFlag': flag})
        levels = []
        score = sum(p[0x48:0x61]) + len(coins)*2
        for idx, name, total, coin_total, thresholds, start, end in LEVELS:
            ticks = u32(p, 0x690+idx*4)
            state = p[0x28+idx]
            record = {'index': idx, 'name': name, 'clearState': state}
            if total:
                captured = p[0x48+idx]
                flags = p[0x2F4+start*8:0x2F4+end*8]
                flag_count = sum(bool(v & 2) for v in flags)
                record.update(monkeys=captured, monkeyTotal=total,
                              remainingMonkeys=total-captured if captured <= total else None,
                              captureFlagCount=flag_count, monkeyCountsAgree=captured == flag_count,
                              coins=sum(start <= c['roomId'] < end for c in coins), coinTotal=coin_total)
                score += 15 if state == 1 else 25 if state == 2 else 0
                rating, points = medal(ticks, thresholds)
                score += points
                record.update(bestTimeTicks=ticks, bestTime=clock(ticks), medal=rating)
            elif idx in (7, 14):
                record.update(bestTimeTicks=ticks, bestTime=clock(ticks))
            levels.append(record)
        challenges = u32(p, 0x668)
        score += 15*(p[0x46] == 2) + 15*bool(challenges & 0x10) + 15*bool(challenges & 0x1000)
        score += sum(8 for o in (0x670, 0x671, 0x674) if p[o] == 5)
        anomalies = []
        if len(coins) > 50:
            anomalies.append('Coin record count exceeds the ordinary 50-coin total')
        if len(coins) == 128:
            anomalies.append('Coin list has no terminator; bounded to 128 records')
        if any(c['rawFlag'] != 1 or not any(a <= c['roomId'] < z and n for _, _, _, n, _, a, z in LEVELS) for c in coins):
            anomalies.append('Unknown coin room/flag; raw records retained')
        if any(r.get('monkeyCountsAgree') is False for r in levels):
            anomalies.append('Stored monkey counts differ from individual capture flags')
        if any(r.get('monkeys', 0) > r.get('monkeyTotal', 0) or r.get('coins', 0) > r.get('coinTotal', 0) for r in levels):
            anomalies.append('Stored level collectibles exceed ordinary totals')
        if score > 1000:
            anomalies.append('Computed completion exceeds 100%; retained without clamping')
        if u32(p, 4) & ~0xFF:
            anomalies.append('Unknown gadget unlock bits retained')
        if any(r['clearState'] not in (0, 1, 2) for r in levels):
            anomalies.append('Unknown level-clear state retained')
        room_name = next((n for _, n, _, _, _, a, z in LEVELS if a <= saved_room < z), None)
        item.update(savedStage=room_name, completionTenths=score,
                    monkeys=sum(p[0x48:0x61]),
                    levelSpecterCoins=len(coins),
                    bonusSpecterCoins=5*(bool(challenges & 0x10)+bool(challenges & 0x1000)),
                    specterCoins=len(coins)+5*(bool(challenges & 0x10)+bool(challenges & 0x1000)),
                    coinRecords=coins,
                    livesStored=p[2], extraLives=(p[2]-1) & 255, energyChips=p[3],
                    gadgetMask=u32(p, 4), gadgets=[n for j,n in enumerate(GADGETS) if u32(p,4) & (1<<j)],
                    buttonGadgetIds=list(p[8:12]), guidedPellets=p[12], explosivePellets=p[13],
                    levels=levels, anomalies=anomalies,
                    research={'trainingMask':u32(p,0x664),'challengeFlags':challenges,
                              'challengeBytes':list(p[0x66C:0x68C]),'otherFlags':u32(p,0x718)})
        profiles.append(item)
    return {'format':'ape-escape-us-research-v1','sourceSha256':hashlib.sha256(source).hexdigest(),
            'container':container,'identifier':identifier,'checksumOk':None,'profiles':profiles}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('saves',type=Path,nargs='+')
    parser.add_argument('--identifier',help='Required release identifier for raw SC payloads')
    args=parser.parse_args()
    results=[]
    for path in args.saves:
        try:
            results.append({'file':path.name,**inspect_save(path.read_bytes(),args.identifier)})
        except (ValueError,UnicodeError,OSError) as error:
            parser.exit(1,f'{path.name}: {error}\n')
    print(json.dumps(results,indent=2,ensure_ascii=False))


if __name__ == '__main__':
    main()
