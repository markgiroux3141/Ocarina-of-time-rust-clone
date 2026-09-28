# 0009: Baking what the runtime state changes: scene layers, Link's face, draw configs

- **Status:** accepted
- **Date:** 2026-09-27

## Context

Some of what the spikes built at load time depends on runtime state, and the pack (ADR 0008) must hold it without the runtime interpreting N64 data or C:

- **Room meshes** depend on the scene's draw config. Its bindings are made for the age, the night flag, the scene layer and the time of day. For example:
  - the Deku Tree binds a day or a night texture;
  - Hyrule Field adds a lit-window display list at night.
- **Link's mesh** depends on:
  - the model group (what's in his hands and on his back);
  - whether he runs (open hands become fists);
  - the eye and mouth textures `Player_DrawImpl` binds to segments 8 and 9 every frame.

  The spikes cached one interpreted draw list per combination, built on demand from the ROM.
- **Draw configs also run every frame:** they scroll textures and set colours, and the spikes re-ran them with the C interpreter each frame.

## Decision

- **Scene layers.** The pack stores every scene's header, collision and rooms for each of the four game layers (child/adult × day/night).
  - The meshes are baked with the layer's own state: age and night flag from the layer, `gameplayFrames` 0.
  - The time of day is 10:00 (a new save's start) for day layers and midnight for night layers. Room 0's time settings are applied, as the scene load does.
  - Each layer's `bake_day_time` records the time used.
  - Identical rooms across layers are stored once.
- **Link.** One mesh per age, model group and hand state (2 × 16 × 2 = 64), baked with eye 0 and mouth 0.
  - The interpreter records which segments each texture's TMEM words were loaded from (`TextureImage::source_segments`). Textures from segment 8 or 9 are the face slots.
  - The pack stores the textures each eye and mouth index puts in those slots, decoded by the same interpreter.
  - At runtime `LinkVariant::with_face` swaps them in.
  - The importer checks the swap gives exactly the interpreted draw list, for every face on the default model group and the last face on the others. `oot_import`'s tests repeat it for four faces on every group.
  - Tracking every word matters: one child texture reads TMEM that the eye load wrote.
- **Draw configs** are ported to Rust (`oot_game::scene_table`) and run every frame with their inputs and their state (`roomCtx.unk_74`).
  - A scene whose draw config isn't ported keeps the import-time values (frame 0): its textures don't scroll, and a note in the scene load says so.
  - Milestone 2 ports Kokiri Forest, Hyrule Field and the Deku Tree (the scenes the golden renders use), and the default config.
  - Each port is checked against the C interpreter over 12 frames in 42 states: six layers (the four game layers and two cutscene ones), seven times of day, each age and the night flag as the layer implies them.

## Consequences

- **Rendering is unchanged.** The golden renders (ADR 0001) are identical from the pack: 78 of 78.
- **The ports are more faithful than the spike's interpreter.** Hyrule Field's night overlay fades in over 51 frames (`Math_StepToS` on `roomCtx.unk_74`). The interpreter never ran the step, so it always drew it at alpha 0. The oracle test asserts exactly that difference.
- **A draw config that picks geometry or textures by the time within a layer** is baked at the layer's time.
  - Hyrule Field's does, in two half hours: its overlay list is drawn from 6:30 to 7:00 (a day-layer time, baked at 10:00 without it) and not from 18:00 to 18:30 (a night-layer time, baked at midnight with it).
  - In both windows the overlay's alpha has already stepped to 0, so the frames look the same.
  - A config whose visible result changes within a layer (say, a sunset texture) needs baking at several times, or the texture as a per-draw parameter (ADR 0006).
- **Draw configs that swap texture pointers per frame** (lava and waterfalls in other scenes) need their frames as per-draw texture parameters. That's decided when the first one is ported.
- **The tunic and shield are fixed** at the default ones: Kokiri tunic, the age's default shield. Other equipment adds variants, or a per-draw colour parameter for the tunic (env colour).
