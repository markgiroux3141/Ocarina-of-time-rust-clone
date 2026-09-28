# GAME-02: the Kokiri Forest vertical slice

**Goal:** Phase 3 of [ARCHITECTURE-PLAN.md](ARCHITECTURE-PLAN.md) §5: Kokiri Forest playable as in the game, one ported system or actor at a time, each checked against the C.

| # | Milestone | Status |
|---|---|---|
| 1 | The collision check; Kokiri's props with their real models; `En_Ko`; ladder and vine climbing; Z-targeting polish | done |
| 2 | `En_Door`, the prerendered backgrounds and the fixed cameras | next |
| 3 | The message box, and items | |

The working rules are the same as for GAME-01:
- no game data in the repo;
- the engine never depends on game code (`cargo test -p layering`);
- the runtime reads only the pack;
- ports go function by function with the decomp's names, every constant is cited, and faithful bugs are marked `@bug (game)`;
- test expectations come from the C.

Decisions are in [docs/adr/](adr/README.md) (0011 to 0013 so far).

## Milestone 1: collisions, props, the first NPC, climbing, Z-targeting

**Answer:** done. The sword cuts Kokiri's bushes and signs and bounces off its rocks, and Link bumps into all three. The Kokiri children stand, animate, fade with distance, turn their heads to Link and block the way to the Lost Woods. Link climbs the ladder to his house and back down. Z-targeting has the game's camera, bars and reticle.

The tests: 157 pass, 1 ignored (151 before the Z-targeting work). The goldens are re-recorded for the camera modes: 28 of 78 hashes, each explained in `golden/README.md`.

### What was built

1. **The collision check** ([ADR 0011](adr/0011-collision-check.md)).
   - `oot_game::collision_check` ports `z_collision_check.c`: the collider types (cylinder, joint spheres, triangles, quads), their init and update, `CollisionCheck_SetAT/AC/OC`, the 16 AT-against-AC shape pairs, the quad's nearest-AC rule, OC pushes by mass, and `CollisionCheck_Damage` with damage tables.
   - `eng_collision::math3d` has the `sys_math3d.c` primitives the checks use.
   - The checks run where `Play_Update` runs them: after the room load, before `Actor_UpdateAll`.
   - Player registers its body cylinder, both sword quads (placed from the left hand's draw, `func_80090480`) and the shield quad, with the C's damage flags per attack (`D_80854488`).
2. **Kokiri's props, with their real models.**
   - `Obj_Hana` (a flower, rock or bush), `En_Ishi` (rocks: the sword bounces off, `AC_HARD`), `En_Kusa` (bushes: cut, stumps, regrowth) and `En_Kanban` (signs: cut along the slash into flying pieces, with the cut mark).
   - Their display lists come from the pack, the multi-list ones as actor bakes ([ADR 0012](adr/0012-actor-bakes.md)).
   - Placeholders for them are gone from room 0.
3. **`En_Ko`, the first NPC with a skeleton.**
   - The whole overlay, apart from talking. It picks the children that spawn for the story's progress (`EnKo_CanSpawn`) and waits for its objects. Each child's animation comes from `sOsAnimeLookup` / `sAnimationInfo`. It fades by Link's distance (`func_80A98DB4`, untargetable when faint), and turns its head and torso (`func_80034A14` and the `D_80116130` presets). Child 3 guards the Lost Woods until Link has the Kokiri Emerald.
   - It's drawn as `EnKo_Draw` draws it: tunic and boots colours per draw, and 14 baked variants (head, eyes, opaque or translucent).
   - New framework under it:
     - `oot_game::skelanime_std` (`SkelAnime` for standard animations);
     - `oot_game::npc` (the talk offers and `func_800343CC`, the head tracking, the limb sway);
     - the save's event, item and info tables;
     - `Rand` on `PlayState`;
     - `Actor_SpawnAsChild` (each child's fairy);
     - cached skeletons and animations in `GameAssets`.
4. **Ladder and vine climbing.**
   - `func_8083F7BC` (walking into a climbable wall) and `func_8083EC18` (onto it: lined up to a ladder's rungs, or turning round at a ladder's top).
   - `func_808458D0` (the item put away first), `func_8084BF1C` (climbing: a rung per animation, sideways on vines) and `func_8084C5F8` (stepping off at the top or bottom).
   - The climbable-edge branch of `func_8083A6AC`, so walking off the porch hangs from the ladder's top and climbs down.
   - `sAgeProperties` is now read whole (33 fields), with the climbing animations.
5. **Z-targeting polish** ([ADR 0013](adr/0013-camera-modes-and-screen.md)).
   - **Camera modes.** `Player_UpdateCamAndSeqModes` asks for a mode each frame, and `Camera_ChangeModeFlags` accepts, refuses or resets it. The pack now holds all 21 NORMAL0 modes' functions and data.
   - **`Camera_Parallel1`** (Z with nothing targeted): the swing behind Link over `R_CAM_DEFAULT_ANIM_TIME` frames with mode changes refused meanwhile, then the target distance and pitch. With it come `Camera_CalcAtForParallel` and `func_800458D4`.
   - **`Camera_KeepOn1`** (locked on to a non-enemy, such as the Kokiri or a sign), with `Camera_CalcAtForLockOn`.
   - **The letterbox:** `shrink_window.c` and `Camera_UpdateInterface`, 10 rows a frame towards 26, 27 or 32. It's drawn as black bars under the overlay.
   - **The real reticle** (`func_8002C124`): `gZTargetLockOnTriangleDL` after `SETUPDL_57`, four spinning triangles per trail entry in the orthographic overlay. It flies in from the screen's centre as it locks and fades when the target is lost, in the category's colour (`sNaviColorList`).
   - **The target arrow:** `gZTargetArrowDL` over the next candidate.
   - **Placeholders stop counting as targets:** they spawn with their profile's flags minus `ACTOR_FLAG_0`, and a test checks that neither the candidate search nor Z picks one.
   - **Two fixes to the target context against `func_8002C7BC`:**
     - a new target now calls `func_8002BE98` (the reticle restarts at 500);
     - the before-lock drop uses `Actor_ProjectPos`'s test, not the candidate search's screen box.
   - **Engine:** `DrawLists::overlay_2d` (the orthographic `OVERLAY_DISP`) and `DrawLists::letterbox_rows`, drawn by `Renderer::render_screen`.
6. **Tools:**
   - `ootx pack-ls <prefix>` lists records;
   - `ootx dl-dump <file> <symbol>` prints a display list's commands and vertices from the ROM;
   - `oot_sandbox --entrance ... --at` places Link standing.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 157 passed, 1 ignored |
| Collision check (`oot_game --test collision_check`, 8; `math3d` unit tests, 15) | OC pushes by mass (equal masses, immovable, type and height skips); a sword quad hitting a bush on both sides; damage from the toucher's table entry; the nearest-quad rule; pairs skipped by type, owner and death; the context's list sizes. The shape pairs themselves are covered through `math3d`'s tests |
| Props (`oot_actors --test props`, 3) | Kokiri's props initialise as their C does (collider dims, flags, types); the sword cuts a village bush; a slash cuts a piece off a sign (the piece, the cut mark's timer from the C). No test covers the rock's bounce (`AC_HARD`) yet |
| `En_Ko` (`--test kokiri`, 3) | The children that spawn and their start animations; the fade (40 a frame, targetable from alpha 10); child 3's guard position, 80 from home facing Link |
| Climbing (`--test climb`, 2) | Up the ladder to the porch (lined up within 8 of the rungs, off the top onto y 100); off the porch onto the ladder's top, facing it, and down to the ground |
| Z-targeting (`--test zcamera`, 5; `letterbox` unit test) | Parallel1's swing lands behind Link and refuses mode changes while animating; the bars reach 27 in 10-row steps after the swing; a request during the swing goes through the frame after it ends; a Kokiri lock uses FOLLOWTARGET / KeepOn1 with bars and the NPC colour; the reticle flies in, locks at 120, and fades 0x100 → 136 → 16 → 0; placeholders are never candidates or locked |
| Camera data (`--test camera`) | 21 NORMAL0 modes, with the NORM1, PARA1 and KEEP1 values checked against `z_camera_data.c` |
| Golden traces and renders | Re-recorded: 28 of 78 changed (see `golden/README.md`), 50 unchanged |
| Import | 9.0 s; format version 4 (camera modes; the reticle's two bakes, 29 actor bakes in all) |
| The windows | Bushes, rocks, the sign and its cutting, the Kokiri, and the ladder checked in the windowed sandbox. The Z-target bars and the reticle on a Kokiri checked in a headless screenshot (27-row bars, light-blue triangles) |

### Decisions

- **[ADR 0011](adr/0011-collision-check.md):** colliders stay in their actors; the context holds references, and the checks take the colliders out and put them back. The hit elements read later are copied at hit time.
- **[ADR 0012](adr/0012-actor-bakes.md):** actors declare what they draw (lists or a skeleton, bound segments, a prelude), and the importer bakes it. Per-draw colours are dynamic segments; states that change geometry or the render mode are separate bakes.
- **[ADR 0013](adr/0013-camera-modes-and-screen.md):**
  - camera modes dispatch on the imported `CAM_FUNC`, and unported modes run Normal1 on NORMAL's data;
  - the letterbox is game state drawn as engine bars;
  - 2D overlay draws are an engine list in the interface's orthographic projection, widened with the aspect;
  - the reticle moves at the game's 20 Hz.
- **`Camera_KeepOn1` came with the milestone**, beyond what it asked for (Parallel1 and the bars). Every lock-on in Kokiri Forest is on a friendly actor, so without KeepOn1 none of them would show the Z camera or the bars.

### Known gaps

- **Camera:**
  - `Camera_Battle1` isn't ported, so locking on to an enemy frames like the Normal camera with no bars. Kokiri Forest has no enemies; the sandbox's course dummy is one.
  - The same Normal1 fallback covers TALK (`Camera_KeepOn3`), jumps and falls (`Jump1`), climbing (`Jump2`), hanging (`Uniq1`) and first person (`Subj3`), with a restart at each mode change.
  - `func_80043F94` (scenes with the skybox disabled), the interface alpha, the mode-change sounds, and KeepOn1's `interactRangeActor` case are left out.
- **Reticle:** no Navi (`naviRefPos`), no lock-on sounds. The reticle moves at 20 Hz while the world is blended at the display rate, so it can trail a moving target by up to one game frame.
- **Talking:** actors offer to talk (`func_8002F1C4`), but Player doesn't accept on A and there's no message box, so `En_Ko`'s and `En_Kanban`'s talk states stay 0. That's milestone 3.
- **Props:** no lifting or throwing (Player can't lift), no drops (`En_Item00`), no leaves, fragments, dust or sounds. The bugs in bushes are `En_Insect` placeholders. `En_Kanban`'s shadow, ocarina repair and hammer quake aren't ported.
- **`En_Ko`:** no paths (child 3 with the emerald), no Fado trade, and each child's fairy is an `En_Elf` placeholder.
- **Climbing:** grabbing a climbable wall from the air (`func_8083EC18` from `func_8084411C`) and crawlspaces aren't ported, and there's no climbing sound. The vine code is shared with the ladder, but only the ladder has a test.
- **Collision check:** hit effects and sounds, `CollisionCheck_LineOC`, colliders without an actor.
- **Placeholders** can't be targeted, even where the real actor can (Mido, Saria). Each one becomes targetable when it's ported.
- **Carried over from GAME-01:** prerendered rooms, doors, time passing, culling (see GAME-01's known gaps).

## Recommended next step

**Milestone 2: `En_Door` with the prerendered backgrounds and the fixed cameras,** as planned. It opens Link's house and the shop from the inside, and brings the bg camera list (`Camera_ChangeBgCamIndex`), which the camera's mode dispatch now has room for.

Two cheap checks first:
- compare the Z-target camera and the bars on a Kokiri child with Project64, side by side at the same spot (the bar height, the swing's length, KeepOn1's framing);
- in the windowed sandbox, lock on to a Kokiri and a sign, check the reticle's colours and spin, and walk away until the lock breaks, to see the fade.
