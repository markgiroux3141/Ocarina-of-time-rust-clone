# 0040: Switches, torches, webs, Navi's tags and the map's data

- **Status:** accepted, built in GAME-05 milestone 4a (2026-10-06)
- **Date:** 2026-10-06
- **Builds on:** [ADR 0007](0007-actor-ownership.md) (overlay statics on the play state),
  [ADR 0008](0008-asset-pack.md) (the pack), [ADR 0012](0012-actor-bakes.md) (bakes),
  [ADR 0023](0023-navi-and-the-opening.md) (Navi) and
  [ADR 0038](0038-sliding-doors-and-room-travel.md) (the interact flags).

## Context

Milestone 4a's other actors: `Obj_Switch` (7 placed), `Obj_Syokudai` (14), `Bg_Ydan_Sp` (8 webs),
`Elf_Msg` and `Elf_Msg2` (8 and 4), and the map and compass's pause data. The user chose to
port each actor whole even where Link can't yet use what triggers it (the eye switches take
seeds; a burning Deku Stick lights the torches and burns most webs; both come later), with the
trigger injected in the tests; and to keep the map and compass as data, drawn by milestone 5's
pause menu.

Five things needed a decision:
- **A collision header an actor rewrites.** `BgYdanSp_UpdateFloorWebCollision` writes the y of
  eight vertices of the object's collision header every frame, in place, so every floor web in
  the scene shares the writes; the engine's bg actors hold `Arc<CollisionHeader>`.
- **Textures swapped on a segment per frame:** the eye switch's eye (four textures) and the
  crystal's colour.
- **Overlay statics:** the torches' `sLitTorchCount`, the floor webs' shared header.
- **The map's tables:** `z_map_data.c`'s `gMapDataTable` and `z_map_mark.c`'s marks, and where
  `z_map_exp.c`'s state lives.
- **Torches' `torchType`**, compared unshifted.

## Decision

- **Every actor is ported whole**, its untriggerable paths included: the eye switch's seed, the
  torches' and webs' burning stick (`Player_IsBurningStickInRange` is a Player method), fire
  arrows. The tests inject those hits. `Obj_Switch`'s frozen kind spawns `Obj_Ice_Poly`, which
  stays a placeholder (so a frozen switch isn't held by its ice).
- **A rewritten collision header is shared state on the play state.** `Bg_Ydan_Sp`'s overlay
  static holds the object's header; a write makes a new `Arc` with the vertices changed and
  `Dyna::replace_shared_header` swaps it into every bg actor holding the old one. It isn't
  re-expanded until the C would re-expand it (a bg actor's transform changes, or the lookup is
  invalidated). The sharing never shows in play: rooms 0 and 3's webs are only loaded together
  after room 0's has broken.
- **A texture swapped on a segment is a bake per texture** (as ADR 0012 does for En_Ko's eyes):
  seven eye bakes, two for the crystal's colour. Scrolls and colours stay dynamic segments.
- **Overlay statics** stay on the play state (`PlayState::overlay_static`): `sLitTorchCount`, the
  floor webs' header.
- **`Obj_Syokudai`'s `torchType` is the params' high bits unshifted** (`PARAMS_GET_NOSHIFT`: 0,
  0x1000, 0x2000), so its compares with 1 and 2 never hold (`@bug (game)`, kept): a wooden torch
  lights from its switch flag too, and lighting one sets its flag with the attention camera.
- **The map's data:** `z_map_exp.c`'s state is one `PlayState::map` (`MapState`: the interface's
  map fields, the `R_MAP_*`, `R_COMPASS_*` and `VREG` registers it uses, its statics), with
  `Map_Init`, `Map_InitData`, `Map_InitRoomData`, `Map_Update`, `Map_SavePlayerInitialInfo` and the
  palette functions under their names, called where the C calls them. `gMapDataTable` and the
  MQ build's `gMapMarkDataTable` are one pack record (`table/map`), read from the C and checked
  against the ROM's bytes, stored flat at the C's sizes so an index past a row reads the next as
  the game does. `mapIndex` is now set by `Map_Init` (Kokiri Forest is 4, not 0). Nothing is drawn.
- **Pack format 20** (`out/data17`): the switches', torches' and webs' bakes, the boss door's, the
  key icon, `table/map`.

## Consequences

- Room 0's switch burns the web over room 10's door and lights its golden torches; the floor web
  breaks under Link's fall from the top floor; Navi calls at the MQ hint spots.
- Milestone 4b's Deku Stick reaches the stick paths already ported here.
- Milestone 5's pause menu and the HUD's minimap read `MapState` and `table/map`; the pause map's
  own marks (`gPauseMapMarkDataTable`) are kaleido data, not in the record yet.
