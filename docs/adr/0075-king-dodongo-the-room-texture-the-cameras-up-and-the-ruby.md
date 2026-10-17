# 0075: King Dodongo, the room texture he writes, the camera's up, and the Goron's Ruby

- **Status:** accepted, built in GAME-09 milestone 4 (2026-10-17)
- **Date:** 2026-10-17
- **Builds on:** [ADR 0050](0050-queen-gohma.md) (a boss's cutscenes on its own sub camera),
  [ADR 0051](0051-textures-replaced-by-source-and-the-object-ram.md) (textures replaced by their
  source, the object RAM the game writes), [ADR 0072](0072-bombs-and-bomb-flowers.md) (the bombs),
  [ADR 0022](0022-cutscenes.md) (the cutscene system), [ADR 0066](0066-the-actor-schema.md) (the
  schemas), [ADR 0068](0068-glb-export.md) (the characters' table).

## Context

Phase 10's fourth milestone (`docs/GAME-09-death-mountain-and-dodongos-cavern.md`), its exit:
King Dodongo (`Boss_Dodongo`, 1,738 lines and 240 of data), his fire breath (`En_Bdfire`), the
lava's effects, `Door_Warp1`'s branch for his room, and the Goron's Ruby's cutscene's actors
(`Demo_Du`'s part, `Demo_Go`). Four things had to be settled before the port:

- **He writes a room texture.** While Link is below y -1000 every update copies his overlay's
  `sLavaFloorLavaTex` into room 1's `gDodongosCavernBossLavaFloorTex` (through segment 3, the
  current room's buffer) wobbling (`func_808C1554`); as he dies a second overlay texture,
  `sLavaFloorRockTex`, is copied into the first; a cleared room's init puts the rock in at once.
  ADR 0051 covered an object's RAM, not a room's.
- **`Play_SetCameraAtEyeUp`.** His intro tilts its sub camera's up vector by `unk_20C` (up to
  about 0.1 across, a roll of some 6°). The port's camera computed an up (from its roll and a
  quake's up offsets) but no view used it: every view matrix took the world's Y.
- **`MREG(64)`'s fill**, a debug register set (`MREG(64)` to `(68)`) that `Play_Draw` turns into a
  screen fill.
- **The lava's damage.** The survey said Player's hot floor covered it, without the polygons.

## Decision

1. **The room RAM the game writes is play state** (`oot_game::room::RoomRam`, `PlayState::room_ram`):
   by room and the region's offset in the room's file, the region's bytes (copied from the pack
   when first written) and the RGBA16 texture they hold. A room load drops what the other buffer
   held (`Room_ProcessRoomRequest`), `Room_FinishRoomChange` keeps only the current room's, a new
   play state starts empty. The room's draws pass the written textures as
   `DrawParams::texture_images` by their segment-3 source (`0x03000000 | offset`) with the draw
   config's, so the renderer replaces them as it does Queen Gohma's object textures. The overlay's
   two textures are the actor's own state (`lava_tex`, `rock_tex`: an overlay's data is loaded
   with its first instance and he's the only one), read from the pack (the overlay's XML names
   them). The floor's offset is this ROM's, `ddan_boss_v2.xml`'s 0x21D8 (the other versions' is
   0x21C8). A write while another room is current (the C would write into that room's buffer)
   logs.
2. **The camera's up is modelled.** `Play_SetCameraAtEyeUp` (`PlayState::camera_set_at_eye_up`)
   sets the camera's up with `CAM_VIEW_UP`; `Camera_Update` takes it for its view once instead of
   its own. `CamView` carries the up, and every view matrix (the play state's view-projection, the
   effects' billboard, the environment's screen positions, the renderer's camera) uses it. An up
   in the vertical plane through the view on its upper side gives the same viewing matrix as the
   world's Y (`guLookAt` takes only the side vector from it), so such an up is kept as Y
   (`camera::view_up`): the views that don't roll keep the matrices they had. A roll (a camera's
   `roll`, a quake's up yaw, King Dodongo's intro) now shows.
3. **`MREG(64)` is `PlayState::mreg_fill`** (the five registers), carried across play states as
   the other debug registers are. `Play_Draw` draws it after the actors and the lens flares, just
   before `envCtx.fillScreen`'s, both at the end of the OPA and XLU lists: so it's composed under
   the environment's fill in the port's one cutscene fill (`draw_fills`, `screen_fill`). In this
   ROM nothing sets King Dodongo's `unk_244` above 0 (his breath only steps it towards 0): the red
   flush never shows; it's ported as written.
4. **The lava's damage is Player's hot floor, ported:** the arena's lava is the 720 square round
   (-890, -3304) at y -1524, eight polygons of `FLOOR_TYPE_3` (`func_80838144`'s second hot
   floor), with the 16-polygon `FLOOR_TYPE_6` rim at -1504 (no fall damage) round it.
5. **`Boss_Dodongo` whole**, Queen Gohma's pattern: his intro and death on his own sub camera
   (the intro's up tilted), the title card (`BOSS_NAMES`: `gKingDodongoTitleCardTex`, 128 by 40
   of its 128 by 120), the fight's every action, the explosive eaten while he inhales
   (`ACTORCAT_EXPLOSIVE` within 40 of his mouth on each axis), his 80 particles, the decay into
   his object's RAM (ADR 0051, its table read past its end as the console does), the heart, the
   warp, the cleared room at init. His draw is one skeleton bake: each limb's wobble and squash
   (and his tail base's scale) multiplied onto that limb's bone after posing (inside its own push
   and pop, so not onto its children), his own fog, his tilt and roll. His `PostLimbDraw` (his
   points, the spheres) runs in `draw_update`. His profile's id is `ACTOR_EN_DODONGO` and
   `En_Bdfire`'s is 0 (`@bug (game)`, kept: the actors' ids come from the profiles).
6. **`En_Bdfire`, `Effect_Ss_G_Magma`, `Effect_Ss_G_Magma2` whole** (`G_Magma2`'s list loads the
   billboard from segment 1: the draw multiplies it in, as `Dead_Db`'s).
7. **`Door_Warp1`'s branch** for his room (`EVENTCHKINF_25`, `Item_Give(ITEM_GORON_RUBY)`,
   `ENTR_DEATH_MOUNTAIN_TRAIL_0` with `CS_INDEX_1`; later `_5`) and Barinade's
   (`ENTR_ZORAS_FOUNTAIN_0`): the logged branch's stale entrance is gone.
8. **`Demo_Du`'s Ruby part and `Demo_Go` whole** (a helper, in a worktree), `Demo_Du`'s other
   cutscenes placeholders chosen at init (the user's call: Phases 15 and 19).
   `ActorShadow_DrawCircle`'s floor and height checks moved to `oot_game::draw::actor_shadow_circle`
   for them.
9. **Two runs** (the user's rule: no fights): `Route::KingDodongo` (the room's spawn, a bomb on the
   cracked floor, the drop, his intro to the fight's start) and `Route::Ruby` (a debug start after
   his defeat: the heart, the warp, `gDMTGoronRubyCs`, the trail's spawn 5). Their ends are the
   presets `cavern-boss-fight` and `goron-ruby` (Phase 10's exit, milestones 5 and 6's start),
   checked against them; `cavern-boss-beaten` is a debug start.

## Consequences

- The Goron's Ruby is reached: Phase 10's exit is met.
- A room texture the game writes, and any other room RAM, can use `RoomRam` as objects use
  `ObjectContext::written`.
- Views roll where the game rolls them: a quake's up yaw (types whose up yaw isn't 0) changes the
  goldens that catch one; King Dodongo's intro rolls.
- `MREG(64)`'s fill is drawn where the game draws it, though nothing in this phase turns it on.
