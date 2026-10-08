# 0047: The pause menu: its frame and the item page

- **Status:** accepted, built in GAME-05 milestone 5b-1 (2026-10-07)
- **Date:** 2026-10-07
- **Builds on:** [ADR 0006](0006-rendering-model.md) (baked meshes, no runtime display lists),
  [ADR 0017](0017-interface-sprites.md) (the interface's sprites), [ADR 0021](0021-mido-the-shop-and-the-pause-stand-in.md)
  and [ADR 0019](0019-inventory-and-saves.md) (Start's stand-in), [ADR 0032](0032-damage-death-and-the-game-over-stand-in.md)
  (the game over's states), [ADR 0045](0045-the-fairy-slingshot-first-person-and-deku-nuts.md) (the
  near plane: F3DZEX2 NoN).

## Context

The pause menu is `ovl_kaleido_scope` (about 9,000 lines with its tables) with
`z_kaleido_setup.c` and `z_kaleido_scope_call.c`. The user split it (2026-10-07): 5b-1 the frame
(`KaleidoSetup`, the opening and closing, the four pages' box and turns, the cursor, the name and
info panels), the item page whole and the pages' backgrounds; 5b-2 the dungeon map page and the
game over screens drawn; the equipment and quest pages' contents, the world map's and the save
prompt (5c) logged. Several things needed a decision:

- **Drawing.** `KaleidoScope_Draw` builds vertex arrays every frame (`KaleidoScope_SetVertices`)
  and draws textured quads from them under per-page matrices and its own view; the engine has no
  runtime display lists.
- **The projection.** The pages stand 93.55 from the origin, the eye 64: the page behind the eye
  and the side pages cross it. The HUD's way of drawing a perspective (a projective transform
  divided on the CPU, the A button) doesn't clip.
- **The cursor's vertices.** `KaleidoScope_SetVertices` rebuilds `cursorVtx` in the draw and the
  draw sets only its first corner; `KaleidoScope_UpdateCursorVtx` spreads the corners in the next
  frame's update, on the same memory.
- **The rate.** The menu runs at `R_UPDATE_RATE` 2 (30 frames a second); the port's loop was a
  fixed 20 Hz.
- **The scene behind.** `Play_Draw` saves the frame (`PreRender`), runs a CPU anti-alias filter
  over it, and from then on restores it instead of drawing the scene.
- **The overlay's statics**, the textures greyed in RAM, the equipment page's A equipping that
  Start's stand-in did.

## Decision

- **A recorder for the C's draw** (`oot_game::kaleido::gfx`): the draw functions are ported as
  written over `KaleidoGfx`, whose state is what the C's commands set (the vertex loads, the
  combiner, prim and env, the matrix, the view); each `gSP1Quadrangle` is a `KQuad`. A quad draws
  a sprite bake (its texture loaded as the C loads it, under `SETUPDL_42` and the C's combiner,
  a unit quad) placed by its four vertices (`quad_transform`), with their colours
  (`DrawParams::vertex_colors`) and the draw's prim and env. Every quad the menu draws covers its
  whole texture (tested over a session). About 370 bakes: the 60 page tiles, the icons up to the
  bows with magic arrows and their greyed copies, the 123 item names, the cursor, the panel, L
  and R, the labels.
- **The pause list** (`eng_gfx::DrawLists::pause`, `pause_view`): the menu's draws carry the view
  and the model in their transform; the renderer draws them after the 3D lists, their fills and
  the letterbox, before the overlay's fill and `overlay_2d`, under `View_Init`'s perspective
  (fovy 60, near 10, far 12800) with the target's aspect (the 3D view's way of widening). The GPU
  clips what's behind the eye. Depth is off (`SETUPDL_42`), back faces culled.
- **The cursor's race kept as its result:** `Graph_TaskSet00` (`graph.c`) hands a frame's task to
  the RSP and returns, and the next frame's update spreads the cursor before the RSP reads it. The
  cursor's loads are references (`VtxSlot::Cursor`) resolved when the frame is drawn, and the port
  calls `KaleidoScope_UpdateCursorVtx` at the end of the menu's draw under the update's own
  condition, instead of at the start of the next update.
- **`R_UPDATE_RATE` is play state** (`PlayState::r_update_rate`): `KaleidoSetup_Update` sets 2,
  `PAUSE_STATE_RESUME_GAMEPLAY` and the game over's end 3. The app's loop runs a frame every
  `R_UPDATE_RATE` sixtieths (`frame_seconds`), `Letterbox_Update` and the flash's fade take it,
  and the offline audio runs that many retraces after the frame (`GameAudio::update_rate`).
- **The scene behind without a copy:** from `PROCESS` on, Play's draw-time state stops (the actors'
  `draw_update`, the effects' draws, the draw config, the lightning and rain, `Actor_DrawAll`'s
  sounds, the reticle), so the scene's lists stay as they were saved and are redrawn; the fills
  are the saved frame's (`PauseContext::bg_fills`); the setup's frame skips the overlay elements.
  `PreRender_ApplyFilters` is left out: it redoes the console's edge anti-aliasing from coverage
  values the port doesn't have (MSAA smooths the same edges); the divot filter is debug-only.
- **The overlay's statics** (`KaleidoStatics`) start from their initial values whenever
  `KaleidoScopeCall_Update` loads the overlay (`KaleidoManager_LoadOvl`); the REGs (`PauseRegs`,
  `Regs_InitDataImpl`'s PAL values) live on the play state.
- **The greyed icons** are made by the importer: `BakeSegment::GrayRgba32` runs the ported
  `KaleidoScope_GrayOutTextureRGBA32` (`oot_game::kaleido::gray_out_texture_rgba32`) on the
  icon's bytes; the draw picks the greyed bake where `!CHECK_AGE_REQ_ITEM` (the menu greys at
  `PAUSE_STATE_INIT`, with the age that can't change while paused).
- **Logged** (the user's choice): the equipment page's, quest page's, dungeon map page's (5b-2)
  and world map's contents, `INIT`'s world map points and trade marker, the equipment page's Link
  (`KaleidoScope_DrawPlayerWork` and its prerender), the game over's message and prompt page
  (5b-2); the quest page's song states are ported with their `AudioOcarina_*` calls logged (the
  ocarina isn't ported); L (the debug inventory editor, `z_kaleido_debug.c`) logs and the menu
  stays; B's save prompt (5c) logs and the menu stays.
- **Start's stand-in keeps its equipment half only**, run as the menu resumes the game
  (`PAUSE_STATE_RESUME_GAMEPLAY`, before `Player_SetEquipmentData`): what's owned and unworn goes
  on (`SaveContext::equip_owned_unworn`). Its item half is gone: the C buttons are equipped from
  the item page. The presets (`deku-tree-sticks`, `deku-tree-slingshot`) still put items on C
  buttons, through the equip's end (`item_equip_write`); `deku-tree-slingshot-owned` leaves the
  slingshot for the menu.
- **The HUD's pause part** (`z_parameter.c`): the START button (grey on GameCube) and its label
  (`doActionSegment`'s third: "Return"'s texture), the B button's label
  (`Interface_LoadActionLabelB`: "SAVE"), `Interface_SetDoAction`'s paused branch, the icon flying
  to its C button (`Interface_Draw`, from the menu's state: `HudPause`), `func_80084BF4`'s
  opening branch.
- **Faithful bugs kept** (`@bug (game)`): `KaleidoScope_SetDefaultCursor` reading one past
  `items[24]` (the save's `ammo[0]`); the outlines' combiner sent to `OVERLAY_DISP` (they draw
  under `G_CC_MODULATEIA_PRIM`); `KaleidoScope_DrawItemSelect`'s uninitialised `cursorSlot` (the
  stack's last value: the page's slot); `ZREG(48)`'s collision with `R_START_LABEL_DD`; C-Down's
  and C-Right's equips not copying the slot to a bow with arrows; the trade marker's write of
  `vtx[bufI]`.
- **`gSaveContext.worldMapArea`** (the scene's misc settings' area) isn't in the pack: it's
  `Play_Init`'s `WORLD_MAP_AREA_HYRULE_FIELD`. Only the world map's trade marker reads it.
- **Pack format 24** (`out/data21`): the menu's bakes and the HUD's do-action rectangles.

## Consequences

- The menu opens, turns and closes with the C's timings: the letterbox's wait, the prerender's
  two frames, `INIT`, 8 frames of `OPENING_1` and 8 of `OPENING_2`; a page turn in 16 frames; the
  closing in 8, then the resume. The item page's cursor, its stick repeat (first push, then after
  10 frames and every 3), the equip's flight (10 frames, `sCButtonPosX/Y`) and the C buttons' swap
  are the C's (`oot_actors --test pause`, `oot_game` `kaleido::tests`).
- **The runs that wore their equipment with Start** (`mido_shop`, its audio, `new_save_deku_tree`,
  `new_file_deku_tree`) open and close the menu: 32 frames paused each time, the runs 66 frames
  longer in all, with the menu's sounds.
- **A wide target shows the side pages** at the edges, which the console's 4:3 never shows (as the
  3D view shows more of the scene).
- **The frame the background is saved on is shown** (the console skips it,
  `R_GRAPH_TASKSET00_FLAGS`): one frame with the HUD.
- **Player's overlay is reloaded when the menu closes** (`KaleidoScopeCall_LoadPlayer`), which
  resets `z_player.c`'s statics; the port keeps them. The ones that last beyond a frame
  (`sPrevFloorProperty`, `sSavedCurrentMask`) matter little; noted.
- 5b-2 adds the dungeon map page (its CI4 maps need a palette per draw: an engine feature) and
  the game over's draw.
