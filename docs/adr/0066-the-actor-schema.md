# 0066: The actor schema: each actor's configuration as data, checked against its code and the game's data

- **Status:** accepted, built (2026-10-14)
- **Date:** 2026-10-14
- **Builds on:** [ADR 0062](0062-the-aim-master-quest-and-the-game-editor.md) (the aim, each
  actor's Params and Flags paragraphs), [ADR 0002](0002-crate-layout.md) (the crate layering).

## Context

ADR 0062 made each actor's configuration (its params' bit fields, the rotation components some
use as settings, the flags it reads and sets) a pair of prose paragraphs at the end of its module
docs, for the game editor's actors tab to show as named fields. 61 of the 131 ported actors had
them; BACKLOG #52 was the pass over the rest, planned with the actors tab (stage E3).

On 2026-10-13 the user decided that the editor's interface (E3 and later) waits until more of the
game is ported, around Phase 12 or 13, but that the per-actor schema the editor will read is
settled now, so that every actor ported from here on gets one. Prose can't be read reliably by a
program, and nothing kept it in step with the code.

## Decision

1. **A const in each actor's module** (the user's call): `pub const SCHEMA: ActorSchema` next to
   the prose, with its fields as the module's own `pub const`s. The types are in
   `oot_game::actor_schema`. Each `ActorProfile` has a `schema` field, so the compiler asks for
   one with every new actor (`ActorSchema::TODO` until it's written; `ActorSchema::NONE` for an
   actor that takes nothing). The editor reads them from `oot_actors::PROFILES` (a tool may link
   the content crates, ADR 0002); `ootx actor-schemas` writes them as JSON
   (`<out>/actor_schemas.json`) for anything outside Rust.
2. **What a field says:**
   - its **source**: the params, or a rotation component (x, y, z) used as a setting; its bits
     (shift and count) and whether it's sign-extended;
   - its **kind**: named values, a number range, a get-item id, a collectible (`ITEM00_*`), a
     text id, a path index, a switch, chest, collectible or temp flag, an object, a debug-only
     value, an unused one (set in the data, read by nothing), or a rotation used as one;
   - its **name** (the C's: the macro's, the member's or the local's) and a few words of
     **meaning**;
   - **`when`**: only for some values of another field (a type's); **`part_of`**: bits of a field
     the code reads whole and picks apart later (`En_Rd`'s `rdFlags & 0x7F`).
   - **Flags** (`FlagUse`): switch, chest and collectible flags (by the field that holds them, or
     a fixed number), the room's clear and temp clear flags, `EVENTCHKINF`, `INFTABLE`,
     `ITEMGETINF` and `EVENTINF` by name and value, and other save or play state used as a flag
     between actors (`sunsSongState`); each read, set or cleared, with a note, and `indirect`
     with the reason when no call in the module does it (a dropped item sets it, a shared
     helper, a table). The inventory, the quest items, the age and the time are conditions, not
     flags: they stay in the prose.
3. **The code reads through the fields** (the user's call), as the C reads through
   `PARAMS_GET_U` and its headers' macros: `TYPE.get(this.actor.params)`. A test
   (`oot_actors/tests/schemas.rs`) checks four things:
   - **the code:** no actor module reads params except through a field (writes, prints and
     whole params handed on carry a `// schema:` note), and every field (but the debug, unused
     and rotation ones) is read;
   - **the flags:** the flag helpers each module calls (`flags.get_switch`,
     `save.set_event_chk_inf(EVENTCHKINF_X)`, ...) are its schema's, in the same direction, and
     every flag in the schema is called, unless `indirect`;
   - **the game's data:** every placement of the actor in every scene layer (room actors,
     transition actors, Player's spawns) decodes: named fields hold named values, ranges hold,
     no bit is set outside the fields, no rotation x or z is set that the schema doesn't name.
     This checks the schema against the ROM, not against the port's reading;
   - **the form:** fields fit their bits, names are unique, `when` and `part_of` name real
     fields, flags name real fields.
4. **The prose stays** as the readable half (ADR 0062's paragraphs), agreeing with the schema.
5. **The pass (BACKLOG #52):** Player and three examples (`Obj_Tsubo`, `En_Okarina_Tag`, `En_Rd`)
   here; the other 127 by six helpers in their own worktrees, by groups of actors, each field
   checked against the decomp's C and headers rather than the port's reading. It's a refactor:
   the values the code reads are the same, and no golden changed. Where the port read something
   differently from the C, the helpers reported it rather than fixing it in the pass.
6. **Authored actors** (ones that aren't in the game: a generic character, a trigger) aren't
   designed for. Nothing here rules them out: a schema is a const like any other, and an authored
   actor would carry one.

## Results (2026-10-14)

- All 131 ported actors have a schema: 299 fields (131 named, 57 ranges, 38 unused, 31 switch
  flags, 22 rotations, 11 path indexes, the rest item, drop, chest, collectible and debug ones;
  249 from the params, 50 from a rotation) and 320 flag uses, 92 of them `indirect` (flags read
  through a table or a computed index, set by a dropped item, or in a part of the actor that isn't
  ported, marked "not ported"). Five actors take nothing. 105 `// schema:` notes mark the reads the
  code keeps as they are (writes, prints, whole params handed on, another actor's params read the
  C's own way).
- The game's data showed what the C's reads hide: the pots' "no drop" written as 0x3F where the C
  reads five bits, the Desert Colossus's tilted gravestones, Jabu-Jabu's leaning cows, a rolled
  heart piece, bits set by a debug scene. Each is a field now (`Unused` or `Rotation`).
- The helpers found no params read where the port differs from the C. They corrected the prose
  where it was wrong (a Skulltula flag's index, flags said to be read that are only set, a
  placement said to be 0 that is 0xFFFF), and found differences from the C other than params
  reads: BACKLOG #63.
- No golden changed (187 of 187), and the whole test suite passes. The checks were tried by
  breaking each on purpose (a raw params read, a flag left out, a value left out of a named list):
  each failed as it should.

## Consequences

- Every new actor comes with its schema and its prose; the test fails while any profile's schema
  is `TODO`. "How a milestone is done" (ROADMAP.md) says so.
- The editor's actors tab (stage E3) reads fields, not prose.
- The game's data documents itself: bits the data sets that nothing reads are `Unused` fields with
  what the data puts there.
- The checks are textual and have limits: a params value copied into a struct field and masked
  later is followed only through `part_of`; flags set by shared code are seen only through
  `indirect`; actors that only other actors spawn have no placements, so only their code is
  checked.

## Amended (2026-10-17): object needs

ADR 0077 adds `ActorSchema::objects`: the objects an actor finds slots for itself besides its
profile's (`ObjectNeed { object, when, note }`, by field value where one decides it), for the
editor and the importer to load into a custom room. `oot_actors/tests/schemas.rs` checks that a
module looking up objects declares them (or notes why not) and that every room of the game
placing the actor loads what it declares.
