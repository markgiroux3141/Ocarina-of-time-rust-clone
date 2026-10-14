# 0069: The Lost Woods: the Skull Kids, the ocarina's memory game, the target scrub and its prize

- **Status:** accepted, built in GAME-08 milestone 2 (2026-10-14)
- **Date:** 2026-10-14
- **Builds on:** [ADR 0061](0061-the-ocarina.md) (the ocarina; the memory game left out then),
  [ADR 0066](0066-the-actor-schema.md) (each actor's schema), [ADR 0067](0067-the-meadow-saria-and-the-wolfos.md)
  (the meadow, the helpers' worktrees).

## Context

Milestone 2 of Phase 9 (`docs/GAME-08-woods-meadow-and-ranch.md`): the Lost Woods' Skull Kids and
scrubs. The survey, checked again against the pack (`ootx scene-info --scene spot10`):

- Room 9: `En_Skj` 0x03FF (type 0, the Skull Kid on his stump) and 0x17FF (type 5, the small
  stump Link stands on). Room 1: 0x07FF and 0x0BFF (types 1 and 2, the memory game's Skull Kids)
  and 0x1BFF (type 6, the stump that runs the game), and `En_Dnt_Nomal` 0xFFFF (the target scrub).
  The adult layers place 0xFFFF Skull Kids (type 63), killed in init without the Poacher's Saw.
- The memory game was left out of the ocarina (ADR 0061, BACKLOG #47): `AudioOcarina_MemoryGameInit`
  and `_MemoryGameNextNote` were logging stubs, and of the message box's `MSGMODE_MEMORY_GAME_*`
  only the start was defined. `Message_StartOcarina`'s branches for it were there.
- The target scrub needs `En_Ex_Item` (the prize), `Effect_Ss_Extra` (the score) and one-point
  cutscene 4140, none ported.
- `En_Okuta` (room 3) never spawns: `object_okuta` is in none of the woods' rooms' object lists,
  and `Actor_Spawn` refuses an actor without its object (the port does the same).

## Decision

1. **The shared code first, here:** the memory game's audio (`sOcaMemoryGameAppendPos`,
   `sOcaMemoryGameEndPos` in `Ocarina`; a note from `AudioThread_NextRandom`, with the C's
   @bug: the first note compares with the song table's entry before the memory game's, the
   scarecrow's last); the message box's eight `MSGMODE_MEMORY_GAME_*` modes in `Message_DrawMain`
   (the metronome, the notes taken from the playback and playing staffs, the success and the
   failure) and `Message_UpdateOcarinaMemoryGame` (`PlayState::message_update_ocarina_memory_game`,
   for the stump); one-point cutscene 4140; `Effect_Ss_Extra` (`oot_game::effect::extra`,
   `EFFECT_SS_EXTRA` 0x1E) and `interfaceCtx.unk_23C`; `SETUPDL_41`.
2. **The actors whole, by two helpers in their own worktrees:** `En_Skj` and `En_Skjneedle`
   (every type: the stump Skull Kid's flight, Saria's Song and its heart piece, his texts and the
   Skull Mask trade, the fight with needles, the memory game's two Skull Kids and its stump);
   `En_Dnt_Nomal` (the target scrub, and the mask contest's audience in the Deku theatre grotto,
   whose director `En_Dnt_Demo` is Phase 20's: the fields it writes are public) and `En_Ex_Item`
   (every prize: the bowling alley's, the treasure chest game's, the Great Fairy's magic, the
   seed bag). Their overlay statics (`sSmallStumpSkullKid`, `sOcarinaMinigameSkullKids`,
   `D_80B01EA0`) live on the play state; an init that doesn't know its own handle yet leaves it
   to the first update.
3. **Masks:** none are ported (BACKLOG #39), so `Player_GetMask` is `PLAYER_MASK_NONE` and the
   Skull Mask trade is reached only in a test that sets `currentMask`; `Player_UnsetMask` logs
   and clears it.
4. **A preset for milestone 1's end, `meadow-saria`:** `graveyard-sun` and what milestone 1's
   run changes (found by diffing the save over a run of `Route::Meadow`): Saria's Song, the woods'
   owl's switch flag 7 and the meadow's gate's 0x1F; and the Fairy Slingshot with 30 seeds on
   C-Right, which the castle's presets leave out though a player has it by then.
5. **The exit's run (`Route::LostWoods`, from `ENTR_LOST_WOODS_0`):** west into room 9 and onto
   the small stump, Saria's Song (the heart piece); east into room 1, the slingshot from the
   clearing's edge at the target's centre (`Task::ShootUntil`: three bullseyes in a row), the
   scrub's camera, text and seed bag; down into the hollow by the clearing's north end (a wall 25
   high lines the rest of its edge) and onto the memory game's stump, the three rounds played back
   (`Task::MemoryGame`: the notes read from `OCARINA_SONG_MEMORY_GAME`'s table as the Skull Kids
   played them; the ocarina taken out only after Link has stood 20 frames, since a round's prize
   is offered the frame after its text).
6. **The coverage table's correction:** `En_Okuta` leaves Phase 9 for Phase 11 (Zora's River,
   where it's first met).

## Consequences

- BACKLOG #47 keeps the scarecrows' and the frogs' parts (Phase 11).
- The target scrub updates everywhere (`ACTOR_FLAG_UPDATE_CULLING_DISABLED`) and blinks by
  `Rand_ZeroFloat` from the scene's start, which moves the game's random numbers for anything
  later in the Lost Woods: the `meadow` goldens changed (the owl's camera), proven by a build
  without the scrub.
- `Effect_Ss_Extra`, `En_Ex_Item` and the stage scrubs are ready for the bowling alley, the
  treasure chest game, the Great Fairies and the Deku theatre (Phase 20).
