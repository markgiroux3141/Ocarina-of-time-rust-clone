# 0064: Kakariko: lifting and carrying, the village's draw configs, a scene's actors destroyed

- **Status:** accepted, built in GAME-07 milestone 5 (2026-10-13)
- **Date:** 2026-10-13
- **Builds on:** [ADR 0060](0060-hyrule-castles-grounds.md) (Player's exchange items, `Obj_Tsubo`,
  `Bg_Gate_Shutter`), [ADR 0051](0051-textures-replaced-by-source-and-the-object-ram.md) (a draw's
  textures replaced by source), [ADR 0062](0062-the-aim-master-quest-and-the-game-editor.md) (each
  actor's Params and Flags), [ADR 0063](0063-the-courtyards-zelda-and-impa.md).

## Context

Milestone 5 takes child Link through Kakariko with Zelda's letter: Anju's cuccos carried into her
pen (one glided down), the letter shown to the gate guard, and the village's people and interiors.

- **Player's lift, carry and throw** were logged since GAME-03: `Player_ActionHandler_2`'s
  `GI_NONE` branch, `func_8083A0F4`, the lift and throw actions (`Player_Action_80846050`,
  `_80846120`, `_80846260`, `_80846358`, `_80846408`, `_808464B0`, `_80846578`),
  `Player_UpperAction_CarryActor` with the cucco's glide, `Player_ActionHandler_9`, the do-actions
  (GRAB, THROW, DROP) and some fifty checks of `PLAYER_STATE1_CARRYING_ACTOR` across `z_player.c`.
  Lifting writes to another actor (its parent, its floor flags, its speed and yaw when thrown), and
  `Player_PostLimbDrawGameplay` places what Player carries between his hands every frame.
- What Player can lift had its lifted states unported: `En_Ishi`'s (`EnIshi_LiftedUp`,
  `EnIshi_Fly`) and `En_Kusa`'s (`EnKusa_LiftedUp`, `EnKusa_Fall`, `EnKusa_UprootedWaitRegrow`).
  `Obj_Tsubo` and `En_Niw` had theirs.
- **Kakariko's and the guard house's draw configs** point a segment at one of two textures by
  `nightFlag` (`sKakarikoWindowTextures`, `sGuardHouseView1Textures`, `sGuardHouseView2Textures`):
  the windows light at 18:00. The import bakes each scene layer's rooms with that layer's choice
  (the day's on the day layers), and `nightFlag` changes inside a layer.
- **A scene change didn't destroy the actors:** `PlayState::reinit` saved the scene flags and built
  the next play state. The C's `Play_Destroy` deletes every actor first (`Actor_CleanupContext`,
  `func_80031C3C`): each one's sounds stop and its destroy runs, and some destroys set flags
  (`BgRelayObjects_Destroy` slows the windmill again, `EVENTCHKINF_65`).
- **`En_Sth`'s dances and head lists live in its overlay** (`ovl_En_Sth`: `sParentDanceAnim`,
  `sChildDanceAnim`, `D_80B0A050`, `D_80B0A3C0`). The importer skipped an overlay's animations
  (their pointers are the overlay's link-time addresses) and couldn't draw a limb from an overlay
  list.

## Decision

1. **Lifting through Player's requests.** What Player does to the carried actor goes through
   `PlayRequest`s applied right after his update, in the C's order: `Lift` (its parent, and for
   the plain lift its floor, wall, ceiling and water flags cleared), `Throw` (its yaw, speed and
   `velocity.y`, then `ReleaseHeld` from `func_80834644`), `PutDown`, and `Launch` (the heavy
   block's speed, gravity and `minVelocityY`). Where it's carried is set at draw time, as the C
   does: the left hand's part (the actor's yaw kept with `unk_3BC.y`, or its pitch with the hand
   for `ACTOR_FLAG_CARRY_X_ROT_INFLUENCE`; with nothing held `unk_3BC` takes the hand's rotation)
   and the right hand's (`sGetItemRefPos`, between the hands). `Bg_Heavy_Block` isn't ported: the
   gold gauntlets' lift is whole in Player, and the block's `unk_164` write is noted. The adult's
   Master Sword pedestal (`Bg_Toki_Swd`) is noted.
2. **The lifted things whole.** `En_Ishi`'s and `En_Kusa`'s lifted, thrown and regrowing states
   are ported, with their Params and Flags paragraphs. A child without a strength upgrade only tugs
   at a bush (`Player_GetStrength() <= PLAYER_STR_NONE`: `Player_Action_80846408`); small rocks,
   pots and cuccos he lifts.
3. **Textures by state as segment textures.** A draw config's pointer segments that change with
   the state are described by `scene_table::segment_textures` (the segment, the buffer, the
   texture's file and symbol) for `SDC_KAKARIKO_VILLAGE` and `SDC_MARKET_GUARD_HOUSE`. The scene
   state decodes every texture they can name at load (`SceneState::texture_cache`) and each frame
   lists the current ones (`segment_textures`); the room draw puts them in place of what the
   meshes loaded from that segment (`DrawParams::texture_images`, keyed by the load's source
   address, 0x08000000 and 0x09000000). The pack test checks them against the C interpreter's
   pointer bindings in every state. `SDC_GROTTOS` is display lists only (its scrolls).
4. **`Actor_CleanupContext` on a scene change.** `reinit` now deletes every actor, category by
   category from each list's head, before `Play_SaveSceneFlags`: its sounds stop
   (`Audio_StopSfxByPos`) and its destroy runs, so a destroy's flags reach the save.
5. **An overlay's assets as RAM holds them.** The importer reads an overlay's animations from the
   overlay placed at its link address (`segments.csv`'s VRAM: `ram_image_at`), and a limb whose
   list is in an overlay draws with the overlay on segment 0, as a `DLists` body's overlay list
   already did. `ovl_En_Sth` is the only overlay whose XML lists animations.
6. **The actors whole**, each with its Params and Flags paragraphs: `En_Niw_Lady`,
   `En_Attack_Niw`, `En_Daiku_Kakariko`, `En_Toryo`, `En_Hs2`, `En_Ani`, `Bg_Spot01_Fusya`,
   `Bg_Spot01_Idohashira`, `Bg_Spot01_Idomizu`, `Bg_Spot01_Objects2`, `En_Cow`, `En_Fu`,
   `Bg_Relay_Objects`, `En_Ssh`, `En_Sth`. The grottos' actors (`En_Torch`, `Obj_Comb`, `En_Fish`)
   stay placeholders for Phase 20 (the user's call). `SaveContext::clear_event_chk_inf`
   (`CLEAR_EVENTCHKINF`) joins the flag helpers.
7. **The exit's run** (`Route::Kakariko`): Anju's pen is fenced all round, so the cuccos are thrown
   over its north fence (`Task::Throw`, from (330, 80, 1350)). The run takes `sKakarikoPosList`'s
   0 (by the way in), 2 (east of the pen) and 1 (on the landing below the gate's stairs, carried
   onto the low roof south of the watchtower and glided off it: `Task::Glide`). `Task::Lift` takes
   the cucco nearest a point; `Task::ShowItem` now presses A at an event's box too (the gate guard
   waits for one). The preset `kakariko-letter` is the end of milestone 4's run.

8. **Kakariko's windows halfway to their walls** (after the hand test, the user's call): a
   departure from the data, which places the window panels 6 to 10 units in front of their walls.
   The import moves each panel (the triangles textured from segment 8, by plane) towards the wall
   triangle right behind it by half the gap (`oot_import::room::kakariko_windows_halfway`), so
   they don't float, and don't fight the wall for depth as flush panels would.

## Consequences

- Pressing A by a bush or a rock now lifts it or tugs at it, where it did nothing: a route that
  presses A there changes. None of the recorded routes did.
- Every ported actor's destroy now runs at a scene change. Link's house's cow (`En_Cow`), killed
  for a child, changed the Kokiri runs' actor counts (golden/README.md).
- A draw config with a pointer segment that changes inside a layer needs a `segment_textures`
  entry, or its meshes keep the layer's bake.
- `Bg_Heavy_Block` (the gold gauntlets) and the splash and ripples of things thrown into water
  (`EffectSsGSplash`, `EffectSsGRipple`, BACKLOG #40) wait for their phases.
