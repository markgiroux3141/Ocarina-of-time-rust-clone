# 0003: Target ROM: gc-eu-mq-dbg for development

- **Status:** accepted; the vanilla-dungeon question settled by ADR 0028 (Master Quest, this ROM only)
- **Date:** 2026-09-27

## Context

Everything so far is built and checked against the Master Quest debug ROM (gc-eu-mq-dbg), which is what the pinned decomp commit targets (ADR 0004). Its overworld (Kokiri Forest, Hyrule Field, towns) is the same as the retail game's. Its dungeons are the Master Quest layouts. A clone that wants the original dungeons needs a ROM version that has them, and decomp support for that version.

The user's ROM, as the importer sees it:

| | |
|---|---|
| File | `D:/OOT Modding/Debug Roms/baserom.z64` (z64 byte order, 56,623,104 bytes, uncompressed) |
| MD5 | `f0b7f35375f9cc8ca1b2d59d78e35405`, the decomp's `checksum.md5` for `zelda_ocarina_mq_dbg.z64` |
| SHA-1 | `079b855b943d6ad8bd1eb026c0ed169ecbdac7da` |
| Header | title `THE LEGEND OF ZELDA`, game code `NZLP`; dmadata at `0x12F70`, 1532 files |

## Decision

- Develop against gc-eu-mq-dbg, identified by the SHA-1 above.
- The asset pack is keyed by ROM SHA-1 (ADR 0008). The importer records the version it was built from and refuses ROMs it doesn't know, rather than guessing.
- Postpone the vanilla-dungeon question until before the first dungeon (the Deku Tree, after the Kokiri Forest slice). Nothing before that point depends on it. The options then are:
  - Add a vanilla ROM version (e.g. gc-eu or ntsc-1.0) as a second import source for dungeon scenes. This needs a decomp commit that supports it (ADR 0004's upgrade).
  - Ship Master Quest dungeons.

## Consequences

- Import code must not hard-code debug-ROM-only facts silently. Offsets come from the decomp's XMLs and `spec`, and anything version-specific gets named as such.
- Debug-ROM extras (the `z_debug` registers such as `OREG`/`R_*`, used through their `*_Init` values) are fine to rely on. Their values are the retail game's too.
