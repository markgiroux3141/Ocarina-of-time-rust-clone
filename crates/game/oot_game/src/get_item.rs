//! `z_actor.c`'s get-item offers: an actor offers Player an item (`Actor_OfferGetItem`, which later
//! decomps call `Actor_OfferGetItem`, and its short form `Actor_OfferGetItemNearby`), Player takes it in
//! its get-item interrupt (`Player_ActionHandler_2`, in `oot_actors::player`) and becomes the actor's
//! `parent` (`Actor_HasParent`), and the actor goes.
//!
//! A positive get-item id is given at once (the get-item animation over Link's head); a
//! negative one is a chest's, opened with A (`En_Box`); `GI_NONE` is something to pick up.

use crate::actor::Actor;
use crate::item::{GI_MAX, GI_NONE};
use crate::play::PlayState;

// `PLAYER_STATE1_*` that stop Player taking items (`Actor_OfferGetItem`).
const STATE1_7: u32 = 1 << 7;
const STATE1_11: u32 = 1 << 11;
const STATE1_12: u32 = 1 << 12;
const STATE1_13: u32 = 1 << 13;
const STATE1_14: u32 = 1 << 14;
const STATE1_18: u32 = 1 << 18;
const STATE1_19: u32 = 1 << 19;
const STATE1_20: u32 = 1 << 20;
const STATE1_21: u32 = 1 << 21;
const STATE1_29: u32 = 1 << 29;

/// `Actor_OfferGetItem`: the updating actor (`play.cur_actor`) offers
/// `get_item_id` if Player can take it (not dead, charging, hanging, jumping, falling, in first
/// person or climbing, and holding no explosive), and it's within `xz_range` and `y_range`. A
/// held actor or the talk target may offer while Player holds or waits; others only when
/// Player is free (`PLAYER_STATE1_CARRYING_ACTOR`, `_29`). Of several offers in a frame the last one wins,
/// except that a `GI_NONE` offer (something to pick up) must be more squarely in front of
/// Link than the last (`getItemDirection`, reset to 0x6000 each Player update).
pub fn offer_get_item_range(play: &mut PlayState, actor: &Actor, get_item_id: i16, xz_range: f32, y_range: f32) -> bool {
    let (Some(ph), Some(me)) = (play.player, play.cur_actor) else { return false };
    let Some(player_yaw) = play.actors.actor(ph).map(|a| a.shape_rot.y) else { return false };
    let Some(pi) = play.actors.get_mut(ph).and_then(|p| p.as_player_mut()) else { return false };
    let s1 = pi.state_flags1();
    // (Player_GetExplosiveHeld: Player holds no bombs here.)
    if s1 & (STATE1_7 | STATE1_12 | STATE1_13 | STATE1_14 | STATE1_18 | STATE1_19 | STATE1_20 | STATE1_21) != 0 {
        return false;
    }
    let (target, _) = pi.talk_target();
    let held_or_target = pi.holds_actor() || target == Some(me);
    if !((held_or_target && get_item_id > GI_NONE && get_item_id < GI_MAX) || s1 & (STATE1_11 | STATE1_29) == 0) {
        return false;
    }
    if actor.xz_dist_to_player < xz_range && actor.y_dist_to_player.abs() < y_range {
        let abs_yaw_diff = (actor.yaw_towards_player.wrapping_sub(player_yaw) as i32).abs() as i16;
        if get_item_id != GI_NONE || pi.get_item_direction() < abs_yaw_diff {
            pi.set_get_item(me, get_item_id, abs_yaw_diff);
            return true;
        }
    }
    false
}

/// `Actor_OfferGetItemNearby`: `Actor_OfferGetItem` within 50 across and 10 up or down.
pub fn offer_get_item(play: &mut PlayState, actor: &Actor, get_item_id: i16) {
    offer_get_item_range(play, actor, get_item_id, 50.0, 10.0);
}

/// `Actor_OfferCarry`: something to pick up (`GI_NONE`).
pub fn offer_pick_up(play: &mut PlayState, actor: &Actor) {
    offer_get_item(play, actor, GI_NONE);
}

/// `Actor_HasParent`: Player (or whoever) has taken it.
pub fn actor_has_parent(actor: &Actor) -> bool {
    actor.parent.is_some()
}
