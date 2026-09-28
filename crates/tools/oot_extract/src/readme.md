# Extracted OoT assets (local only)

Everything here was decoded from **your own ROM** by `ootx extract`. It is Nintendo's data:
keep it on this machine, never commit it (the folder is git-ignored) and never distribute
it. Re-create it any time with `target/release/ootx extract` (or `--only textures,models`).

## Layout

| Folder | Contents |
|---|---|
| `raw/` | Every ROM file, decompressed, under its decomp name (`files.json` lists them) |
| `textures/<xml path>/<file>/` | Every texture named in the decomp XMLs as PNG. `textures.json` per file records the N64 format, size, ROM offset and palette source, so edited PNGs can be re-encoded |
| `models/<xml path>/<file>/` | `<Skeleton>.glb`: skinned model with every animation that belongs to it. `dlists/<DList>.glb`: standalone display lists (props, items, parts). `models.json` indexes each file |
| `models/objects/object_link_boy`, `object_link_child` | Link with Player's default loadout and all 573 Player animations |
| `scenes/<xml path>/<scene>/` | Scene glb (rooms + collision), `scene.json` (actors, spawns, exits, lights, ...), prerendered backgrounds |
| `audio/` | `samples/` WAVs, `soundfonts/` JSON, `sequences/` raw `.seq` (+ `.mid` where converted) |
| `text/` | All messages per language as JSON and readable TXT |
| `manifest.json` | What was extracted, counts, errors |

## Units and conventions

- 1 glTF unit = 1 game unit, Y up (Blender converts to Z up on import).
- Object and actor models sit under a node scaled by 0.01, the usual actor scale, so they
  match scenes. Some actors use other scales at runtime (see their `Actor_SetScale` call).
- Animations are sampled at 20 fps (one keyframe per game frame).
- Materials:
  - **Single-texture materials** have the N64 colour combiner's constant inputs
    (prim/env colour, LOD fraction) baked into the embedded texture, so they look like the
    game. The unmodified texture is in `textures/`.
  - **Two-texture materials** use texture 0, with the combiner's tint as the colour factor.
  - The full N64 material state (combiner, render mode, geometry mode, prim/env) is in each
    material's custom properties (`n64_*`). Blender keeps these when you edit and
    re-export.
- Segments 0x08-0x0D are bound by actor draw code at runtime (eyes, colour variants,
  animated textures). Where they can't be resolved, parts are untextured or missing; each
  glb's extras list unresolved references. NPC eyes/mouths are guessed from texture names
  when that doesn't disturb the geometry.
- `texgen` materials (sphere-mapped shine, e.g. sword blades) can't be expressed in glTF
  and show their texture flat.

## Opening in Blender

File → Import → glTF 2.0. For skinned models, pick an animation in the Action Editor (each
animation is a separate action). Render in Material Preview / Eevee to see colours; the
Workbench "Texture" view ignores material colour factors.
