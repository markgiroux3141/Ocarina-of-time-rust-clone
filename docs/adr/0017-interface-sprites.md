# 0017: The message box and the HUD as baked sprites

- **Status:** accepted, built in GAME-02 milestone 3; extends ADR 0012 (actor bakes) and ADR 0013 (the orthographic overlay list)
- **Date:** 2026-09-28

## Context

`Message_Draw` (`z_message_PAL.c`) and `Interface_Draw` (`z_parameter.c`) build their display
lists every frame. They mostly use two kinds of draw:

- **`gSPTextureRectangle`** with a texture just loaded:
  - the textbox (`message_static`);
  - each glyph (`nes_font_static`, one 16x16 I4 texture per character, loaded as it's drawn);
  - the end-of-text icons;
  - the hearts, the rupee icon and its digits (`parameter_static`);
  - the button backgrounds and the B item's icon (`icon_item_static`).
- **Quads of vertices under a matrix:**
  - the beating heart (`beatingHeartVtx`, scaled about its centre);
  - the A button and its do-action label, turning about X for the flip, in their own 45x45
    viewport with a 60° perspective (`func_8008A8B8`).

Colours come from `gDPSetPrimColor`/`gDPSetEnvColor`, which change per draw (fades, the
text colour, the hearts' colours).

The engine has no texture rectangles. `eng_gbi` interprets triangles, and the runtime has
only the pack, not the ROM files these draws load from. There were two ways to do it:

1. **A runtime display list:** port the draw functions as written, with `gSPTextureRectangle`
   support in `eng_gbi`. Each frame's list would be interpreted into a new mesh, uploaded, and
   drawn. That needs the raw texture files in the pack and a mesh per frame.
2. **Baked sprites:** bake each texture with its setup once into a mesh. Each frame then only
   picks which sprites to draw, where, and in what colours.

Every texture rectangle in these draws covers its whole texture, one texel step per pixel
scaled by `dsdx`/`dtdy`:

- the textbox grows with `width * dsdx` = 256 texels throughout;
- the glyphs are 12 pixels at `1 / 0.75` texels a pixel;
- the hearts are 10.88 pixels at `1 / 0.68`.

So a rectangle is fully described by its screen corners and its texture.

## Decision

- **Each texture and its render setup is a `SpriteBake`** (`oot_game::sprite`). The bake is
  the display list the C would run for one draw, turned into a mesh by the actor-bake path
  (ADR 0012):
  - the setup (`SETUPDL_39`, `SETUPDL_42`, the combiner the function sets) in segment 0x0A;
  - the texture load in segment 0x08;
  - one quad: a unit square (0, 0)-(1, -1) with full-texture coordinates for a rectangle,
    or the C's own vertices for a vertex quad.
- **Colours are a dynamic segment (0x0B):** the bake's prim and env colour commands read a
  per-draw value, so a fade or a text colour is the draw's parameter, not another mesh.
- **A frame's draws are `Sprite`s in `DrawLists::overlay_2d`**, each with a transform in the
  overlay's 320x240 space:
  - a rectangle's is `rect_transform` (translate to its top-left corner, scale to its size);
  - the beating heart's is its `Matrix_SetTranslateScaleMtx2` matrix;
  - the A button's is the viewport's perspective placed on its part of the screen, times the
    C's matrices.
- **A projective transform is divided by w.** The engine poses vertices on the CPU; a
  transform whose bottom row isn't `0 0 0 1` now goes through `project_point3` (only the A
  button and its label have one).
- **Stateful drawing runs once per game frame.** `Message_DrawText` advances the typing and
  plays with `textDrawPos`, and the icons flash. It runs in `PlayState::tick_with` where
  `Play_Draw` would call it (`MessageContext::draw_update`) and keeps the frame's sprites;
  `draw` only submits them. That's the same split as the Z-target reticle.
  `Interface_Draw` reads state only, so it builds its sprites in `draw`.
- **The pack holds 203 sprite bakes:**
  - the message box's 147: 4 box types, 140 glyphs, 3 icons;
  - the HUD's 56: 5 hearts, drawn both as rectangles and as the beating heart; the rupee icon;
    10 digits; the button background and 3 empty-C arrows; the A button; the two swords' B
    icons; 28 do-action labels.

## Consequences

- **No new engine feature but the projective division:** no texture rectangles, and no
  per-frame meshes or uploads. A frame's HUD and text are a few hundred draw commands of
  cached meshes.
- **A texel may land half a texel off the console's.** The RDP samples a texture rectangle at
  the pixel's top-left corner; the engine samples at the pixel centre through the triangle's
  interpolated coordinates. The console's small text can differ by a pixel's blend at the
  glyph edges.
- **The flipping A button's texture is interpolated affinely.** Its vertices are divided by w
  on the CPU, so each of its two triangles maps the texture without perspective. Mid-flip the
  label can bend slightly along the diagonal; flat on, it's exact.
- **Only what's baked can be drawn.** A new item icon, message background or text icon means
  another bake. Not baked yet:
  - the item icons shown in text (`MESSAGE_ITEM_ICON`);
  - the message backgrounds (`MESSAGE_BACKGROUND`);
  - the C items' and the other B items' icons;
  - the German and French fonts' extra characters and labels.
- **The text's layout stays the C's.** The typing, the character widths (`sFontWidths`), the
  line spacing and the colours are ported as they are; only the final "draw this glyph here"
  becomes a sprite.
