# 0051: Textures replaced by their source, and the object RAM the game writes

- **Status:** accepted, built in GAME-05 milestone 6a (2026-10-09)
- **Date:** 2026-10-09
- **Extends:** [ADR 0006](0006-rendering-model.md) (baked meshes only) and
  [ADR 0048](0048-the-pause-map-and-the-game-over.md) (`DrawParams::image`, one per-draw image in
  texture slot 0).

## Context

As Queen Gohma dies, `BossGoma_ClearPixels` writes zeros into six textures in her object's RAM,
four steps a frame over two passes: `gGohmaBodyTex`, `gGohmaShellUndersideTex`,
`gGohmaDarkShellTex`, `gGohmaEyeTex` (16x16 RGBA16, a pixel a step) and `gGohmaShellTex`,
`gGohmaIrisTex` (32x32, a 2x2 block a step), one after another in `object_goma`. Her draw then
shows them with holes (her segment 8 list is the cutout render mode by then), and so do the pieces
she breaks into, which draw her limbs' lists from her object. At step 256 the C reads one byte
past its first table (the second table's first, 1) and writes the next texture's first pixel
(`@bug (game)`; the tables' order checked in the overlay's bytes).

The engine draws baked meshes; ADR 0048's one per-draw image replaces texture slot 0 of every
material of a draw, which can't stand for six different textures in one skeleton. The textures'
texels in a bake carried a content hash and their source segments, not where they came from.
The play state kept no object bytes: every object is in the pack.

The user chose (2026-10-09) per-draw images keyed by the texture's source address, over baking
her mesh in parts (one per texture) or logging the decay.

## Decision

- **A texture remembers its source:** `eng_gfx::TextureImage::source_addr`, the address
  (`G_SETTIMG`'s, segmented: `0x06xxxxxx` for an object) the texel at the tile's first TMEM word
  was loaded from (the interpreter now tracks each TMEM word's address beside its segment).
  Textures are interned by content and source, so the same texels loaded from two places stay two
  textures. **Pack format 27** (`out/data24`).
- **A draw can replace textures by source:** `eng_gfx::DrawParams::texture_images`, a list of
  `SourceImage { source, image }` (RGBA8). The renderer gives the instance its own textures for
  the mesh's textures with those sources, in whichever slot they're bound, and its own bind
  groups (made again only when the replaced textures or their sizes change; the texels written
  each draw that brings new ones). It combines with ADR 0048's slot-0 image.
- **The object RAM the game writes is play state:** `ObjectContext::written`, by bank and the
  region's offset in the file, the region's bytes as they are now, copied from the pack (the
  textures' stored texels) when first written, and dropped when the bank's object changes (a new
  DMA overwrites the RAM).
- **Her decay** writes into the region of her six textures (0x183A8, 0x1800 bytes) as the C's
  functions index it, so the step-256 write lands where the C's does. Her draw and her pieces'
  decode the region's textures (`object_ctx::rgba16_image`, the importer's own decoder) and pass
  them as `texture_images` with their sources (`boss_goma::decay_images`); before she decays the
  region doesn't exist and nothing is replaced.

## Consequences

- Her death draws as the game's: her textures holed bit by bit, the pieces holed as she was when
  they broke off, the second pass erasing everything but one pixel.
- Any other game write into an object's textures (or a per-draw swap of a texture by its address)
  can use the same two pieces.
- Every texture in the pack carries its source; the renders and traces didn't change (110 hashes
  the same), only the pack's bytes.
