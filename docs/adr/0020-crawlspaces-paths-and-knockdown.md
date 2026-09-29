# 0020: Crawlspaces, paths and the knockdown: the scene's path lists in the pack, `Camera_Subj4` writes Player back through the camera update, Player's knockdown pulled forward, overlay statics in the play state, and routes of steering tasks

- **Status:** accepted, built in GAME-03 milestone 2; extends ADR 0008 (the pack), ADR 0015 (camera settings and bg cameras), ADR 0016 (Player's requests) and ADR 0018 (the playthrough)
- **Date:** 2026-09-28

## Context

On a new save, Kokiri Forest's Kokiri Sword is in room 2, the training area, which is reached
only through a crawlspace. Four pieces of the C stand between Link's bed and the chest.

**The crawl is shared between Player and the camera.**
- Player's side is small: `func_8083F0C8` enters at a crawlspace's wall (`WALL_FLAG_4`, `_5`),
  `func_8084C760` sets the speed from the stick, `func_8083F570` and `func_8084C81C` climb out.
- `Camera_Subj4` (`CAM_SET_CRAWLSPACE`) does the rest. At the end of `Play_Draw`, in a second
  `Camera_Update` that its first call asks for (`view.unk_124`), it moves Player: onto the
  crawlspace's line, to the ground, facing along it (`camera->player->actor.world.pos`,
  `shape.rot.y`).
- The port's camera has only read Player until now (`PlayerView`).

**The boulder follows a scene path.**
- `En_Goroiwa` reads `play->setupPathList[params & 0xFF]`, from `SCENE_CMD_ID_PATH_LIST`, which
  the pack didn't import.
- Like the exit list, the path list has no count.

**The boulder knocks Link down.**
- Its AT sphere hits Player's AC cylinder, and it asks for a knockdown (`func_8002F6D4`: kind 2).
- Player takes both in `func_808382DC`: the body hit's damage and stagger (`func_80837C0C`),
  then the knockdown (`func_8084377C`, `func_80843954`, `func_80843A38`), with the invincibility
  timer.
- The roadmap had Player's damage in Phase 6. Without it, the boulder would roll through Link.

**`En_Wonder_Item`'s tag points are overlay statics.** The tag point instances write their
positions to `sTagPointsFree` / `sTagPointsOrdered` and go; the multitag instances read them.
No ported actor had shared statics before.

**The scripted run can't be a recorded input list.** The boulder loops round its corridors in
144 frames, so where it is when Link arrives depends on everything before. The run has to wait
for it, as a player does.

## Decision

- **The scene path lists are pack data** (`LayerData.paths`, format 9; `oot_game::scene::Path`),
  read at import like the exit list:
  - the list runs up to the next pointer target, while each entry points at points in the scene
    file (`oot_import::scene::path_list`);
  - the importer's test checks every header's count against the decomp XMLs' `NumPaths`, and the
    points against the ROM;
  - `PlayState::setup_path_list` is `play->setupPathList`.
- **`Camera_Subj4` writes Player back through the camera.** The mode function stores what it
  writes in `GameCamera::player_write`. `PlayState::camera_update` applies it to Player right
  after that update, which is the draw-time second update, as in the C. The camera still only
  reads Player (`PlayerView`, with `shape_pitch` added for the ease-in); the one write goes through
  the play state.
- **Player's knockdown is pulled forward from Phase 6, as far as the boulder uses it.**
  - Ported:
    - the invincibility timer (and the roll's and the fall damage's uses of it, which the
      spikes left out);
    - `func_808382DC`: the crush and void-floor respawns, the knockback an actor asks for, the
      body hit, the hurting walls and floors;
    - `func_80837C0C` for kinds 0 to 2;
    - the stagger and the knockdown's actions;
    - `Player_InflictDamage`;
    - `Player_InBlockingCsMode` around the water and ledge checks, as the C has it.
  - Actors ask through `oot_game::actor_ctx::func_8002f698` and its short forms, which write
    Player's `unk_8A0` to `unk_8A8` (`PlayerIface::set_knockback`).
  - Not ported, and logged:
    - dying at 0 health;
    - being frozen (kind 3) or shocked (kind 4);
    - the hit while swimming;
    - burning;
    - the hit's red flash (the draw's fog);
    - the rumble and the sounds.
- **Overlay statics live in the play state** (`PlayState::overlay_statics`, by actor id,
  `overlay_static::<T>()`).
  - A new play state starts without them, as `Play_Init`'s fresh overlay loads zero their
    `.bss`.
  - An overlay unloading mid-scene when its last actor goes (which would zero them again) isn't
    modelled.
- **`sys_matrix.c`'s rotations are ported on an `MtxF`** with the C's field names
  (`oot_game::sys_matrix`: `Matrix_RotateAxis`, `Matrix_RotateX/Y/Z` applied,
  `Matrix_MtxFToYXZRotS`, and a truncating `RAD_TO_BINANG`). The boulder builds its roll with
  them and reads its angles back. glam's matrices would round differently.
- **The playthroughs are routes of steering tasks** (`oot_actors::playthrough::Route`).
  - The Deku Tree's run (unchanged) and the Kokiri Sword's run share the driver.
  - New tasks:
    - `Crawl`: to the mouth, A on "Enter", the stick forward until out;
    - `WaitBoulder(from, to, fraction)`: idle until the boulder has covered that much of that
      path segment;
    - `Hurry`: a walk at full tilt;
    - `OpenChest`.
  - The text is read with A whenever the box waits, box breaks included (`MSGMODE_TEXT_AWAIT_INPUT`,
    which `Message_GetState` reports as its fallback, `TEXT_STATE_DONE_FADING`).
  - The sandbox's `sword-chest` script runs the new route, and its trace is a golden case.
- **The crawl's one-point cutscenes aren't ported** (`OnePointCutscene_Init` 9601 and 9602, and
  their switch of the main camera back to its previous setting). On the way out, the next floor's
  bg camera takes over.

## Consequences

- **A new save reaches the Kokiri Sword the game's way,** from Link's bed. `game-sword-chest.bat`'s
  debug start stays as a shortcut.
- **The Deku Tree's playthrough is unchanged,** except for the actor count in its trace:
  - `En_Wonder_Item` now runs in room 0;
  - its four tag points go at spawn;
  - the ordered multitag by the ford times out once Link has passed one of its points.

  Its golden hash is re-recorded for that reason.
- **Link can be hurt now.** Falls of 400 and more take half a heart or a heart. There is no
  dying yet: at 0 health Link carries on (logged).
- **The camera can move Player,** but only through `player_write`, and only `Camera_Subj4` uses
  it. Another mode that moves Player (the C has a few) would use the same channel.
- **The playthrough depends on the boulder's cycle.** Changing the route's timing before the
  training area moves where the boulder is when Link arrives. The waits absorb that, as long as
  the walks between them stay shorter than the lap (about 25 frames to spare at the tightest point, from the path's timings).
- **The pack grows** by the path lists only: 51.8 MB, format 9.
