# 0045: The Fairy Slingshot, first person and Deku nuts (Player's side)

- **Status:** accepted, built in GAME-05 milestone 5a (2026-10-07)
- **Date:** 2026-10-07
- **Builds on:** [ADR 0041](0041-the-deku-stick-and-the-item-buttons.md) (Player's item buttons,
  the Start stand-in), [ADR 0016](0016-player-requests.md) (Player's requests),
  [ADR 0037](0037-the-deku-trees-other-enemies.md) (draw-time state in `draw_update`) and
  [ADR 0012](0012-actor-bakes.md) (bakes).

## Context

Milestone 5a gives Link the Fairy Slingshot and Deku nuts. Player's item functions were whole
since 4b, but the bow's and slingshot's upper-body actions (`func_8083501C` to `func_80835588`),
first person (`Player_ActionHandler_13`'s look and aim, `Player_Action_8084B1D8`), the nut's throw
(`func_8083C61C`, `Player_Action_8084E604`) and the camera they ask for (`Camera_Subj3`) weren't
ported, and their callers logged. Seven things needed a decision:
- **`Camera_RequestMode`'s result.** `func_8083AD4C` asks the main camera for the first person's
  mode and uses the answer in the same update (its caller enters first person only if the camera
  didn't refuse it while in its normal mode). Player is out of the actor arena during his update
  and makes his calls on the play state afterwards (`PlayRequest`s).
- **`heldActor`.** The seed in hand is an `En_Arrow` spawned as Player's child, placed every frame
  by Player's draw (`Player_PostLimbDrawGameplay`, the left hand), and let go by clearing its
  parent; Player had no held actor at all.
- **The first-person draw.** `Player_OverrideLimbDrawGameplayFirstPerson` replaces every limb's
  list: none in the look, only the arms aiming. Link's meshes are baked per loadout (variants).
- **The string** is drawn from the right hand, stretched by `unk_858`, which the draw itself sets
  (and `unk_85C`, also the stick's length, as its spring's speed).
- **Equipping before the pause menu** (5b): `Item_Give` puts nothing on a button.
- **The boomerang's and hookshot's branches** in the same functions (and their own upper-body
  actions), and the magic arrows' cost.
- **BACKLOG #18**: `Player_UpdateCamAndSeqModes` left out the knockdown's `CAM_MODE_STILL`.

## Decision

- **Ported whole:** `func_8083501C`, `func_80834F2C`, `func_80834D2C`, `func_80834EB8`,
  `func_8083442C`, `func_80834380`, `func_808351D4`, `func_808350A4`, `func_808353D8`,
  `func_80835588`, `func_80834FBC` (the hookshot's hook), `func_80834E44`/`E7C` (the shooting
  gallery, never), `func_8084FF7C` (the string's spring), `func_8083B8F4` (C-Up's look),
  `Player_ActionHandler_13`'s first-person branch with `func_8083AD4C`, `Player_Action_8084B1D8`
  with `func_8084ABD8`, `func_8083C61C` and `Player_Action_8084E604`, `func_8083356C`, the aim's
  branches in `func_8083DC54`, `func_8083DDC8`, `func_8083FC68`, `func_8083FD78` and
  `Player_UpdateCamAndSeqModes` (`CAM_MODE_Z_AIM`), and `Player_DetachHeldActor`. The
  boomerang's upper-body actions (`func_80835800` and the rest, `En_Boom`) aren't: where the C
  starts them, a note says so (the boomerang is a later dungeon's). A magic arrow's cost
  (`Magic_RequestChange`) logs and the arrow is a plain one; the magic meter isn't ported. The
  cutscene items' `unk_6AD` 4 (spells, trades, bottles, the ocarina) still log.
- **The main camera's answers are computed by Player** from a view of it taken as his update
  begins (its setting's modes, its mode, `CAM_STATE_LOCK_MODE`), with his own requests this
  update applied (`Player::camera_request_mode`, `camera_check_valid_mode`): the same values
  `Camera_RequestModeImpl` and `Camera_CheckValidMode` return. The request itself is made on the
  camera after his update, in its place among his requests (`PlayRequest::CamRequestMode`), with
  the camera's sounds; `Player_UpdateCamAndSeqModes`' own request comes after, as in the C.
- **`heldActor` is Player's handle** (`held_actor`). `func_8083442C` asks for the spawn
  (`PlayRequest::SpawnHeldArrow`); Player's update makes it right after his own update
  (`Actor_SpawnAsChild`, he the parent), so the seed updates the same frame, as in the C (the
  ITEMACTION category comes after Player's). Letting go is `PlayRequest::ReleaseHeld` (the
  seed's parent cleared before its update). His draw (`draw_update`) places the held seed at the
  left hand (`D_80126128`, turned by `Matrix_RotateZYX(0x69E8, -0x5708, 0x458E)`), as
  `Player_PostLimbDrawGameplay` does, and sets the string's `unk_858`/`unk_85C` (draw-time state).
  A held actor gone (`update == NULL`) is let go at the start of Player's update, as
  `Player_Update` does. `unk_A73` is Player's field, read by `En_Arrow` through `PlayerIface`.
- **The first-person draw is a Link variant:** `Loadout::first_person` gives every limb its
  first-person list (`PlayerRules::first_person_dls`, read by the importer from
  `sFirstPersonLeftForearmDLs` to `sFirstPersonRightHandHoldingWeaponDLs`; the hookshot's hand
  from the function's own code) or none, baked like the others. Player's draw picks it with
  `unk_6AD` 2 and the head projected behind -4, and draws nothing of Link in the look, as
  `Player_Draw` does. The string (`gLinkChildSlingshotStringDL`, the adult's bow string) is drawn
  from the right hand in the XLU list whatever the limbs draw.
- **Start's stand-in** (ADR 0021, 0041) also puts an owned slingshot and nuts on the first empty
  C button, as `KaleidoScope_UpdateItemEquip` swaps the buttons
  (`SaveContext::equip_item_on_c`, generalised from C-Left); the `deku-tree-slingshot` preset
  puts sticks, nuts and the slingshot on C-Left, C-Down and C-Right. 5b's pause menu replaces the
  item half.
- **The first raise is dry, as in the C:** the slingshot comes out with `unk_860` -2
  (`Player_InitBowOrSlingshotIA`), and `func_8083501C` only makes it positive when it's already
  positive, so the first draw has no seed or sound, and the real draw comes after the ready
  animation (`func_808353D8` with the button held). Tested, not "fixed".
- **`Camera_Subj3`** is ported whole (with `view.unk_124`'s second call at the end of
  `Play_Draw`, as `Camera_Subj4`), and the camera's view of Player has his focus
  (`Actor_GetFocus`).
- **BACKLOG #18:** knocked down (`Player_Action_8084377C`), Player asks for `CAM_MODE_STILL`.
- **Player's `actor.focus.pos` is the C's** (found by the first person's view: the camera sat at
  the neck and the slingshot showed at the top of the screen): `sPlayerFocusOffsetFromHead`
  (1100, -700, 0) in the head limb's drawn space (`Player_PostLimbDrawGameplay`), set where the
  draw sets it (`draw_update`), instead of the head limb's origin; `PlayerIface::focus` (what the
  attention system reads) returns it too. Its other readers move with it: the one-point
  cutscenes' shots aimed at Link, Navi's resting point, the attention system's line of sight
  (five goldens changed, logged in `golden/README.md`).
- **No clipping at the near plane, as the game's microcode** (found playing by hand: the string's
  pouch, nearer than the near plane in first person, was cut away). The game runs F3DZEX2's NoN
  variant (`graph.c`: `gspF3DZEX2_NoN_fifo`), which clips only at the far plane. The renderer's
  triangle pipelines draw with unclipped depth (`PrimitiveState::unclipped_depth`, wgpu's
  `DEPTH_CLIP_CONTROL`, asked for where the adapter has it: `eng_render::NON_FEATURES`, in
  `headless_device` and `eng_app::run_window`'s device), which clamps a nearer depth to the near
  plane per sample, and `shader.wgsl`'s `fs_main` discards a pixel past the far plane (the clip
  position's `z > w`). Not writing the depth from the shader keeps MSAA's per-sample depth; not
  clamping the vertices keeps the depth's interpolation. Without the feature the GPU clips both
  planes, as before.
- **The exit's run starts with room 1 cleared** (`Route::start_clears`: `Flags_SetClear` before
  the room loads, so `Actor_Spawn` spawns no enemy, as on a save that beat them). Room 1's big
  Deku Baba (scale 2.5) wakes within 500 of Link anywhere on that floor and bites him while he
  aims: `Rand`'s fight, which the run isn't about. The game's `game-slingshot.bat room1` keeps it.

## Consequences

- C-Right takes the slingshot out and raises it into first person (`CAM_MODE_AIM_CHILD`, the
  letterbox); holding it draws a seed, letting go shoots it (one less), the button again draws
  the next; A, B (which takes the sword out, the item buttons running first) or R end it;
  Z-targeted it aims in third person (`CAM_MODE_Z_AIM`). C-Up looks around in first person.
  C-Down throws a nut. Start puts them on C buttons.
- The knockdown's camera mode changes no golden (none knocks Link down); Player's focus moved
  five (the one-point shots aimed at Link, Navi, the attention system's line of sight).
- Whatever comes nearer the camera than the near plane draws, as on the console: 28 renders
  changed (the course's posts, the edges of Kokiri Forest's shots).
- 5b's pause menu equips them for real; the boomerang, the bow, the hookshot and the magic
  arrows keep their logged starts until their items come.
