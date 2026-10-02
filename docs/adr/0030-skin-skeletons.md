# 0030: Skin skeletons: a bone per vertex group, the skeleton as a pack record, foreign animations

- **Status:** accepted, built in GAME-04b milestone 6; extends ADR 0012 (actor bakes)
- **Date:** 2026-10-01

## Context

The horses (`object_horse_zelda`, `object_horse_ganon`, Epona's `object_horse` and Phase 6's
Phantom Ganon's) are *skin* skeletons (`z64skin.h`, `LimbType="Skin"` in the XMLs), which the
importer skipped. The opening's nightmare needs two of them (`En_Viewer` types 0 and 4).

**How the C draws a skin** (`z_skin.c`, `z_skin_awb.c`, `z_skin_matrix.c`). The skeleton's limbs
are of two kinds:
- a *normal* limb (`SKIN_LIMB_TYPE_NORMAL`, 11) draws its display list under its limb matrix,
  as any skeleton does;
- an *animated* limb (`SKIN_LIMB_TYPE_ANIMATED`, 4: the horse's body, one limb holding most of
  the mesh) draws a list that loads its vertices from segment 8, a buffer the game rewrites
  every frame (`Skin_ApplyLimbModifications`). Its vertices come in groups (`SkinLimbModif`):
  every vertex of a group gets the *same* position, a blend of offsets through limb matrices
  (`SkinTransformation`s weighted by `scale / 100`; one transformation unweighted), stored as
  the vertex's s16 `ob`, and its normal turned by one limb's rotation
  (`limbTransformations[unk_4]`). The list is drawn under the skin's own matrix only
  (`skin->mtx`, the actor's place, rotation and scale).

The renderer skins on the CPU: every vertex follows one bone's matrix (`GpuModel::pose`), any
number of `u16` bones (ADR 0006).

## Decision

1. **A bone per vertex group.** The bake (`BakeBody::Skin`) gives normal limb `i` bone `i` and
   each group of each animated limb a bone of its own after them, in limb order
   (`SkinSkeleton::modif_bone`). The group's vertices are baked at the bone's origin with their
   texture coordinates, normals and alpha (`Skin_InitAnimatedLimb`). Each frame
   `oot_game::skin::skin_bones` makes a group's bone the translation to its point (truncated to
   s16, as the vertex stores it) times the normal limb's rotation; the draw's transform is
   `skin->mtx`. A group is one point, so this is exact: no blended skinning in the renderer,
   no per-frame vertex upload.
2. **The interpreter's vertex bones.** `eng_gbi::Interpreter::vertex_bones[seg]`: a vertex loaded
   from entry `i` of a segment's buffer takes that buffer's bone `i` instead of the current
   matrix's. The bake binds the group buffer on segment 8 with its bones.
3. **The skeleton is a pack record** (`keys::skin`, `oot_game::skin::SkinSkeleton`: the limbs'
   joints, hierarchy, types and groups with their transformations). The runtime's matrices are
   `z_skin_matrix.c`'s functions on `MtxF` in the C's operation order.
4. **Foreign animations.** An animation in an object without a skeleton
   (`object_opening_demo1`'s, played on Impa's `gImpaSkel` and Zelda's skeleton) has no joint
   count the importer can find from its file. The content crate names the skeleton
   (`oot_actors::foreign_anims()`, as it lists its bakes) and the importer decodes it for that
   skeleton.

## Consequences

- The skin's normals are rotated from the bake's (the `SkinVertex` s8 normal) and renormalised
  by the renderer; the C truncates the rotated normal to s8 first. Not visible.
- `func_800A698C` (the limbs' matrices, each times its parent's) is walked with a stack, not
  recursively as the C writes it: rustc 1.95 (the pinned toolchain) and 1.98 miscompile the
  recursive form at `opt-level` 1 and above. A grandchild limb took its grandparent's matrix,
  which crumpled the horses. 1.92 and unoptimised builds were correct, and a small standalone
  program reproduces it. The unit test `skin::tests::children_follow_their_parents` guards the
  walk.
- `Skin_DrawImpl`'s override and post-draw callbacks, `SKIN_DRAW_FLAG_CUSTOM_*` and
  `Skin_GetLimbPos` aren't ported (the nightmare's horses use none); Epona and Phantom Ganon's
  horse will want them.
