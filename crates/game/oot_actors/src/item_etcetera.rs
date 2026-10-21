//! `Item_Etcetera` (`ovl_Item_Etcetera/z_item_etcetera.c`): an item lying in the world, shown
//! as its get-item model (`GetItem_Draw`) once its get-item object is in
//! (`ItemEtcetera_WaitForObject`), and offered within 30 across and 50 up or down
//! (`Actor_OfferGetItem`) until Player takes it (`Actor_HasParent`), which kills it.
//!
//! - **Ruto's letter** (`ITEM_ETC_LETTER`, Lake Hylia's bottom at (-916, -1580, 4297)): scale 0.5,
//!   gone at init once `EVENTCHKINF_31` is set; offered as `GI_BOTTLE_RUTOS_LETTER` with a bubble
//!   (`EffectSsBubble_Spawn`, 10 across, scale 0.13) on every frame where `gameplayFrames & 0xD` is
//!   0 (`func_80B858B4`). Player's dive takes it from the bottom (`Player_ActionHandler_2`) and
//!   surfaces holding it up; taken, it sets `EVENTCHKINF_31` and switch 0xB.
//! - **The fire arrows** (`ITEM_ETC_ARROW_FIRE`, spawned by `Shot_Sun` at (700, -800, 7261) with
//!   `gLakeHyliaFireArrowsCS`): scale 0.5, `shape.yOffset` 50. In a cutscene it waits for its cue's
//!   action 2 on channel 0 (`ItemEtcetera_UpdateFireArrow`), outside one it starts at once; then it
//!   falls at -0.1 a frame to -4 (`ItemEtcetera_MoveFireArrowDown`), spinning 0x400 a frame, with
//!   a red sparkle (`EffectSsKiraKira_SpawnDispersed`, scale 5000, 16 frames) every frame it's off
//!   the ground, and is offered as `GI_ARROW_FIRE` as it goes.
//! - **The treasure chest game's** (types 8 to 13, spawned over the chests by `En_Changer`, not
//!   ported): scale 0.5, 15 higher, offered as nothing (`GI_NONE`): shown only through the Lens of
//!   Truth (`ItemEtcetera_DrawThroughLens`) and gone once their chest's flag is set
//!   (`func_80B85B28`).
//! - **The others** (an empty bottle, the Hylian Shield, the 40 quiver, the silver and golden
//!   scales, a small key): scale 0.25, offered as their item (`func_80B85824`). Nothing in the game
//!   places or spawns them.
//!
//! The whole overlay is ported, every type: the fire arrows' and the chest game's are a few lines
//! each, so none is a placeholder. Not drawn: the get-item models' highlights (`func_8002EBCC`,
//! `func_8002ED80`), as for `En_Ex_Item`. The init's `actor.draw = NULL` for the fire arrows is
//! overwritten by `ItemEtcetera_WaitForObject` once the object is in, as in the game: they show
//! before their cue.
//!
//! Params: `type` bits 0 to 7 (`ItemEtceteraType`: 0 the bottle, 1 Ruto's letter, 2 the Hylian
//! Shield, 3 the quiver, 4 and 5 the silver and golden scales, 6 a small key, 7 the fire arrows, 8
//! to 13 the treasure chest game's green, blue, red and purple rupees, piece of heart and small
//! key); `chestFlag` bits 8 to 12 (types 8 to 13: their chest's treasure flag). Lake Hylia places
//! 0x0001; `Shot_Sun` spawns 7.
//! Flags: reads EVENTCHKINF_31 (Ruto's letter taken: gone) and sets it with switch 0xB when the
//! letter (type 1) is taken; reads the chest game's treasure flag `chestFlag` (set: gone).

use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, ObjectNeed};
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::event_chk_inf;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_ITEM_ETCETERA` (`actor_table.h`: 0x010F).
pub const ACTOR_ITEM_ETCETERA: i16 = 0x010F;

/// `Item_Etcetera_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`,
/// `OBJECT_GAMEPLAY_KEEP`, no draw (set once its object is in).
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_ITEM_ETCETERA, name: "Item_Etcetera", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

// `ItemEtceteraType` (`z_item_etcetera.h`).
pub const ITEM_ETC_BOTTLE: i16 = 0x00;
pub const ITEM_ETC_LETTER: i16 = 0x01;
pub const ITEM_ETC_SHIELD_HYLIAN: i16 = 0x02;
pub const ITEM_ETC_QUIVER: i16 = 0x03;
pub const ITEM_ETC_SCALE_SILVER: i16 = 0x04;
pub const ITEM_ETC_SCALE_GOLD: i16 = 0x05;
pub const ITEM_ETC_KEY_SMALL: i16 = 0x06;
pub const ITEM_ETC_ARROW_FIRE: i16 = 0x07;
pub const ITEM_ETC_RUPEE_GREEN_CHEST_GAME: i16 = 0x08;
pub const ITEM_ETC_RUPEE_BLUE_CHEST_GAME: i16 = 0x09;
pub const ITEM_ETC_RUPEE_RED_CHEST_GAME: i16 = 0x0A;
pub const ITEM_ETC_RUPEE_PURPLE_CHEST_GAME: i16 = 0x0B;
pub const ITEM_ETC_HEART_PIECE_CHEST_GAME: i16 = 0x0C;
pub const ITEM_ETC_KEY_SMALL_CHEST_GAME: i16 = 0x0D;

/// `PARAMS_GET_U(this->actor.params, 0, 8)`: the type.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[
        (0x00, "ITEM_ETC_BOTTLE"),
        (0x01, "ITEM_ETC_LETTER"),
        (0x02, "ITEM_ETC_SHIELD_HYLIAN"),
        (0x03, "ITEM_ETC_QUIVER"),
        (0x04, "ITEM_ETC_SCALE_SILVER"),
        (0x05, "ITEM_ETC_SCALE_GOLD"),
        (0x06, "ITEM_ETC_KEY_SMALL"),
        (0x07, "ITEM_ETC_ARROW_FIRE"),
        (0x08, "ITEM_ETC_RUPEE_GREEN_CHEST_GAME"),
        (0x09, "ITEM_ETC_RUPEE_BLUE_CHEST_GAME"),
        (0x0A, "ITEM_ETC_RUPEE_RED_CHEST_GAME"),
        (0x0B, "ITEM_ETC_RUPEE_PURPLE_CHEST_GAME"),
        (0x0C, "ITEM_ETC_HEART_PIECE_CHEST_GAME"),
        (0x0D, "ITEM_ETC_KEY_SMALL_CHEST_GAME"),
    ]),
    "the item: 1 Ruto's letter (Lake Hylia), 7 the fire arrows (Shot_Sun), 8 to 13 the treasure chest game's (En_Changer)",
);
/// `PARAMS_GET_U(this->actor.params, 8, 5)`: the chest game's chest's treasure flag.
pub const CHEST_FLAG: Field = Field::params("chestFlag", 8, 5, Kind::ChestFlag, "its chest's flag: set, it's gone").when("type", &[8, 9, 10, 11, 12, 13]);

/// `EVENTCHKINF_31` (`save.h`: 0x31): Ruto's letter taken from the lake.
pub const EVENTCHKINF_31: u16 = 0x31;
/// The switch flag the letter taken sets (`Flags_SetSwitch(play, 0xB)`).
pub const LETTER_SWITCH_FLAG: i32 = 0xB;

/// Item_Etcetera's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, CHEST_FLAG],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_31)).reads().sets().note("Ruto's letter taken: gone at init"),
        FlagUse::new(Flag::Switch("0xB")).sets().note("Ruto's letter taken (type 1)"),
        FlagUse::new(Flag::Chest("chestFlag")).reads().note("the chest game's chest opened: gone"),
    ],
    todo: false,
    // sObjectIds by type (Object_GetSlot: without it, ASSERT).
    objects: &[
        ObjectNeed::new("object_gi_bottle").when("type", &[0]).note("the empty bottle"),
        ObjectNeed::new("object_gi_bottle_letter").when("type", &[1]).note("Ruto's letter"),
        ObjectNeed::new("object_gi_shield_2").when("type", &[2]).note("the Hylian Shield"),
        ObjectNeed::new("object_gi_arrowcase").when("type", &[3]).note("the quiver"),
        ObjectNeed::new("object_gi_scale").when("type", &[4, 5]).note("the scales"),
        ObjectNeed::new("object_gi_key").when("type", &[6, 13]).note("the small keys"),
        ObjectNeed::new("object_gi_m_arrow").when("type", &[7]).note("the fire arrows"),
        ObjectNeed::new("object_gi_rupy").when("type", &[8, 9, 10, 11]).note("the chest game's rupees"),
        ObjectNeed::new("object_gi_hearts").when("type", &[12]).note("the chest game's piece of heart"),
    ],
    texts: &[],
};

// The get-item objects (`object_table.h`).
/// `OBJECT_GI_BOTTLE` (0x00C6).
pub const OBJECT_GI_BOTTLE: i16 = 0x00C6;
/// `OBJECT_GI_BOTTLE_LETTER` (0x010B).
pub const OBJECT_GI_BOTTLE_LETTER: i16 = 0x010B;
/// `OBJECT_GI_SHIELD_2` (0x00DC).
pub const OBJECT_GI_SHIELD_2: i16 = 0x00DC;
/// `OBJECT_GI_ARROWCASE` (0x00BE).
pub const OBJECT_GI_ARROWCASE: i16 = 0x00BE;
/// `OBJECT_GI_SCALE` (0x00DB).
pub const OBJECT_GI_SCALE: i16 = 0x00DB;
/// `OBJECT_GI_KEY` (0x00AA).
pub const OBJECT_GI_KEY: i16 = 0x00AA;
/// `OBJECT_GI_M_ARROW` (0x0158).
pub const OBJECT_GI_M_ARROW: i16 = 0x0158;
/// `OBJECT_GI_RUPY` (0x017F).
pub const OBJECT_GI_RUPY: i16 = 0x017F;
/// `OBJECT_GI_HEARTS` (0x00BD).
pub const OBJECT_GI_HEARTS: i16 = 0x00BD;

/// `sObjectIds`, by type.
pub const OBJECT_IDS: [i16; 14] = [
    OBJECT_GI_BOTTLE,
    OBJECT_GI_BOTTLE_LETTER,
    OBJECT_GI_SHIELD_2,
    OBJECT_GI_ARROWCASE,
    OBJECT_GI_SCALE,
    OBJECT_GI_SCALE,
    OBJECT_GI_KEY,
    OBJECT_GI_M_ARROW,
    OBJECT_GI_RUPY,
    OBJECT_GI_RUPY,
    OBJECT_GI_RUPY,
    OBJECT_GI_RUPY,
    OBJECT_GI_HEARTS,
    OBJECT_GI_KEY,
];

/// `sDrawItemIndices`: the `GetItem_Draw` ids, by type.
pub const DRAW_ITEM_INDICES: [i16; 14] = [
    GID_BOTTLE_EMPTY,
    GID_BOTTLE_RUTOS_LETTER,
    GID_SHIELD_HYLIAN,
    GID_QUIVER_40,
    GID_SCALE_SILVER,
    GID_SCALE_GOLDEN,
    GID_SMALL_KEY,
    GID_ARROW_FIRE,
    GID_RUPEE_GREEN,
    GID_RUPEE_BLUE,
    GID_RUPEE_RED,
    GID_RUPEE_PURPLE,
    GID_HEART_PIECE,
    GID_SMALL_KEY,
];

/// `sGetItemIds`: what each type offers, by type.
pub const GET_ITEM_IDS: [i16; 14] =
    [GI_BOTTLE_EMPTY, GI_BOTTLE_RUTOS_LETTER, GI_SHIELD_HYLIAN, GI_QUIVER_40, GI_SCALE_SILVER, GI_SCALE_GOLDEN, GI_SMALL_KEY, GI_ARROW_FIRE, GI_NONE, GI_NONE, GI_NONE, GI_NONE, GI_NONE, GI_NONE];

/// `actionFunc` and `futureActionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `ItemEtcetera_WaitForObject`.
    WaitForObject,
    /// `func_80B85824`: offered until taken.
    Offer,
    /// `func_80B858B4`: Ruto's letter, offered with its bubbles.
    OfferLetter,
    /// `ItemEtcetera_MoveFireArrowDown`.
    MoveFireArrowDown,
    /// `func_80B85B28`: the chest game's, gone once its chest is open.
    ChestGame,
    /// `ItemEtcetera_UpdateFireArrow`: waiting for its cue.
    UpdateFireArrow,
}

/// `drawFunc` and `actor.draw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawFn {
    /// `ItemEtcetera_Draw`.
    Draw,
    /// `ItemEtcetera_DrawThroughLens`.
    DrawThroughLens,
}

pub struct ItemEtcetera {
    pub actor: Actor,
    pub future_action: Action,
    pub gi_draw_id: i16,
    pub get_item_id: i16,
    pub required_object_slot: Option<usize>,
    pub draw_func: DrawFn,
    /// `actor.draw`: none until the object is in.
    pub draw: Option<DrawFn>,
    pub action: Action,
}

impl ItemEtcetera {
    /// `ItemEtcetera_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let ty = TYPE.get(actor.params);
        log::debug!("no = {ty}");
        let Some(&object_id) = OBJECT_IDS.get(ty as usize) else {
            // The C reads past sObjectIds: nothing spawns one.
            log::warn!("Item_Etcetera: type {ty:#x} reads past its tables: killed");
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let slot = play.object_ctx.get_index(object_id);
        log::debug!("bank_ID = {slot:?}");
        let mut this = ItemEtcetera {
            actor,
            future_action: Action::Offer,
            gi_draw_id: DRAW_ITEM_INDICES[ty as usize],
            get_item_id: GET_ITEM_IDS[ty as usize],
            required_object_slot: None,
            draw_func: DrawFn::Draw,
            draw: None,
            action: Action::WaitForObject,
        };
        match slot {
            Some(s) => this.required_object_slot = Some(s),
            None => {
                // ASSERT(0, "0", "../z_item_etcetera.c", 241): the debug build stops.
                log::error!("Item_Etcetera: type {ty:#x}'s object {object_id:#06x} isn't loaded (the C asserts): killed");
                this.actor.kill();
                return Box::new(this);
            }
        }
        this.actor.scale = Vec3::splat(0.25);
        match ty {
            ITEM_ETC_LETTER => {
                this.actor.scale = Vec3::splat(0.5);
                this.future_action = Action::OfferLetter;
                if play.save.get_event_chk_inf(EVENTCHKINF_31) {
                    this.actor.kill();
                }
            }
            ITEM_ETC_ARROW_FIRE => {
                this.future_action = Action::UpdateFireArrow;
                this.actor.scale = Vec3::splat(0.5);
                this.draw = None;
                this.actor.shape_y_offset = 50.0;
            }
            ITEM_ETC_RUPEE_GREEN_CHEST_GAME
            | ITEM_ETC_RUPEE_BLUE_CHEST_GAME
            | ITEM_ETC_RUPEE_RED_CHEST_GAME
            | ITEM_ETC_RUPEE_PURPLE_CHEST_GAME
            | ITEM_ETC_HEART_PIECE_CHEST_GAME
            | ITEM_ETC_KEY_SMALL_CHEST_GAME => {
                this.actor.scale = Vec3::splat(0.5);
                this.future_action = Action::ChestGame;
                this.draw_func = DrawFn::DrawThroughLens;
                this.actor.world_pos.y += 15.0;
            }
            _ => {}
        }
        Box::new(this)
    }

    /// `ItemEtcetera_WaitForObject`: its object in, drawn, and on to its own action.
    fn wait_for_object(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot else { return };
        if play.object_ctx.is_loaded(slot) {
            self.actor.obj_bank_index = Some(slot);
            self.draw = Some(self.draw_func);
            self.action = self.future_action;
        }
    }

    /// Taken: Ruto's letter (type 1) sets `EVENTCHKINF_31` and switch 0xB; gone.
    fn taken(&mut self, play: &mut PlayState) {
        if TYPE.get(self.actor.params) == ITEM_ETC_LETTER {
            play.save.set_event_chk_inf(EVENTCHKINF_31);
            play.flags.set_switch(LETTER_SWITCH_FLAG);
        }
        self.actor.kill();
    }

    /// `func_80B85824`: taken, or offered within 30 across and 50 up or down.
    fn offer(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.taken(play);
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, self.get_item_id, 30.0, 50.0);
        }
    }

    /// `func_80B858B4`: the letter's offer, with a bubble when `gameplayFrames & 0xD` is 0.
    fn offer_letter(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.taken(play);
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, self.get_item_id, 30.0, 50.0);
            if play.gameplay_frames & 0xD == 0 {
                let pos = self.actor.world_pos;
                play.with_ss(|s| s.bubble_spawn(pos, 0.0, 0.0, 10.0, 0.13));
            }
        }
    }

    /// `ItemEtcetera_SpawnSparkles`: a red sparkle within 6 across and 6 above it, drifting down.
    fn spawn_sparkles(&mut self, play: &mut PlayState) {
        // The statics velocity { 0, 0.2, 0 } and accel { 0, 0.05, 0 }, rewritten before use.
        let r = &mut play.rand;
        let vx = r.centered_float(3.0);
        let vz = r.centered_float(3.0);
        let velocity = Vec3::new(vx, -0.05, vz);
        let accel = Vec3::new(0.0, -0.025, 0.0);
        let px = r.centered_float(12.0) + self.actor.world_pos.x;
        let py = (r.zero_one() * 6.0) + self.actor.world_pos.y;
        let pz = r.centered_float(12.0) + self.actor.world_pos.z;
        let pos = Vec3::new(px, py, pz);
        // primColor { 255, 255, 255, 0 }, envColor { 255, 50, 50, 0 }.
        play.with_ss(|s| s.kirakira_spawn_dispersed(pos, velocity, accel, [255, 255, 255, 0], [255, 50, 50, 0], 5000, 16));
    }

    /// `ItemEtcetera_MoveFireArrowDown`: falling, sparkling while in the air, spinning, offered.
    fn move_fire_arrow_down(&mut self, play: &mut PlayState) {
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        self.actor.move_forward();
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            self.spawn_sparkles(play);
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x400);
        self.offer(play);
    }

    /// `func_80B85B28`: gone once its chest's flag is set.
    fn chest_game(&mut self, play: &mut PlayState) {
        if play.flags.get_treasure(CHEST_FLAG.get(self.actor.params) as i32) {
            self.actor.kill();
        }
    }

    /// `ItemEtcetera_UpdateFireArrow`: in a cutscene, falls from channel 0's cue action 2 (drawn
    /// from then); outside one, falls at once.
    fn update_fire_arrow(&mut self, play: &mut PlayState) {
        let cue = if play.cs_ctx.state != CS_STATE_IDLE { play.cs_ctx.npc_actions.first().copied().flatten() } else { None };
        if play.cs_ctx.state != CS_STATE_IDLE && cue.is_some() {
            let id = cue.map(|c| c.action).unwrap_or(0);
            log::debug!("(game_play->demo_play.npcdemopnt[0]->dousa) = {id}");
            if id == 2 {
                self.draw = Some(DrawFn::Draw);
                self.actor.gravity = -0.1;
                self.actor.min_velocity_y = -4.0;
                self.action = Action::MoveFireArrowDown;
            }
        } else {
            self.actor.gravity = -0.1;
            self.actor.min_velocity_y = -4.0;
            self.action = Action::MoveFireArrowDown;
        }
    }
}

impl ActorImpl for ItemEtcetera {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ItemEtcetera_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitForObject => self.wait_for_object(play),
            Action::Offer => self.offer(play),
            Action::OfferLetter => self.offer_letter(play),
            Action::MoveFireArrowDown => self.move_fire_arrow_down(play),
            Action::ChestGame => self.chest_game(play),
            Action::UpdateFireArrow => self.update_fire_arrow(play),
        }
    }

    /// `ItemEtcetera_Draw` (`GetItem_Draw` of its draw id at `Actor_Draw`'s matrix), or
    /// `ItemEtcetera_DrawThroughLens` (the same, only while the Lens of Truth is on).
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let Some(f) = self.draw else { return };
        if f == DrawFn::DrawThroughLens && !play.actors.lens_active {
            return;
        }
        let Some(a) = &play.assets else { return };
        // func_8002EBCC, func_8002ED80: the highlights, not drawn.
        oot_game::draw::get_item_draw(&a.items, self.gi_draw_id, actor_draw_matrix(rs), play.gameplay_frames, view, out);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_follow_the_types() {
        assert_eq!(OBJECT_IDS[ITEM_ETC_LETTER as usize], OBJECT_GI_BOTTLE_LETTER);
        assert_eq!(GET_ITEM_IDS[ITEM_ETC_LETTER as usize], GI_BOTTLE_RUTOS_LETTER);
        assert_eq!(DRAW_ITEM_INDICES[ITEM_ETC_LETTER as usize], GID_BOTTLE_RUTOS_LETTER);
        assert_eq!(GET_ITEM_IDS[ITEM_ETC_ARROW_FIRE as usize], GI_ARROW_FIRE);
        assert!(GET_ITEM_IDS[8..].iter().all(|&g| g == GI_NONE));
        // The letter's bubble frames: gameplayFrames & 0xD == 0 (0, 2, 16, 18, 32, ...).
        let frames: Vec<u32> = (0..20).filter(|f| f & 0xD == 0).collect();
        assert_eq!(frames, vec![0, 2, 16, 18]);
    }
}
