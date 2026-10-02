# 0023: Navi and the opening: every scene's cutscene layers in the pack, their unnamed scripts keyed by offset; `Play_Init` on a cutscene layer with the cutscene transitions; `En_Elf` ported whole but its effects; Navi's C-Up texts as the ROM's bytes; the actors' point lights drawn; a new file as the file select starts it

- **Status:** accepted, built in GAME-03 milestone 5; extends ADR 0008 (the pack), ADR 0012 (actor bakes), ADR 0019 (saves) and ADR 0022 (cutscenes)
- **Date:** 2026-09-29

## Context

**A new file starts with cutscenes.** The file select's `Sram_InitSave` (`z_sram.c:696`) enters
`ENTR_LINKS_HOUSE_0` as a child at 10:00 with `cutsceneIndex` 0xFFF1. `Play_Init` then loads the
scene's cutscene layer `SCENE_LAYER_CUTSCENE_FIRST + (cutsceneIndex & 0xF)`: Link's house's
layer 5. Traced from the headers and the terminators (`CutsceneCmd_Destination`), the
opening is a chain of four layers:

| Layer | Script | What it shows | Its terminator |
|---|---|---|---|
| Link's house 5 (0xFFF1) | 0x15D0 | The Deku Tree's narration (0x109D to 0x109F) over Link asleep (cues 0x1C, 0x1D) | 35 at 280: `ENTR_HYRULE_FIELD_0`, 0xFFF0, `FADE_BLACK_FAST` |
| Hyrule Field 4 (0xFFF0) | 0x12400 | The nightmare: Link at the drawbridge in the storm (cues 5, 1, 6), Zelda's escape (`En_Viewer`s) | 11 at 540: `ENTR_KOKIRI_FOREST_0`, 0xFFF3, `FADE_WHITE` |
| Kokiri Forest 7 (0xFFF3) | 0xA6D0 | The Deku Tree sends Navi (0x1099, 0x109A, her cues in `npcActions[8]`), then her flight through the village (the camera's 64-point list) | 10 at 940: `ENTR_LINKS_HOUSE_0`, 0xFFF0, `FADE_BLACK` |
| Link's house 4 (0xFFF0) | 0x1040 | Navi wakes Link (0x1095, 0x1096, 0x1000, 0x1098; cues 0x1C to 0x1F, then 5) | none: `CS_MISC` 12 at 647 ends it |

So the wake-up is layer 4, not the new file's layer 5.

**What that needs.** The pack held scene layers 0 to 3 only, and a layer's script was the key of
the XML `<Cutscene>` symbol at its header's offset: no XML names these four. The entrances use
two transitions the port ended at once, `TRANS_TYPE_CS_BLACK_FILL` (`ENTR_LINKS_HOUSE_0_5`) and
`TRANS_TYPE_FADE_WHITE_CS_DELAYED` (`ENTR_KOKIRI_FOREST_0_7`), both driven by the script's
`cutsceneTransitionControl`. Player's cutscene modes 9 and 38 to 41 weren't ported.

**Navi was a placeholder** everywhere `En_Elf` spawns (Player, the Kokiri, Mido, the
shopkeeper, the item drops), and with her the target context's `naviRefPos`, Player's
`naviTextId` and C-Up, and her cues in the Deku Tree's talk. Her C-Up text comes from a
per-scene message script (`play->naviQuestHints`, `sNaviQuestHintFiles`, picked by
`SCENE_CMD_ID_SPECIAL_FILES`) that `z_elf_message.c` interprets. She carries two point lights
(`z_lights.c`), and the renderer had only the frame's ambient and two directional lights.

## Decision

- **Every scene's cutscene layers are in the pack.** `SceneData.layers` holds the four game
  layers, then every layer up to the last one the scene's alternate header list names (the
  list's length is bounded like the other length-less lists: up to the next thing pointed at,
  while its entries are `NULL` or headers). That's 106 layers in 30 scenes: 106 more headers and
  199 more room records. A `NULL` entry falls back as `Scene_CommandAlternateHeaderList` does; a
  layer past the list (where the C reads whatever follows it) loads the main header, logged.
  - Measured: 59.6 MB (52.0 before), 12.9 s. Room meshes are content-deduplicated, so most of
    the growth is the layers' own actor lists and headers. Importing only the opening's four
    would save little, and the terminators already reach dozens of the others.
  - A cutscene layer's rooms are baked as a child's by day at 10:00, the new file's; the layer
    number reaches the draw configs that read it (`Scene_DrawConfigKokiriForest`'s layers 4 and 6).
- **A layer's script without an XML name is keyed by its file and offset**
  (`keys::cutscene_at`: `cutscene/spot04_scene/0xA6D0`; ADR 0031's decomp names them all, that one
  `gKokiriForestIntroNaviFlyingCs`), found by walking every layer's header
  and the script to its `CS_END_OF_SCRIPT` (83 of them). Named ones keep their symbol's key. They join
  `CutsceneTables.scripts` as `<file>/0x<offset>`.
- **`Play_Init` loads the cutscene layer,** with the special cases (Hyrule Field, Kokiri Forest)
  only outside them, and `Environment_Init`'s `cutsceneTransitionControl = 0`.
  `TRANS_MODE_CS_BLACK_FILL` holds a black fill at the control's value until 100 or less;
  `TRANS_MODE_INSTANCE_WAIT` holds `FADE_WHITE_CS_DELAYED`'s fade, drawn, until the control is
  set. `linkAgeOnLoad` isn't needed by the chain and stays logged.
- **`En_Elf` is ported whole** (`oot_actors::en_elf`): Navi's modes (`func_80A0461C`,
  `func_80A03CF8`: out, into the hat, back out, at the target, at the camera in first person and
  shops, her cues), her colours (`func_80A04414`), her talk (`func_80A053F0` and the five talk
  updates, Saria's texts included), `naviTimer`, the Kokiri fairies, the healing and revival
  fairies and the spawner. Not ported: the sparkles (`EffectSsKiraKira`: the spawn's `Rand` calls
  are made, the effect's own per-frame ones aren't), the sounds, `Environment_AdjustLights`
  while she talks (the environment has no per-frame adjustments), `Elf_Msg`.
  - Drawn from two bakes of `gFairySkel` (Navi's without the z-buffer, `fairyFlags & 4`), with
    SETUPDL_27 written out, segment 8's prim colour and render mode as a dynamic list, and
    segment 1 (the billboard `gGlowCircleSmallDL` multiplies in) as the identity, applied at draw
    time to limb 8, which `EnElf_OverrideLimbDraw` puts at its parent's origin, unrotated, at its
    pulsing scale.
- **The target context has Navi's half of `Attention_Update`:** `naviRefPos` eased over four frames
  (`naviMoveProgressFactor`), `activeCategory`, `Attention_SetNaviState`'s colours, `Attention_Init` after Player
  spawns.
- **Navi's C-Up texts are the ROM's bytes** (`table/elf_messages`): `elf_message_field` and
  `elf_message_ydan` whole, and `code`'s `sChildSariaQuestHints` and `sAdultSariaQuestHints`. The importer
  builds each from its `QUEST_HINT_*` macros (`quest_hint_commands.h`'s packing, the constants read from
  the headers) and checks them against the ROM; `oot_game::elf_message` is `z_elf_message.c`.
  `LayerData.c_up_elf_msg_num` is the special-files command's number.
- **Player:** `Player_SpawnFairy` (`naviActor`, only in `GAMEMODE_NORMAL` and the credits),
  `naviTextId` (cleared at the end of each update), the Navi branch of `Player_ActionHandler_Talk` and
  `Player_StartTalking` (her talk request, `Player_SetTurnAroundCamera(play, 0xB)`), `Interface_SetNaviCall`; the
  opening's modes 9 and 38 to 41 through the typed handlers they use (`Player_AnimChangeOnceMorphZeroRootYawSpeed`,
  `Player_AnimReplacePlayOnce`, `Player_AnimReplacePlayLoop`), and the shadow they switch off and on; init modes 5 and 6 as
  13 in a cutscene layer. Player's `actor.focus.pos` is now the head, as its draw sets it.
- **Point lights are drawn.** `oot_game::lights` is `LightContext`'s list and `Lights_BindPoint`.
  `PlayState::draw` binds the list at each actor's position (`Actor_Draw`'s `Lights_BindAll`, none
  with `ACTOR_FLAG_IGNORE_POINT_LIGHTS`) into its draws' `DrawParams::lights`; the renderer adds up to three to
  the lit materials' directional lighting. The rooms get none, as in the C. The glow halo
  (`Lights_GlowCheck`, `Lights_DrawGlow`) isn't drawn.
- **An init's children get their parent.** `Actor_SpawnAsChild` from an actor's init (Mido's
  and the shopkeeper's fairies) linked the child to whatever actor was updating; the child is now
  linked to the parent once the parent is in the actor context.
- **A new file as the file select starts it:** `SaveContext::file_select_new` (`Sram_InitSave`
  for file 2, then `FileSelect_LoadGame`), `--new-file` in the game and the sandbox.
- **The new routes don't move the old ones.** `Route::NewFileDekuTree` plays the opening, then
  the new save's run from where the wake-up leaves Link, with C-Up to Navi on the plateau. The
  existing routes still start at `ENTR_LINKS_HOUSE_0` with `cutsceneIndex` 0, so their traces keep
  their frames.

## Consequences

- **A new file plays the game's opening** from its first frame, then Navi is Link's: she follows,
  flies to targets in their colours, hides in his cap, and calls with her text after 600 frames
  in one scene (her init resets `naviTimer` below 3000 on every scene change); C-Up talks to her.
  Phase 4's exit run takes 11748 frames from the new file's first frame into the Deku Tree.
- **Every route's trace changed a little:** Navi stays across room changes (no room, where her
  placeholder was room 0's), and the fairies' `Rand` calls change the random bush drops.
- **The Hyrule Field nightmare shows only Link:** its `En_Viewer`s (Zelda, Impa, Ganondorf and
  their horses) and the drawbridge are placeholders, and the rain and lightning aren't ported.
- **The effects are the next gap Navi shows:** her sparkles, and the `Rand` calls the effects
  would make every frame.
- **The pack is 59.6 MB,** format 12.
