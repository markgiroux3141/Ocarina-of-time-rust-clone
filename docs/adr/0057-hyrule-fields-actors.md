# 0057: Hyrule Field's actors: limbs at draw time, broken bodies as part bakes, no culling volume, the enemy music

- **Status:** accepted, built in GAME-06 milestone 4 (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0006](0006-rendering-model.md) (baked meshes, the draw lists),
  [ADR 0012](0012-actor-bakes.md) (bakes), [ADR 0056](0056-the-clock.md) (the clock, `IS_DAY`).

## Context

Hyrule Field's actors are the first ones whose update reads what their draw left:

- **The draw sets state.** `EnSkb_PostLimbDraw` moves its collider's spheres onto its limbs
  (`Collider_UpdateSpheres`) and hands `BodyBreak_SetInfo` each limb's list and matrix.
  `EnPeehat_PostLimbDraw` puts its weak point's sphere 1,000 behind its body, takes its blades'
  tips (`Matrix_MultVec3f`), and `EnPeehat_Draw` its blades' quad. The next update reads them:
  the Stalchild breaks into the limbs the draw took, the Peahat's blades dig where their tips
  cut the ground.
- **Overrides change the skeleton's walk.** An override that returns true has drawn its limb
  itself, at the parent's matrix with its own changes; the limb's transform isn't applied, and
  its children hang from the parent's (`SkelAnime_DrawLimbOpa`). The Peahat's body jiggles that
  way, its blades turn by `bladeRot`.
- **A broken body's parts are limbs.** `BodyBreak_SpawnParts` spawns an `En_Part` per limb taken,
  drawing that limb's list. The pack keeps a skeleton's limb lists only inside its skeleton's mesh
  (ADR 0012): an `En_Part` has nothing to draw by name.
- **The culling volume.** `Actor_CullingVolumeTest` decides each frame whether an actor is in
  view (`ACTOR_FLAG_INSIDE_CULLING_VOLUME`); the port has never had it, and counts every actor in.
  `En_Wood02`'s spawned trees leave when out of view; the Peahat digs only in view.
- **The enemy music.** `Attention_FindActorInCategory` notes the nearest targetable, hostile
  enemy within 500 (`attention.bgmEnemy`); `Player_UpdateCamAndSeqModes` turns the field's
  music to `SEQ_MODE_ENEMY` while there is one. The port had the music's side and not the note.
- **Spawns of unported actors.** The Peahat's death spawns a bomb (`En_Bom`); `En_Encount1`'s
  other kinds spawn Leevers, Tektites and Wolfos.

## Decision

- **Limbs at draw time** (`oot_game::skelanime_std::draw_opa_pose`): a walk of the skeleton as
  `SkelAnime_DrawLimbOpa` does it, with an override that says what it did (`LimbDraw::Default`
  with its pre-matrix, or `LimbDraw::Drawn` at a matrix relative to the parent's, the children
  then from the parent's) and a post hook with the stack's matrix. An actor runs it in
  `draw_update` (`Play_Draw`'s time, once a game frame) for the state its draw sets: spheres,
  blade tips, the quad, `BodyBreak_SetInfo`; and again in `draw` for the bones it draws with.
- **`BodyBreak` and `En_Part` whole** (`en_part.rs`): `BodyBreak` as the C's, taking matrices in
  the world. Each ported enemy's breakable limbs are baked on their own, one bake per list after
  `Gfx_SetupDL_25Opa` (`En_Part/<object>/<symbol>`): for now the Stalchild's 17. An `En_Part`
  draws its list's bake.
- **A skeleton drawn in parts:** limbs an override draws itself are left out of the skeleton's
  bake (`LimbOverride` with no symbol) and drawn as their lists' own bakes at their matrices
  (the Peahat's body, top and second body; the Stalchild's head and jaw left out when headless).
  An env colour the C sets before one limb is the whole skeleton's (the Stalchild's head pulse:
  no other limb reads it).
- **No culling volume, still.** Every actor counts as in view; the actors keep their culling
  fields (`cullingVolumeDistance`, `Scale`, `Downward`) for when it's ported. A spawned tree,
  once in, stays; the Peahat digs wherever it is.
- **The enemy music** (`TargetCtx::bgm_enemy`): set in the attention search as the C's, the
  locked target included; Player turns it into `SEQ_MODE_ENEMY` and `Audio_SetBgmEnemyVolume`
  before `Audio_SetSequenceMode`.
- **Unported spawns are placeholders,** logged: `En_Bom` for the Peahat's explosion (the bomb's
  `timer = 0` has nowhere to go), `En_Reeba`, `En_Tite` and `En_Wf` for `En_Encount1`'s other
  kinds.
- **The roll's bonk:** `Player_Action_Roll` sets a tree's `home.rot.y` (`En_Wood02`) or a large
  crate's `home.rot.z` (`Obj_Kibako2`) through a play request (`PlayRequest::Bonk`), as the C
  writes the other actor.
- **Two exit runs:** the field by day (`Route::Field`: a Peahat, Castle Town's entrance) and at
  20:00 (`Route::FieldNight`: Stalchildren, the owl, Kakariko). The drawbridge is up at night,
  and the day's clock takes some 2,700 frames from 10:00 to 20:00.

## Consequences

- The field has its trees, bushes, rock circles, signs and grottos' holes, its Peahats by day
  and its Stalchildren by night, with their music. Its two runs are goldens (`field`,
  `field_night`, with `field_peahat` and `field_night_fight`).
- Kokiri Forest's signs and its rock circle are real: the signs block the Mido and shop run's
  old line (moved round the shop's sign), and the rocks take `Rand` calls (a different bush
  drops in the Deku Tree run) and nudge the farewell's walk.
- The dusk run's Stalchildren rise at nightfall and hit Link in front of the drawbridge, and
  its spawners' trees come in as they come into view.
- **A decal's depth bias is clamped** (2e-3, `eng_render`'s pipelines). Its slope part is taken
  over a whole triangle; Hyrule Field's paths are huge triangles whose near end is clamped at the
  eye, and the bias put the path behind Link in front of him. The RDP draws a decal only where it
  meets the depth already there; the clamp keeps a decal on its ground and off what stands on it.
- **Deviations from the console, accepted:**
  - No culling: spawned trees stay, the Peahat digs out of view, and everything draws.
  - The Peahat's bomb is a placeholder: no explosion, and a marker where it died (drawn only
    with `--placeholders`).
  - A drawn limb's matrices are the walk's, computed in `draw_update` and again in `draw`
    (interpolated frames draw the render state's joints).
