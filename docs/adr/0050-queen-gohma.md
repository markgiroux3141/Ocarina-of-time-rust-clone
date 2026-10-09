# 0050: Queen Gohma: `Boss_Goma` whole with her intro and death as her own actions, her own sub camera, her draw from one skeleton bake and two limbs apart, and the jump slash pulled forward

- **Status:** accepted, built in GAME-05 milestone 6a (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0022](0022-cutscenes.md) (sub cameras, the manual cutscenes, Player's
  cutscene modes), [ADR 0029](0029-one-point-cutscenes.md), [ADR 0033](0033-effects.md),
  [ADR 0037](0037-the-deku-trees-other-enemies.md) (`En_Goma`'s boss side, draw-time `Rand`,
  per-limb colours in bakes), [ADR 0039](0039-quakes.md), and
  [ADR 0051](0051-textures-replaced-by-source-and-the-object-ram.md) (her decay).

## Context

Milestone 6 is split in three (the user's choice, 2026-10-09): 6a Queen Gohma, 6b the blue warp,
6c the run through the Deku Tree. 6a ports `Boss_Goma` (`z_boss_goma.c`, 1,660 lines in functions,
no version branch) and `Item_B_Heart` (67). Several things needed a decision:

- **Her cutscenes aren't scene scripts.** The intro (`BossGoma_Encounter`, 305 lines) and the
  death (`BossGoma_Defeated`, 271) are her own actions: a manual cutscene
  (`Cutscene_StartManual`), her own sub camera whose eye and at she sets every frame
  (`Play_SetCameraAtEye`), Player's modes set from her (`Player_SetCsActionWithHaltedActors`), and
  she writes the main camera's eye and at directly before handing back. No ported actor drove a
  camera this way yet.
- **Her draw** sets her env colour before every limb, the eye's (random while she's invincible:
  draw-time `Rand`) and the iris's apart, turns the eyelids and iris, scales the iris and tail
  limbs inside a push and pop, hides limbs, and puts one of two lists on segment 8 (back faces
  culled or not). Her post draw computes points, the colliders' spheres, her focus, the dead
  limbs' points, and spawns the pieces she breaks into.
- **Her decay** writes zeros into six of her object's textures (ADR 0051).
- **The boss's title card** is a texture in her object, 128 by 40 (over 0x1000 bytes): drawn in
  two blocks.
- **The fight wants the jump slash** (A while locked on with a sword): `Player_ActionHandler_10`
  rolled instead, the sword's branch not ported.
- **The slab** (`Door_Shutter`'s Gohma block, ported in 4a) shakes her sub camera, which a hook
  stood in for with the main one.
- **The exit's run** fights an enemy whose timers come from `Rand`.

## Decision

- **`Boss_Goma` is ported whole** (`oot_actors::boss_goma`), function by function, her intro and
  death included: `BossGoma_Encounter` with its states 0 to 3 (the zoom on Link, the slab at frame
  176, the turn at 190, the hand-back at 228, the wait to be seen) and 4 to 150 (the eye roll, the
  run, the drop, the landing, `TitleCard_InitBossName`, `NA_BGM_BOSS`,
  `EVENTCHKINF_BEGAN_GOHMA_BATTLE`); `BossGoma_Defeated` whole; every floor and ceiling action, the
  eye, the hit, the colours, the tail's swell, her eggs (`En_Goma` 0 to 2, her children).
  - **She drives her own sub camera** through the play state's helpers (`create_sub_camera`,
    `change_camera_status`, `camera_set_at_eye`, `play_return_to_main_cam`), and writes the main
    camera's `eye`, `eye_next` and `at` herself before handing back, as the C does.
    `SUB_CAM_ID_DONE` (0) is hers.
  - **Faithful bugs kept** (`@bug (game)`): the dust and fragments' limb index can be 0 (never
    written: the origin) and never the last limb; `decayingProgress` reaching 0x100, past the first
    table's end (ADR 0051); state 5's unreachable branch; `En_Goma`'s writes past
    `childrenGohmaState` (logged).
- **Her draw is one skeleton bake and two limbs apart, each in two variants:** `gGohmaSkel` with
  her env colour on a dynamic segment for every limb and her eye and iris left out; the eye's and
  the iris's lists alone, each with its own env colour, drawn at their bones; and each of the three
  with segment 8 culling back faces or not. Hidden limbs (broken off) have their bones zeroed;
  the tail limbs' and the iris's scale is multiplied onto their bones after posing, so their
  children don't take it. Her `OverrideLimbDraw`'s and `PostLimbDraw`'s effects on the game
  (the eye's three `Rand`, the points, the spheres, the pieces spawned) run in `draw_update`, once
  per game frame, walking her limbs in `SkelAnime_DrawOpa`'s order.
  - **Her limbs' lists** are a table (`LIMB_DLISTS`): limb `gGohma<X>Limb` draws `gGohma<X>DL`
    (checked against the skeleton in the object). The 20 that break off have a piece's bake each
    (`en_goma::boss_limb_bake`).
- **`TitleCard_InitBossName` is ported**, and `TitleCard_Draw`'s second block: a texture over 0x1000
  bytes is drawn as `0x1000 / width` rows, then the rest below from 0x1000 on. The boss names are
  sprite bakes of their object's texture, English (the first language), a block each
  (`title_card::boss_bakes`, `BOSS_NAMES`).
- **`Item_B_Heart` is ported whole**, its mesh the get-item draw's bake (`GID_HEART_CONTAINER`) in
  the opaque or translucent list.
- **The jump slash is pulled forward** into Player: `Player_ActionHandler_10` whole (its sword
  branch: `func_8083BA90(PLAYER_MWA_JUMPSLASH_START, 5, 5)`), `func_8083BBA0` (the slash from a
  jump, in `Player_Action_8084411C`), `func_8083BA90` and `Player_Action_80844AF4` (the slash in
  the air and its finish on landing).
- **The slab's quake** reads its parent's `subCamId` when its parent is Queen Gohma; a slab with no
  such parent (a test's) shakes the main camera, logged (the C would read through it).
- **`En_Goma`'s hook** is now the write into her `childrenGohmaState`.
- **The exit's run** (`Route::Gohma`) is reactive: each frame it acts on her action (the slingshot
  locked on and let go only while her eye is red and open; the sword out and jump slashes while
  she's stunned; on the ceiling a seed while she prepares her eggs, in first person when the lock
  is lost), so her `Rand` decides when, not whether. It stops with a failure if she lays eggs
  (the run doesn't fight her larvae). Debug starts: `deku-tree-gohma`, and
  `deku-tree-gohma-again` (her battle begun).
- **Logged:** the rumble (`Rumble_Override`); the circle shadow (no actor has one yet).

## Consequences

- The Deku Tree's boss is fought as in the game: her intro, the title card, the fight, her death
  with the pieces and the decay, the heart. The blue warp she spawns is a placeholder until 6b.
- An actor can drive a sub camera; the next bosses' cutscenes can follow this one.
- A Kokiri Sword's jump slash works everywhere (2 damage where a slash does 1). No golden pressed
  A while locked on with the sword out: none changed.
- The goldens gain `gohma` (the trace and its end), `gohma_title` and `gohma_decay`.
