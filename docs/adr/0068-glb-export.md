# 0068: The characters as .glb: `ootx export-glb` from the pack

- **Status:** accepted, built (2026-10-14)
- **Date:** 2026-10-14
- **Builds on:** [ADR 0008](0008-asset-pack.md) (the pack), [ADR 0012](0012-actor-bakes.md) (the
  bakes), [ADR 0030](0030-skin-skeletons.md) (the skin skeletons).

## Context

The game's characters should open in Blender and, later, in the game editor, which will take
glTF in (stage E9). `oot_extract` already writes .glb files, but it reads the ROM through the
XMLs and guesses the textures actors bind at run time (eyes, mouths). The pack has the characters
as the game draws them: every skeleton's mesh, the actors' bakes with their real faces, hands and
colours, Link's variants, and the horses' skin skeletons.

The first version wrote the pack's records as they are. Opened in Blender, they were hard to use
(the user's view):
- every model sat inside a large sphere;
- the characters lay down, folded in half (a skeleton's zero pose: every rotation zero);
- the files had the bakes' names (`opa_eye0`), one per eye state and pass.

The sphere was Blender's importer: it draws every bone with an icosphere sized from the
armature, and the first version scaled the armature node by 0.01, which the importer turned into
bones hundreds of metres long.

## Decision

1. **`ootx export-glb [--out out\glb] [--filter <part of a path>] [--anims] [--all]`**, in
   `crates/tools/ootx/src/glb.rs`.
2. **By default, the characters by name:** `characters\<Name>.glb` for each of
   `glb/characters.rs`'s table (about 85: the ported actors' people and creatures, Link at both
   ages, the seven horses), e.g. `Saria.glb`, `Kokiri Boy.glb`, `Hylian (Old Woman).glb`. Each
   entry gives:
   - the mesh: the actor's default bake (eyes open, opaque, nothing extra in hand), a plain
     skeleton mesh when the actor draws that (the Business Scrub with its nose), Link's default
     loadout, or a skin skeleton;
   - the parts the draw puts at a limb's matrix after the skeleton (a carpenter's head, Queen
     Gohma's eye and iris, the Peahat's body), merged onto that limb;
   - the colours the draw code sets on the bake's dynamic segments (a Kokiri's tunic and boots),
     which the bake stores as black;
   - an idle animation, and each arm's limbs, shoulder to hand.

   **The rest pose is a T-pose:** the idle animation's first frame, at the origin, with each arm
   turned limb by limb to point straight out along X on its shoulder's side
   (`straighten_arm`). It is the file's rest pose (the nodes' rotations and the inverse bind
   matrices), so the character opens standing. Characters without arms keep the idle pose;
   horses take their idle frame through `oot_game::skin::skin_bones`. The table was read from each
   actor's module and C.
3. **With `--all`, everything the pack has,** into `all\` and in the skeletons' zero pose: every
   skeleton (`all\<file>\<symbol>.glb`), every actor bake of a skeleton's mesh
   (`all\bake\<actor>\<bake>.glb`), every Link variant (`all\player\<age>\<lists>.glb`) and every skin
   skeleton, with its actor's bake or baked by `ootx` through the importer's
   `ObjectSegments::bake_mesh` (five of the seven horses have no actor's bake).
4. **The file:** a node per limb in the rest pose, a skin whose joints are the limbs, with their
   inverse bind matrices, and the mesh moved to model space for the rest pose, each vertex on its
   bone with weight 1. A skin skeleton's vertex groups (bones past the limbs) are put where
   `skin_bones` puts them, and weighted on their `SkinTransformation`s' limbs by `scale / 100`
   (the four heaviest, normalised). **The 0.01 actor scale is in the data** (positions, joint
   translations, inverse bind matrices, animated translations), not on a node, so bones are as
   long as the limbs. With `--anims`, the object's standard animations whose joint count fits, as
   a rotation channel per limb (the s16 triple as binary angles, Z then Y then X) and the root's
   translation, at 20 frames a second; Player's 573 animations go on Link's default loadout only.
5. **The writer is `oot_extract`'s** (`gltf::Gltf`, `models::SkinnedModel`, which
   `write_skinned` wraps), rather than a glTF crate: it already writes the GLB container, PNG
   textures, skins and animations, and its materials evaluate the combiner. A single-texture
   material bakes the combiner's constants (prim, env, the LOD fraction) into the texture with
   the shade at white; the vertex colours (unlit) or normals (lit) stand in for the shade. The
   alpha mode comes from the `BlendMode`; the samplers' wrap from the `TextureSlot`s (mirror is
   glTF's mirrored repeat, clamp its clamp to edge). The N64 state glTF can't hold is in each
   material's extras.
6. **Names from the decomp.** The pack keys limbs and textures by address, so their names come
   from the XMLs when `oot.toml`'s project opens: limbs through the skeleton's limb table,
   textures by their source address (`TextureImage::source_addr`: segment 4 `gameplay_keep`, 5
   `gameplay_field_keep`, 6 the object or a limb override's file, a segment the bake binds a
   texture to its symbol). A material is named after its textures (`gSariaHairTex`, a suffix for
   a second one). Without the project the geometry is the same and the names are generic
   (`limb_NN`, `<name>_matN`). A limb the XML doesn't name stays `limb_NN` (most of Epona's).
7. **Out of git:** the output is derived from the ROM. It goes under `out\`, which `.gitignore`
   covers; `git status` stays clean after a run.

## Consequences

- The characters (2026-10-14): 86 files, 29,413 triangles, in about a second. `--all`: 523
  files, 220,657 triangles; with `--anims` 4,516 animations and 134 MB. Checked in Blender 4.1:
  each imports as an armature with named bones the size of its limbs, a vertex group per limb,
  its materials and actions; rendered, the characters stand with their arms out.
- The units are the game's: a character is its world size in metres (adult Link about 50 tall).
- A few poses are the game's own oddities: the Man on the Roof lies down (his only idle), the
  Skulltula House's sons squat (their dance), the dancing couple and the haggling townsfolk are
  several people on one skeleton and keep their pose.
- Some bakes bind a different texture to one eye on purpose (Saria's and Zelda's half-closed
  left eye); the characters keep it.
- Segments the draw code binds that no bake records (Epona's eyes, the plain skeletons' faces)
  stay unresolved, as in the pack; each file lists them in its extras (`unresolved_segments`).
- Linear blend skinning only approximates a skin's groups, which the game puts at one exact
  point each: posed, the horses show hairline seams at a few joints.
- Curve skeletons (`curve/`) and display lists that aren't a skeleton's are left out. A newly
  ported actor's character is added to the table by hand.
- Tests (`cargo test -p ootx glb`, skipped without a pack): `gArrowSkel` read back (the GLB
  header and chunks, the node, joint and mesh counts, no scaled node, the joint positions, the
  inverse bind matrices, the POSITION bounds against `posed_bounds` in the zero pose), Saria's
  animations frame by frame against the joint tables, Zelda's horse's weights, and adult Link's
  T-pose (standing, both arms level and out to their own sides).
