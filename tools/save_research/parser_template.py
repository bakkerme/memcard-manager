"""Replace this file with a bounded, game-specific read-only decoder."""


def inspect_save(source: bytes, metadata: dict) -> dict:
    """Accept MCS bytes or a normalized SC payload, as declared by the suite.

    Validate the exact supported product code, region, identifier, size, header
    and relevant layout markers. Raise ValueError for unsupported/malformed data.
    Offsets must have a declared origin. Do not guess checksums or field meanings.
    Return fields (JSON values) and checksumOk (bool, or None if unverified).
    """
    raise ValueError('No game parser implemented yet')
