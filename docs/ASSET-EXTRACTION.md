# Asset extraction

`ootx extract` decodes the user's ROM into editable formats in a git-ignored folder (`extracted/` by default). This speeds up asset development: you can look at, modify and build on the original assets in normal tools like Blender and image editors.

**The output is Nintendo's data.** It stays on the machine that made it. The repo contains only the extractor code, and anyone using it runs it against their own ROM.

## What's extracted

| Part | Output | Count (gc-eu-mq-dbg) |
|---|---|---|
| `raw` | Every ROM file, decompressed, by decomp name | 1532 files, 54 MB |
| `textures` | PNG per texture in the decomp XMLs, plus `textures.json` (format, size, offset, palette source) | 4960 (all 9 N64 formats). 68 whose palette is set by code are exported as grey ramps |
| `models` | Skinned glTF per skeleton with all its animations; static glTF per standalone display list | 191 skeletons, 2395 animations, 1316 props, 97k triangles |
| `scenes` | glTF per scene (rooms, collision, water boxes, actor markers), `scene.json`, `collision.json`, prerendered background JPEGs | 110 scenes, 401 rooms, 247 layers, 169k triangles, 90k collision polys, 7760 actor placements, 36 backgrounds |
| `audio` | WAV samples (with loop points), soundfont JSON, raw `.seq` + MIDI per sequence, SFX id map | 450 samples from 38 soundfonts, 109 sequences (+1 alias), 1252 of 1259 SFX ids mapped |
| `text` | Messages per language as JSON + readable TXT, control codes as `{TAGS}` | 2116 English, 2115 German, 2115 French, 48 staff |

Whole run: about 15 seconds, about 250 MB.

## How it works

Everything reuses the spike 01/02 decoders. The decomp's XMLs name every asset and give its offset, and the decomp's C source supplies runtime rules:

- **Models**:
  - `Skeleton::parse` + `build_draw_list`. Vertices are placed in the bind pose, and each limb becomes a glTF joint.
  - Animations are sampled per game frame (20 fps) as quaternion keyframes. Standard animations are matched to a skeleton by the joint count implied by their index table.
  - **Link** uses Player's draw rules (`oot_core::player`): the default loadout, eyes, tunic and all 573 Player animations. His extra display lists (hand poses, swords, shields, boots, gauntlets, masks) are exported as props in the same bind-pose space, so they line up with the skeleton.
  - **Horses** (Skin skeletons) are exported with real multi-weight skinning, rebuilt from the `SkinLimbModif` data the game blends at runtime.
- **Materials**:
  - Single-texture materials have the N64 combiner's constant inputs (prim/env/LOD fraction) baked into the texture, so glTF viewers show what the game shows. Example: Link's tunic is a grey I4 texture tinted by env colour.
  - The complete N64 state is kept as material custom properties (`n64_*`). The raw textures are in `textures/`.
- **Scenes** (written by a separate module, `scenes.rs`):
  - Every scene and room header command is decoded for every layer: actors, spawns, transition actors, objects, exits, lights, paths, skybox, sound, camera.
  - Room shapes 0/1/2 are run through the interpreter.
  - Animated-texture segments (08–0D) are resolved by interpreting each scene's draw-config function in `z_scene_table.c`, assuming child-day, frame 0.
- **Text**: `tools/msgdis.py` ported to Rust.
- **Audio** (`audio.rs`), from the four audio tables in `code` plus the `Audiobank`, `Audioseq` and `Audiotable` files:
  - **Samples:** VADPCM (9-byte) and small ADPCM (5-byte) are decoded to 16-bit WAV, with loop points in a `smpl` chunk.
    - Drums and SFX are written at their exact playback rate.
    - Instruments are written at a standard rate, with a root note that keeps the pitch exact.
  - **Sequences** are converted to MIDI by a tick-accurate re-implementation of the sequence player: seq, channel and layer scripts, including loops, calls and branches.
    - Looping songs are cut after one pass.
    - Channels use the soundfont's instrument ids as programs, not General MIDI, so a GM player plays the right notes with the wrong instruments.
  - **SFX:** `sfx.json` maps SFX ids to the sounds sequence 0 plays for them.

## How it was checked

- **Numeric**:
  - Every count matches the decomp XMLs or tables.
  - 0 texture failures, 0 unknown GBI opcodes in models or scenes, 0 unknown text control codes.
  - Every scene header parses to END.
  - Scene visual bounds match collision bounds.
  - Actors stand on collision floors (Kokiri Forest: 110 of 111).
- **Audio:**
  - The VADPCM decoder is bit-exact. The game stores the decoder state at each loop start, and 115 of 116 looped samples match it exactly (the exception is broken, unused font 37).
  - All WAVs and MIDIs parse with ffprobe.
  - The melodies of Saria's Song, Epona's Song, Zelda's Lullaby, Song of Time and Song of Storms were checked note for note.
- **Blender 3.0.1** (headless import + Eevee renders):
  - Link (adult and child, walking, sword slash) skins and animates correctly, including the flex seams.
  - Saria, child Zelda, Gohma, Tektite and the dog work, with eyes guessed correctly.
  - Epona gallops and rears with smooth weighted deformation.
  - Kokiri Forest and the Deku Tree render as the recognisable levels.
  - Material custom properties survive import.

## Known gaps

- **Runtime-bound segments.** Actors bind segments 08–0D in their draw code (eyes, colour variants such as Tektite red/blue, dog coat colours). Unresolved references leave parts untextured; each glb's extras list them. NPC eyes are guessed from texture names, and a guess is kept only if it doesn't change the geometry.
- **Two Curve skeletons** aren't exported: the treasure chest lid and the time-warp effect.
- **Sphere-mapped shine** (`G_TEXTURE_GEN`, e.g. sword blades) can't be expressed in glTF, so the texture shows flat.
- **Two-texture materials** use texture 0 plus a tint rather than the full blend.
- **Skyboxes** (including the market panorama, which is how OoT draws the market square) are exported as textures only, not assembled into a sky mesh.
- **Scene assumptions:** scene draw configs assume child-day at frame 0. 4 segment references stay unresolved: `hairal_niwa2` segments 9/A/B, and one segment-6 reference in Water Temple room 0.
- **Near LOD only** is exported for models.
- **MIDI** leaves out envelopes, vibrato, portamento, reverb and filters. Parts the game drives at runtime (SFX sequence 0, nature ambience, cutscene effects, and Hyrule Field's dynamic mixing) come out empty or only partly. Soundfont 37 is unused and broken in the ROM.

## Towards custom levels

This folder is step 1 of the level plan:

1. **Extract** (this): original scenes as glTF + JSON, editable in Blender.
2. **Engine loader**: the clone loads levels from glTF + JSON (geometry, collision with surface types, actors, spawns, exits, lights), not from the ROM. Extracted levels in `extracted/` and your own levels in a committed `levels/` folder then load through the same code.
3. **Round trip**: edit a room in Blender, re-export, and load it in the engine. The custom properties carry N64 material and collision data through Blender.
4. **Shareable custom levels**: a level refers to original textures by decomp name (e.g. `gKokiriForestGrassTex`) instead of embedding pixels, and the engine resolves the name from the player's own ROM. The level file then contains only your work.
