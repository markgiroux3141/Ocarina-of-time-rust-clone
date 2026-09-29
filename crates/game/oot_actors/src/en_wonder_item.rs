//! `En_Wonder_Item` (`ovl_En_Wonder_Item/z_en_wonder_item.c`): an invisible collectable. Where
//! Link comes near, or hits it, or touches a set of tag points, it drops an item and sets its
//! switch flag.
//!
//! Params: bits 11..15 the mode (`wonderMode`), bits 6..10 the drop (`itemDrop`,
//! `WONDERITEM_DROP_*`), bits 0..5 the switch flag (0x3F for none); `rot.z` is the mode's
//! number (drops, tag points, or the tag index). Kokiri Forest's training area has two of
//! 0x123F with `rot.z` 1: `WONDERITEM_PROXIMITY_DROP`, a green rupee, no switch flag, one drop.
//!
//! The whole overlay is ported. `EnWonderItem_BombSoldier`'s soldier (`En_Heishi2`) spawns as
//! a placeholder; the debug arrows (`BREG(0)`) and the sound aren't ported.

use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_0, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::collision_check::*;
use oot_game::play::PlayState;

use crate::en_item00::{
    ITEM00_ARROWS_LARGE, ITEM00_ARROWS_MEDIUM, ITEM00_ARROWS_SMALL, ITEM00_FLEXIBLE, ITEM00_HEART_PIECE, ITEM00_MAGIC_LARGE, ITEM00_MAGIC_SMALL, ITEM00_NUTS, ITEM00_RECOVERY_HEART, ITEM00_RUPEE_BLUE,
    ITEM00_RUPEE_GREEN, ITEM00_RUPEE_RED, item_drop_collectible, item_drop_collectible_random,
};

pub const ACTOR_EN_WONDER_ITEM: i16 = 0x0112;
/// `ACTOR_EN_HEISHI2` (`actor_table.h`): the bomb soldier.
const ACTOR_EN_HEISHI2: i16 = 0x00B3;

/// `En_Wonder_Item_InitVars`: `FLAGS` 0, no draw.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_WONDER_ITEM, name: "En_Wonder_Item", category: ACTORCAT_PROP, flags: 0, object: "gameplay_keep" };

/// `EnWonderItemMode`.
pub const WONDERITEM_MULTITAG_FREE: i16 = 0;
pub const WONDERITEM_TAG_POINT_FREE: i16 = 1;
pub const WONDERITEM_PROXIMITY_DROP: i16 = 2;
pub const WONDERITEM_INTERACT_SWITCH: i16 = 3;
pub const WONDERITEM_UNUSED: i16 = 4;
pub const WONDERITEM_MULTITAG_ORDERED: i16 = 5;
pub const WONDERITEM_TAG_POINT_ORDERED: i16 = 6;
pub const WONDERITEM_PROXIMITY_SWITCH: i16 = 7;
pub const WONDERITEM_BOMB_SOLDIER: i16 = 8;
pub const WONDERITEM_ROLL_DROP: i16 = 9;

/// `WONDERITEM_DROP_FLEXIBLE`, `WONDERITEM_DROP_RANDOM` (the drops from 0xC on are random, from
/// drop table `itemDrop - 0xC`).
const WONDERITEM_DROP_FLEXIBLE: i16 = 0xB;
const WONDERITEM_DROP_RANDOM: i16 = 0xC;

/// `EnWonderItem_DropCollectible`'s `dropTable`, by `itemDrop`.
const DROP_TABLE: [i16; 12] = [
    ITEM00_NUTS,
    ITEM00_HEART_PIECE,
    ITEM00_MAGIC_LARGE,
    ITEM00_MAGIC_SMALL,
    ITEM00_RECOVERY_HEART,
    ITEM00_ARROWS_SMALL,
    ITEM00_ARROWS_MEDIUM,
    ITEM00_ARROWS_LARGE,
    ITEM00_RUPEE_GREEN,
    ITEM00_RUPEE_BLUE,
    ITEM00_RUPEE_RED,
    ITEM00_FLEXIBLE,
];

/// `DMG_ARROW`.
const DMG_ARROW: u32 = DMG_ARROW_NORMAL | (1 << 11) | (1 << 12) | (1 << 13) | (1 << 14) | (1 << 15) | (1 << 16);

/// `EnWonderItem_Init`'s `damageFlags`: what hits an interact switch, by `rot.z`.
const DAMAGE_FLAGS: [u32; 7] = [DMG_SLASH | DMG_DEKU_STICK, DMG_ARROW, DMG_HAMMER_SWING, DMG_EXPLOSIVE, DMG_SLINGSHOT, DMG_BOOMERANG, DMG_HOOKSHOT];

/// `sCylinderInit` (the radius and height are set by the mode).
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COLTYPE_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderInfoInit {
        elem_type: ELEMTYPE_UNK0,
        toucher: ColliderTouch { dmg_flags: 0, effect: 0, damage: 0 },
        bumper: ColliderBumpInit { dmg_flags: 0xFFCF_FFFF, effect: 0, defense: 0 },
        toucher_flags: TOUCH_NONE,
        bumper_flags: BUMP_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 20, height: 30, y_shift: 0, pos: [0; 3] },
};

/// `sTagPointsFree`, `sTagPointsOrdered`: the overlay's statics, which the tag point instances
/// fill and the multitag instances read.
#[derive(Debug, Clone, Copy, Default)]
struct TagPoints {
    free: [Vec3; 9],
    ordered: [Vec3; 9],
}

/// `updateFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Update {
    /// `EnWonderItem_MultitagFree`.
    MultitagFree,
    /// `EnWonderItem_ProximityDrop`.
    ProximityDrop,
    /// `EnWonderItem_InteractSwitch`.
    InteractSwitch,
    /// `EnWonderItem_MultitagOrdered`.
    MultitagOrdered,
    /// `EnWonderItem_ProximitySwitch`.
    ProximitySwitch,
    /// `EnWonderItem_BombSoldier`.
    BombSoldier,
    /// `EnWonderItem_RollDrop`.
    RollDrop,
    /// NULL: `WONDERITEM_UNUSED` never sets one (the C would call through NULL).
    None,
}

pub struct EnWonderItem {
    pub actor: Actor,
    update: Update,
    /// `unkHeight`: the unused mode's focus height (never set).
    pub unk_height: f32,
    pub wonder_mode: i16,
    pub item_drop: i16,
    pub num_tag_points: i16,
    pub drop_count: i16,
    pub timer: i16,
    pub tag_flags: i16,
    pub tag_count: i16,
    pub switch_flag: i16,
    pub next_tag: i16,
    pub timer_mod: i16,
    /// `unkPos`: the bomb soldier mode's start position, never read.
    pub unk_pos: Vec3,
    pub collider: ColliderCylinder,
}

impl EnWonderItem {
    /// `EnWonderItem_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.flags &= !ACTOR_FLAG_0;
        let params = actor.params as u16;
        let mut this = EnWonderItem {
            wonder_mode: ((params >> 0xB) & 0x1F) as i16,
            item_drop: ((params >> 6) & 0x1F) as i16,
            switch_flag: (params & 0x3F) as i16,
            update: Update::None,
            unk_height: 0.0,
            num_tag_points: 0,
            drop_count: 0,
            timer: 0,
            tag_flags: 0,
            tag_count: 0,
            next_tag: 0,
            timer_mod: 0,
            unk_pos: Vec3::ZERO,
            // Collider_InitCylinder only for the modes that use it; the dims stay 0 otherwise.
            collider: ColliderCylinder::default(),
            actor,
        };
        if this.switch_flag == 0x3F {
            this.switch_flag = -1;
        }
        this.actor.target_mode = 1;
        if this.switch_flag >= 0 && play.flags.get_switch(this.switch_flag as i32) {
            this.actor.kill();
            return Box::new(this);
        }
        let rot_z = this.actor.world_rot.z;
        match this.wonder_mode {
            WONDERITEM_MULTITAG_FREE | WONDERITEM_MULTITAG_ORDERED => {
                // timerMod = rot.z / 10 seconds, numTagPoints = rot.z % 10.
                let mut rot_z_over_10 = 0;
                if rot_z >= 10 {
                    rot_z_over_10 = rot_z / 10;
                    this.timer_mod = rot_z_over_10 * 20;
                }
                this.num_tag_points = rot_z - rot_z_over_10 * 10;
                this.update = if this.wonder_mode == WONDERITEM_MULTITAG_FREE { Update::MultitagFree } else { Update::MultitagOrdered };
            }
            WONDERITEM_TAG_POINT_FREE | WONDERITEM_TAG_POINT_ORDERED => {
                let tag_index = (rot_z & 0xFF) as usize;
                let pos = this.actor.world_pos;
                let points = play.overlay_static::<TagPoints>(ACTOR_EN_WONDER_ITEM);
                // (Indices past 8 write past the arrays in the C.)
                let list = if this.wonder_mode == WONDERITEM_TAG_POINT_FREE { &mut points.free } else { &mut points.ordered };
                if let Some(p) = list.get_mut(tag_index) {
                    *p = pos;
                }
                this.actor.kill();
            }
            WONDERITEM_PROXIMITY_DROP => {
                this.drop_count = rot_z & 0xFF;
                this.update = Update::ProximityDrop;
            }
            WONDERITEM_INTERACT_SWITCH => {
                let col_type_index = (rot_z & 0xFF) as usize;
                this.collider = ColliderCylinder::new(&CYLINDER_INIT);
                // (An index past the table reads past it in the C.)
                this.collider.info.bumper.dmg_flags = DAMAGE_FLAGS.get(col_type_index).copied().unwrap_or(0);
                this.collider.dim.radius = 20;
                this.collider.dim.height = 30;
                this.update = Update::InteractSwitch;
            }
            WONDERITEM_UNUSED => {}
            WONDERITEM_PROXIMITY_SWITCH => this.update = Update::ProximitySwitch,
            WONDERITEM_BOMB_SOLDIER => {
                this.collider = ColliderCylinder::new(&CYLINDER_INIT);
                this.collider.info.bumper.dmg_flags = DMG_SLINGSHOT;
                this.unk_pos = this.actor.world_pos;
                this.collider.dim.radius = 35;
                this.collider.dim.height = 75;
                this.update = Update::BombSoldier;
            }
            WONDERITEM_ROLL_DROP => {
                this.drop_count = rot_z & 0xFF;
                this.update = Update::RollDrop;
            }
            _ => this.actor.kill(),
        }
        Box::new(this)
    }

    /// `EnWonderItem_DropCollectible`: `dropCount` drops (at least one) of `itemDrop` at its
    /// position (collected at once with `autoCollect`, the 0x8000 flag, except the flexible
    /// drop), or random ones from drop table `itemDrop - 0xC`; then the switch flag, and it
    /// goes. (`func_80078884(NA_SE_SY_GET_ITEM)`: the sound isn't ported.)
    fn drop_collectible(&mut self, play: &mut PlayState, auto_collect: bool) {
        if self.drop_count == 0 {
            self.drop_count += 1;
        }
        let pos = self.actor.world_pos;
        for _ in 0..self.drop_count {
            if self.item_drop < WONDERITEM_DROP_RANDOM {
                let item = DROP_TABLE[self.item_drop as usize];
                if self.item_drop == WONDERITEM_DROP_FLEXIBLE || !auto_collect {
                    item_drop_collectible(play, pos, item);
                } else {
                    item_drop_collectible(play, pos, (item as u16 | 0x8000) as i16);
                }
            } else {
                let random_drop = self.item_drop - WONDERITEM_DROP_RANDOM;
                if !auto_collect {
                    item_drop_collectible_random(play, None, pos, random_drop);
                } else {
                    item_drop_collectible_random(play, None, pos, (random_drop as u16 | 0x8000) as i16);
                }
            }
        }
        if self.switch_flag >= 0 {
            play.flags.set_switch(self.switch_flag as i32);
        }
        self.actor.kill();
    }

    /// Player's position, if there's a Player.
    fn player_pos(play: &PlayState) -> Option<Vec3> {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos)
    }

    /// `EnWonderItem_MultitagFree` and `EnWonderItem_MultitagOrdered`: each untagged point
    /// within 50 of Link is tagged (for the ordered one, only the next in order; a wrong one
    /// ends it), restarting the timer; all tagged drops the item. The timer running out ends
    /// it.
    fn multitag(&mut self, play: &mut PlayState, ordered: bool) {
        let Some(link) = Self::player_pos(play) else { return };
        let points = *play.overlay_static::<TagPoints>(ACTOR_EN_WONDER_ITEM);
        let list = if ordered { points.ordered } else { points.free };
        let prev_tag_flags = self.tag_flags as i32;
        let mut mask = 1;
        for i in 0..self.num_tag_points.max(0) as usize {
            if prev_tag_flags & mask == 0 {
                // (The C reads past the arrays for more than 9 points.)
                let p = list.get(i).copied().unwrap_or(Vec3::ZERO);
                let d = link - p;
                if (d.x * d.x + d.y * d.y + d.z * d.z).sqrt() < 50.0 {
                    if ordered && i as i16 != self.next_tag {
                        self.actor.kill();
                        return;
                    }
                    self.tag_flags |= mask as i16;
                    self.tag_count += 1;
                    if ordered {
                        self.next_tag += 1;
                    }
                    self.timer = self.timer_mod + 81;
                    return;
                }
            }
            mask <<= 1;
        }
        if self.timer == 1 {
            self.actor.kill();
            return;
        }
        if self.tag_count == self.num_tag_points {
            // The free one sets the switch flag here as well as in the drop.
            if !ordered && self.switch_flag >= 0 {
                play.flags.set_switch(self.switch_flag as i32);
            }
            self.drop_collectible(play, true);
        }
    }

    /// Link within 50 across and 30 up or down.
    fn near(&self, play: &PlayState) -> bool {
        Self::player_pos(play).is_some_and(|p| self.actor.xz_dist_to_player < 50.0 && (self.actor.world_pos.y - p.y).abs() < 30.0)
    }
}

impl ActorImpl for EnWonderItem {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnWonderItem_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        match self.update {
            Update::MultitagFree => self.multitag(play, false),
            Update::MultitagOrdered => self.multitag(play, true),
            // EnWonderItem_ProximityDrop.
            Update::ProximityDrop => {
                if self.near(play) {
                    self.drop_collectible(play, true);
                }
            }
            // EnWonderItem_InteractSwitch.
            Update::InteractSwitch => {
                if self.collider.base.ac_flags & AC_HIT != 0 {
                    self.collider.base.ac_flags &= !AC_HIT;
                    self.drop_collectible(play, false);
                }
            }
            // EnWonderItem_ProximitySwitch.
            Update::ProximitySwitch => {
                if self.near(play) {
                    if self.switch_flag >= 0 {
                        play.flags.set_switch(self.switch_flag as i32);
                    }
                    self.actor.kill();
                }
            }
            // EnWonderItem_BombSoldier: En_Heishi2 with params 9, facing Link.
            Update::BombSoldier => {
                if self.collider.base.ac_flags & AC_HIT != 0 {
                    self.collider.base.ac_flags &= !AC_HIT;
                    let (pos, yaw) = (self.actor.world_pos, self.actor.yaw_towards_player);
                    if let Err(e) = play.actor_spawn(ACTOR_EN_HEISHI2, pos, [0, yaw, 0], 9) {
                        log::debug!("En_Heishi2: {e:?}");
                    }
                    if self.switch_flag >= 0 {
                        play.flags.set_switch(self.switch_flag as i32);
                    }
                    self.actor.kill();
                }
            }
            // EnWonderItem_RollDrop: near, while Link's invincibility is the roll's (negative).
            Update::RollDrop => {
                let rolling = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).is_some_and(|pi| pi.invincibility_timer() < 0);
                if rolling && self.near(play) {
                    self.drop_collectible(play, true);
                }
            }
            Update::None => {}
        }
        if self.wonder_mode == WONDERITEM_UNUSED {
            // Actor_SetFocus.
            self.actor.focus_pos = self.actor.world_pos + Vec3::Y * self.unk_height;
        }
        if self.wonder_mode == WONDERITEM_INTERACT_SWITCH || self.wonder_mode == WONDERITEM_BOMB_SOLDIER {
            self.collider.update(&self.actor);
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
