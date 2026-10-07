# 0046: `En_Arrow`, the nut's stun and the screen's flash

- **Status:** accepted, built in GAME-05 milestone 5a (2026-10-07)
- **Date:** 2026-10-07
- **Builds on:** [ADR 0045](0045-the-fairy-slingshot-first-person-and-deku-nuts.md) (Player's side),
  [ADR 0033](0033-effects.md) (effects), [ADR 0037](0037-the-deku-trees-other-enemies.md)
  (draw-time state in `draw_update`) and [ADR 0040](0040-switches-torches-webs-and-the-map-data.md)
  (the eye switch, its seed injected until now).

## Context

The slingshot's seeds and the thrown nuts are one overlay, `En_Arrow` (513 lines), shared with the
adult's arrows. A nut's hit spawns `En_M_Fire1` (the stun) and flashes the screen through Play's
own fade (`transitionFadeFlash`, `R_TRANS_FADE_FLASH_ALPHA_STEP`); a seed's or nut's hit bursts with
`Effect_Ss_Stone1`. The flight tests the walls with `BgCheck_ProjectileLineTest`, and the collider
quad follows the projectile through `Player_UpdateWeaponInfo` (Player's own, until now). Three
things needed a decision:
- **The adult's arrows' parts** can't happen for child Link: their trail (`EffectBlure`,
  `z_eff_blure.c`, 1,056 lines, not ported) and the magic arrows' children (`Arrow_Fire`, `_Ice`,
  `_Light`, not ported).
- **The flash's register** (`R_TRANS_FADE_FLASH_ALPHA_STEP`, `iREG(50)`) is a debug register, so it
  outlives the play state; `Effect_Ss_Stone1` writes it too.
- **`Player_UpdateWeaponInfo`** for an actor's collider.

## Decision

- **`En_Arrow` is ported whole** (`en_arrow.rs`): every params value (`ARROW_CS_NUT` becoming a nut
  with `isCsNut`), the arrows' skeleton and animations with their LOD (`MREG(95)` from the play's
  registers), sticking into walls and actors and carrying one (`ACTOR_FLAG_CAN_ATTACH_TO_ARROW`,
  `_ATTACHED_TO_ARROW`), `func_809B4800` in `draw_update` (draw-time state), the seed's and nut's
  billboarded sparkle (`gEffSparklesDL`, its colours dynamic). The user chose: the trail's calls
  (`Effect_Add(EFFECT_BLURE2)`, `Effect_Delete`, `EffectBlure_AddVertex`) and the magic arrows'
  children keep their checks and log; the magic part of `Player_InBlockingCsMode` is left out
  with magic.
- **`En_M_Fire1`** (the nut's stun cylinder, `DMG_DEKU_NUT`, 200 across, while its timer runs from
  0.2 to 1.0) and **`Effect_Ss_Stone1`** (its 8 frames as bakes) are ported whole.
- **The flash:** `TransitionFade`'s flash type, Play's `transitionFadeFlash` set up in `Play_Init`
  (grey 160), updated where `Play_Update` updates it and drawn where `Play_Draw` draws it; the
  register lives on the play state and is carried across scene changes (it's a debug register,
  not part of the play state in the game).
- **`BgCheck_ProjectileLineTest`** in `eng_collision` (`IGNORE_PROJECTILES`) and `func_8002F9EC`
  (Jabu Jabu's walls) in `oot_game`.
- **`Player_UpdateWeaponInfo` for any actor** (`oot_game::collision_check::player_update_weapon_info`,
  `WeaponInfo` with the C's `posA`/`posB`). Player keeps its own equal copy for now (its fields
  are `tip`/`base`, read by many tests); folding it in is left for a quiet moment.

## Consequences

- Seeds fly 15 frames at 80 (gravity from the 8th), burst on walls with the reflect sound, and hit
  the eye switches and room 2's ladder for real; the tests no longer need the injection there
  (the injected tests stay as they were). Nuts fly whatever `unk_A73` says, burst on any wall or
  floor with the flash and `En_M_Fire1`, which stuns (room 5's Skulltula, for one).
- **The nut has no collider** (`EnArrow_Init` sets the quad up only to `ARROW_SEED`): it passes
  through actors and only bursts on walls and floors, as in the game.
- The adult's arrows' trail and the magic arrows wait for `EffectBlure` and their own milestone.
