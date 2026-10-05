# 0032: Damage, death and the game over menu's stand-in

- **Status:** accepted, built in GAME-05 milestone 2 (2026-10-05)
- **Date:** 2026-10-05
- **Builds on:** [ADR 0020](0020-crawlspaces-paths-and-knockdown.md) (Player's knockdown, the
  body hit and the invincibility timer, pulled forward for the boulder) and
  [ADR 0021](0021-mido-the-shop-and-the-pause-stand-in.md) (the pause menu's equipping stand-in).

## Context

Phase 6's second milestone is damage and health: the rest of Player's hit responses, death and
game over, and the enemies' damage tables, with "the training dummy and a Deku Baba hit Link"
as its exit. Six things needed a decision:

- **The game over menu.** `GameOver_Update` (`z_game_over.c`) hands over to the pause menu
  (`ovl_kaleido_scope`, about 7,700 lines), whose game over states ask "Save?" and "Continue?"
  and respawn Link. The pause menu is milestone 5's, and only its item screen.
- **The Deku Baba.** It's milestone 3's, but the exit needs one to bite Link. Porting only the
  bite would be redone in milestone 3.
- **The red flash and the colour filter.** `Player_Draw` tints Link while he's invincible with
  `Gfx_SetFog2(255, 0, 0, 0, 0, far)`, and `Actor_Draw` tints a hit enemy through
  `Actor_SetColorFilter`'s fog. Both are a per-draw change of the fog's colour and range, which
  the renderer took only from the scene.
- **Who updates while Link is dead or talking.** `Actor_UpdateAll` freezes whole actor
  categories by Player's state (`sCategoryFreezeMasks`), and an enemy's finishing blow stops
  every actor for 4 frames (`freezeFlashTimer`). The port updated every actor every frame.
- **Random numbers.** Player's burning and shock call `Rand_ZeroOne`, the game's shared
  generator (`sRandInt`). Player is an overlay with no access to the play state's generator.
- **Something that hurts Link with each hit kind on demand.** The game's fire, ice and
  electric attackers are in later dungeons.

## Decision

- **The game over menu's states run, undrawn** (`oot_game::kaleido`). From
  `PAUSE_STATE_GAME_OVER_START` to `PAUSE_STATE_GAME_OVER_FINISH`, the states run as
  `KaleidoScopeCall_Update` and `KaleidoScope_Update` run them, with the C's timers, inputs and
  effects:
  - the pause background's prerender, stepped in the draw as `Play_Draw` steps it;
  - the message's 30 frames, the window's 40, the window turning in by `promptPitch`;
  - `KaleidoScope_UpdatePrompt`'s stick (±30) for the cursor;
  - the deaths counted.
  "Save?" with Yes saves the scene flags (`Play_SaveSceneFlags`, `savedSceneId`); writing the
  save to SRAM (`Sram_WriteSave`) waits for milestone 5. "Continue?" with Yes respawns as the C
  does: a boss room's entrance becomes its dungeon's, `Play_TriggerRespawn`, the fade to black
  (`interfaceCtx->unk_244`), `respawnFlag` -2, three hearts. "No" would go to the title screen,
  which isn't ported: it's logged and respawns too. Nothing of the menu is drawn (the message,
  the window, the prompts: `KaleidoScope_DrawGameOver` needs the pause menu's textures): the
  windowed game logs "Save?" and "Continue?" to the console, and the scene stays frozen as Link
  fell, under the game over lights and the fade.
- **`En_Dekubaba` is pulled forward, whole but its effects.** Every action, both sizes, its
  tables (`sDamageTableNormal`, `sDamageTableBig`, the child's jump attack rewritten), its
  drops and its floor shadow. The effects (`EffectSsHahen`, `func_8002829C`, `func_800286CC`,
  `EffectSsEnFire`) stay milestone 3's, and their `Rand` calls aren't made, so a run with a
  Deku Baba's effects will draw other numbers once they exist.
- **The engine takes a per-draw fog** (`eng_gfx::FogOverride` in `DrawParams`: a colour and
  the RDP's fog multiplier and offset, `gSPFogPosition`'s). A draw with one uses it in place of
  the scene's, in the material's uniform; game code computes it as `Gfx_SetFog` does
  (`oot_game::gbi::gfx_set_fog`). Player's flash and the colour filter (`Actor_Draw`'s
  `func_80026400`, applied to an actor's draws that have no fog of their own) both go through
  it. With no scene fog (the test course), the renderer's fallback fog takes the camera's clip
  planes for its range, so an override's factor stays the C's.
- **`Actor_UpdateAll`'s freezes are ported:** the category masks by Player's state
  (`PLAYER_STATE1_TALKING`, `_DEAD`, `_28`, `_29` and the rest), with the C's exemptions (the
  talk actor unless its text is a 0x6xx one, Navi, Player's children), `colorFilterTimer`
  counted down before each update, and `freezeFlashTimer` (`Enemy_StartFinishingBlow`) stopping
  the actors and the cutscenes, with its screen fill.
- **Player reaches the game's generator through `PlayIo`** (`PlayIo.rand`, taken from the play
  state and put back each frame, as `PlayIo`'s other fields are). The new ports draw from it.
  The idle fidget, ported earlier, keeps Player's own sequence: moving it would change every
  golden trace with a fidget, so it moves when a milestone re-records them.
- **The sandbox's training dummy grows colliders:** an AC cylinder that takes damage through
  the Deku Baba's normal table (it flashes red and is back to full at no health), and, as
  `DummyTarget::hurting` (`--target-hurts KIND` in `oot` and `oot_sandbox`), an AT and an OC
  cylinder that hurt Link with any `HIT_SPECIAL_EFFECT_*` and stand immovable. It stays a
  sandbox actor, not a game one.
- **Pack format 17** (`out/data14`): the ice block Link is frozen in, baked from
  `gameplay_keep` with its scrolling texture as a dynamic segment (`Player/ice`).

## Consequences

- Link can die: the game over runs to "Continue?" and back to the entrance, and a bottled fairy
  revives him (`func_80836448`, `func_80843AE8`, the fairy's states 20 to 24). The menu's draw
  and saving to SRAM wait for milestone 5's pause menu; "No" does what "Yes" does until the
  title screen exists.
- Any actor can be hit by a damage table and tinted: `CollisionCheck_ApplyDamage`,
  `Actor_ApplyDamage` and `Actor_SetColorFilter` are generic, and milestone 3's enemies use them
  as they are.
- The red flash changes the course's fall damage frames (`course_pit/sheet.png`); no trace
  changed, since the freezes only differ while Link talks, dies or is in a cutscene state the
  goldens' actors don't reach (golden/README.md).
- Milestone 3 has `En_Dekubaba` already; its effects and their `Rand` calls are the part left.
