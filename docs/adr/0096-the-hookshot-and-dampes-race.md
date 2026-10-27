# 0096: The hookshot and Dampé's race

- **Status:** accepted, built in GAME-13 milestone 1 (2026-10-27)
- **Date:** 2026-10-27
- **Builds on:** [ADR 0091](0091-riding-epona.md) (Player's held-actor requests),
  [ADR 0095](0095-the-ranch-after-epona-and-the-fields-poes.md) (`epona-won`'s chain).

## Context

Phase 14's first milestone (GAME-13): the hookshot, which the story gets from Dampé's race in his
grave. The port had the hookshot's half that the bow and the slingshot share (the aim, the shot's
upper actions and animations, the targets' flags, `Obj_Hsblock`), but not the hook (`Arms_Hook`),
Player's flight to it, its camera (`Camera_Special5`), nor Dampé's ghost and his fire.

## Decision

1. **`Arms_Hook` whole**, in the main session with Player (one system): it reads and writes Player
   during its own update (`PlayState`'s Player borrowed by downcast), as the C does through
   `parent` and `child`. Its draw-time state (`unk_1E8`, the quad's edge, the hand's `unk_3C8`)
   runs in `draw_update`, as Player's post-limb positions do.
2. **Player's hookshot:** `Player_InitHookshotIA`'s spawn (a held-spawn request, as the arrows'),
   `Player_DestroyHookshot`, `Player_UpdateUpperBody`'s flight start, `Player_Action_HookshotFly`,
   `CAM_MODE_HOOKSHOT_FLY`, the right hand's post-limb part and the reticle
   (`Player_DrawHookshotReticle`, `BgCheck_AnyLineTest3`).
3. **`Camera_Special5`**, `SurfaceType_CanHookshot`, `SETUPDL_7`. The reticle draws in the XLU list
   with `SETUPDL_7`'s mode (no depth compare), where the C uses `OVERLAY_DISP` in the 3D view.
4. **`En_Po_Relay` and `En_Honotrap` whole** (a helper), `Item_DropCollectible2`.
5. **The runs:** `dampe-race` (`epona-won`: to Dampé, his greeting, the race on, the first door) and
   `hookshot` on the preset **`hookshot`** (`epona-won` with Dampé's chest opened and the hookshot on
   C-Left): Goron City's stone post aimed at, the pull, up onto it. The race is the hand test's.
6. **Pack format 37** for the hook's, the reticle's, Dampé's and the eyes' bakes.

## Consequences

- Adult Link can hookshot: targets, hookable enemies and items, moving platforms.
- `Actor_SwapHookshotAttachment` waits for its only user, `En_Fd` (the Fire Temple, Phase 15).
- Milestone 2 (the Lost Woods, the meadow, the Minuet) starts from `hookshot`.
