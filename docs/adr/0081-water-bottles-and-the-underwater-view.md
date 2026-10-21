# 0081: Water, bottles and the underwater view

- **Status:** accepted, built in GAME-10 milestone 1 (2026-10-20)
- **Date:** 2026-10-20
- **Builds on:** [ADR 0033](0033-effects.md) (the soft sprites), [ADR 0064](0064-kakariko-lifting-and-the-village.md)
  (an actor offered to Player), [ADR 0075](0075-king-dodongo-the-room-texture-the-cameras-up-and-the-ruby.md)
  (the camera's view), [ADR 0066](0066-the-actor-schema.md) (the schemas).

## Context

Phase 11's first milestone (`docs/GAME-10-zoras-waters-and-jabu-jabu.md`): the systems the phase's
story leans on before any of its places. The dive went no deeper than 120 whatever the scale, and
nothing picked up from the bottom surfaced with it (Ruto's letter); a bottle on a button did
nothing but show itself (no swing, no catch, no drink, nothing let out: Lord Jabu-Jabu's fish); the
fish and the bugs were placeholders; water's currents, its ripples, splashes and bubbles, and the
eye under water (its lights, its sway, its muffled sound) were logged. Queen Gohma's heart
container was missing from the story's presets after her (GAME-09's exit review).

## Decision

1. **Player's water whole for a child:** the dive by the scale (`D_80854784`), the bottom's item
   picked up (`Player_ActionHandler_2` in the dive) and held up surfacing (`Player_Action_8084E1EC`'s
   `PLAYER_STATE1_10`), swimming in from an entrance in deep water (`Player_Action_8084D7C4`), the
   cutscene holds in water (`func_80851368`, `func_808513BC`), `Player_IsTalking`'s check while
   treading, and `func_8083D6EC` whole (the sand's sinking, the ripples wading, the splash running,
   the bubbles). The iron boots' parts stay an adult's.
2. **The currents are the C's statics** (`sConveyorSpeed`, `sConveyorYaw`, `sIsFloorConveyor` in
   `PlayerStatics`), read in `Player_ProcessSceneCollision` and eased into `pushedSpeed`
   (`sWaterConveyorSpeeds`, `sFloorConveyorSpeeds`); the push applies only as the C applies it (not
   in a cutscene mode, on a ledge or ladder, or climbing up).
3. **Zora's Domain's finite water box** is a general field of the engine's collision context
   (`special_water_box`, a box checked before the scene's), set by the game for that scene only
   (`BgCheck_GetWaterSurfaceAllHack`); everything that asks for water asks through it.
4. **The bottles' actions whole** (`Player_Action_SwingBottle`, `8084EAC0`, `8084EED8`, `8084EFC0`,
   `Player_IsDroppingFish` on `PlayerIface`), and the bottle drawn in hand (`sBottleColors`, the
   shield's `@bug (game)` teal). `Player_UpdateBottleHeld` runs its Player half at once and its
   inventory half as a request (`PlayRequest::UpdateBottleItem`), the C's order within the frame.
5. **`En_Fish` and `En_Insect` whole**, and the five effects whole (`G_Ripple`, `G_Splash`,
   `Bubble`, `Dt_Bubble`, `Lightning`) with every ported caller wired. The ripples an actor makes
   entering water (`Actor_UpdateBgCheckInfo`) are recorded by the check and spawned right after
   that actor's update: the check has no play state and about 140 callers; only the slot order
   within that one frame differs, with no `Rand` call.
6. **The underwater view** (BACKLOG #43): `Camera_UpdateWater`'s camera half (the water box's
   lights, `Audio_SetExtraFilter`, the `QUAKE_TYPE_5` sway), `Camera_UpdateHotRoom`,
   `Camera_UpdateDistortion`, and `z_view.c`'s distortion (`crate::view`): stepped once per
   `Play_Draw`, it multiplies the projection, both `play->viewProjectionMtxF` (culling,
   `projectedPos`) and the renderer's (`DrawLists::distortion`, after the perspective). The camera's
   writes go through `CamRequest`s.
7. **The story's presets after the Deku Tree have Queen Gohma's heart container** and her room's
   flags (`deku_tree_dead`, which `deku-tree-dead`, the castle's chain and `ocarina-songs` build
   on): four hearts there, five from the Goron's Ruby on.
8. **The coverage table's corrections** (the survey's): Hyrule Field's, the Temple of Time's, the
   Zora places' layers, `Bg_Spot08_Iceblock` in Phase 16, `En_Fish` held in Phase 11.
9. **The run** is a short debug start (`Route::Bottle`, `zora-bottle`): the milk drunk twice, a
   fish caught in the emptied bottle, let out. Night field's second scripted Stalchildren fight is
   dropped from `Route::FieldNight`: the stream's current moved the run, and a scripted fight is a
   matter of luck (fights are the hand test's).

## Consequences

- Ruto's letter (milestone 3) can be dived for with the silver scale; Lord Jabu-Jabu's fish can be
  caught and let out; Jabu-Jabu's breathing (milestone 4) has its view distortion to drive.
- Every golden whose HUD shows the hearts from the castle's runs on changed (the container); the
  runs crossing water changed (their ripples' `Rand` calls, a current).
- The fishing rod's cast (Phase 20), the iron boots, blue fire's catch (`En_Ice_Hono`, Phase 16)
  and the spells' bottles remain.
