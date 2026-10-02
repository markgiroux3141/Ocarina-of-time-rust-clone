# 0014: Prerendered rooms: backgrounds decoded at import, drawn as screen quads; room skyboxes baked

- **Status:** accepted, built in GAME-02 milestone 2
- **Date:** 2026-09-28

## Context

Link's house, the shop and the other interiors are `ROOM_SHAPE_TYPE_IMAGE` rooms: a little
geometry and a prerendered picture. The game shows them in one of two ways, by the active
camera's setting:

- **`CAM_SET_PREREND_FIXED`** (a fixed camera). `Room_DrawImage` draws the room's opaque list,
  then its background: a 320x240 JPEG, which `Room_DecodeJpeg` decodes once into RGBA16
  (`Jpeg_Decode`, with the IDCT and colour conversion in the RSP's JPEG microcode).
  `gSPBgRectCopy` then copies it to the frame in `G_CYC_COPY`, with no z. So the room's
  geometry keeps only its depth under the picture, and the actors are hidden where the
  picture's furniture should hide them. A room with several backgrounds picks the one for the
  camera's bg camera (`Room_GetImageMultiBgEntry`).
- **Any other setting.** Every house's default view is `CAM_SET_PREREND_PIVOT`: the viewpoint
  starts at `VIEWPOINT_PIVOT` in `SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT` scenes. With it
  `Play_Draw` draws the scene's *room skybox* after the rooms: a 360° picture of the room on
  two to four faces around the eye (`skyboxCtx.drawType != 0`, `Skybox_Setup`). Its display
  lists are built once by `Skybox_Init` (`Skybox_Calculate256` → `Skybox_CalculateFace256`) and drawn after
  `SETUPDL_40`, which has no z-buffer either.

So a house needs both a background and a skybox before it looks like itself. Three questions:
1. where the JPEG is decoded;
2. where the background is drawn, since it's a screen-space copy in the middle of the 3D OPA
   list;
3. what the skybox becomes, when its display lists are built by code.

## Decision

- **The importer decodes the backgrounds** (`oot_import::background`).
  - It uses a standard baseline JPEG decoder (`zune-jpeg`), then packs the result to RGBA5551,
    as `Jpeg_Decode` writes it (5 bits a channel, alpha 1). Data that isn't JPEG
    (`JPEG_MARKER`) is decoded in its own `fmt`/`siz`, with its TLUT.
  - Each background becomes a mesh in the room's record (`RoomData::backgrounds`): one quad
    over the 320x240 screen, textured in copy mode (point sampled, no depth test or write, no
    combiner).
  - The runtime never sees JPEG data, which keeps the rule that it reads only the pack.
- **The background is a screen-space draw in the OPA list.**
  - `eng_gfx::DrawParams::screen` marks a draw command to be drawn in the interface's
    orthographic projection (the `overlay_2d` one), where it stands in its list. The
    renderer switches the globals bind group for it.
  - The room code (`oot_game::room::image_background`) submits it after the room's opaque
    entries, only while the game camera is `CAM_SET_PREREND_FIXED`: the single background,
    or the multi-image entry for the camera's bg camera or its
    `roomImageOverrideBgCamIndex`.
  - On a target wider than 4:3, the quad covers the middle 4:3 of the view, which is exactly
    the game's view, since the 3D view keeps the vertical fov (ADR 0013). Outside it is the
    clear colour.
- **The room skyboxes are bakes** (ADR 0012).
  - `oot_game::skybox` ports `Skybox_Calculate256`, `Skybox_CalculateFace256` and `Skybox_Draw` as
    display-list generators. They produce the vertex buffer (`roomVtx`), the eight
    `dListBuf` lists, `SETUPDL_40` written out, and the draw's TLUT loads and calls.
  - The importer reads `Skybox_Setup` from the C: each case that sets `drawType`, its
    `SKYBOX_*` value, and its `vr_*_static` and `vr_*_pal_static` files (`RoomSkybox`, kept
    in `SceneTable`). It bakes each as `bake/skybox/<name>`.
  - Two new `BakeSegment` kinds carry the data: `File` (a whole ROM file on a segment) and
    `Bytes` (generated data).
  - The runtime draws the bake after the rooms, at the eye, when the camera isn't
    `CAM_SET_PREREND_FIXED`.

## Consequences

- The pack grows by the backgrounds and skyboxes: 47.2 MB, from 40.7 MB. The import takes
  10.0 s (9.0 before).
- **A background pixel can differ from the console's by one step of a 5-bit channel.** The
  JPEG microcode isn't in the decomp, so its IDCT rounding can't be matched. The 5-bit packing
  bounds the difference.
- **The skyboxes are exact to the display lists.** They go through the same interpreter as
  every other mesh, textures and palettes included.
- **@bug (game): `SKYBOX_HAPPY_MASK_SHOP` gets four faces** (`Skybox_Calculate256` only gives two to
  the shops after `SKYBOX_HOUSE_KAKARIKO`), but its files hold two. The last two read past
  them. The bake keeps those draws with unresolved textures, and the import notes it.
- **Not modelled:**
  - `Room_GetImageMultiBgEntry`'s write of the bg camera index into Player's params;
  - the quake offset of the background and the skybox (no quakes);
  - `R_ROOM_IMAGE_NODRAW_FLAGS`.
- A room whose multi-image list has no entry for the camera hangs the game
  (`LogUtils_HungupThread`). Here it draws no background.
- Scenes shown the spikes' way (`--scene` without `--entrance`) draw every room without its
  background or skybox, as before.
