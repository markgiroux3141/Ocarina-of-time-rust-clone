# 0070: Lon Lon Ranch: Epona's Song's end, the horses, Ingo and the Guays

- **Status:** accepted, built in GAME-08 milestone 3, step (a) (2026-10-14)
- **Date:** 2026-10-14
- **Builds on:** [ADR 0030](0030-skin-skeletons.md) (skin skeletons), [ADR 0061](0061-the-ocarina.md)
  (the ocarina; `R_EPONAS_SONG_PLAYED` kept), [ADR 0064](0064-kakariko-lifting-and-the-village.md)
  (draw configs' textures by state), [ADR 0066](0066-the-actor-schema.md) (each actor's schema),
  [ADR 0069](0069-the-lost-woods-skull-kids-and-target-scrub.md) (the helpers' worktrees).

## Context

Milestone 3 of Phase 9 (`docs/GAME-08-woods-meadow-and-ranch.md`) is Lon Lon Ranch, about 3,500
lines of C. The user split it in two steps (2026-10-14): (a) the ranch's story (Epona's Song's
end, the draw configs, the foal, the horses, Ingo, the Guays, adult Malon's init); (b) Talon's
house (the milk, the Super Cucco game, the HUD timer). This record is step (a).

What the survey said, checked again (`ootx scene-info` on `spot20` layers 0, 1 and 5,
`malon_stable`, `souko`):

- Malon (`En_Ma1`) and her lesson were ported; the playback's end reloads the ranch with
  `CS_INDEX_1` (layer 5, `gLonLonRanchEponasSongCs`), whose terminator is destination 94
  (`ENTR_LON_LON_RANCH_1`). **The survey said 94 had no case in the port's terminator: it had**
  (`TERMINATOR_DESTINATIONS`, transcribed with the others in GAME-04b). Nothing was missing there.
- `SDC_LON_LON_RANCH` (the ranch, segment 8 in OPA) and `SDC_LON_LON_BUILDINGS` (Talon's house,
  segment 8 in XLU) point a segment at a texture by `nightFlag`; the stable is `SDC_DEFAULT`.
- Not ported: `En_Horse_Link_Child` (the foal), `En_Horse_Normal` (the paddock's and the stable's
  horses), `En_In` (Ingo), `En_Crow` (the Guays, the ranch's night layer), `En_Ma2` (adult Malon,
  killed for a child); `Horse_RotateToPoint`, `Skin_GetLimbPos`, `Interface_SetTimer`.

## Decision

1. **The shared code first, here:** `oot_game::horse` (`Horse_RotateToPoint`, the only `z_horse.c`
   function a child's horses call); `skin::get_limb_pos` (`Skin_GetLimbPos`); the two draw configs
   as `segment_textures` entries (checked against the C interpreter and against the rooms'
   textures loaded from segment 8); the HUD timers' save fields (`timerState`, `timerSeconds`,
   `subTimerState`, `subTimerSeconds`, `timerX`, `timerY`: in the SRAM image's `SaveContext`
   part, cleared by the file select and the game over as the C does) and `Interface_SetTimer`.
   The timers' countdown and draw are step (b)'s. `QUEST_SONG_EPONA`.
2. **The actors, by three helpers in their own worktrees:**
   - `En_Horse_Link_Child` and `En_Horse_Normal` whole, the first live actors on skin skeletons.
     The foal's eyes are swapped per draw by source address (`DrawParams::texture_images`), not
     baked per eye: the skin's animated limb 5 loads its vertices through segment 8 before limb
     13 draws the eye from it. The draw-time work (the head spheres placed with `get_limb_pos`, the
     horse's pitch, the clones' sounds and cylinders) runs in `draw_update`.
   - `En_In`: the child's Ingo whole (where he is by scene, time, rot.z and Talon's return, his
     texts and flags, his head and chest, the draw with the basket and pitchfork, the adult-era
     body for `params > 0`); `EnIn_GetStartMode` whole for both ages. **The adult's race and rental
     (Phase 13) are a placeholder**, by the scope rule: where the C would start them, Ingo's init
     makes the same checks and spawns the placeholder (the C decides on his first update; the port
     can't turn a live actor into a placeholder, so the choice moves to the init, the kills keep
     the first update).
   - `En_Ma2`: `EnMa2_GetSpawnIndex` whole (a child: killed), a placeholder for the adult's.
   - `En_Crow` whole: the dive, the turn-away from a nut, the death, the respawn, `sDeathCount` as
     an overlay static (the tenth to die comes back big, with a red rupee).
3. **A preset for milestone 2's end, `lost-woods-games`:** `meadow-saria` and what milestone 2's run
   changes (found by diffing the save over a run of `Route::LostWoods`): `ITEMGETINF_16`, `_17` and
   `_1D`, two pieces of heart, the 40 seed bag full, `ocarinaGameRoundNum` 3, six more rupees. No
   scene flag changes. `EVENTCHKINF_TALON_RETURNED_FROM_CASTLE` (0x14) was set already (the castle's
   presets), so the ranch has its day layout after Talon's return.
4. **The run (`Route::Ranch`, from `ENTR_LON_LON_RANCH_0`):** the ranch's intro; south down the
   gate's lane and through the corral's gate (found with a breadth-first search over the
   collision); Malon's three talks (0x2047, 0x2048, 0x2049); the ocarina out by her
   (`Task::OcarinaOut`: no song, her lesson takes over); the playback; layer 5 and its terminator;
   the foal comes to Link; Link walks off until she has gone home and faces him; Epona's Song, and
   she comes again.

## Consequences

- Step (b) has the timers' state to drive: the countdown, the draw, the sub timer and the cucco
  game's music (`func_800F5ACC`); BACKLOG #44 stays open until then.
- The horses' skin draw is ready for the adult's Epona (`En_Horse`, Phase 13, BACKLOG #51).
- The stable's interior is a skybox (`SKYBOX_STABLES`) with untextured depth-only geometry for the
  rails and posts: a horse behind a rail is hidden by the invisible rail, where the painted one is
  seen from the game's camera. Seen from elsewhere (a debug view) a band of the horse is missing.
  That's the game's.
- Sounds the C plays at a point of their own (the horses' `unk_204`, the clones' positions) play at
  the actor's projected position: `SfxPos` has no free position (BACKLOG #68).
