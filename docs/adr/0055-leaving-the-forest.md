# 0055: Leaving the forest: Saria, the soft soil, the owl, and a camera's finished flag

- **Status:** accepted, built in GAME-06 milestone 2 (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0012](0012-actor-bakes.md) (bakes), [ADR 0022](0022-cutscenes.md) (the
  cutscene layers and the actors' cues), [ADR 0029](0029-one-point-cutscenes.md) (one-point
  cutscenes), [ADR 0054](0054-the-128-skies-and-demo-kankyo.md) (the chain that leaves Link at
  `ENTR_KOKIRI_FOREST_11`).

## Context

From `ENTR_KOKIRI_FOREST_11` the story leaves the forest. Mido blocks the path out of the
meadow until Link talks to him (`EnMd_BlockPath` with the emerald); the forest's exit to the
Lost Woods' bridge plays `gLostWoodsFairyOcarinaCs` (Saria's goodbye, the Fairy Ocarina, given by
`Cutscene_HandleConditionalTriggers`, which the port has); its terminator enters Hyrule Field,
whose intro is an entrance cutscene (ported); and the owl waits a few steps from there.

What was missing:
- **Saria in cutscenes** (`Demo_Sa`, 1,066 lines): five uses, of which the bridge is the only one
  this phase reaches. Her face is two eye textures and a mouth on segments 8 to 0xA, her hand
  holds the ocarina by a limb override, and she fades in translucent.
- **The soft soil** (`Obj_Bean`, 962 lines): a child with no bean sees its patch and can offer
  one; the rest is planting, watering and the adult's beanstalk lift.
- **The owl** (`En_Owl`, 1,437 lines): two skeletons (flying, perching) he switches between, a
  head that turns, bobs and tilts by its own state machine, talks taken at once within a range,
  one-point cutscene 8700 on them, and a flight away.
- **Link's ocarina in a cutscene** (`func_80851D2C`, `func_808526EC`): the C calls
  `Player_SetModels` with the ocarina's model group without changing `modelGroup`.
- **A camera told it's finished:** the port's `Camera_SetFinishedFlag` flagged only the camera it
  was given. The C also flags the active camera when it is called on the main camera with
  another active. 8700's action 16 holds until that flag, so the owl's talk never ended.

The user chose (2026-10-09) the parts of `Demo_Sa` and `Obj_Bean` this phase reaches, the rest
logged, and `En_Owl` whole.

## Decision

- **`Demo_Sa`: the bridge's actions and the helpers they share** (`oot_actors::demo_sa`). The
  other four uses are logged and draw nothing. Her draws are bakes, one per face, pass and hand the
  bridge shows (sad and opaque, sad and translucent, eyes shut with the ocarina):
  - the skeleton with the eyes and mouth bound to their segments;
  - `gActorSetupXluDL` or the empty list on 0xC, and the env alpha dynamic;
  - limb 15 replaced by `gSariaRightHandAndOcarinaDL` for the ocarina.
  A combination the bridge doesn't show is logged, not drawn.
- **`Obj_Bean`: the child's soft soil whole.** That is `ObjBean_SetupWaitForBean` and
  `_WaitForBean`: the patch drawn and the talk offered for the magic bean, with text 0x2F. Planting
  (`func_80B8FE00` on), watering and the adult's lift are logged; their actor stands and draws
  nothing.
- **`En_Owl` whole** (`oot_actors::en_owl`):
  - every type's wait, talk, question and end;
  - the flights away and the carrying owls' flights;
  - the cue-driven cutscene owls;
  - the head's state machine.
  His draw is one bake per skeleton and eye, after `SETUPDL_37`. Each limb is turned by
  `EnOwl_OverrideLimbDraw` as it is posed, and his focus is set in `draw_update` from limb 3, as
  his post-limb function does. One-point cutscene 8700 is ported, with its `D_80122E44[timer & 1]`
  half picked by the talk's timer. The perched animation on the flying skeleton (cue 2) reads its
  missing joints as zero: `@bug (game)`.
- **`Camera_SetFinishedFlag` whole** (`PlayState::camera_set_finished_flag`). Player's request
  (`PlayRequest::CamDone`) uses it.
- **`Player_SetModels`' group is kept apart** (`Player::models_group`): the hands the draw shows
  when a cutscene sets another group than `modelGroup`; the next `Player_SetModelGroup` puts
  `modelGroup`'s back. `func_80851D2C` and `func_808526EC` are ported with it. The ocarina's
  sparkles (`EffectSsKiraKira_SpawnDispersed`) are not ported; their `Rand` calls are made.
- **The rest of the way:**
  - `En_Ko` child 3 stands at his path's last point with the emerald (`Path_CopyLastPoint`).
  - `Play_Init`'s layer rules are whole: Hyrule Field's child layer 1 needs the three stones,
    and Kokiri Forest's adult layer 3 needs `EVENTCHKINF_48`.
  - `Scene_DrawConfigLostWoods` is ported, with Cojiro's cry through the draw config's state.

## Consequences

- The exit holds: from `ENTR_KOKIRI_FOREST_11` past Mido, Saria's goodbye and the ocarina, the
  field's intro, and the owl's talk and flight (`Route::Farewell`).
- Every talk that ends while a one-point cutscene holds on action 16 now ends it, as in the C.
  No earlier golden had one. `creation`'s end shot changed: `En_Ko` child 3 now stands at his
  path's end, down the tunnel's view.
- Saria's other cutscenes (the Chamber of Sages, the sages' magic, the credits) and the soft
  soil's beans wait for their phases (BACKLOG #28, #29).
