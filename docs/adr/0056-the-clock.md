# 0056: The clock: time on the save, the sun and the moon, the skybox filters, the lens flare and a depth probe

- **Status:** accepted, built in GAME-06 milestone 3 (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0006](0006-rendering-model.md) (baked meshes, the draw lists),
  [ADR 0012](0012-actor-bakes.md) (bakes), [ADR 0054](0054-the-128-skies-and-demo-kankyo.md)
  (the 128 skies by `skyboxTime`).

## Context

Time didn't pass in the port. The lights and the normal sky read a copy of the time kept in
`EnvCtx` (`day_time`, `skybox_time`, the save's `dayTime` last seen), which the room's time
settings set at `Play_Init` and a cutscene's changes reached. The C keeps it differently:

- **The time is the save's.** `Scene_CommandTimeSettings` writes `gSaveContext.save.dayTime`
  and `skyboxTime` from the room's header (0xFF keeps them), sets `sceneTimeSpeed` and with it
  `gTimeSpeed`, puts the sun, and with time stopped outside a cutscene snaps the sky's time out
  of the dawn's and the dusk's blends. It runs on every room's load, after `Environment_Init`,
  which zeroes `gTimeSpeed` and puts the sun by the time so far.
- **`Environment_Update` runs the clock:**
  - the time advances by `gTimeSpeed` (twice that by night, unless it's the Sun's Song's 400)
    with no message open, no game over, no sky change and no transition;
  - the sky's time follows it in a cutscene layer from 5, or while time passes, or after
    midnight until 1:00 (PAL's rule);
  - `nightFlag` is set past 18:00 and before 6:30;
  - a new day's sound effect is counted down from `nextDayTime`.
- **Around the clock:** `Play_Init` counts a new day (`totalDays`, `bgsDayCount`, the eggs
  hatching); `Environment_PlayTimeBasedSequence` fades the day's music and brings the night's
  critters by the time; the Sun's Song (`Interface_Update`) speeds the time or reloads the scene
  at the next noon or midnight; the drawbridge sets the time speed in Hyrule Field's layer 5; the
  carrying owls stop it.
- **`Play_Draw` draws by the time:** the sun and the moon after the sky (`gSunDL`, `gMoonDL`);
  the skybox filters (the fog's colour over the sky when `fogNear` is under 980, always over
  `SKYBOX_UNSET_1D`); the lightning's flash; and after the actors the sun's lens flare and glare.
  The flare hides when the sun is behind something: `Environment_GraphCallback` reads the
  z-buffer at the pixel under the sun once the frame is drawn (`sSunScreenDepth`), and the next
  frame compares it with the far plane's (`GPACK_ZDZ(G_MAXFBZ, 0)`).
- **The frame starts black** (`Gfx_SetupFrame(gfxCtx, 0, 0, 0)`); the port cleared it to the
  fog's colour as a stand-in for the filters.

The user chose (2026-10-09) to read the depth back from the renderer for the lens flare.

## Decision

- **The time lives on the save** (`SaveContext::day_time`, `skybox_time`, `next_day_time`,
  `suns_song_state`, `dog_is_lost`), `gTimeSpeed` on `EnvStatics::time_speed`. `EnvCtx`'s copy
  is gone; the lights and the sky read the save. `oot_game::clock` holds the C's clock functions:
  - `Scene_CommandTimeSettings`, run from the room's header (`execute_room_commands`, with the
    skybox disables);
  - `Environment_Update`'s clock (`environment_update_clock`, between the time-based music and
    the lights);
  - `Play_Init`'s new day (`play_init_next_day_time`);
  - the Sun's Song's part of `Interface_Update` (`interface_update_suns_song`).
  `Play_Init` takes `nextDayTime` into the time, and `Environment_Init` resets the song, the
  night and `gTimeSpeed`. `Skybox_Init` picks its first textures by the sky's time before the
  room's header, as the C does: the right ones load a frame later.
- **Everything that reads or writes the time does it on the save:**
  - the cutscenes (`CutsceneCmd_SetTime`, the Zelda's courtyard destination, `CS_MISC` 33 and 34);
  - the time-based music (with its precipitation checks, the new day's count and the eggs);
  - the scene draw configs, fed the time and `nightFlag` every frame;
  - the file select's load, `Map_InitRoomData` and `Map_Update`'s Sun's Song resets;
  - the drawbridge's layer 5 and the carrying owls.
- **The draws** (`oot_game::env_draw`) are bakes:
  - the sun is `gSunDL` after `SETUPDL_54` and the moon `gMoonDL` after `SETUPDL_51`. Their own
    `gSPMatrix(D_01000000)` puts them on the billboard, so the whole matrix is their bone 0;
  - the lens flare is `gLensFlareCircleDL` and `gLensFlareRingDL` after `func_800947AC`'s setup;
  - a screen fill is `SETUPDL_57` with the prim colour over a quad, in the screen's space (the
    `screen` draws), wide enough for a wider target. It draws `gDPFillRectangle` where the C's
    list has it: the skybox filters and the lightning's flash under the rooms, the glare over
    the actors.
  Their state moves once per game frame at `Play_Draw`'s time; the draws use the render's eye.
  The app draws a play state's frame on black (the spikes' views keep the fog's colour).
- **The depth probe** (`eng_gfx::DrawLists::depth_probe`, `eng_render::probe`):
  - a frame names a pixel of the 320x240 screen;
  - the renderer keeps the frame's depth, loads that pixel's (sample 0) into a one-pixel target,
    and copies it out;
  - `Renderer::read_depth_probe` maps it after the submit, and the app hands it to
    `PlayState::environment_graph_callback`: the far plane (1.0) is `G_MAXFBZ`.

  The engine knows nothing of the sun: it reads a pixel's depth.
- **The headless sandbox** draws a probe frame (320x240, for its depth alone) after each game
  frame whose lens flare read the depth, when it takes screenshots. The flare only changes
  draws, so a run's trace is the same without it.

## Consequences

- Time passes where the rooms say: 10 a frame by day and 20 by night in Hyrule Field, none in
  the forest. The field goes from dusk to night: the sky, the lights, the music's stop after
  17:10, the dog at 18:00, the drawbridge raised, the night's critters after 19:00
  (`Route::Dusk`).
- A cutscene layer's room time is now the save's time. The forest after the emerald's chain is
  at 12:00 (its layer 6's time), not the save's 10:00. The sun's light comes from overhead, so
  the `emerald` and `creation` end shots changed.
- The skybox filters cover the sky with fog where `fogNear` is low: Death Mountain's layer 4 in
  the creation shows its fog's grey over the sky.
- **Deviations from the console, accepted:**
  - The renderer reads the depth of the frame it just drew. The console reads the frame before
    (its RDP runs a frame behind the CPU), at the pixel the newer frame asked for.
  - A pixel off the target reads as drawn over (the C reads past its z-buffer there).
  - The interactive app reads the depth on every frame it renders, which can be more or fewer
    than the game's frames.
  - Tests that don't render keep `sSunScreenDepth` at its initial 0: no flare.
- `Lights_GlowCheck` (the point lights' glows), which reads the same z-buffer in the same
  callback, isn't ported; the probe is where it would go.
- **The sun's texture:** `gSunDL` loads its strips with I8 tiles. The decomp's XML calls those
  strips I4 textures (`gSun1Tex` 64x31 and on). The port draws what the list says; its look is a
  hand-test item.
