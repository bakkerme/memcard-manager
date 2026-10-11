#!/usr/bin/env python3
"""Read-only byte diff of equal-sized, already normalized save payloads.

No card extraction, game detection, checksum guessing, or file writes.
JSON includes private bytes; retain it in an ignored evidence directory.
"""
import argparse
import hashlib
import json
from pathlib import Path


def compare(before, after):
    if not before or not after:
        raise ValueError('Payloads must be nonempty')
    if len(before) != len(after):
        raise ValueError('Payload sizes differ; verify layout and normalization before comparing')
    ranges = []
    start = None
    for offset in range(len(before) + 1):
        changed = offset < len(before) and before[offset] != after[offset]
        if changed and start is None:
            start = offset
        elif not changed and start is not None:
            old, new = before[start:offset], after[start:offset]
            ranges.append({
                'start': start, 'endExclusive': offset,
                'startHex': hex(start), 'endExclusiveHex': hex(offset),
                'length': offset - start,
                'beforeHex': old.hex(), 'afterHex': new.hex(),
                'xorHex': bytes(a ^ b for a, b in zip(old, new)).hex(),
            })
            start = None
    return {
        'schema': 'ps1-normalized-payload-diff-v1',
        'offsetOrigin': 'start of each caller-supplied normalized payload',
        'formatValidated': False,
        'size': len(before),
        'beforeSha256': hashlib.sha256(before).hexdigest(),
        'afterSha256': hashlib.sha256(after).hexdigest(),
        'changedBytes': sum(r['length'] for r in ranges),
        'ranges': ranges,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    args = parser.parse_args()
    try:
        result = compare(args.before.read_bytes(), args.after.read_bytes())
    except (OSError, ValueError) as error:
        parser.exit(2, f'{error}\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
