# 0084: Barinade, Ruto's warp and the Zora's Sapphire

- **Status:** accepted, built in GAME-10 milestone 6 (2026-10-21); its hand test owed ([HAND-TESTS-OWED.md](../HAND-TESTS-OWED.md))
- **Date:** 2026-10-21
- **Builds on:** [ADR 0075](0075-king-dodongo-the-room-texture-the-cameras-up-and-the-ruby.md) (King
  Dodongo: the boss pattern, the camera's up, the screen fill), [ADR 0050](0050-queen-gohma.md) (a
  boss's cutscenes on its own sub camera), [ADR 0083](0083-jabu-jabus-belly-to-barinade.md) (Ruto
  whole, `Door_Warp1`'s `rutoWarpState`), [ADR 0066](0066-the-actor-schema.md) (the schemas),
  [ADR 0079](0079-the-level-texts.md) (the texts).

## Context

Phase 11's sixth milestone (`docs/GAME-10-zoras-waters-and-jabu-jabu.md`), the exit's first part:
Barinade (`Boss_Va`, 4,011 lines, the largest overlay yet: one actor typed by params for the body,
three supports, three zappers, three stumps, ten Bari and the door), `Door_Warp1`'s Ruto warp (its
four states, about 84 lines), and the fountain's `gZorasFountainSapphireCs`. Things to settle:

- `Boss_Va`'s statics (`sEffects[400]`, `sCsState`, `sFightPhase`, the sub camera's vectors, ...)
  are shared by every instance of the overlay.
- He writes `envCtx.screenFillColor` while `envCtx.fillScreen` is off (his intro, the begun
  battle's init), and the port had only the fill that's on (`TransitionState::screen_fill`).
- His death spawns Ruto at one of three `sWarpPos`; the survey called it the nearest to Link.
- The warp's talk uses one-point 9705, not ported.
- The fountain's spawn 0 (`ENTR_ZORAS_FOUNTAIN_0`, where the cutscene's terminator 13 leads) is in
  the water.

## Decision

1. **`Boss_Va` whole**, by a helper, in `boss_va.rs` with its particle system in
   `boss_va_effects.rs` (every effect type: sparks, spark balls, the zappers' charge, blood
   droplets and splatter, tumours, gore with its floor raycast; their draws). Every params type
   and state, the damage tables (the supports and the body take only `DMG_BOOMERANG`; stunned, the
   sword, boomerang and stick), the intro and death on his own sub camera, the title card
   (`object_bv` 0x1230: in this ROM's `object_bv_pal.xml` and the pack an IA8 128 by 120 texture,
   the English block 128 by 40 drawn; the other XMLs say 128 by 80), the boss and clear music, the
   cleared room's init (Ruto, or with `EVENTCHKINF_37` a plain `Door_Warp1` 0, and the heart), the
   begun battle's init (the Bari called at once). The overlay's statics are
   `PlayState::overlay_statics[ACTOR_BOSS_VA]`, dropped with its last instance (as `En_Insect`'s).
   His body is three skeleton bakes (limb 7; the rest with limb 20's scroll and glow; the
   translucent limbs 29 to 55).
2. **`EnvCtx::screen_fill_color`** holds `screenFillColor` whatever `fillScreen` is; `boss_va.rs`'s
   `// shared:` helpers keep it and the transition's fill in step. Barinade's light and fog
   overrides (`BossVa_SetSparkEnv`, `_SetDeathEnv`) write the existing `adj*` fields.
3. **Ruto's spot is the C's, not the nearest.** `BossVa_BodyDeath`'s loop (`:1674-1679`) takes warp
   `i - 1` whenever `i` is nearer Link than `i - 1`, from 2 down: Link at warp 1 gets Ruto at warp
   0, at warp 0 warp 2 (or 1). Ported as written.
4. **`Door_Warp1`'s Ruto warp whole** (`DoorWarp1_RutoWarpIdle`, `func_80999EE0`,
   `func_80999FE4`, `DoorWarp1_RutoWarpOut`): once Ruto has set its state and Link stands in it,
   `ENTERED` and his walk; when she folds her arms (state 3) a sub camera at eye height (fov 90),
   `TALKING` and text 0x4022; the text closed: one-point 9705 (added: `D_8012395C`'s three
   keyframes, already in the pack with every `z_onepointdemo.c` table), the sub camera copied into
   it, `WARPING`; Link floats up (gravity 0.02 from frame 61) and 100 frames on
   `EVENTCHKINF_37`, `Item_Give(ITEM_ZORA_SAPPHIRE)`, `ENTR_ZORAS_FOUNTAIN_0` with `CS_INDEX_0`, the
   slow white fade (the next transition type left alone, unlike the child warp's). The plain
   warp's later visits (params 0 in `SCENE_JABU_JABU_BOSS`) were already ported.
5. **The fountain's cutscene plays through with what's ported:** layer 4's `En_Ru1` type 1,
   `Demo_Effect`'s jewel and light, `En_Jj`; texts 0x4025 (a choice), 0x4026, 0x4027, 0x0082 and,
   the Ruby held, 0x4035, 0x4028; Ruto's cues 2 to 9 (gone on 8, her swim away done); the
   terminator (13) at frame 1110 to `ENTR_ZORAS_FOUNTAIN_0`, layer 0. Its spawn 0 is in the water
   by Jabu-Jabu (-1480, -10, 1755): Link treads water there.
6. **Two runs, no fights:** `Route::Barinade` (from `jabu-barinade`: Link falls in, the intro takes
   him at once, to `EVENTCHKINF_BEGAN_BARINADE_BATTLE` with the camera back: frame 555) and
   `Route::Sapphire` (from `barinade-beaten`, a debug start: the room's clear flag, as his death
   leaves it: the heart, Ruto's warp, the cutscene, the fountain: frame 2812). New presets
   `barinade-fight` (the run's end, and for the fight's starts the boomerang on C-Down in place of
   the bombs), `barinade-beaten` and `zora-sapphire` (the run's end: the three spiritual stones,
   milestone 7's start), each end checked against its preset.

## Consequences

- The Zora's Sapphire is reached: Phase 11's exit, part 1. The three spiritual stones are held
  from `zora-sapphire` on.
- Approximations, logged in the module: the fog swaps round a stunned Bari's shadow
  (`func_80026A6C`, `func_80026860`) aren't ported; a tumour's sound plays at the default position
  (the C passes a hidden effect's position); the blast sparks reuse the large spark's bake (their
  `Gfx_SetupDL_25Xlu2` ignored); the body's translucent pass's segment 8 and environment colour
  are approximated; an effect whose parent was freed keeps the parent's last state (the C reads
  freed memory); when the begun battle's init holds Link, his cutscene actor is none.
- The game's bugs kept (`@bug (game)`): the Bari's flex skeleton set up as a normal one; an s8
  `invincibilityTimer` compared with 160 (never true); `effectPos[10]` and `[11]` read past the
  array (the struct's next fields, kept as entries 10 and 11); the gore's scale, the tumour's × 1.2
  cast to s16, is 0 for every piece.
