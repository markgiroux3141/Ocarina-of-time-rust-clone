# 0053: `Demo_Effect`, curve skeletons, and vertex colours rebuilt by source

- **Status:** accepted, built in GAME-06 milestone 1a (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0022](0022-cutscenes.md) (the cutscene layers and the actors' cues),
  [ADR 0051](0051-textures-replaced-by-source-and-the-object-ram.md) (the object RAM the game
  writes, things replaced by their source address), [ADR 0052](0052-the-blue-warp.md) (the
  emerald's cutscene chain after part 1, BACKLOG #23).

## Context

The Kokiri Emerald's cutscene chain after part 1 (BACKLOG #23) is mostly `Demo_Effect`
(`z_demo_effect.c`, 2,092 lines): the goddesses' lights, the Triforce, the Kokiri Emerald held up
and floating over Link, a light. By hand (2026-10-09) the user found the Triforce's part missing
pieces and the emerald never shown. Its 26 types are one file; this ROM's Deku Tree also reaches
one more type, the Song of Time blocks' time warp, which `Obj_Timeblock` spawns (a placeholder
since GAME-05 milestone 4c, ADR 0044).

Three things in it had no counterpart in the port:
- **The time warps are a curve skeleton** (`SkelCurve`, `z_fcurve_data_skelanime.c` and
  `z_fcurve_data.c`, 332 lines): limbs with no position of their own, animated by knots
  (constant, linear or cubic Hermite) for each of nine values. The importer skipped curve
  skeletons and their animations.
- **Two draws write their object's vertices:** `DemoEffect_TimewarpShrink` sets the alpha of
  `gTimeWarpVtx`'s vertices as the warp fades (by `sTimewarpVertexSizeIndices`), and the Triforce's
  draw sets eight of `gTriforceVtx`'s to the light column's opacity. The writes are object RAM:
  they stay, and every draw of those lists reads them. The engine draws baked meshes (ADR 0006).
- **Its colours carry a LOD fraction:** its lists blend two tiles by `PRIM_LOD_FRAC`, which comes
  from the draw's `gDPSetPrimColor(m, l, ...)`. The dynamic colour segment the bakes used
  (`BakeSegment::DynamicColor`) bakes `l` as 0.

The user chose (2026-10-09) to port `Demo_Effect` whole, and to play the whole chain (milestone
1b plays the other scenes' parts).

## Decision

- **`Demo_Effect` is ported whole** (`oot_actors::demo_effect`): every init case, the 25 update
  functions, the 11 draws, the cue helpers. The struct's union is kept as its bytes (three `u8`s
  and an `s16`) read through the C's member names, so each type's aliasing stays as the C has it.
  The draws' changes to the actor (the god lights' turn, the light's first-draw flicker, the blue
  orb's spin, a medal's first frame, the vertex writes) run in `draw_update`, once per game frame;
  their sounds in `draw_sfx`. A draw the C skips on its first frame skips it here too
  (`draw_skipped`). `EffectSsKiraKira_SpawnDispersed` (the jewels' and medals' sparkles) isn't
  ported: its three `Rand` calls are made, as `En_Elf`'s are.
- **Curve skeletons are ported whole** (`oot_game::skel_curve`): `Curve_CubicHermiteSpline`,
  `Curve_Interpolate`, `SkelCurve_Clear`, `_Init`, `_Destroy`, `_SetAnim`, `_Update` (the
  constants read as `u16`, the scales ×1024, the rotations to binary angles, the positions ×100),
  `_Draw` and `_DrawLimb` (the override and post functions as closures; the draw returns each
  limb's matrix and list, which the actor draws as bakes). The pack holds them as records of their
  own: `curve/<file>/<skeleton>` (the limbs) and `curve_anim/<file>/<animation>` (the knot counts,
  the knots, the constants), the animation sized by the skeleton at its `SkelOffset`.
- **Bakes record each vertex's source:** `eng_gfx::Batch::sources`, the segmented address of the
  `Vtx` each triangle vertex was loaded from, filled when the interpreter is asked
  (`Interpreter::track_vertex_sources`); the importer asks for every display-list bake. A draw that
  needs its vertices as the game wrote them rebuilds the colours (`DrawParams::vertex_colors`, in
  the mesh's order) from the object RAM by source. The written regions hold the arrays' colour
  words (bytes 12 to 15 of each `Vtx`), keyed by the array's offset, started from the bake's own
  vertices: what the draws read back.
- **The prim colour's LOD fraction is baked from the draw's command:** `Demo_Effect`'s colour
  segments are `BakeSegment::Dynamic` lists holding the draw's own `gDPSetPrimColor(m, l, ...)`
  (128 or 64), so the bake keeps `l` and a draw's `SegmentValues` replaces only the colour.
- **A debug start can enter a cutscene layer:** `--cutscene 0xFFF2` (the game and the sandbox)
  sets the save's `cutsceneIndex` for `Play_Init`, as this debug ROM's map select can; routes name
  theirs (`Route::cutscene`).

## Consequences

- Kokiri Forest's layers 4 and 6 play with their effects: Farore's light and her shower, the
  emerald's green light out of the Deku Tree, the emerald over Link. The goddesses, the Triforce
  and the rest wait only for their scenes (milestone 1b).
- The Deku Tree's Song of Time blocks show their time warp when the song is played (injected in
  the tests: the ocarina isn't ported).
- Any actor can draw a curve skeleton; `Demo_Tre_Lgt` (the chest's light) can have its draw
  (BACKLOG #25).
- Any bake can have vertices the game writes at run time; the pack's bakes are a little bigger
  (four bytes a vertex). No render changed.
- `BakeSegment::DynamicColor` stays for draws whose prim `l` is 0; a draw with another `l` bakes
  its own command.
