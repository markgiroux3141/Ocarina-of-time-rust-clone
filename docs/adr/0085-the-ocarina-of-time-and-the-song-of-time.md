# 0085: The Ocarina of Time and the Song of Time

- **Status:** accepted, built in GAME-10 milestone 7 (2026-10-21); its hand test owed ([HAND-TESTS-OWED.md](../HAND-TESTS-OWED.md))
- **Date:** 2026-10-21
- **Builds on:** [ADR 0084](0084-barinade-rutos-warp-and-the-zoras-sapphire.md) (the three stones,
  `zora-sapphire`), [ADR 0081](0081-water-bottles-and-the-underwater-view.md) (the dive and the
  bottom's item), [ADR 0030](0030-skin-skeletons.md) (`En_Viewer` and its skins, the drawbridge),
  [ADR 0066](0066-the-actor-schema.md) (the schemas).

## Context

Phase 11's seventh milestone (`docs/GAME-10-zoras-waters-and-jabu-jabu.md`), the exit's second part:
with the three stones, the drawbridge's trigger, Zelda's escape, the Ocarina of Time in the moat and
the Song of Time in the Temple of Time. Two overlays were missing: `Item_Ocarina` (213 lines, the
moat's ocarina and Impa's throws) and `En_Ganon_Mant` (505, Ganondorf's cape, an old gap since the
nightmare). Things to settle:

- The cape is a cloth simulation whose vertices the actor rewrites every frame, in a display list
  that lives in its overlay; the pack's bakes had no way to move single vertices.
- `En_Viewer` writes the cape's attachment points only for type 5 (`EnViewer_UpdateGanondorfCape`
  returns otherwise), but types 3, 7, 8 and 9 spawn it too.
- The escape's run: entering the field at `ENTR_HYRULE_FIELD_0` puts Link inside the trigger, and the
  trigger halts Link wherever he is, so a walk to a point past it never finishes.
- The ocarina lies under the moat: what the dive needs.

## Decision

1. **`Item_Ocarina` whole**, every type (0 and 1 Impa's throws, driven by the cutscene's frame; 2
   nothing; 3 the moat's; others killed). Its draw is `GetItem_Draw(GID_OCARINA_OF_TIME)` without
   the highlights (`func_8002EBCC`, `func_8002ED80`), as `Item_Etcetera`'s. Taken, it isn't killed:
   it stays undrawn and starts `gHyruleFieldZeldaSongOfTimeCs` on every frame the get-item text is
   closing, as the C does.
2. **`En_Ganon_Mant` whole**, by a helper. Each of its 144 vertices is drawn as a bone of its own:
   `BakeSegment::BonedVertices` binds the overlay's vertex array on segment 0x0C and gives every
   vertex a bone (the interpreter's `vertex_bones`, the skins'), so the per-frame positions and
   normals the C writes move the baked mesh (a translation, and a rotation from the file's normal to
   the written one). The tear works on the cape's own copy of `gMantTex` (the C's is the overlay's,
   shared by every cape until it unloads; only one cape is ever in play).
3. **The cape as the C leaves it:** `EnViewer_UpdateGanondorfCape` ported (BREG and KREG read as 0);
   for types 3, 7, 8 and 9 nothing writes the cape, so in the nightmare and the escape it is a cape
   of no width at the world's origin. Ported as written; the hand test looks at Ganondorf's cape in
   the escape (the user's view of the game wins, if it differs). `sGanondorfCape` (the last cape
   spawned) is `En_Viewer`'s overlay static.
4. **Two runs, no fights.** `Route::Escape` from `zora-sapphire` with a debug start short of the
   trigger (160, 0, 1865), walked on with the new `Task::WalkUntil` (on until the escape's layer runs);
   `Route::OcarinaOfTime` from `zelda-fled` on the moat's south bank (299, 0, 1160), into the water,
   the new `Task::DiveFor` (the dive held until `EVENTCHKINF_43`), then A through the texts, the
   Temple of Time's playback (`Task::Playback`, the Song of Time) and the field. The moat's floor is
   -140 under a surface at -60: any scale's dive reaches the ocarina.
5. **Presets:** `zelda-fled` (the escape's end: `EVENTCHKINF_ZELDA_FLED_CASTLE`, `_82`) and
   `ocarina-of-time` (`EVENTCHKINF_43`, Hyrule Field's switch 3, `ITEM_OCARINA_OF_TIME`,
   `QUEST_SONG_TIME`, `EVENTCHKINF_A9`): Phase 12's start, each end checked against its preset.

## Consequences

- The Ocarina of Time and the Song of Time are reached: Phase 11's exit, part 2. Phase 12 (the Temple
  of Time, the Door of Time) starts from `ocarina-of-time`.
- Phase 11's actors left: `En_Fr`, `En_Kakasi`, `En_Kakasi3` (milestone 9), `En_Tp`, `En_Vali`
  (milestone 8).
- The cape's normals are written 30 long (of 127): dimmer under the N64's lighting, but the renderer
  normalises them. Only the ending's cape (type 5) is ever written.
- The game's bug kept (`@bug (game)`): `EnGanonMant_UpdateStrand`'s `rot[11] = rot[10]` writes past
  the end of the rotations (into the velocities).
