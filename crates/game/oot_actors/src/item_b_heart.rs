//! `Item_B_Heart` (`ovl_Item_B_Heart/z_item_b_heart.c`): the heart container a boss leaves.
//!
//! It grows in to 0.4, bobs 20 to 25 above where it was spawned and spins; within 30 across and
//! 40 up or down Link can take it (`GI_HEART_CONTAINER_2`), which sets the room's collectible
//! flag 0x1F, so it isn't spawned again. Drawn opaque, or translucent while a `Door_Warp1` is
//! further from the camera than it (the warp's rays seen through it).
//!
//! Not ported: the culling volume (`cullingVolumeDistance` and the rest: no actor culls).

use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, sin_s};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ACTORCAT_MISC, ActorImpl, ActorProfile};
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::GI_HEART_CONTAINER_2;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::boss_goma::{ACTOR_DOOR_WARP1, ACTOR_ITEM_B_HEART};

/// `Item_B_Heart_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_ITEM_B_HEART, name: "Item_B_Heart", category: ACTORCAT_MISC, flags: 0, object: "object_gi_hearts" };

/// The room's collectible flag a heart container taken sets.
const COLLECTIBLE_FLAG: i32 = 0x1F;
/// `GID_HEART_CONTAINER` (`item.h`): its get-item draw, `GetItem_DrawXlu01`'s
/// `gGiHeartBorderDL` and `gGiHeartContainerDL`, the lists `ItemBHeart_Draw` draws.
const GID_HEART_CONTAINER: usize = 0x12;

pub struct ItemBHeart {
    pub actor: Actor,
    pub unk_158: f32,
    pub unk_164: i16,
}

impl ItemBHeart {
    /// `ItemBHeart_Init`: gone if its flag is set; else at scale 0 (`sInitChain`).
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        if play.flags.get_collectible(COLLECTIBLE_FLAG) {
            actor.kill();
        } else {
            actor.scale = Vec3::ZERO;
            // ActorShape_Init(&shape, 0, NULL, 0.8).
            actor.shape_y_offset = 0.0;
        }
        Box::new(ItemBHeart { actor, unk_158: 0.0, unk_164: 0 })
    }

    /// `func_80B85264`: the bob (`sin(unk_164 × 0x60C) × 5 + 20` above home, approached at up to
    /// `unk_158`, which grows to 2), the spin, the growth to 0.4.
    fn func_80b85264(&mut self) {
        self.unk_164 = self.unk_164.wrapping_add(1);
        let y_offset = sin_s(self.unk_164.wrapping_mul(0x60C)) * 5.0 + 20.0;
        approach_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + y_offset, 0.1, self.unk_158);
        approach_f(&mut self.unk_158, 2.0, 1.0, 0.1);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x400);
        approach_f(&mut self.actor.scale.x, 0.4, 0.1, 0.01);
        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
    }
}

impl ActorImpl for ItemBHeart {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ItemBHeart_Update`: taken, its flag is set and it's gone; else it's offered.
    fn update(&mut self, play: &mut PlayState) {
        self.func_80b85264();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        if actor_has_parent(&self.actor) {
            play.flags.set_collectible(COLLECTIBLE_FLAG);
            self.actor.kill();
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_HEART_CONTAINER_2, 30.0, 40.0);
        }
    }

    /// `ItemBHeart_Draw`: `gGiHeartBorderDL` and `gGiHeartContainerDL` at `Actor_Draw`'s matrix,
    /// in the translucent list when a `Door_Warp1` (an `ACTORCAT_ITEMACTION`) is further from the
    /// camera (`projectedPos.z`), else the opaque one.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let behind_warp = play.actors.category(ACTORCAT_ITEMACTION).iter().filter_map(|&h| play.actors.actor(h)).any(|a| a.id == ACTOR_DOOR_WARP1 && a.projected_pos.z > self.actor.projected_pos.z);
        let cmd = DrawCmd::new(MeshKey::named(keys::bake(&oot_game::draw::bake_name(GID_HEART_CONTAINER, 0))), oot_game::play::actor_draw_matrix(rs));
        if behind_warp {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
