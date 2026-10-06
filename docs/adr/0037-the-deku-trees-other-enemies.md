# 0037: The Deku Tree's other enemies

- **Status:** accepted, built in GAME-05 milestone 3b (2026-10-06)
- **Date:** 2026-10-06
- **Builds on:** [ADR 0007](0007-actor-ownership.md) (actors own their state, the arena, statics
  on the play state), [ADR 0033](0033-effects.md) (the effects) and
  [ADR 0034](0034-guard-battle-camera-and-the-first-enemies.md) (the guard, `En_Nutsball`, short
  scripted runs).

## Context

Milestone 3b is the rest of the Master Quest Deku Tree's enemies:
- the Skullwalltulas and Gold Skulltulas (`En_Sw`, 7);
- the Skulltulas (`En_St`, 2);
- the Deku Scrubs (`En_Dekunuts` 2, `En_Hintnuts` 3, `En_Shopnuts` 1);
- Gohma's eggs and larvae (`En_Goma`, 28, pulled forward from milestone 6).

Seven things needed a decision:
- **What the enemies turn into.** A killed Gold Skulltula leaves its token (`En_Si`), which needs
  the save's token count and Gold Skulltula flags. A caught Business Scrub (`En_Shopnuts`) becomes
  the salesman (`En_Dns`, 547 lines), who sells with a choice.
- **The effects they call** that weren't ported: `Effect_Ss_Fcircle` (a scrub set alight),
  `Effect_Ss_Blast` (the Skulltula's landing), `Effect_Ss_K_Fire` and `Effect_Ss_Sibuki` (a larva's
  death and its hits). `En_St` also adds a trail (`EffectBlure`) while it spins, which ADR 0033
  left unported.
- **`En_Goma`'s boss side.** Its overlay is also `Boss_Goma`'s: the eggs she lays (params 0 to 2,
  writing `childrenGohmaState` in her) and her pieces after death (params 100 and up, drawing a
  display list she gives). She's milestone 6's.
- **Draw-time `Rand` in an actor.** A hurt larva's body flashes random colours, three `Rand`
  calls in `EnGoma_OverrideLimbDraw`.
- **The rooms.** Room-to-room travel isn't solid until milestone 4, and every enemy but the
  Mad Scrubs' is in a room other than the entrance's, or several.
- **Per-limb colours** set in `OverrideLimbDraw` (the Skulltula's teeth, the larva's eyes and
  body), where a skeleton is one baked mesh.
- **The exit's run** against enemies driven by `Rand`, and the older runs that new `Rand` callers
  shift.

## Decision

- **Every enemy overlay is ported whole, and so is what it turns into.**
  - `En_Si` (the token) with the save's `gsFlags` (`GET_GS_FLAGS`, `SET_GS_FLAGS`), and the
    messages' token count (`MESSAGE_TOKENS`, which the House of Skulltula's texts show; the
    token's own text, 0xB4, has none).
  - `En_Dns` (the salesman), whole: it's small, and the choice, the offer and the payment use what
    the Kokiri shop already has.
  - Code the enemies call that wasn't there is ported where the C has it: `Actor_SetTextWithPrefix`
    (`oot_game::npc`), `CollisionCheck_GetSwordDamage`, `func_80028990` and its point
    (`func_80028894`), Player's `unk_860` and `play->damagePlayer`
    (`player::play_damage_player`), `Effect_Delete`.
- **The four effect overlays are ported whole, with their bakes** (`oot_game::effect::fcircle`,
  `blast`, `k_fire`, `sibuki`). `Effect_Ss_K_Fire` shares `Effect_Ss_En_Fire`'s bake (the same
  list and setup). The blast's draw lies on the floor below it, so an effect's draw reads the
  collision (`DrawCtx::col`). `CollisionCheck_WaterBurst` now spawns its bubbles.
- **`EffectBlure` stays unported** (ADR 0033): `En_St`'s calls go through `Effect_Add`, which
  finds no slot, and its trail's calls then do nothing, as the C's do with no slot free. The
  trail is the Skulltula's only visual it lacks; the same port would give Link's sword its trail.
- **`En_Goma`'s boss side is ported and waits for its caller.** The writes into `Boss_Goma` go
  through a marked hook that logs them (`en_goma::boss_goma_set_child_state`), and a boss piece's
  bake is made by `en_goma::boss_limb_bake` for the lists milestone 6 will list. The profile's id
  is `ACTOR_BOSS_GOMA`, as the C's is (`@bug (game)`): the port's `Actor_Spawn` gives an actor its
  profile's id, so every egg and larva has that id, as in the game.
- **An actor's draw-time `Rand` runs in `draw_update`,** once per game frame in `Actor_DrawAll`'s
  order, before the effects' draws (ADR 0033), and its result reaches the draw through the render
  state. The renderer's draws at the display rate make none.
- **Per-limb colours are dynamic segments in the skeleton's bake.** Where only the limbs after
  the colour's limb read it, the colour is the bake's prelude (the Skulltula's teeth). Where two
  limbs need different colours, the skeleton is baked without one limb and that limb's list is a
  bake of its own, drawn at its bone (the larva's body). The Business Scrub's spitting nose is the
  skeleton without its nose and the nose drawn scaled, as `EnShopnuts_PostLimbDraw` draws it.
- **Overlay statics are the play state's** (`PlayState::overlay_static`, ADR 0007's): the hint
  scrubs' `sPuzzleCounter`, the eggs' `sSpawnNum`.
- **An actor whose init waited for its object can change its category there.** The
  Skullwalltula's init moves it from `ACTORCAT_NPC` to `ACTORCAT_ENEMY`. When that init runs
  inside `Actor_UpdateAll` (its object came in after it spawned), the port changed the actor's
  `category` but not its list; now the list changes too, and the loop goes on in the new list,
  as the C's `actor = actor->next` does after `Actor_Init` (ADR 0034's walk).
- **Debug starts change room first.** `Route::debug_start` (and the tests, and the game's
  `--room`) request the start's room (`Room_RequestNewRoom`), run a frame for it to load, finish
  the change (`Room_FinishRoomChange`, which removes the old room's actors), then place Link. Each
  enemy is tested in its own room.
- **The exit's run is a Mad Scrub's** (`Route::Scrub`, room 4): the guard up until its own nut
  comes back off the Deku Shield and knocks it out, then the chase and the slash. It's short (170
  frames), and the scrub's only `Rand` before the knock-out is its wait in the flower, which Link
  ends at once from where he stands. The other enemies are tested by C-derived tests and by hand.
- **Pack format 19:** the new bakes (the four effects, the nose-less Business Scrub, the
  Skulltula with its teeth, the egg, the larva and its body, the Gold Skulltula's limbs).

## Consequences

- A Gold Skulltula's token is counted and flagged, and a killed one stays dead in the save.
- Room 0's new actors (two eggs, a Skullwalltula, a Gold Skulltula) call `Rand` from their first
  frame, and its enemies start updating a frame sooner (the category walk). 3a's combat route
  stalled and was re-tuned (its chase of the hovering Keese now goes on through a sidestep), and
  the Deku Tree's runs changed (golden/README.md). So did the Kokiri Forest runs: its Gold
  Skulltula (out only at night) was a placeholder, and now calls `Rand` too.
- Milestone 6's `Boss_Goma` replaces the hook with its own field, and lists its pieces' bakes.
- `EffectBlure` remains the effects' gap: the Skulltula's spin trail and the sword's.
