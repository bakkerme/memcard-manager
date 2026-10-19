# Bomberman Fantasy Race US: read-only summary

Supported boundary: `BA` / `SLUS-00823` / `SYSTEM`, exact `SC 11 01` header,
8192-byte reconstructed payload. Offsets include the SC header; an MCS wrapper
adds 128 bytes. Existing card-chain and Raw/GME/VGS extraction remain in the
attributed MemcardRex Core port. The reader performs no repair or writes.

## Fields and integrity

| Field | Offset | Interpretation |
| --- | --- | --- |
| Capacity | `0xd03`, u8 | Clamp to 50; normal constructors use 5 or 50, unusual values receive a note |
| Profile record | `0xd08 + slot * 0x44` | 50 physical records, stride 68 decimal |
| Occupancy | record `+0`, u8 | With global/record integrity passing: `00` empty, `ff` occupied; otherwise unknown |
| Player | record `+1..+6` | Five glyphs: `0a..23` A–Z, `24..3d` a–z, `49` blank; trim trailing blanks |
| Normal tickets | record `+6`, u8 mask | Count owned course IDs 0–7 |
| Mirrored tickets | record `+7`, u8 mask | Count owned course IDs 0–7 in mirrored direction |

Unknown glyphs withhold the full name and retain all five glyph IDs in an explicit
Unknown value. Blank occupied names display Unnamed profile. Course names,
race-win flags, currency, animals/inventory, ranking identities and overall
completion are not decoded. Ticket counts describe owned tickets, not every
course offered for purchase in 1P. The US booklet's printed pages 17–18 limit
Time Attack to courses for which tickets are owned; native mode-1 selector code
and controlled observations bind that terminology to these saved masks.

Global integrity uses XOR8 and wrapping sum8 over `[0x200,0xd04)`, stored at
`0xd04/0xd05`. Each of 50 profiles has the same checks over its first `0x40`
bytes (64 decimal), stored at `+0x40/+0x41`. Aggregate `checksumOk` requires all
51 regions, including records outside the declared capacity. Each profile's
integrity requires the global check and that record's checks. A damaged unused
record can make the aggregate fail without suppressing an independently valid
visible profile. Checks do not certify arbitrary native loadability.

## Unknown slots and UI compatibility

Bomberman profiles include an additive `status` of `occupied`, `empty` or `unknown`.
The legacy `empty` boolean is true only for a positively identified empty slot;
unknown is never represented as empty. Unknown profiles carry no name/progress
fields and the dialog labels them Unknown with an explanation. Global checksum
failure makes all visible slot states unknown; an individual record failure
makes only that slot unknown. An unfamiliar marker is unknown even if arithmetic
checks pass. Existing readers omit `status` and retain their prior JSON shape.

All declared slots remain available in fixed order. A bounded scrolling selector
keeps up to 50 slots from pushing the selected details out of view. Capacity zero
shows no profile slots and an explicit note, rather than implying recovery or
changing the stored capacity. No preferred bank or save chronology is inferred.

## Evidence and limits

Private provenance is retained in ignored
`tools/save_research/runs/review-68a145a8a67645f09f9c3686112730ac-g51/`:
`REVIEW.md`, reviewer hash/evidence receipts, exported `outputs/final/` readers and
fixtures, and `outputs/research/` native observations and bounded integer probes.
The review reran 99 expectation cases and 20 integer-fragment probes plus two
execution/data guards, verified 341 artifact entries, and compared saved bytes
to native selected-player and Time Trial selector RAM. Synthetic/model checks
are separate from native observations.

Exact US executable SHA-256:
`c11f23041e0aeca37bcdc7ea5eecf08a745bf67b8a0cdc8e3dfeda5fa41c668b`.
Native SYSTEM loader `0x800143b8..0x8001447c` and writer
`0x80014718..0x800147c8` translate names/tickets to/from selected-player RAM.
Marker scan is `0x80012cac..0x80012cfc`; the name-entry table is at `0x80010ce4`.
Mode-1 Time Trial selector `0x8001e7cc..0x8001e90c` uses the owned-ticket masks.
No game code, ROM/BIOS bytes, private saves, manual scan or third-party reader
implementation is included in production.

The original Blb profile has 1 normal / 0 mirrored ticket. Normal new-game/name
entry and Save created profile a in slot 02; a fresh process listed and loaded it
with the same default 1/0 tickets while Blb stayed unchanged. A controlled fixture
with masks `03/81` gives 2 normal / 2 mirrored tickets in native Time Trial.
That is representation evidence, not gameplay-earned progression or normal-save
persistence of edited ticket masks. Rich ownership diversity is controlled;
there is only one original source and one newly saved default profile.

The supplied original directory entry is deleted A1 and produced No Data in native
Load. Disposable directory-only activation loaded its unchanged payload; no repair
is performed by the app. Native payload-prefix equality is qualified only through
`0x1a80`; the failed full-heap comparison is retained. Earlier scratch-write
preservation failures remain failures, while later read trials preserve a separately
frozen initialized whole card. Disc revision labels, other releases and general
replay/loadability certification remain unqualified.

## Validation

Default Rust tests use constructed data to cover the known values, glyph boundaries,
unknown glyphs/markers, global/record integrity failures (including empty markers),
capacity 0/5/50/51/255, last slot, ticket masks/neighbor flags, layout and metadata
rejection, JSON status, Raw/GME/VGS containers, MCS reimport, deleted metadata,
malformed allocation/terminal links and input preservation. Frontend render tests
assert that unknown and empty slots suppress stale fields and remain distinguishable.

The ignored three-fixture test pins the original MCS, native-saved a payload and
controlled 2/2 payload by SHA-256. With the retained outputs present:

```sh
PS1_BFR_REVIEW=/absolute/path/to/review-68a145a8a67645f09f9c3686112730ac-g51 \
  cargo test --manifest-path src-tauri/Cargo.toml bomberman_fantasy_race::tests::private_known_value_corpus --lib -- --ignored
npm test
npm run build
```

These tests launch no emulator or hardware operation. Native Save/fresh Load
qualification comes from the retained research observations, not from Rust tests.

Integration validation on macOS (19 October 2026): 54 default Rust tests passed,
the separate three-fixture private corpus passed, and TypeScript/Vite build passed.
Browser checks of the actual dialog with synthetic 50-slot data verified occupied,
unknown and empty states, selector scrolling and close behavior at 1280×800 and
390×700, with no horizontal dialog overflow. Native Tauri UI, USB and a new emulator
save experiment were not run during integration.
