# 0059: Castle Town's actors, the game's 4:3 frame, two bake forms

- **Status:** accepted, built in GAME-07 milestone 1 (2026-10-11)
- **Date:** 2026-10-11
- **Builds on:** [ADR 0014](0014-prerendered-backgrounds.md) (the pictures, which this amends at
  16:9), [ADR 0012](0012-actor-bakes.md) (bakes), [ADR 0058](0058-the-culling-volume.md) (the
  culling volume, which the market's crowd needs).

## Context

Castle Town by day is the first crowd: about thirty townsfolk (`En_Hy`), the haggling townsfolk at
the stall (`En_Mu`), the dancing couple (`En_Tg`), child Malon (`En_Ma1`), the cucco girl and her
cucco (`En_Niw_Girl`, `En_Niw`), a dog (`En_Dog`), guards (`En_Heishi4`), a butterfly group
(`Obj_Mure`, `En_Butte`); the Temple of Time's outside has gossip stones (`En_Gs`) and its altar a
text spot, windows and a warp pad (`En_Wonder_Talk`, `Bg_Toki_Hikari`, `Bg_Mjin`). Five things
needed a decision:

- **The 16:9 edges of the pictures (BACKLOG #1).** A `CAM_SET_PREREND_FIXED` room's picture
  covers the middle 4:3 of a wider target (ADR 0014); beyond it the room's own geometry and actors
  showed, which the console never draws (its frame is 4:3).
- **Draws the bake forms couldn't say.** `En_Gs_Draw` runs `gGossipStoneMaterialDL`, then sets a
  prim colour (the bomb's flash), then the stone's lists; the material list sets its own prim colour
  at its end, so a prelude before it is overwritten. `func_808BA2CC` draws
  `&object_toki_objects_DL_0009C0[10]`: a list from its eleventh command.
- **The overlays' statics** that several actors share: `En_Niw`'s `D_80AB85E0` (a cucco has
  called the attack) and its Zora's River flags, `En_Butte`'s transformation effect and
  `EnButte_FollowLink`'s swing.
- **What Player had to answer for them**: the held item (`heldItemAction`, a butterfly follows a
  Deku Stick) and the sword's tip (`MELEE_WEAPON_INFO_TIP`).
- **The Temple's room 0**, behind the Door of Time: `En_Xc`, `Demo_6K`, `Bg_Toki_Swd`,
  `Bg_Spot02_Objects`, planned as "killed for a child, init checks only".

## Decision

- **The game's 4:3 frame** (amended 2026-10-12, after playing it). `eng_gfx::DrawLists::pillarbox`
  asks the renderer for black bars beside the middle 4:3: over the 3D lists and the pause menu,
  under the overlay's fill and the HUD; the fills stay inside the 4:3. Every play state sets it,
  every frame. First it was set only behind a prerendered picture (`CAM_SET_PREREND_FIXED`), but
  playing at 16:9 showed the townsfolk vanishing at the screen's sides: the game culls through its
  4:3 `viewProjectionMtxF` (ADR 0058), so beyond the 4:3 it neither draws nor, for most actors,
  updates. Widening the culling would change the game (actors updating where they don't); showing
  the game's frame doesn't. The 3D view keeps the vertical fov (ADR 0013), so the middle 4:3 is the
  game's view exactly.
- **A list from an offset**: a `BakeBody::DLists` symbol may be `symbol+0x50`, run from that many
  bytes in (the importer resolves the symbol and adds the offset).
- **A list run before others from a segment**: a `BakeSegment::Texture` binds any symbol's data
  to a segment, so a material list bound there and named in the prelude runs first, then the
  prelude's next segment (the draw's dynamic prim colour), then the body: the gossip stone's order,
  with no new bake kind.
- **Overlay statics** in `PlayState::overlay_static::<T>(actor id)`, as `Obj_Syokudai`'s; a
  static that isn't zero at load (`D_809CE410` = 1500) gets its own `Default`.
- **Player's interface** gains `held_item_action` and `melee_weapon_tip` (defaults 0 and the
  origin, Player's from its fields).
- **The Temple's room 0 stays unported.** It loads only through the door's `En_Holl`, which a child
  can't pass; and the plan was wrong about two of them: `Demo_6K` type 16 waits for a cutscene cue
  and `Bg_Toki_Swd` is drawn for a child. BACKLOG #37 holds them for the door.
- **Whole actors with logged gaps**, each named in its module's docs: the ocarina's calls
  (`Message_StartOcarina`: `En_Ma1`'s Epona's Song, `En_Gs`'s songs) until milestone 3; the mask
  reactions (no masks, BACKLOG #39); the splash, ripples and `EffectSsBomb2` (BACKLOG #40); Player's
  lift (BACKLOG #4: `En_Niw` offers itself, Player doesn't take it yet). Spawned but unported actors
  are placeholders (BACKLOG #38).
- **`Camera_Fixed1`** (the market's balcony view) and **one-point camera 2290** (the cuccos'
  attack) are ported; the market's message box rule (`SCENE_CAM_TYPE_FIXED_MARKET`) already was.
- **`Environment_ChangeLightSetting`** (added 2026-10-12, after the hand test's question about the
  haze outside the Temple of Time): Player asks for his floor's light setting every frame
  (`SurfaceType_GetLightSetting`, `PlayRequest::ChangeLightSetting`), as z_player.c does where
  his floor is the scene's. The haze is the game's: the shrine's floors ask for 31, past
  `LIGHT_SETTING_MAX`, so setting 0 stays, with grey fog from 970 to 3000. The Temple of Time's
  floors ask for 2 and 3, which the port had ignored.
- **The exit's run, `Route::Market`**: from Castle Town's entrance into the market, a townsman's and
  Malon's talks, out to the castle's road and its intro (`gHyruleCastleIntroCs`).

## Consequences

- At 16:9 the game shows its 4:3 frame and black beside it: the Kokiri shop's shapes and Castle
  Town's entrance's walls are gone, and nothing pops in or out at the sides. 69 renders gained the
  bars (proved, golden/README.md). Widescreen, if wanted, would need the culling widened and its
  effect on the game's behaviour decided.
- The circle shadows are still the stand-in (a soft disc of the shadow's scale), as before.
- Malon's Weird Egg (her texts at the castle, `GI_WEIRD_EGG`) is ported but not on a route until
  milestone 2's castle grounds.
- **Not modelled:** the gossip stones' and Malon's songs (no ocarina); the butterflies' fairy needs a
  Deku Stick held still far from home, which no route does (the hand-test list does).
