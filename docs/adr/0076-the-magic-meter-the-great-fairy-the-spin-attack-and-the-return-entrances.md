# 0076: The magic meter, the Great Fairy, the spin attack and the return entrances

- **Status:** accepted, built in GAME-09 milestone 6 (2026-10-17)
- **Date:** 2026-10-17
- **Builds on:** [ADR 0017](0017-interface-sprites.md) (the HUD's sprites),
  [ADR 0071](0071-the-hud-timers-and-talons-cucco-game.md) (`z_parameter.c`'s statics outliving the
  play state), [ADR 0072](0072-bombs-and-bomb-flowers.md) (a spawn from Player after his update),
  [ADR 0022](0022-cutscenes.md) (the cutscene system), [ADR 0066](0066-the-actor-schema.md) (the
  schemas), [ADR 0068](0068-glb-export.md) (the characters' table).

## Context

Phase 10's sixth milestone (`docs/GAME-09-death-mountain-and-dodongos-cavern.md`): Death Mountain's
summit, the Great Fairy's fountain and magic. Taken before milestone 5 (the user's order), since the
magic meter and `En_M_Thunder` are systems every later phase leans on, and the fountain was a dead
end (its exit is a return entrance). Four things had to be settled from the C first:

- **Where the magic state lives.** `gSaveContext`'s `magicState`, `prevMagicState`,
  `magicCapacity`, `magicFillTarget` and `magicTarget` weren't in the port, nor `Magic_Fill`,
  `_Reset`, `_RequestChange`, `_Update` and `_DrawMeter`.
- **`Bg_Dy_Yoseizo`'s other rewards** (double magic, double defence, the three spells): whole, or
  placeholders chosen at init as `Demo_Du`'s other cutscenes were (ADR 0075).
- **The return entrances** (`sReturnEntranceGroupData`): until they're ported Link can't leave the
  fountain.
- **`En_M_Thunder`**, which Player spawns for every spin attack (`func_80837530`), and what Player
  needs beyond the spawn.

## Decision

1. **The magic state is the save context's, outside the save** (`SaveContext::magic_state` and the
   four others; the SRAM image's tail now carries them, as `gSaveContext` does, and nothing reads
   them back). `oot_game::magic` holds `Magic_Fill`, `Magic_Reset`, `Magic_RequestChange` (and
   `magic_request_add` for `Item_Give`'s jars, which have no audio or lens to hand) and
   `Magic_Update`; the actors call them through the play state (`PlayState::magic_fill`, ...).
   `z_parameter.c`'s border statics (`sMagicBorderR/G/B`, `sMagicBorderRatio`, `sMagicBorderStep`)
   are `MagicStatics`, kept on the interface and carried over scene changes with the timers'.
   `Interface_Update` runs the meter where the C does (after the A button's flip, before the hazard
   timer), under the C's conditions (not paused, no message, no transition, no game over, and in a
   cutscene only with Player out of his cutscene mode), with the meter's start (`magicLevel` from 0
   with magic acquired: `MAGIC_STATE_STEP_CAPACITY`). `Magic_DrawMeter` is drawn after the rupee
   counter, as `Interface_Draw` draws it: the ends and the middle (one bake per capacity, a multiple
   of 8) and the fill (one bake per width). The file select, the map select and the game over's
   continue set the fill target to the saved magic as the C does; the pause menu won't open while
   the meter grows or fills; the songs' destroys call `Magic_Reset` (BACKLOG #49 closed).
   `actorCtx.lensActive` is `ActorContext::lens_active` (the lens isn't ported: false).
2. **The fill texture's load:** the C loads `gMagicMeterFillTex` as I4 16×16 (the XML has it IA8
   8×8). It's `parameter_static`'s last texture, 64 bytes: the load's lower half reads past the
   file's end. The fill's rectangle is 7 rows high from t 0 (mask 0 clamps s at column 15), so only
   rows 0 to 6 are sampled: the bake loads the 16×8 the file holds.
3. **`Bg_Dy_Yoseizo` whole, every reward.** The C doesn't split by reward: the same action functions
   branch on the scene (the magic fountains or the spells') and on `this->reward = play->spawn` in a
   few switch arms (the script, the new-acquire checks, the save's writes, the light's colour, the
   `En_Ex_Item`). The other cutscenes are data the cutscene system plays; `En_Ex_Item` and
   `Demo_Effect` were ported. So nothing is a placeholder; one-points 8603 and 8604 (the revisits)
   come with her. `En_Dy_Extra` (the revisit's beams) and `Door_Warp1`'s orange warp
   (`func_809998A4`, and the coloured warps' rays at `warpAlpha` as the C draws them) too.
4. **The return entrances are Player's** (the C's only user): `RETURN_ENTRANCE_GROUP_DATA` and
   `_INDICES` in `player.rs`, looked up by `return_entrance(next, play->spawn)` in
   `Player_HandleExitsAndVoids`, then the existing `Scene_SetTransitionForNextEntrance`. A spawn past
   its group reads on into the next group (`@bug (game)`, never placed); past the table the exit
   logs. The scripted runs' exit look-up (`script::exit_to`) follows them too.
5. **The spin attack:** Player's charged spin wasn't ported at all, so it came with `En_M_Thunder`:
   `Player_ActionHandler_8`, the three charging actions, the release (`func_80844BE4`: the big spin
   from 0.85), their animation tables and cutscene cues, and the `unk_844` countdown in
   `Player_UpdateCommon` with `func_80832440`'s reset (both missing before: the combo window was
   never closed, so slashes 15 frames apart comboed; in the C the combo count resets 8 frames after B
   is let go). `func_80837530`'s spawn is a `PlayRequest` after Player's update carrying a snapshot
   of him (his waist, yaw, `stateFlags2`) that `En_M_Thunder`'s init reads, handing back the
   `PLAYER_STATE2_17` clear; it's `ACTORCAT_ITEMACTION`, so it still updates after him in that
   frame. Its draw reads Player's `mf_9E0` (now kept). `Eff_Dust` (the charge's sparkles, its
   child) whole; `Camera_Battle4` (`CAM_MODE_CHARGE`'s function, which fell back to `Normal1` and
   ran away) ported.
6. **The falling rocks:** `En_Encount2` and `En_Fire_Rock` whole (the summit's spawner, the
   Ganon's Castle collapse's ceiling and floor rocks), reusing `Demo_Kankyo`'s rock bake.
7. **Three short runs** from debug starts (the user's rule: no fights, no long walks):
   `Route::FairyWall`, `Route::GreatFairy`, `Route::SummitOwl`; the presets `fairy-fountain` and
   `fairy-magic` (milestone 5 and Phase 11 start from it), each checked against its run.

8. **The hand test's fixes** (the user's report: the owl's chest black and white, the shadows broken
   up on the trail's slopes):
   - **A clamped texture axis with a mask** is masked inside its tile (`eng_gbi::texture`): the RDP
     clamps a coordinate to the tile's size and then masks it, so a tile wider than its mask
     repeats the texture up to its edge (the owl's chest: a 16-texel texture on a 640-texel
     clamped tile). The decoder made the image as wide as the tile and read past the texture into
     the rest of TMEM. It reads only the mask's texels now, masked (mirrored with the mirror bit).
   - **The shadows are `ActorShape`'s, drawn by `Actor_Draw`** (`oot_game::shadow`): an actor's
     `ActorShape_Init` sets `Actor::shape_shadow_draw` (none, the circle, the white circle, the
     horse's, the feet's), `shape_shadow_scale` and `shape_shadow_alpha`; the render state carries
     them with the floor (`ShadowRs`); `PlayState::draw` draws them after the actor and its colour
     filter, as the C does: the ROM's `gCircleShadowDL`, `gHorseShadowDL` and `gFootShadowDL` under
     `SETUPDL_44` and the shadow's combiner, laid on the floor polygon's plane (`func_80038A28`;
     the feet on each foot's floor, `func_800BFCB8`), shrunk and faded with the height; the feet
     shadow per foot per light shining down (the bound point lights, then the environment's two).
     Every ported actor's `ActorShape_Init` and shadow writes are ported; the port's stand-in (a
     soft disc laid flat at the floor's height, translucent, an approximation in each actor's draw)
     is gone. BACKLOG #84 closed; `En_Kanban`'s own shadow is #88.

## Consequences

- Magic exists: a save with it grows and fills its meter on load, the jars, fairies and the Great
  Fairy fill it, the charged spin and `En_M_Thunder` spend it. The spells (`Magic_Wind`, `_Dark`,
  `_Fire`, Player's spell actions, `nayrusLoveTimer`) and the magic arrows' cost are what's left of
  magic: they come with their items.
- Slash combos follow the C's timing: the goldens' routes that slashed three times 15 frames apart
  now make three first slashes (16 hashes re-recorded).
- The shops' and the spell fountains' return entrances work when their scenes come.
- An actor ported from now on gets its shadow by porting its `ActorShape_Init` (and its writes to
  the shape's shadow): nothing to draw by hand.
