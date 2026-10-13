//! GAME-07 milestone 5 against the C: the House of Skulltula's family (`kinsuta`, entered at
//! `ENTR_HOUSE_OF_SKULLTULA_0`), cursed (`En_Ssh`, `z_en_ssh.c`: params 0 the father at (0, 280,
//! 20), the children 1 to 5 round the room at y 280) and cured (`En_Sth`, `z_en_sth.c`: the same
//! params on the floor under them), by the Gold Skulltula tokens held (`gsTokens`).
//!
//! Expected values are worked out from the C in the comments. Where a run can't reach a state
//! reliably (a hit on its back while it turns to face Link), the test sets it directly and says so.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, BTN_B, PadState};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_ssh::{self, EnSsh, INFTABLE_196, INFTABLE_197};
use oot_actors::en_sth::{self, EnSth};
use oot_actors::player::Action as PA;
use oot_game::actor::{COLORFILTER_BUFFLAG_OPA, COLORFILTER_COLORFLAG_BLUE};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::*;
use oot_game::collision_check as cc;
use oot_game::message::{TEXT_STATE_AWAITING_NEXT, TEXT_STATE_DONE, TEXT_STATE_DONE_HAS_NEXT, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::sys_matrix::{MtxF, binang_to_rad};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const NONE: PadState = PadState { button: 0, stick_x: 0, stick_y: 0 };
const A: PadState = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };
const B: PadState = PadState { button: BTN_B, stick_x: 0, stick_y: 0 };

struct Run {
    w: PlayState,
    prev: PadState,
}

impl Run {
    /// Child Link (`deku-tree-dead`: the Kokiri Sword and the Deku Shield) entered at
    /// `ENTR_HOUSE_OF_SKULLTULA_0` holding `tokens`, `edit` applied last, the sound log on; three
    /// frames on (the room's actors in, `En_Sth`'s object found and its setup run).
    fn enter(tokens: i16, edit: impl FnOnce(&mut SaveContext)) -> Option<Run> {
        let a = assets()?;
        let e = a.scenes.entrance_index("ENTR_HOUSE_OF_SKULLTULA_0").expect("entrance");
        let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
        save.apply_preset("deku-tree-dead").unwrap();
        save.inventory.gs_tokens = tokens;
        edit(&mut save);
        let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
        let w = PlayState::play_init_with(a.clone(), data()?, rules()?, save, audio).expect("Play_Init");
        let mut r = Run { w, prev: NONE };
        r.frames(NONE, 3);
        Some(r)
    }

    fn frames(&mut self, pad: PadState, n: usize) {
        for _ in 0..n {
            self.w.tick_with(scripted_input(self.prev, pad));
            self.prev = pad;
        }
    }

    fn press(&mut self, pad: PadState) {
        self.frames(pad, 1);
        self.frames(NONE, 1);
    }

    /// Runs frames with no input until `done`, at most `n`; the frames run.
    fn until(&mut self, n: usize, done: impl Fn(&Run) -> bool) -> usize {
        for i in 0..n {
            if done(self) {
                return i;
            }
            self.frames(NONE, 1);
        }
        assert!(done(self), "not done in {n} frames");
        n
    }

    /// Through the message box until `done` (at most `n` frames): A on each box waiting for it.
    fn advance(&mut self, n: usize, done: impl Fn(&Run) -> bool) {
        for _ in 0..n {
            if done(self) {
                return;
            }
            if matches!(self.w.message_state(), TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_DONE | TEXT_STATE_AWAITING_NEXT | TEXT_STATE_EVENT) {
                self.press(A);
            } else {
                self.frames(NONE, 1);
            }
        }
        assert!(done(self), "not done in {n} frames: text {:#x}, state {}", self.w.msg_ctx.text_id, self.w.message_state());
    }

    /// The live `T`s by params.
    fn by_params<T: ActorImpl>(&self) -> Vec<(i16, ActorHandle)> {
        let mut v: Vec<_> = self.w.actors.all().into_iter().filter_map(|h| self.w.actors.downcast::<T>(h).filter(|a| !a.base().killed).map(|a| (a.base().params, h))).collect();
        v.sort();
        v
    }

    fn params<T: ActorImpl>(&self) -> Vec<i16> {
        self.by_params::<T>().into_iter().map(|(p, _)| p).collect()
    }

    #[track_caller]
    fn one<T: ActorImpl>(&self, params: i16) -> ActorHandle {
        self.by_params::<T>().into_iter().find(|&(p, _)| p == params).map(|(_, h)| h).expect("the member")
    }

    #[track_caller]
    fn ssh(&self, h: ActorHandle) -> &EnSsh {
        self.w.actors.downcast::<EnSsh>(h).expect("En_Ssh")
    }

    #[track_caller]
    fn ssh_mut(&mut self, h: ActorHandle) -> &mut EnSsh {
        self.w.actors.downcast_mut::<EnSsh>(h).expect("En_Ssh")
    }

    #[track_caller]
    fn sth(&self, h: ActorHandle) -> &EnSth {
        self.w.actors.downcast::<EnSth>(h).expect("En_Sth")
    }

    fn sfx_on(&self, frame: u32, id: u16) -> bool {
        self.w.audio.log.as_ref().unwrap().sfx.iter().any(|&(f, s, _)| f == frame && s == id)
    }

    /// Link `dist` in front of `h` (along its home facing), facing it, on the floor.
    fn link_in_front(&mut self, h: ActorHandle, dist: f32) {
        let a = self.w.actors.actor(h).unwrap();
        let (home, yaw) = (a.home_pos, a.home_rot.y);
        let p = Vec3::new(home.x + sin_s(yaw) * dist, 0.0, home.z + cos_s(yaw) * dist);
        let (y, _) = self.w.col.entity_raycast_down(Vec3::new(p.x, 100.0, p.z));
        self.w.place_player(Vec3::new(p.x, y, p.z), yaw.wrapping_add(i16::MIN));
    }
}

#[test]
fn the_family_is_cursed_or_cured_by_the_tokens() {
    // EnSsh_Init: the father gone from 100 tokens, a child from params × 10; EnSth_Init the
    // other way round (too few: Actor_Kill).
    let cases: [(i16, &[i16], &[i16]); 5] = [
        (0, &[0, 1, 2, 3, 4, 5], &[]),
        (9, &[0, 1, 2, 3, 4, 5], &[]),
        (25, &[0, 3, 4, 5], &[1, 2]),
        (50, &[0], &[1, 2, 3, 4, 5]),
        (100, &[], &[0, 1, 2, 3, 4, 5]),
    ];
    for (tokens, ssh, sth) in cases {
        let Some(r) = Run::enter(tokens, |_| {}) else { return };
        assert_eq!(r.params::<EnSsh>(), ssh, "En_Ssh at {tokens} tokens");
        assert_eq!(r.params::<EnSth>(), sth, "En_Sth at {tokens} tokens");
    }
}

#[test]
fn a_cursed_member_hangs_at_its_size_with_its_colliders() {
    let Some(r) = Run::enter(0, |_| {}) else { return };
    // EnSsh_Init → EnSsh_SetColliderScale(0.5, 1) for a child, (0.75, 1) for the father: the
    // scale 0.04 × it, the hover height 40 × it, colliderScale 1.5 × it; each cylinder's shift,
    // radius and height (s16) and the sphere's radius times it (sCylinderInit1 32/50/-24,
    // sCylinderInit2 20/60/-30, the sphere 28). Health 1 (sColChkInfoInit), gravity 0.
    let dims = |s: &EnSsh, i: usize| {
        let d = s.collider_cylinders[i].dim;
        (d.radius, d.height, d.y_shift)
    };
    let child = r.ssh(r.one::<EnSsh>(1));
    assert_eq!((child.actor.scale, child.floor_height_offset, child.collider_scale), (Vec3::splat(0.04 * 0.5), 20.0, 0.75));
    assert_eq!((dims(child, 0), dims(child, 3), child.collider_jnt_sph.elements[0].dim.model_sphere.radius), ((16, 25, -12), (10, 30, -15), 14));
    assert_eq!((child.actor.col_chk_info.health, child.actor.gravity), (1, 0.0));
    let father = r.ssh(r.one::<EnSsh>(0));
    // 50 × 0.75 = 37.5 → 37; -24 × 0.75 = -18; -30 × 0.75 = -22.5 → -22; 28 × 0.75 = 21.
    assert_eq!((father.actor.scale, father.floor_height_offset, father.collider_scale), (Vec3::splat(0.04 * 0.75), 30.0, 1.125));
    assert_eq!((dims(father, 0), dims(father, 3), father.collider_jnt_sph.elements[0].dim.model_sphere.radius), ((24, 37, -18), (15, 45, -22), 21));
    // The body (0) takes arrows, Din's Fire, the hookshot, the hammer, explosives and Deku Nuts;
    // the front (2) metal and hookable.
    assert_eq!(father.collider_cylinders[0].info.ac_dmg_info.dmg_flags, en_ssh::BODY_DMG_FLAGS);
    assert_eq!(father.collider_cylinders[2].base.col_type, cc::COL_MATERIAL_METAL);
    // Link at the door (z 507): far, so each waits on the ceiling (EnSsh_Start → EnSsh_Wait),
    // bobbing about its home at y 280.
    for (_, h) in r.by_params::<EnSsh>() {
        let s = r.ssh(h);
        assert_eq!(s.action, en_ssh::Action::Wait);
        assert!((s.actor.world_pos.y - 280.0).abs() < 2.0, "{}", s.actor.world_pos.y);
    }
}

#[test]
fn a_cured_child_appears_with_its_object_and_dances() {
    let Some(r) = Run::enter(20, |_| {}) else { return };
    // EnSth_Init: object_boj's slot, scale 0.01, ATTENTION_RANGE_6, not drawn, EnSth_WaitForObject;
    // the object in, EnSth_SetupAfterObjectLoaded: drawn, gHylianMan2Skel dancing
    // sChildDanceAnim, its reward not taken: EnSth_RewardUnobtainedWait.
    let s = r.sth(r.one::<EnSth>(2));
    assert_eq!((s.actor.scale, s.actor.target_mode, s.drawn, s.update2), (Vec3::splat(0.01), 6, true, true));
    assert_eq!(s.action, en_sth::Action::RewardUnobtainedWait);
    assert_eq!(s.skeleton.as_ref().map(|k| k.limbs.len()), Some(15));
    assert!(s.skel.anim_length > 1.0);
    assert_eq!(s.actor.obj_bank_index, r.w.object_ctx.get_index(en_sth::OBJECT_BOJ));
    assert_eq!(s.collider.dim.radius, 30);
}

#[test]
fn a_cured_childs_text_follows_its_reward_and_the_tokens() {
    // Its bit set (EVENTCHKINF_SKULLTULA_REWARD_10: eventChkInf[0xD] bit 10): the 10's child
    // only talks (sRewardObtainedWaitActions: EnSth_ChildRewardObtainedWait), 0x20 under 50
    // tokens, 0x1F from 50; the 20's, unrewarded, offers its reward with 0x21.
    for (tokens, text) in [(30, 0x20), (60, 0x1F)] {
        let Some(mut r) = Run::enter(tokens, |s| s.set_event_chk_inf(en_sth::EVENTCHKINF_SKULLTULA_REWARD_10)) else { return };
        r.frames(NONE, 1);
        let s = r.sth(r.one::<EnSth>(1));
        assert_eq!((s.action, s.actor.text_id), (en_sth::Action::ChildRewardObtainedWait, text));
        let s = r.sth(r.one::<EnSth>(2));
        assert_eq!((s.action, s.actor.text_id), (en_sth::Action::RewardUnobtainedWait, 0x21));
    }
    // The father: 0x28 unrewarded; and his reward's bit is 0, so he's never rewarded.
    let Some(mut r) = Run::enter(100, |s| s.event_chk_inf[en_sth::EVENTCHKINF_INDEX_SKULLTULA_REWARD] = 0xFFFF) else { return };
    r.frames(NONE, 1);
    let s = r.sth(r.one::<EnSth>(0));
    assert_eq!((s.action, s.actor.text_id, s.event_flag), (en_sth::Action::RewardUnobtainedWait, 0x28, 0));
}

#[test]
fn a_cured_child_gives_its_reward_and_sets_its_bit() {
    let Some(mut r) = Run::enter(10, |_| {}) else { return };
    let h = r.one::<EnSth>(1);
    assert_eq!(r.w.save.cur_upg_value(oot_game::item::UPG_WALLET), 0);
    // Link 50 in front of the 10's child, facing it: within 100 it offers its talk, 0x21.
    r.link_in_front(h, 50.0);
    r.until(10, |r| r.w.player().target_actor == Some(h));
    assert_eq!(r.w.actors.actor(h).unwrap().text_id, 0x21);
    // A: EnSth_RewardUnobtainedTalk.
    r.press(A);
    r.until(10, |r| r.w.msg_ctx.text_id == 0x21 && r.w.player().action == PA::Talk);
    assert_eq!(r.sth(h).action, en_sth::Action::RewardUnobtainedTalk);
    // 0x21's event and A: the box closed, EnSth_GiveReward, the item offered
    // (EnSth_GivePlayerItem: params 1 with no wallet upgrade, GI_WALLET_ADULT).
    r.advance(600, |r| r.sth(h).action != en_sth::Action::RewardUnobtainedTalk);
    assert_eq!(r.sth(h).action, en_sth::Action::GiveReward);
    // Link takes it (Actor_HasParent): the bit set, EnSth_RewardObtainedTalk; the adult's wallet
    // (Item_Give: UPG_WALLET 1).
    r.until(30, |r| r.sth(h).action != en_sth::Action::GiveReward);
    assert_eq!(r.sth(h).action, en_sth::Action::RewardObtainedTalk);
    assert!(r.w.save.get_event_chk_inf(en_sth::EVENTCHKINF_SKULLTULA_REWARD_10));
    assert_eq!(r.w.save.event_chk_inf[en_sth::EVENTCHKINF_INDEX_SKULLTULA_REWARD] & (1 << 10), 1 << 10);
    assert!(r.sth(h).actor.parent.is_none());
    r.advance(600, |r| r.w.save.cur_upg_value(oot_game::item::UPG_WALLET) == 1);
    // The get-item text closes: EnSth_ChildRewardObtainedWait, 0x20 (under 50 tokens).
    r.advance(600, |r| r.sth(h).action == en_sth::Action::ChildRewardObtainedWait);
    r.until(60, |r| r.w.message_state() == TEXT_STATE_NONE);
    r.frames(NONE, 1);
    assert_eq!(r.w.actors.actor(h).unwrap().text_id, 0x20);
}

#[test]
fn the_10s_and_30s_children_give_the_next_wallet_up() {
    // EnSth_GivePlayerItem: params 1 and 3 by CUR_UPG_VALUE(UPG_WALLET): 0 the adult's wallet,
    // 1 the giant's, else their own (sGetItemIds: GI_WALLET_ADULT, GI_WALLET_GIANT).
    assert_eq!(en_sth::S_GET_ITEM_IDS, [0x56, en_sth::GI_WALLET_ADULT, en_sth::GI_STONE_OF_AGONY, en_sth::GI_WALLET_GIANT, en_sth::GI_BOMBCHUS_10, 0x3E]);
    assert_eq!((en_sth::GI_WALLET_ADULT, en_sth::GI_WALLET_GIANT, en_sth::GI_STONE_OF_AGONY, en_sth::GI_BOMBCHUS_10), (0x45, 0x46, 0x39, 0x03));
    let Some(mut r) = Run::enter(30, |s| s.inventory_change_upgrade(oot_game::item::UPG_WALLET, 1)) else { return };
    let h = r.one::<EnSth>(3);
    r.link_in_front(h, 50.0);
    r.until(10, |r| r.w.player().target_actor == Some(h));
    r.press(A);
    r.advance(600, |r| r.sth(h).action == en_sth::Action::GiveReward);
    r.until(30, |r| r.sth(h).action != en_sth::Action::GiveReward);
    r.advance(600, |r| r.w.save.cur_upg_value(oot_game::item::UPG_WALLET) == 2);
    assert!(r.w.save.get_event_chk_inf(en_sth::EVENTCHKINF_SKULLTULA_REWARD_30));
}

/// Link under the father (40 in front of his home), until he's down and offering to talk.
fn father_down(r: &mut Run) -> ActorHandle {
    let h = r.one::<EnSsh>(0);
    r.link_in_front(h, 40.0);
    // EnSsh_Wait: within 160 across and below: EnSsh_Drop (10 a frame), EnSsh_Land (at floor + 30),
    // EnSsh_Idle.
    r.until(120, |r| r.ssh(h).action == en_ssh::Action::Idle && r.ssh(h).anim_timer == 0);
    r.frames(NONE, 1);
    h
}

/// A talk to `h` from the A press to the box closed; the texts shown.
fn talk(r: &mut Run, h: ActorHandle) -> Vec<u16> {
    r.until(10, |r| r.w.player().target_actor == Some(h));
    r.press(A);
    let mut texts = Vec::new();
    for _ in 0..1000 {
        if r.w.message_state() != TEXT_STATE_NONE && texts.last() != Some(&r.w.msg_ctx.text_id) {
            texts.push(r.w.msg_ctx.text_id);
        }
        if !texts.is_empty() && r.w.message_state() == TEXT_STATE_NONE && r.w.player().action != PA::Talk {
            return texts;
        }
        if matches!(r.w.message_state(), TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_DONE | TEXT_STATE_AWAITING_NEXT | TEXT_STATE_EVENT) {
            r.press(A);
        } else {
            r.frames(NONE, 1);
        }
    }
    panic!("the talk never ended: {texts:x?}");
}

#[test]
fn the_cursed_father_drops_lands_and_talks() {
    let Some(mut r) = Run::enter(0, |_| {}) else { return };
    let h = r.one::<EnSsh>(0);
    r.link_in_front(h, 40.0);
    // Link within 160 across and below: EnSsh_Drop at 10 a frame; two frames from his hover
    // height (floor + 40 × 0.75) EnSsh_SetLandAnimation puts him on it, and EnSsh_Land rises
    // (towards 2 a frame) until he's at or over it: EnSsh_Idle.
    r.until(60, |r| r.ssh(h).action == en_ssh::Action::Drop);
    assert_eq!(r.ssh(h).actor.velocity.y, -10.0);
    r.until(60, |r| r.ssh(h).action == en_ssh::Action::Idle);
    let s = r.ssh(h);
    let over = s.actor.world_pos.y - (s.actor.floor_height + 30.0);
    assert!((0.0..2.0).contains(&over), "{} over {}", s.actor.world_pos.y, s.actor.floor_height);
    let h = father_down(&mut r);
    // Under 10 tokens, INFTABLE_196 not set: 0x26.
    assert_eq!(r.ssh(h).actor.text_id, 0x26);
    // Talked to: EVENTCHKINF_96 (the father), INFTABLE_196 (0x26); then 0x27.
    let texts = talk(&mut r, h);
    assert_eq!(texts.first(), Some(&0x26));
    assert!(r.w.save.get_event_chk_inf(en_ssh::EVENTCHKINF_96));
    assert!(r.w.save.get_inf_table(INFTABLE_196));
    assert!(!r.w.save.get_inf_table(INFTABLE_197));
    r.until(20, |r| r.ssh(h).action == en_ssh::Action::Idle);
    r.frames(NONE, 1);
    assert_eq!(r.w.actors.actor(h).unwrap().text_id, 0x27);
}

#[test]
fn the_cursed_fathers_text_follows_the_tokens() {
    // EnSsh_Idle: from 10 tokens 0x25 (0x24 with INFTABLE_197), from 50 0x29; a child 0x22.
    for (tokens, inf197, text) in [(10, false, 0x25), (10, true, 0x24), (49, false, 0x25), (50, false, 0x29)] {
        let Some(mut r) = Run::enter(tokens, |s| {
            if inf197 {
                s.set_inf_table(INFTABLE_197);
            }
        }) else {
            return;
        };
        let h = father_down(&mut r);
        assert_eq!(r.w.actors.actor(h).unwrap().text_id, text, "{tokens} tokens");
    }
    let Some(mut r) = Run::enter(0, |_| {}) else { return };
    let h = r.one::<EnSsh>(4);
    r.link_in_front(h, 40.0);
    r.until(120, |r| r.ssh(h).action == en_ssh::Action::Idle && r.ssh(h).anim_timer == 0);
    r.frames(NONE, 1);
    assert_eq!(r.w.actors.actor(h).unwrap().text_id, 0x22);
    // Talking to a child sets neither the father's flag nor the tables.
    let texts = talk(&mut r, h);
    assert_eq!(texts.first(), Some(&0x22));
    assert!(!r.w.save.get_event_chk_inf(en_ssh::EVENTCHKINF_96));
    assert!(!r.w.save.get_inf_table(INFTABLE_196));
}

#[test]
fn a_slash_in_front_sways_a_cursed_child_on_its_thread() {
    let Some(mut r) = Run::enter(0, |_| {}) else { return };
    let h = r.one::<EnSsh>(4);
    r.link_in_front(h, 40.0);
    r.until(120, |r| r.ssh(h).action == en_ssh::Action::Idle && r.ssh(h).anim_timer == 0);
    r.frames(NONE, 8);
    // B: the Kokiri Sword drawn and slashed at it as it faces Link: the front's cylinder (2).
    let mut n = 0;
    while r.ssh(h).sway_timer == 0 {
        r.frames(if n < 2 { B } else { NONE }, 1);
        n += 1;
        assert!(n < 30, "the slash never landed");
    }
    let f = r.w.audio.frames;
    // EnSsh_CheckHitFront: invincibilityTimer 8 (7 after EnSsh_SetColliders), swayTimer 60;
    // no stun, no damage, no sound. Then its draw's EnSsh_Sway: swayAngle 0x640, swayTimer 59,
    // the angle (s16) 59/6 × (0x10000/360) × sin(0x640), across its facing from its ceiling
    // (its distance below it the thread), leaning by twice the angle.
    let s = r.ssh(h);
    assert_eq!((s.sway_timer, s.sway_angle, s.invincibility_timer, s.stun_timer, s.actor.col_chk_info.health), (59, 0x640, 7, 0, 1));
    assert!(!r.sfx_on(f, en_ssh::NA_SE_VO_ST_DAMAGE));
    let angle = (59.0f32 * (1.0 / 6.0) * (65536.0f32 / 360.0) * sin_s(0x640)) as i32 as i16;
    assert_eq!(s.actor.shape_rot.z, -(angle * 2));
    let len = s.actor.world_pos.y - s.ceiling_pos.y;
    let c = s.ceiling_pos;
    let mut m = MtxF::set_translate(c.x, c.y, c.z);
    m.rotate_y(binang_to_rad(s.actor.world_rot.y));
    let p = m.mult_vec3f(Vec3::new(sin_s(angle) * len, cos_s(angle) * len, 0.0));
    assert_eq!((s.actor.world_pos.x, s.actor.world_pos.z), (p.x, p.z));
    // 59 more frames of it, then swayAngle 0, back under its thread.
    for k in 1..=59 {
        r.frames(NONE, 1);
        assert_eq!(r.ssh(h).sway_timer, 59 - k, "frame {k}");
    }
    assert_eq!(r.ssh(h).sway_angle, 0);
    assert_eq!(r.ssh(h).actor.shape_rot.z, 0);
}

#[test]
fn struck_on_its_back_a_cursed_child_is_stunned_then_spins() {
    let Some(mut r) = Run::enter(0, |_| {}) else { return };
    let h = r.one::<EnSsh>(4);
    r.link_in_front(h, 40.0);
    r.until(120, |r| r.ssh(h).action == en_ssh::Action::Idle && r.ssh(h).anim_timer == 0);
    r.frames(NONE, 4);
    // It turns to face Link (EnSsh_Turn), so a slash lands on its front: the back's cylinder (1)
    // hit is set directly, as CollisionCheck_AC leaves it.
    r.ssh_mut(h).collider_cylinders[1].base.ac_flags |= cc::AC_HIT;
    r.frames(NONE, 1);
    let f = r.w.audio.frames;
    // EnSsh_CheckHitBack: invincibilityTimer 8, hitCount 1, NA_SE_EN_GOMA_JR_FREEZE and
    // NA_SE_VO_ST_DAMAGE, EnSsh_SetStunned (stunTimer 120, SSH_STATE_ATTACKED) and
    // SSH_STATE_STUNNED. Then EnSsh_Damaged: blue for 120 (Actor_SetColorFilter(BLUE, 200, OPA)),
    // stunTimer 119, maxTurnRate 0 → 1000 (by a tenth, at most 0x3E8); EnSsh_SetColliders: 7.
    let s = r.ssh(h);
    assert_eq!((s.stun_timer, s.hit_count, s.invincibility_timer, s.max_turn_rate, s.spin_timer), (119, 1, 7, 1000, 0));
    assert_eq!(s.state_flags & (en_ssh::SSH_STATE_STUNNED | en_ssh::SSH_STATE_ATTACKED), en_ssh::SSH_STATE_STUNNED | en_ssh::SSH_STATE_ATTACKED);
    assert!(r.sfx_on(f, NA_SE_EN_GOMA_JR_FREEZE) && r.sfx_on(f, en_ssh::NA_SE_VO_ST_DAMAGE));
    assert_eq!(s.actor.color_filter_params, COLORFILTER_COLORFLAG_BLUE | COLORFILTER_BUFFLAG_OPA | ((200 & 0xF8) << 5) | 120);
    assert_eq!(s.actor.col_chk_info.health, 1);
    let yaw = s.actor.world_rot.y;
    // Stunned for 118 more frames: no action, no turn (EnSsh_Stunned's turn needs stunTimer 0);
    // shaking (±0x7D0) for the last 30.
    r.frames(NONE, 118);
    let s = r.ssh(h);
    assert_eq!((s.stun_timer, s.spin_timer, s.actor.world_rot.y), (1, 0, yaw));
    assert_eq!(s.actor.shape_rot.y, yaw.wrapping_add(0x7D0));
    // The 120th frame: stunTimer 0, SSH_STATE_STUNNED off, a 30-frame spin, NA_SE_EN_STALTU_ROLL
    // and NA_SE_VO_ST_ATTACK; EnSsh_Turn the same frame: spinTimer 29, the yaw + 10000 × 29/30.
    r.frames(NONE, 1);
    let f = r.w.audio.frames;
    let s = r.ssh(h);
    assert_eq!((s.stun_timer, s.spin_timer, s.state_flags & en_ssh::SSH_STATE_STUNNED), (0, 29, 0));
    assert!(r.sfx_on(f, NA_SE_EN_STALTU_ROLL) && r.sfx_on(f, en_ssh::NA_SE_VO_ST_ATTACK));
    let yaw = (yaw as f32 + 10000.0 * (29.0 / 30.0)) as i32 as i16;
    assert_eq!(s.actor.world_rot.y, yaw);
    assert_eq!(s.actor.shape_rot.y, yaw);
    // Next frame EnSsh_UpdateColliderScale: SSH_STATE_SPIN, EnSsh_SetColliderScale(0.5, 2): the
    // radius 16 × 0.5 × 2 = 16, the height 25 × 0.5 = 12, the shift -12 × 0.5 = -6 (multiplied
    // again: the bug); the OC cylinders 10, 15, -7; the sphere 7.
    r.frames(NONE, 1);
    let s = r.ssh(h);
    let d = |i: usize| (s.collider_cylinders[i].dim.radius, s.collider_cylinders[i].dim.height, s.collider_cylinders[i].dim.y_shift);
    assert_eq!(s.state_flags & en_ssh::SSH_STATE_SPIN, en_ssh::SSH_STATE_SPIN);
    assert_eq!((d(0), d(3), s.collider_jnt_sph.elements[0].dim.model_sphere.radius), ((16, 12, -6), (10, 15, -7), 7));
    let yaw2 = (yaw as f32 + 10000.0 * (28.0 / 30.0)) as i32 as i16;
    assert_eq!(s.actor.world_rot.y, yaw2);
    // The spin ends 28 frames on; the frame after, EnSsh_SetColliderScale(0.5, 1): halved again
    // (8, 6, -3; 5, 7, -3; 3).
    r.until(40, |r| r.ssh(h).state_flags & en_ssh::SSH_STATE_SPIN == 0);
    let s = r.ssh(h);
    let d = |i: usize| (s.collider_cylinders[i].dim.radius, s.collider_cylinders[i].dim.height, s.collider_cylinders[i].dim.y_shift);
    assert_eq!((d(0), d(3), s.collider_jnt_sph.elements[0].dim.model_sphere.radius), ((8, 6, -3), (5, 7, -3), 3));
    assert_eq!(s.actor.scale, Vec3::splat(0.02));
}
