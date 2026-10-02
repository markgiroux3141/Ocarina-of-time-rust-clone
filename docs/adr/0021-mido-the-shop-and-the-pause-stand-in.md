# 0021: Mido and the shop: the pause menu's stand-in in the play frame, `YREG(31)` in the message box, the shopkeeper's states whole with only the Kokiri shopkeeper's body, the shop items as a table of functions, Player's state flags written through its interface, and `EVENTCHKINF_04` in the presets

- **Status:** accepted, built in GAME-03 milestone 3; extends ADR 0017 (interface sprites), ADR 0018 (save presets), ADR 0019 (the pause menu's stand-in) and ADR 0020 (routes of steering tasks)
- **Date:** 2026-09-28

## Context

On a new save, Mido (`En_Md`) stands between Link and the Deku Tree until Link wears the Kokiri
Sword and the Deku Shield. The shield is bought in the Kokiri shop for 40 rupees.

**Wearing things needs the pause menu.** `Item_Give` only owns the sword and the shield. The
pause menu's equipment screen puts them on (ADR 0019), and the game binds a stand-in to Start.
Until now the game app called it itself, outside the play frame:
- a scripted run only gives the play state pad input, so it couldn't equip;
- the app's own checks guessed at what `KaleidoSetup_Update` checks.

`Play_Update` calls `KaleidoSetup_Update` before `Actor_UpdateAll`, and only while no message box
is up (`msgMode == MSGMODE_NONE`, `z_play.c:833`). It reads Start from the frame's input.

**The shopkeeper drives the message box.** `En_Ossan` sets the debug register `YREG(31)` while
Link shops. `z_message.c` checks it in five places: no do-action changes, no B to skip the
typing, no A at a wait-for-input code, and no A to close the box or answer a choice. The
shopkeeper reads the choices and the stick himself, and continues the box with each item's
texts. The message box port had `YREG(31)` fixed at 0.

**One overlay, eleven shopkeepers.** `En_Ossan` is every shop: the Kokiri shop, the potion
shops, the bazaar, the bombchu shop, the Zora and Goron shops, the Happy Mask Shop. They share
27 states (`sStateFunc`). Each has its own objects, skeleton, init and draw (`sInitFuncs`, the
`EnOssan_Draw*` functions).

**The shop's items are functions.** `En_GirlA`'s `sShopItemEntries` gives each of 50 items:
- its object, its get-item model and its price;
- its two texts;
- three functions the shopkeeper calls with the play state and the item: can it be bought, give
  it, and charge for it after a fanfare.

**Actors write Player.** The shopkeeper hides Link while he browses (`PLAYER_STATE2_29`, which
`Player_Draw` checks). He also turns Link round when shopping goes on. Mido sets
`PLAYER_STATE2_23` and `_25` in the Lost Woods. The port's actors had only written Player
through `PlayerIface`'s narrow setters.

**The presets skip the walk past Mido.** `deku-tree-open` stands for a save that has talked to
the Deku Tree. That save has been past Mido, who then stands aside for good (`EVENTCHKINF_04`).
The presets didn't set that flag. With Mido ported, he would block the playthrough, which never
talks to him.

## Decision

- **The pause menu's stand-in runs in the play frame, as `KaleidoSetup_Update`**
  (`PlayState::kaleido_setup_update`).
  - It runs where `Play_Update` calls it: before the actors, only with no message box.
  - It stops on the same conditions: a transition, or `Play_InCsMode`. The shooting gallery,
    magic filling and the bowling alley's switch never come up.
  - Start then runs `PlayState::pause_menu_equip` (ADR 0019's `equip_owned_unworn`, then
    `Player_SetEquipmentData`) at once, in place of opening the menu. L with C-Up (the debug
    menu, `BREG(0)`) does nothing.
  - `PlayerIface` gains `set_equipment_data`, so `oot_game` can run it without knowing Player.
  - The game app no longer binds Start itself: it passes the pad on. Scripted runs press Start
    like any button (`Task::Equip`).
- **`YREG(31)` is a message context field** (`MessageContext::yreg_31`), zeroed with the box as
  `Message_Init` does. The five checks read it. `En_Ossan` sets it to 1 in
  `EnOssan_SetStateStartShopping` and to 0 in `EnOssan_EndInteraction`, as the C does. It stays
  1 through a bought item's get-item text, whose last box the shopkeeper closes.
- **`En_Ossan`'s states are ported whole; only the Kokiri shopkeeper's body is.**
  - All 27 states and their helpers are ported: the milk, the egg, the bombs, the masks, the
    Hylian Shield's discount, and the Happy Mask Shop's payback and angry exit.
  - `EnOssan_Init`'s checks run for every type (Ingo for Talon as an adult, the Kakariko,
    bombchu and mask shops' conditions, the objects). Then:
    - the Kokiri shopkeeper (`OSSAN_TYPE_KOKIRI`) goes on to `EnOssan_InitActionFunc`;
    - any other type logs that it isn't ported and stands as a placeholder (ADR 0010).
  - The Kokiri shopkeeper's draw is a bake per eye (ADR 0012):
    - `object_km1`'s skeleton with `object_masterkokirihead`'s head list on limb 15;
    - the eyes on segment 0x0A;
    - `EnOssan_SetEnvColor`'s fixed colours as commands on segments 8 and 9.
  - The cursor and the stick prompts (`EnOssan_DrawCursor`, `EnOssan_DrawStickDirectionPrompts`)
    are sprites (ADR 0017). The shopkeeper's draw puts them in `overlay_2d`, from values in his
    render state.
- **`En_GirlA`'s table is a Rust table of functions** (`SHOP_ITEM_ENTRIES`), with the C's 50
  rows and all its `CanBuy`, `ItemGive` and `BuyEvent` functions.
  - When the shopkeeper calls an item's function, the item is taken out of the actor context and
    put back (`with_item`), as ADR 0007 does for updates. The C calls them with both the play
    state and the item.
  - The items' highlight setup (`func_8002EBCC`, `func_8002ED80`) is kept as data and not drawn,
    as for `En_Item00`'s rupees.
- **Actors write Player's `stateFlags2` through `PlayerIface::change_state_flags2`** (bits on,
  bits off). The shopkeeper's turn of Link (`shape.rot.y += 0x8000`) writes the base actor
  directly, as actors already do through the arena (ADR 0016). Player's draw checks
  `PLAYER_STATE2_29` (`Player_Draw`), through a render-state switch.
- **The presets set `EVENTCHKINF_04`** (`deku-tree-open`, `deku-tree-dead`), the flag Mido sets
  when he steps aside (`z_en_md.c:743`). He then starts at path 1's last point
  (`EnMd_SetMovedPos`). The Deku Tree's playthrough goes round him there, on the bank west of
  him.
- **The route to Mido collects only placed rupees** (`Route::MidoShop`). The C places 42 rupees'
  worth that a run can reach on foot (the three proximity drops on the plateau hang 45 above
  its floor, out of their 30's reach):
  - room 2's two blue rupees;
  - the plateau sign's hidden switch;
  - four green rupees;
  - Mido's house's chests;
  - the free multitag;
  - the shop's own proximity drop.

  No `Rand` drop is needed, so a change to who calls `Rand` can't break the route. The run:
  - presses Start twice: after the chest (text 0xA4 says to equip the sword), and after the shop;
  - stops its walks to read a text that opens by itself, as a player would. This is the forced
    `En_Wonder_Talk2` by the shop, which in the C holds Link in Player's cutscene mode 8 (not
    ported).

## Consequences

- **A new save reaches past Mido the game's way.** The run wears both pieces through the same
  code the game's Start runs.
- **The pause menu's stand-in now behaves like the menu's trigger:** nothing while a text is up
  (the old binding refused that too, as a guess). When the pause menu is ported, it replaces
  `pause_menu_equip` inside `kaleido_setup_update`, and nothing else changes.
- **The other shops are one init and one draw away.** Each shop's states, texts and items already
  run. Its objects, skeleton, animation, draw and bakes are what's missing.
- **The traces change** (golden/README.md):
  - Mido's fairy is a new placeholder;
  - Mido calls `Rand` every frame, so the Deku Tree playthrough's bushes draw other drops;
  - that run walks round Mido where he stands aside.
- **The pack grows by the new bakes only:**
  - Mido: 6, an eye and a pass each;
  - the Kokiri shopkeeper: 3;
  - the stone shelves: 2;
  - the shop's cursor and prompts: 3 sprites.

  51.9 MB, format 10.
