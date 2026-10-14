//! `En_Ex_Item` (`ovl_En_Ex_Item/z_en_ex_item.c`): the minigames' prizes, shown spinning
//! (`GetItem_Draw`) by the actor that gives them; each type waits for its get-item object first
//! (`EnExItem_WaitForObject`).
//!
//! - **The bowling prizes** (`EXITEM_*_BOWLING`, spawned by `En_Bom_Bowl_Pit` as its child, not
//!   ported): the bomb bag, the piece of heart (`NA_SE_SY_PIECE_OF_HEART`), bombchus, bombs and the
//!   purple rupee rise towards the camera for 35 frames (`EnExItem_BowlPrize`, 5 a frame along the
//!   view, aimed 250 past `at` in z, 220 for bombchus), turning 0x1000 a frame until they face
//!   0x9000 after those 35, then settling to 0; after 65 frames the parent's
//!   `hasRewardFinishedAppearing` is set and it's gone.
//! - **The counter's** (`EXITEM_*_COUNTER`, `En_Bom_Bowl_Man`, not ported): the same items on the
//!   shelf (`EnExItem_BowlCounter`), turning 0x800 a frame, each at its own `shape.yOffset`, until
//!   the shopkeeper sets `killItem`.
//! - **The treasure chest game's** (`EXITEM_*_RUPEE_CHEST`, `EXITEM_13`, `EXITEM_14` and
//!   `EXITEM_SMALL_KEY_CHEST`, `En_Box` and `En_Changer`, not ported): out of the chest at 10 a
//!   frame for 7 frames, spinning 0x1000 a frame, then still for 20 and gone
//!   (`EnExItem_ExitChest`); the key (`gItemDropDL`, `gDropKeySmallTex`) needs no object.
//! - **The Great Fairy's magic** (`EXITEM_MAGIC_*`, `Bg_Dy_Yoseizo`, not ported): Din's Fire,
//!   Farore's Wind or Nayru's Love turning 0x800 a frame (`EnExItem_FairyMagic`).
//! - **The target scrub's bullet bag** (`EXITEM_BULLET_BAG`, `En_Dnt_Nomal`): growing to 0.8 as it
//!   flies 5 a frame at the camera for 35 frames (`EnExItem_TargetPrizeApproach`), turning to
//!   0x9000 then -0x4000; at 80 frames hidden, Player freed (`PLAYER_CSACTION_7`), and offered
//!   (`Actor_OfferGetItem`, 2000 across and 1000 up: `GI_BULLET_BAG_40` with the 30 bag, else
//!   `GI_BULLET_BAG_50`; `EnExItem_TargetPrizeGive` offers `_50` only with the 40 bag); taken, its
//!   text done: `ITEMGETINF_1D`, gone (`EnExItem_TargetPrizeFinish`).
//!
//! The whole overlay is ported. Not drawn: the items' highlights (`unk_17C`, `unk_180`:
//! `func_8002EBCC`, `func_8002ED80`), kept as data, as for `En_GirlA`. Logged: the bowling
//! prize's write of its parent's `hasRewardFinishedAppearing` (`En_Bom_Bowl_Pit` isn't ported).
//! `Actor_SetScale` in the draw takes effect at the next draw (`Actor_Draw` scaled the matrix
//! before calling it), as in the game.
//!
//! Params: `type` bits 0 to 7 (`EnExItemType`: 0 to 4 the bowling prizes, 5 to 9 the counter's,
//! 10 to 15 the treasure chest game's rupees and key, 16 to 18 the Great Fairy's magic, 19 the
//! target scrub's bullet bag); `unusedParam` bits 8 to 15 (only printed). Spawned by `En_Bom_Bowl_Pit`,
//! `En_Bom_Bowl_Man`, `En_Changer`, `Bg_Dy_Yoseizo` and `En_Dnt_Nomal`; placed nowhere.
//! Flags: sets ITEMGETINF_1D (the bullet bag's text done).

use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, smooth_step_to_s};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_SY_PIECE_OF_HEART;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::UPG_BULLET_BAG;
use oot_game::item_get_inf;
use oot_game::message::TEXT_STATE_DONE;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_EN_EX_ITEM` (`actor_table.h`: 0x0168).
pub const ACTOR_EN_EX_ITEM: i16 = 0x0168;

/// `En_Ex_Item_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_GAMEPLAY_KEEP`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_EX_ITEM,
    name: "En_Ex_Item",
    category: ACTORCAT_PROP,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// `PARAMS_GET_U(this->actor.params, 0, 8)`: `EnExItemType`.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[
        (0, "EXITEM_BOMB_BAG_BOWLING"),
        (1, "EXITEM_HEART_PIECE_BOWLING"),
        (2, "EXITEM_BOMBCHUS_BOWLING"),
        (3, "EXITEM_BOMBS_BOWLING"),
        (4, "EXITEM_PURPLE_RUPEE_BOWLING"),
        (5, "EXITEM_BOMB_BAG_COUNTER"),
        (6, "EXITEM_HEART_PIECE_COUNTER"),
        (7, "EXITEM_BOMBCHUS_COUNTER"),
        (8, "EXITEM_BOMBS_COUNTER"),
        (9, "EXITEM_PURPLE_RUPEE_COUNTER"),
        (10, "EXITEM_GREEN_RUPEE_CHEST"),
        (11, "EXITEM_BLUE_RUPEE_CHEST"),
        (12, "EXITEM_RED_RUPEE_CHEST"),
        (13, "EXITEM_13"),
        (14, "EXITEM_14"),
        (15, "EXITEM_SMALL_KEY_CHEST"),
        (16, "EXITEM_MAGIC_FIRE"),
        (17, "EXITEM_MAGIC_WIND"),
        (18, "EXITEM_MAGIC_DARK"),
        (19, "EXITEM_BULLET_BAG"),
    ]),
    "the prize and who shows it",
);
/// `PARAMS_GET_U(this->actor.params, 8, 8)`: `unusedParam`, only printed.
pub const UNUSED_PARAM: Field = Field::params("unusedParam", 8, 8, Kind::Debug, "printed at init, used by nothing");

/// `ITEMGETINF_1D` (`save.h`): the target game's bullet bag won.
pub const ITEMGETINF_1D: u16 = 0x1D;

/// En_Ex_Item's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema =
    ActorSchema { fields: &[TYPE, UNUSED_PARAM], flags: &[FlagUse::new(item_get_inf!(ITEMGETINF_1D)).sets().note("EXITEM_BULLET_BAG: its text done")], todo: false };

// `EnExItemType` (`z_en_ex_item.h`).
pub const EXITEM_BOMB_BAG_BOWLING: i16 = 0;
pub const EXITEM_HEART_PIECE_BOWLING: i16 = 1;
pub const EXITEM_BOMBCHUS_BOWLING: i16 = 2;
pub const EXITEM_BOMBS_BOWLING: i16 = 3;
pub const EXITEM_PURPLE_RUPEE_BOWLING: i16 = 4;
pub const EXITEM_BOMB_BAG_COUNTER: i16 = 5;
pub const EXITEM_HEART_PIECE_COUNTER: i16 = 6;
pub const EXITEM_BOMBCHUS_COUNTER: i16 = 7;
pub const EXITEM_BOMBS_COUNTER: i16 = 8;
pub const EXITEM_PURPLE_RUPEE_COUNTER: i16 = 9;
pub const EXITEM_GREEN_RUPEE_CHEST: i16 = 10;
pub const EXITEM_BLUE_RUPEE_CHEST: i16 = 11;
pub const EXITEM_RED_RUPEE_CHEST: i16 = 12;
pub const EXITEM_13: i16 = 13;
pub const EXITEM_14: i16 = 14;
pub const EXITEM_SMALL_KEY_CHEST: i16 = 15;
pub const EXITEM_MAGIC_FIRE: i16 = 16;
pub const EXITEM_MAGIC_WIND: i16 = 17;
pub const EXITEM_MAGIC_DARK: i16 = 18;
pub const EXITEM_BULLET_BAG: i16 = 19;

// The get-item objects (`object_table.h`).
pub const OBJECT_GI_HEARTS: i16 = 0x00BD;
pub const OBJECT_GI_BOMBPOUCH: i16 = 0x00BF;
pub const OBJECT_GI_BOMB_1: i16 = 0x00CE;
pub const OBJECT_GI_BOMB_2: i16 = 0x00D9;
pub const OBJECT_GI_GODDESS: i16 = 0x015D;
pub const OBJECT_GI_DEKUPOUCH: i16 = 0x017B;
pub const OBJECT_GI_RUPY: i16 = 0x017F;

// `GetItemDrawID` (`item.h`).
pub const GID_HEART_PIECE: i16 = 0x13;
pub const GID_BOMB_BAG_30: i16 = 0x18;
pub const GID_BOMB: i16 = 0x1F;
pub const GID_BOMBCHU: i16 = 0x27;
pub const GID_DINS_FIRE: i16 = 0x63;
pub const GID_FARORES_WIND: i16 = 0x64;
pub const GID_NAYRUS_LOVE: i16 = 0x65;
pub const GID_BULLET_BAG: i16 = 0x6B;
pub const GID_RUPEE_GREEN: i16 = 0x6C;
pub const GID_RUPEE_BLUE: i16 = 0x6D;
pub const GID_RUPEE_RED: i16 = 0x6E;
pub const GID_RUPEE_PURPLE: i16 = 0x70;

// `GetItemID` (`item.h`).
pub const GI_BULLET_BAG_40: i16 = 0x60;
pub const GI_BULLET_BAG_50: i16 = 0x7B;

/// `PLAYER_CSACTION_7` (`player.h`).
const PLAYER_CSACTION_7: u8 = 7;

/// `EnExItem_DrawMagic`'s `giDrawIds`.
const MAGIC_GI_DRAW_IDS: [i16; 3] = [GID_DINS_FIRE, GID_FARORES_WIND, GID_NAYRUS_LOVE];

/// The key's mesh (`EnExItem_DrawKey`).
pub const KEY_BAKE: &str = "En_Ex_Item/key";
const SEG_TEX: u8 = 0x08;
const SEG_SETUP: u8 = 0x0D;
/// `gItemDropDL` multiplies in segment 1's matrix (`play->billboardMtx`): the bake takes the
/// identity, the draw the billboard.
const SEG_BILLBOARD: u8 = 0x01;

/// `EnExItem_DrawKey`'s mesh: `Gfx_SetupDL_41Opa`, `gDropKeySmallTex` on segment 8, `gItemDropDL`.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: KEY_BAKE.into(),
        object: "gameplay_keep".into(),
        segments: vec![
            (SEG_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: "gDropKeySmallTex".into() }),
            (SEG_SETUP, BakeSegment::Commands({
                let mut d = oot_game::gbi::setup_dl::setup_dl_41();
                d.end();
                d.0
            })),
            (SEG_BILLBOARD, BakeSegment::Bytes(crate::en_item00::identity_mtx())),
        ],
        prelude: vec![SEG_SETUP],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gItemDropDL".into())]),
    }]
}

/// `unk_17C`, `unk_180`: the highlight an item draws after (`EnExItemLightFunc`; not drawn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightFunc {
    /// `func_8002EBCC`.
    Opa,
    /// `func_8002ED80`.
    Xlu,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// No case set one (a type past `EXITEM_BULLET_BAG`): NULL in the C, a crash.
    None,
    WaitForObject,
    BowlPrize,
    SetupBowlCounter,
    BowlCounter,
    ExitChest,
    FairyMagic,
    TargetPrizeApproach,
    TargetPrizeGive,
    TargetPrizeFinish,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::None => "(NULL)",
            Action::WaitForObject => "EnExItem_WaitForObject",
            Action::BowlPrize => "EnExItem_BowlPrize",
            Action::SetupBowlCounter => "EnExItem_SetupBowlCounter",
            Action::BowlCounter => "EnExItem_BowlCounter",
            Action::ExitChest => "EnExItem_ExitChest",
            Action::FairyMagic => "EnExItem_FairyMagic",
            Action::TargetPrizeApproach => "EnExItem_TargetPrizeApproach",
            Action::TargetPrizeGive => "EnExItem_TargetPrizeGive",
            Action::TargetPrizeFinish => "EnExItem_TargetPrizeFinish",
        }
    }
}

pub struct EnExItem {
    pub actor: Actor,
    pub action_func: Action,
    pub get_item_object_id: i16,
    pub ty: i16,
    pub unused_param: i16,
    pub gi_draw_id: i16,
    pub stop_rotate: i16,
    pub timer: i16,
    pub chest_kill_timer: i16,
    pub prize_rotate_timer: i16,
    /// `killItem`: set by `En_Bom_Bowl_Man` (not ported) to clear the counter.
    pub kill_item: i16,
    pub scale: f32,
    /// `unkFloat`: set as `scale`, read by nothing.
    pub unk_float: f32,
    pub required_object_slot: Option<usize>,
    /// `initPos`: unused.
    pub init_pos: Vec3,
    pub unk_17c: Option<LightFunc>,
    pub unk_180: Option<LightFunc>,
    /// `actor.draw != NULL`.
    pub drawn: bool,
    /// The scale `Actor_Draw` draws with: the last draw's `Actor_SetScale`.
    draw_scale: Vec3,
}

impl EnExItem {
    /// `EnExItem_Init`: untargetable; its type, and the get-item object it waits for (none for
    /// the key, which comes out of its chest at once: scale 0.05, rising at 10 for 7 frames);
    /// hidden until it's in, or gone without it in a bank.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        let ty = TYPE.get(actor.params);
        let unused_param = UNUSED_PARAM.get(actor.params);
        log::debug!("☆☆☆☆☆ What will you get? ☆☆☆☆☆ {ty}");
        log::debug!("☆☆☆☆☆ What will you get? ☆☆☆☆☆ {unused_param}");
        let draw_scale = actor.scale;
        let mut this = EnExItem {
            init_pos: actor.world_pos,
            actor,
            action_func: Action::None,
            get_item_object_id: -1,
            ty,
            unused_param,
            gi_draw_id: 0,
            stop_rotate: 0,
            timer: 0,
            chest_kill_timer: 0,
            prize_rotate_timer: 0,
            kill_item: 0,
            scale: 0.0,
            unk_float: 0.0,
            required_object_slot: None,
            unk_17c: None,
            unk_180: None,
            drawn: true,
            draw_scale,
        };
        match ty {
            EXITEM_BOMB_BAG_BOWLING | EXITEM_BOMB_BAG_COUNTER => this.get_item_object_id = OBJECT_GI_BOMBPOUCH,
            EXITEM_HEART_PIECE_BOWLING | EXITEM_HEART_PIECE_COUNTER => this.get_item_object_id = OBJECT_GI_HEARTS,
            EXITEM_BOMBCHUS_BOWLING | EXITEM_BOMBCHUS_COUNTER => this.get_item_object_id = OBJECT_GI_BOMB_2,
            EXITEM_BOMBS_BOWLING | EXITEM_BOMBS_COUNTER => this.get_item_object_id = OBJECT_GI_BOMB_1,
            EXITEM_PURPLE_RUPEE_BOWLING | EXITEM_PURPLE_RUPEE_COUNTER | EXITEM_GREEN_RUPEE_CHEST | EXITEM_BLUE_RUPEE_CHEST | EXITEM_RED_RUPEE_CHEST | EXITEM_13 | EXITEM_14 => {
                this.get_item_object_id = OBJECT_GI_RUPY
            }
            EXITEM_SMALL_KEY_CHEST => {
                this.scale = 0.05;
                this.actor.velocity.y = 10.0;
                this.timer = 7;
                this.action_func = Action::ExitChest;
            }
            EXITEM_MAGIC_FIRE | EXITEM_MAGIC_WIND | EXITEM_MAGIC_DARK => this.get_item_object_id = OBJECT_GI_GODDESS,
            EXITEM_BULLET_BAG => this.get_item_object_id = OBJECT_GI_DEKUPOUCH,
            _ => {}
        }
        if this.get_item_object_id >= 0 {
            this.required_object_slot = play.object_ctx.get_index(this.get_item_object_id);
            this.drawn = false;
            if this.required_object_slot.is_none() {
                this.actor.kill();
                log::debug!("What? {ty} / The bank is weird! (object {:#06x})", this.get_item_object_id);
                return Box::new(this);
            }
            this.action_func = Action::WaitForObject;
        }
        Box::new(this)
    }

    /// `EnExItem_WaitForObject`: its object in, shown, and set up by type: the get-item model,
    /// the highlights, the timers (65 and 35 for the bowling and counter items, 7 out of a chest,
    /// 80 and 35 for the bullet bag), the scale, and the action.
    fn wait_for_object(&mut self, play: &mut PlayState) {
        let Some(slot) = self.required_object_slot else { return };
        if !play.object_ctx.is_loaded(slot) {
            return;
        }
        log::debug!("☆☆☆☆☆ Transfer completed ☆☆☆☆☆ type {} {}", self.ty, self.unused_param);
        self.actor.obj_bank_index = Some(slot);
        self.drawn = true;
        self.stop_rotate = 0;
        // onCounter: the counter's case falls through into the bowling one's.
        match self.ty {
            EXITEM_BOMB_BAG_COUNTER | EXITEM_BOMB_BAG_BOWLING => {
                let on_counter = self.ty == EXITEM_BOMB_BAG_COUNTER;
                self.unk_17c = Some(LightFunc::Opa);
                self.gi_draw_id = GID_BOMB_BAG_30;
                self.timer = 65;
                self.prize_rotate_timer = 35;
                self.scale = 0.5;
                if !on_counter {
                    self.action_func = Action::BowlPrize;
                } else {
                    self.action_func = Action::SetupBowlCounter;
                    self.actor.shape_y_offset = -18.0;
                }
            }
            EXITEM_HEART_PIECE_COUNTER | EXITEM_HEART_PIECE_BOWLING => {
                let on_counter = self.ty == EXITEM_HEART_PIECE_COUNTER;
                self.unk_17c = Some(LightFunc::Xlu);
                self.timer = 65;
                self.prize_rotate_timer = 35;
                self.scale = 0.5;
                if !on_counter {
                    play.audio.play_sfx_centered(NA_SE_SY_PIECE_OF_HEART);
                    self.action_func = Action::BowlPrize;
                } else {
                    self.action_func = Action::SetupBowlCounter;
                    self.actor.shape_y_offset = -10.0;
                }
            }
            EXITEM_BOMBCHUS_COUNTER | EXITEM_BOMBCHUS_BOWLING => {
                let on_counter = self.ty == EXITEM_BOMBCHUS_COUNTER;
                self.unk_17c = Some(LightFunc::Opa);
                self.gi_draw_id = GID_BOMBCHU;
                self.timer = 65;
                self.prize_rotate_timer = 35;
                self.scale = 0.5;
                self.action_func = if !on_counter { Action::BowlPrize } else { Action::SetupBowlCounter };
            }
            EXITEM_BOMBS_BOWLING | EXITEM_BOMBS_COUNTER => {
                self.unk_17c = Some(LightFunc::Opa);
                self.gi_draw_id = GID_BOMB;
                self.timer = 65;
                self.prize_rotate_timer = 35;
                self.scale = 0.5;
                self.unk_float = 0.5;
                if self.ty == EXITEM_BOMBS_BOWLING {
                    self.action_func = Action::BowlPrize;
                } else {
                    self.action_func = Action::SetupBowlCounter;
                    self.actor.shape_y_offset = 10.0;
                }
            }
            EXITEM_PURPLE_RUPEE_BOWLING | EXITEM_PURPLE_RUPEE_COUNTER => {
                self.unk_17c = Some(LightFunc::Opa);
                self.unk_180 = Some(LightFunc::Xlu);
                self.gi_draw_id = GID_RUPEE_PURPLE;
                self.timer = 65;
                self.prize_rotate_timer = 35;
                self.scale = 0.5;
                self.unk_float = 0.5;
                if self.ty == EXITEM_PURPLE_RUPEE_BOWLING {
                    self.action_func = Action::BowlPrize;
                } else {
                    self.action_func = Action::SetupBowlCounter;
                    self.actor.shape_y_offset = 40.0;
                }
            }
            EXITEM_GREEN_RUPEE_CHEST | EXITEM_BLUE_RUPEE_CHEST | EXITEM_RED_RUPEE_CHEST | EXITEM_13 | EXITEM_14 => {
                self.unk_17c = Some(LightFunc::Opa);
                self.unk_180 = Some(LightFunc::Xlu);
                self.timer = 7;
                self.scale = 0.5;
                self.unk_float = 0.5;
                self.actor.velocity.y = 10.0;
                // EXITEM_13 has no case: its giDrawId stays 0 (GID_BOTTLE_EMPTY).
                match self.ty {
                    EXITEM_GREEN_RUPEE_CHEST => self.gi_draw_id = GID_RUPEE_GREEN,
                    EXITEM_BLUE_RUPEE_CHEST => self.gi_draw_id = GID_RUPEE_BLUE,
                    EXITEM_RED_RUPEE_CHEST => self.gi_draw_id = GID_RUPEE_RED,
                    EXITEM_14 => self.gi_draw_id = GID_RUPEE_PURPLE,
                    _ => {}
                }
                self.action_func = Action::ExitChest;
            }
            EXITEM_MAGIC_FIRE | EXITEM_MAGIC_WIND | EXITEM_MAGIC_DARK => {
                self.scale = 0.35;
                self.action_func = Action::FairyMagic;
            }
            EXITEM_BULLET_BAG => {
                self.unk_17c = Some(LightFunc::Opa);
                self.gi_draw_id = GID_BULLET_BAG;
                self.scale = 0.1;
                self.timer = 80;
                self.prize_rotate_timer = 35;
                self.action_func = Action::TargetPrizeApproach;
            }
            _ => {}
        }
    }

    /// The turn of `EnExItem_BowlPrize` and `EnExItem_TargetPrizeApproach`: 0x1000 a frame until,
    /// `prizeRotateTimer` out, it faces 0x9000; then settling to `rest`.
    fn prize_turn(&mut self, rest: i16) {
        if self.stop_rotate == 0 {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x1000);
            if self.prize_rotate_timer == 0 && self.actor.shape_rot.y as u16 == 0x9000 {
                self.stop_rotate += 1;
            }
        } else {
            smooth_step_to_s(&mut self.actor.shape_rot.y, rest, 5, 0x1000, 0);
        }
    }

    /// `EnExItem_BowlPrize`: turning; while `prizeRotateTimer` runs, 5 a frame towards a point 5
    /// in front of the camera along the view (its `at` 250 further in z, 220 for bombchus);
    /// `timer` out, the parent's `hasRewardFinishedAppearing` set, gone.
    /// `@bug (game)`: the print before the parent's NULL check reads `parent->update`.
    fn bowl_prize(&mut self, play: &mut PlayState) {
        self.prize_turn(0);
        if self.timer != 0 {
            if self.prize_rotate_timer != 0 {
                let sp3c = if self.ty == EXITEM_BOMBCHUS_BOWLING { 220.0 } else { 250.0 };
                let (eye, at) = (play.view.eye, play.view.at);
                let tmpf1 = at.x - eye.x;
                let tmpf2 = at.y - eye.y;
                let tmpf3 = at.z + sp3c - eye.z;
                let tmpf4 = (tmpf1 * tmpf1 + tmpf2 * tmpf2 + tmpf3 * tmpf3).sqrt();
                let tmpf5 = (tmpf1 / tmpf4) * 5.0;
                let tmpf6 = (tmpf2 / tmpf4) * 5.0;
                let tmpf7 = (tmpf3 / tmpf4) * 5.0;
                let tmpf1 = eye.x + tmpf5 - self.actor.world_pos.x;
                let tmpf2 = eye.y + tmpf6 - self.actor.world_pos.y;
                let tmpf3 = eye.z + tmpf7 - self.actor.world_pos.z;
                self.actor.world_pos.x += (tmpf1 / tmpf4) * 5.0;
                self.actor.world_pos.y += (tmpf2 / tmpf4) * 5.0;
                self.actor.world_pos.z += (tmpf3 / tmpf4) * 5.0;
            }
        } else {
            log::debug!(" ☆☆☆☆☆ Mother?     ☆☆☆☆☆ {:?}", self.actor.parent);
            let parent_alive = self.actor.parent.and_then(|h| play.actors.actor(h)).is_some_and(|a| !a.killed);
            if parent_alive {
                // ((EnBomBowlPit*)this->actor.parent)->hasRewardFinishedAppearing = true.
                log::debug!("En_Ex_Item: the parent's hasRewardFinishedAppearing (En_Bom_Bowl_Pit isn't ported)");
                log::debug!(" ☆☆☆☆☆ Now it's gone! ☆☆☆☆☆ ");
            }
            self.actor.kill();
        }
    }

    /// `EnExItem_SetupBowlCounter`: facing 0x4268.
    fn setup_bowl_counter(&mut self) {
        self.actor.shape_rot.y = 0x4268;
        self.actor.world_rot.y = 0x4268;
        self.action_func = Action::BowlCounter;
    }

    /// `EnExItem_BowlCounter`: turning 0x800 a frame until `killItem`.
    fn bowl_counter(&mut self) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
        if self.kill_item != 0 {
            self.actor.kill();
        }
    }

    /// `EnExItem_ExitChest`: spinning 0x1000 a frame; at `timer` 1 a `chestKillTimer` of 20;
    /// `timer` out, stopped, and gone when that's out.
    fn exit_chest(&mut self) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x1000);
        if self.timer != 0 {
            if self.timer == 1 {
                self.chest_kill_timer = 20;
            }
        } else {
            self.actor.velocity.y = 0.0;
            if self.chest_kill_timer == 0 {
                self.actor.kill();
            }
        }
        self.actor.move_forward();
    }

    /// `EnExItem_FairyMagic`.
    fn fairy_magic(&mut self) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
    }

    /// `EnExItem_TargetPrizeApproach`: growing to 0.8, turning (settling at -0x4000); while
    /// `prizeRotateTimer` runs, 5 a frame towards a point 5 in front of the camera (10 lower and
    /// 10 further in z) along the view; `timer` out: hidden, Player freed, its parent let go, and
    /// offered (`GI_BULLET_BAG_40` with the 30 bag, else `_50`).
    fn target_prize_approach(&mut self, play: &mut PlayState) {
        approach_f(&mut self.scale, 0.8, 0.1, 0.02);
        self.prize_turn(-0x4000);
        if self.timer != 0 {
            if self.prize_rotate_timer != 0 {
                let (eye, at) = (play.view.eye, play.view.at);
                let tmpf1 = at.x - eye.x;
                let tmpf2 = at.y - 10.0 - eye.y;
                let tmpf3 = at.z + 10.0 - eye.z;
                let tmpf4 = (tmpf1 * tmpf1 + tmpf2 * tmpf2 + tmpf3 * tmpf3).sqrt();
                let tmpf5 = (tmpf1 / tmpf4) * 5.0;
                let tmpf6 = (tmpf2 / tmpf4) * 5.0;
                let tmpf7 = (tmpf3 / tmpf4) * 5.0;
                let tmpf1 = eye.x + tmpf5 - self.actor.world_pos.x;
                let tmpf2 = eye.y - 10.0 + tmpf6 - self.actor.world_pos.y;
                let tmpf3 = eye.z + 10.0 + tmpf7 - self.actor.world_pos.z;
                self.actor.world_pos.x += (tmpf1 / tmpf4) * 5.0;
                self.actor.world_pos.y += (tmpf2 / tmpf4) * 5.0;
                self.actor.world_pos.z += (tmpf3 / tmpf4) * 5.0;
            }
        } else {
            self.drawn = false;
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.actor.parent = None;
            let item_id = if play.save.cur_upg_value(UPG_BULLET_BAG) == 1 { GI_BULLET_BAG_40 } else { GI_BULLET_BAG_50 };
            let a = self.actor.clone();
            offer_get_item_range(play, &a, item_id, 2000.0, 1000.0);
            self.action_func = Action::TargetPrizeGive;
        }
    }

    /// `EnExItem_TargetPrizeGive`: taken, waiting for the text; else offered again (`_50` with
    /// the 40 bag, else `_40`).
    fn target_prize_give(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.action_func = Action::TargetPrizeFinish;
        } else {
            let item_id = if play.save.cur_upg_value(UPG_BULLET_BAG) == 2 { GI_BULLET_BAG_50 } else { GI_BULLET_BAG_40 };
            let a = self.actor.clone();
            offer_get_item_range(play, &a, item_id, 2000.0, 1000.0);
        }
    }

    /// `EnExItem_TargetPrizeFinish`: the text done, `ITEMGETINF_1D`, gone.
    fn target_prize_finish(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            log::debug!("☆☆☆☆☆ Normal termination ☆☆☆☆☆");
            play.save.set_item_get_inf(ITEMGETINF_1D);
            self.actor.kill();
        }
    }

    /// `EnExItem_Draw`'s get-item draw id, or `None` for the key.
    pub fn draw_id(&self) -> Option<i16> {
        match self.ty {
            EXITEM_BOMB_BAG_BOWLING | EXITEM_BOMBCHUS_BOWLING | EXITEM_BOMBS_BOWLING | EXITEM_BOMB_BAG_COUNTER | EXITEM_BOMBCHUS_COUNTER | EXITEM_BOMBS_COUNTER | EXITEM_BULLET_BAG => Some(self.gi_draw_id),
            EXITEM_HEART_PIECE_BOWLING | EXITEM_HEART_PIECE_COUNTER => Some(GID_HEART_PIECE),
            EXITEM_PURPLE_RUPEE_BOWLING | EXITEM_PURPLE_RUPEE_COUNTER | EXITEM_GREEN_RUPEE_CHEST | EXITEM_BLUE_RUPEE_CHEST | EXITEM_RED_RUPEE_CHEST | EXITEM_13 | EXITEM_14 => Some(self.gi_draw_id),
            EXITEM_MAGIC_FIRE | EXITEM_MAGIC_WIND | EXITEM_MAGIC_DARK => Some(MAGIC_GI_DRAW_IDS[(self.ty - EXITEM_MAGIC_FIRE) as usize]),
            _ => None,
        }
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `switches`: drawn (`actor.draw != NULL`), what (0 nothing, 1 a get-item model, 2 the
    /// key), and the get-item draw id.
    pub const DRAWN: usize = 0;
    pub const KIND: usize = 1;
    pub const DRAW_ID: usize = 2;
}

impl ActorImpl for EnExItem {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnExItem_Update`: the timers down, then the action.
    fn update(&mut self, play: &mut PlayState) {
        for t in [&mut self.timer, &mut self.chest_kill_timer, &mut self.prize_rotate_timer] {
            if *t != 0 {
                *t -= 1;
            }
        }
        match self.action_func {
            Action::None => log::error!("En_Ex_Item: type {} has no action (a NULL actionFunc in the C)", self.ty),
            Action::WaitForObject => self.wait_for_object(play),
            Action::BowlPrize => self.bowl_prize(play),
            Action::SetupBowlCounter => self.setup_bowl_counter(),
            Action::BowlCounter => self.bowl_counter(),
            Action::ExitChest => self.exit_chest(),
            Action::FairyMagic => self.fairy_magic(),
            Action::TargetPrizeApproach => self.target_prize_approach(play),
            Action::TargetPrizeGive => self.target_prize_give(play),
            Action::TargetPrizeFinish => self.target_prize_finish(play),
        }
    }

    /// `EnExItem_Draw`'s `Actor_SetScale(&this->actor, this->scale)`: after `Actor_Draw` made the
    /// matrix with the scale it had.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if !self.drawn {
            return;
        }
        self.draw_scale = self.actor.scale;
        self.actor.scale = Vec3::splat(self.scale);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.scale = self.draw_scale;
        let kind = match self.draw_id() {
            Some(_) => 1,
            None if self.ty == EXITEM_SMALL_KEY_CHEST => 2,
            None => 0,
        };
        rs.switches = vec![self.drawn as u32, kind, self.draw_id().unwrap_or(0) as u32];
        rs
    }

    /// `EnExItem_Draw`: `EnExItem_DrawItems`, `_DrawHeartPiece`, `_DrawRupee` and `_DrawMagic`
    /// (`GetItem_Draw` of its draw id; the highlights aren't drawn), or `EnExItem_DrawKey`
    /// (`gItemDropDL` facing the camera).
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.get(rs::DRAWN).copied().unwrap_or(0) == 0 {
            return;
        }
        let m = actor_draw_matrix(rs);
        match rs.switches.get(rs::KIND).copied().unwrap_or(0) {
            1 => {
                let Some(a) = &play.assets else { return };
                let id = rs.switches.get(rs::DRAW_ID).copied().unwrap_or(0) as i16;
                oot_game::draw::get_item_draw(&a.items, id, m, play.gameplay_frames, view, out);
            }
            2 => out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(KEY_BAKE)), m * view.billboard)),
            _ => {}
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
