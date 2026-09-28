# 0018: The Deku Tree's mouth: save flags until there are cutscenes, its alpha as a dynamic colour, and debug save presets

- **Status:** accepted, built in GAME-02 milestone 4; extends ADR 0012 (actor bakes) and ADR 0017 (colours on segment 0x0B)
- **Date:** 2026-09-28

## Context

`Bg_Treemouth` (`z_bg_treemouth.c`) is the Deku Tree's mouth: a DynaPoly actor
(`gDekuTreeMouthCol`) whose place follows `unk_168`, from closed (0: raised, blocking the way
in) to open (1: lowered 399, its top bridging a gap in the tree's floor).

**The story moves it through cutscenes.** `func_808BC8B8` starts one when Link first comes near
(`D_808BCE20`, setting `EVENTCHKINF_0C`), or when he Z-targets the tree afterwards
(`D_808BD2A0`). `func_808BC9EC` waits for the cutscene to start and reads the answer to the
tree's question. Yes sets `EVENTCHKINF_05` and plays `D_808BD520`, whose cue 3 opens the mouth;
no plays `D_808BD790`. From then on, `EVENTCHKINF_05` holds the mouth open. There's no
cutscene system until GAME-03 milestone 4.

**The roadmap had the flag wrong.** It said the open mouth comes from `EVENTCHKINF_07`. In the
C, `EVENTCHKINF_07` is set by `Door_Warp1`'s blue warp out of Gohma's room, with
`EVENTCHKINF_09` and the Kokiri Emerald: the tree is dead. It only changes the draw.
`BgTreemouth_Draw` sets `gDPSetEnvColor(128, 128, 128, alpha * 0.1f)` before
`gDekuTreeMouthDL`, with `alpha` 500, or 2150 with `EVENTCHKINF_07`, or
`roomCtx.unk_74[0] + 500` on scene layer 6 (the tree's death). The list's combiner is
`(TEXEL1 - TEXEL0) * ENV_ALPHA + TEXEL0`, a blend from the living bark to the dead.
`Scene_DrawConfigSpot04` does the same for the tree through segment 0x0B.

**The playthrough needs the mouth open,** and a new save can't open it without the cutscenes.

**Room changes delete and respawn it.** It lives in room 1, so going back to room 0 runs
`BgTreemouth_Destroy` (`DynaPoly_DeleteBgActor`), which `eng_collision::dyna` didn't have.

## Decision

- **The env alpha is a dynamic colour.** The mouth is a `MeshBake` whose prelude is segment
  0x0B as `DynamicColor { env }`, then `gDekuTreeMouthDL`. The draw passes
  `(128, 128, 128, alpha * 0.1)` for that segment.
  - The list uses only segment 6, so 0x0B is free. It's the segment the scene draw config and
    the sprite bakes already use for the same kind of colour.
  - Two bakes (500 and 2150) would cover the flag, but not layer 6's alpha, which changes every
    frame.
- **The flag logic is ported; the cutscene triggers are logged.**
  - Where the C sets `play->csCtx.segment` and `gSaveContext.cutsceneTrigger`, the port logs
    the script's name as not ported and does everything else: the flags, the targetable flag,
    the state change.
  - `PlayState::cs_ctx` (`oot_game::cutscene`) has the `CutsceneContext` fields actors read
    (the state, the frame counter, the actors' cues). It stays `CS_STATE_IDLE`, so after a
    trigger the mouth waits in `func_808BC9EC`, as it would for a cutscene that never starts.
  - The cutscene-driven states (`func_808BC9EC`, `func_808BCAF0`, `func_808BC65C`,
    `func_808BC80C`, `func_808BC864`, `func_808BC6F8`) are ported against it. A test drives
    it by hand: yes on the question sets `EVENTCHKINF_05`, and cue 3 opens the mouth by 0.01 a
    frame.
- **Debug save presets stand in for the missing events** (`oot_game::save::SAVE_PRESETS`,
  `--preset` in the game and the sandbox; they need an entrance, since they set the save that
  `Play_Init` enters with). They're not in the game.
  - `deku-tree-open`: `EVENTCHKINF_0C` and `EVENTCHKINF_05`, as after the tree's talk. This is
    the playthrough's.
  - `deku-tree-dead`: those and `Door_Warp1`'s `EVENTCHKINF_07`, `EVENTCHKINF_09` and the Kokiri
    Emerald. It shows the dead colours, for comparing with a finished save in an emulator.

  The scene draw config now reads `EVENTCHKINF_07` from the save each frame, as
  `Scene_DrawConfigSpot04` does. Only `Play_Init` saves can set it, so the goldens don't change.
- **DynaPoly deletion follows the C's slot flags.** `BGACTOR_IN_USE` and `BGACTOR_1`:
  - `delete_bg_actor` marks the slot, and it keeps colliding until the next
    `DynaPoly_UpdateContext` frees it;
  - `set_bg_actor` takes the first free slot, or fails with `BG_ACTOR_MAX` when all 50 are
    taken.
- **The playthrough is a steering script** (`oot_actors::playthrough`), shared by the test and
  the sandbox's trace.
  - Its waypoints are picked from Kokiri Forest's collision.
  - It reads the play state each frame to decide the next input, as a player would. It isn't
    a recorded input list, so a change to one step's timing doesn't derail the later ones.
  - It reports each step on the state that step finished on, before the next frame's input.

## Consequences

- **A new save can't get in.** Link near the tree sets `EVENTCHKINF_0C`, and the mouth waits
  for a cutscene that never comes. That's the C's behaviour without `z_demo.c`, and GAME-03
  milestone 4 is what fixes it. Until then the preset is the way in.
- **The preset replaces the story for now.** Phase 4's exit (a new save to the Deku Tree, the
  game's way) replaces the preset in the playthrough.
- **The playthrough's drop depends on `Rand`.** It's deterministic, but a change to who draws
  random numbers, and when, changes which bush drops. With four bushes, about one such change in
  five leaves none dropping. The run then stops, says so, and the bush list needs changing. The
  golden trace changes with any such change anyway.
- **Placeholders don't block.** Mido (`En_Md`) stands in the path; the real one blocks it until
  Link has the sword and shield (GAME-03 milestone 3). The test checks that Link walks right
  past both placeholders, so porting either one will show up here.
