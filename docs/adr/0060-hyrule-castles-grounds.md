# 0060: Hyrule Castle's grounds: the exchange, the circle wipe, the water camera

- **Status:** accepted, built in GAME-07 milestone 2 (2026-10-12)
- **Date:** 2026-10-12
- **Builds on:** [ADR 0059](0059-castle-towns-actors.md) (Castle Town's actors, the 4:3 frame,
  overlay statics), [ADR 0058](0058-the-culling-volume.md) (the culling volume),
  [ADR 0012](0012-actor-bakes.md) (bakes), [ADR 0013](0013-camera-modes-and-screen.md) (the camera).

## Context

The castle's grounds (`spot15`) are the story's next step: Malon's Weird Egg hatches into the
Cucco; shown to Talon asleep by the milk crates, it wakes him and he runs home; the crates, free to
go then, are pushed into the moat, and up them Link reaches the drain into the guards' courtyard.
Sentries watch the grounds, and one that sees Link has him thrown out through a starburst circle.
Six things needed a decision:

- **Player's items.** Showing the Cucco is `Player_ActionHandler_13`'s exchange branch. The same
  function uses the bottles, casts the spells and takes the ocarina out, and the beggar's bottle
  (BACKLOG #41) needs `Player_UpdateBottleHeld`.
- **`En_Ta`** is 1,344 lines; the castle uses about 150 of them, Kakariko's guest house its own
  part, and Lon Lon Ranch's (the Super Cucco game, the milk) about 670.
- **`TransitionCircle`** (`z_fbdemo_circle.c`): the castle's throw-out (`CIRCLE(STARBURST, WHITE,
  FAST)`) and the drain's arrival in the courtyard (`CIRCLE(WAVE, BLACK, FAST)`) are its first
  circle types. It draws `code`'s `sTransCircleDL` in its own projection, over the scene.
- **The moat's camera.** Swimming in it, the camera takes the water box's bg camera
  (`NORMAL3`, `Camera_Jump3`) through `Camera_UpdateWater`, which also handles the eye going under
  water (lights, a quake, a distortion, an audio filter).
- **`En_Heishi2` type 5**, Kakariko's gate guard, opens `Bg_Gate_Shutter`, which the plan put in
  milestone 5.
- **The scripted run** has to climb vines, show an item, push crates and get caught.

## Decision

- **Player's exchange items whole; the other uses noted** (the user's call). The exchange branch
  of `Player_ActionHandler_13` (here `start_exchange_item`: the talk handed to the actor offering
  one for the item, else Ruto's letter read or the item looked at, 0xCF, with the turn-around
  camera), `Player_Action_ExchangeItem` (the Cucco's crow) and
  `Player_Action_8084279C`. The bottles' uses, the spells and the ocarina note that they're reached
  (BACKLOG #45). `Player_UpdateBottleHeld` and `Inventory_UpdateBottleItem` are ported (the beggar's
  bottle empties: #41 fixed). `Actor_OfferTalkExchange`'s item comes back through
  `npc::get_player_exchange_item_id`.
- **`En_Ta`: the castle's, Kakariko's and the ranch's night** (the user's scope). The ranch by day
  is a placeholder (BACKLOG #44). Its eyes are three bakes (setup DL 37 on 0xC, the eye on 8, the
  head's skin on 9). `@bug (game)` kept: the fidget's flag is cleared at the root limb, so the sway
  never stops. The run home is one-point cutscene 4175 (`D_8012147C`, `CAM_SET_CS_C`).
- **`TransitionCircle` as a pause-pass draw.** `TransitionState` holds the circle; `INSTANCE_INIT`
  sets it up (`SetType` twice, as the C), `RUNNING` updates it, and a shut circle is a full fill for
  the frame after. Its draw is `play_scene::circle_draw_cmd`: the bake of `sTransCircleDL` with
  the appearance's texture (segment 8), the scroll (`Gfx_BranchTexScroll`, segment 9) and the
  colours (0xC) as segment values, in `transition::CIRCLE_VIEW` (fovy 60, near 10, far 12800) with
  `guLookAt` from (0, 0, 400) and `guScale(14.8, 14.8, 1)`, put in the pause pass when that's
  empty. The ring's outer radius (25, 370 on screen) leaves the 4:3 frame's corners uncovered, as on
  the console. To bake a `code` list, the importer merges XMLs that name the same ROM file and puts
  `code`'s lists on segment 0 at their link address (VRAM 0x8001CE60).
- **`Camera_UpdateWater`'s settings half, and `Camera_Jump3`** (the user's call). Swimming in a
  water box with a bg camera (`Camera_GetWaterBoxBgCamIndex`, `@bug (game)` index 0 as none) takes
  its index; a box without one takes `NORMAL3`; diving takes `PIVOT_WATER_SURFACE` and back. The
  eye under water is only tracked and logged once (BACKLOG #43). `Camera_Jump3` is whole: the
  water mode switch (`OREG(44)`, `OREG(45)`), the swing round walls, the pitch clamps.
- **`Bg_Gate_Shutter` pulled forward** (the user's call), so `En_Heishi2` is whole: types 2 (the
  castle's gate, with `Bg_Spot15_Saku`), 5 (Kakariko's gate: the letter, the Keaton Mask), 6 (the
  window's guards, `gHeishiKingGuardDL`) and 9 (not placed; its bomb is an `En_Bom` placeholder,
  BACKLOG #33). The mask's draw is a bake of `gLinkChildKeatonMaskDL` from 8 bytes in, at limb 16.
- **`Obj_Tsubo` whole**, its object chosen by kind (`gameplay_dangeon_keep` or `object_tsubo`);
  without it the pot dies at init ("Bank danger!"), as the castle's two do. The lifted and thrown
  states are ported but unreachable until Player lifts (BACKLOG #4); the splash is logged (#40).
  Its fragments join `KAKERA_DLISTS`.
- **`En_Light`'s glow light** is a `LIGHT_POINT_GLOW` in the light list; its halo isn't drawn
  (BACKLOG #31). The flame is two bakes (the fire with its colours and scroll, the candle).
- **The routes.** `Route::Castle` (with the Cucco: the vines, Talon, two crates, the drain) and
  `Route::CastleCaught` (with the egg: a sentry's sight, the throw-out), both goldens. New tasks:
  `ClimbTo` (a full tilt: a hang climbs up only at magnitude 55), `ShowItem`, `WaitGone`,
  `PushCrate` (from a spot beside the crate, so Link doesn't climb it), `Caught`; the owl task
  knows the castle owl's first text (0x2068). The route goes round the sentries' sight (found with
  a sight map of the scene's floors and walls, kept out of the repo).

## Consequences

- The castle's story step plays from the Weird Egg to the drain. Its hatch was already ported
  (`Play_Init`'s new day and `Environment_UpdateTimeBasedSequence`'s dawn): the survey's "left" was
  wrong.
- Every route that swims now has the water's camera: 10 goldens changed for it (Kokiri Forest's
  stream, Hyrule Field's river, the Deku Tree's basement, the field's night run 36 frames longer, so it now reaches Kakariko just after 06:30, by day),
  each proved by taking it out (golden/README.md).
- `En_Light`'s `Rand` call at init moves the rocks' yaws on the castle's road (`En_Ishi` takes
  them from the next call): faithful, and the market's end shot shows it.
- The courtyard the drain leads to is milestone 4's: its guards (`En_Heishi1`) are placeholders and
  its side-scrolling camera (`Camera_Special6`) isn't ported.
- **Not modelled:** the eye under water (#43); the ranch's Talon by day (#44); the bottles, spells
  and ocarina from a C button (#45; the ocarina is milestone 3); the glows (#31).
