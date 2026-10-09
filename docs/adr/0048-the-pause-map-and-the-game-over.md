# 0048: The pause menu's dungeon map page and the game over's screens

- **Status:** accepted, built in GAME-05 milestone 5b-2 (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0047](0047-the-pause-menu.md) (the recorder of the menu's quads, the pause
  list), [ADR 0006](0006-rendering-model.md) (baked meshes, no runtime display lists),
  [ADR 0017](0017-interface-sprites.md) (sprite bakes), [ADR 0032](0032-damage-death-and-the-game-over-stand-in.md)
  (the game over's states), [ADR 0040](0040-switches-torches-webs-and-the-map-data.md) (`table/map`, `Map_*`).

## Context

5b-2 (agreed 2026-10-07) ports `KaleidoScope_DrawDungeonMap` whole (`z_kaleido_map.c`) with
`KaleidoScope_LoadDungeonMap`, `_UpdateDungeonMap` and `_OverridePalIndexCI4`, `z_lmap_mark.c` whole
with `gPauseMapMarkDataTable`, and the game over drawn: `KaleidoScope_DrawGameOver` and
`KaleidoScope_DrawPages`' prompt page. Several things needed a decision:

- **The room maps.** Each floor's two 48x85 maps (`map_48x85_static`) are CI4 textures the menu
  copies into `interfaceCtx->mapSegment` and edits in RAM: on Link's floor
  `KaleidoScope_OverridePalIndexCI4` moves the current room's texels to palette index 14. Their
  palette (`interfaceCtx->mapPalette`, 16 RGBA16 colours) is built at run time
  (`Map_SetFloorPalettesData`: the visited rooms, the map's colour) and entry 14 changes every
  draw (`mapBgPulse*`). The renderer had only decoded RGBA textures baked into meshes.
- **The marks' table** is `ovl_kaleido_scope`'s data, not in the pack.
- **The vertices the RSP reads late.** `KaleidoScope_DrawDungeonMap` moves Link's head's and the
  skull's quads (`mapPageVtx[116..123]`) after `gSPVertex` has loaded them.
- **The marks' combiner.** `PauseMapMark_Draw` sets none.
- **"GAME OVER"** is three `gSPTextureRectangle`s in screen space, in 2-cycle mode, each texture
  mixed with a mask on tile 1 whose tile size scrolls every frame, between prim and env colours.
- **The game over's states** (ADR 0032) left out the fields only the draw reads.

## Decision

- **The room maps are drawn from the game's own texels** (the user's choice of three: a per-draw
  image; a GPU palette; per-index mask bakes):
  - the pack holds `map_48x85_static` as the ROM has it (`MapTables::map_48x85_static`, in
    `table/map`);
  - `interfaceCtx->mapSegment` is bytes (`MapState::segment`, `MapSegment::PauseMap`):
    `KaleidoScope_LoadDungeonMap` copies the two maps in as the DMA does, and
    `KaleidoScope_OverridePalIndexCI4` runs on them as written;
  - the recorder keeps the palette `gDPLoadTLUT_pal16` loads, and the CI4 quad's texels are
    decoded through it when the quad is recorded (`KaleidoGfx::quad_ci4`), by the importer's own
    decoder (`eng_gbi::texture::decode_linear`: `oot_game` now depends on `eng_gbi`);
  - **the engine** takes them per draw: `eng_gfx::DrawParams::image` (`DrawImage`, RGBA8) replaces
    texture slot 0's image for that draw; the renderer gives the mesh's instance its own texture
    and bind groups (the mesh's samplers and slot 1 kept), made again only when the size changes,
    the texels written each draw that brings new ones;
  - the bake (`kaleido/room_map/iap`) is a 48x85 quad under the menu's setup with
    `G_TF_POINT`, as the C sets around the draw; its baked texture is the file's first map as
    the XML extracts it (i4), which the draw's image replaces.
- **`gPauseMapMarkDataTable` is imported** from the C (`z_lmap_mark_data_mq.c`, the file `spec`
  builds into `ovl_kaleido_scope` for this version), each entry with the `Vtx` array it points at
  (`PauseMapMarkData`, `MapTables::pause_marks`); the importer's test finds it in the overlay
  and checks its pointers. **Pack format 25** (`out/data22`).
- **The late-read vertices:** the head's and skull's moves are made before their `gSPVertex` in
  the port; the RSP reads them once the frame's list is done, so it sees the moved positions
  either way. The CI4 texels and the palette are read when the quad is recorded, where the RDP
  reads them a frame later; nothing writes them in between.
- **The marks draw under whatever combiner is set:** on the page looked at,
  `KaleidoScope_DrawCursor`'s LERP (left set); otherwise `Gfx_SetupDL_42Opa`'s
  `G_CC_MODULATEIA_PRIM`. Both are baked (with prim white and env black they look the same). The
  RSP keeps the last mark's matrix after `Matrix_Pop`, as in the C.
- **"GAME OVER" is `KRect`:** three rectangles recorded after the quads (`KaleidoScope_DrawGameOver`
  is the last thing `KaleidoScope_Draw` draws), drawn at the end of the pause list in the
  interface's projection (`DrawParams::screen`, which the pause list now honours), so the game
  over's black (`interfaceCtx->unk_244`, the overlay's) still covers them. The sprite bakes gain
  a second texture (`sprite::Tile1`: `gDPLoadMultiBlock` at TMEM 0x100 onto tile 1, its tile size
  from a dynamic segment, `SEG_TILE`, the prim colour's LOD fraction baked). The mask's texture
  is on segment 0x0F, since 0x0C is the importer's culling list.
- **The game over's states** gain the fields the draw reads: `INIT`'s `VREG(88)` 98;
  `SHOW_WINDOW`'s four page pitches, `infoPanelOffsetY`, `startAlpha`, `VREG(88)`, the L and R
  buttons, `XREG(5)`, the alpha, and its end values. `VREG(87..89)` and `GREG(92..93)` are
  `PauseRegs` (`Regs_InitDataImpl`'s 64, 66, 0; 0, 0); `gBossMarkState` and `gBossMarkScale` live
  on the `PauseContext` (they're `code`'s, not the overlay's); `mapBgPulse*` are overlay statics.
- **Still logged:** the world map's contents, the equipment and quest pages' contents, and the
  save prompt's page (5c). "Game saved." (`sSaveConfirmationTexs`) isn't drawn on GameCube.
- **Faithful bugs kept** (`@bug (game)`): the floors' column's down loop tries floors 8 to 10,
  past the dungeon's row (no floor bit is set there, and `floorID` reads the next dungeon's
  first floors, which none has); the prompt page's `gSPVertex` loads 32 vertices where there are
  20.
- **The debug starts:** `deku-tree-compass` (the compass, 3F to 1F and rooms 0 to 2 visited) and
  `deku-tree-quarter-heart` (health 4, at `DekuBaba`'s start); `Route::DungeonMap` and
  `Route::GameOver`.

## Consequences

- The map page in a dungeon is the C's: the title, the items owned, the visited floors' buttons
  (all with the map), Link's head, the skull with the compass, the Gold Skulltula icon, the
  floor's room maps with the current room pulsing on Link's floor, the compass's chest marks
  (hidden once opened), and its cursor over the floors and items (`oot_actors --test
  dungeon_map`).
- The game over draws "GAME OVER" fading in from orange to dark red, the window turning in, and
  the prompts with their cursor (`--test game_over_screens`).
- **The engine can take texels the game makes** (`DrawParams::image`): a texture the C writes in
  RAM can be drawn as it is, a draw's own copy uploaded when it changes. The meshes stay baked.
- **Two goldens changed** (`pause_item`, `pause_map`): the map page's contents are drawn where it
  was empty. New: `dungeon_map` with `dungeon_map_1f`, `dungeon_map_2f`; `game_over` with
  `game_over_message`, `game_over_save`, `game_over_continue`.
- The decoded room maps cost two 48x85 decodes a frame while the menu is up, and an upload when
  the pulse changes the palette.
