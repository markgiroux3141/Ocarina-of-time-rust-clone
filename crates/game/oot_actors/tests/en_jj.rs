//! GAME-10 milestone 3 against the C: Lord Jabu-Jabu (`z_en_jj.c`) in Zora's Fountain (spot08
//! room 0: `En_Jj` -1 at (-1445, 20, -48), facing -x). His body's collision, his eyes and his
//! groan; the fish let out by him (`Player_IsDroppingFish`), his 100 frames, `gJabuInhalingCs`
//! with the fish's cues (channel 1) and his (channel 2), `EVENTCHKINF_OPENED_JABU_JABU` and the
//! way into Jabu-Jabu's Belly (`CS_DEST_JABU_JABU`); and on later visits his mouth opening.
//!
//! Entered by `ENTR_ZORAS_FOUNTAIN_2` (from Zora's Domain) on the `fairy-magic` save, its intro
//! seen (`EVENTCHKINF_B7`). Expected values are worked out from the C in the comments; nothing
//! asserted depends on a `Rand` result.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_CLEFT, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::eff_dust::{ACTOR_EFF_DUST, EFF_DUST_TYPE_0};
use oot_actors::en_fish::{ACTOR_EN_FISH, EN_FISH_TYPE_DROPPED};
use oot_actors::en_jj::*;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, SFX_FLAG};
use oot_game::collision_check::MASS_IMMOVABLE;
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::env::clock_time;
use oot_game::item::{ITEM_BOTTLE_EMPTY, ITEM_BOTTLE_FISH, ITEM_BOTTLE_MILK_FULL};
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `EVENTCHKINF_B7` (`save.h`: 0xB7): the fountain's intro seen.
const EVENTCHKINF_B7: u16 = 0xB7;

/// The fountain by `ENTR_ZORAS_FOUNTAIN_2`, `tweak` applied to the save, the sound logged.
fn fountain(a: &Arc<GameAssets>, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_FOUNTAIN_2").expect("entrance");
    let mut save = SaveContext::new(e, false, clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_B7);
    tweak(&mut save);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    w.audio.log = Some(Default::default());
    w
}

fn step(w: &mut PlayState, prev: &mut PadState, pad: PadState) {
    w.tick_with(scripted_input(*prev, pad));
    *prev = pad;
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn jjs(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<EnJj>(h).is_some_and(|j| !j.actor.killed)).collect()
}

/// Lord Jabu-Jabu himself (`JABUJABU_MAIN`).
fn main_jj(w: &PlayState) -> ActorHandle {
    let v: Vec<_> = jjs(w).into_iter().filter(|&h| jj(w, h).ty() == JABUJABU_MAIN).collect();
    assert_eq!(v.len(), 1, "one Lord Jabu-Jabu");
    v[0]
}

fn jj(w: &PlayState, h: ActorHandle) -> &EnJj {
    w.actors.downcast::<EnJj>(h).expect("an En_Jj")
}

fn sfx_frames(w: &PlayState, id: u16) -> Vec<u32> {
    w.audio.log.as_ref().unwrap().sfx.iter().filter(|&&(_, s, _)| s == id).map(|&(f, _, _)| f).collect()
}

/// The meshes the last frame draws.
fn drawn(w: &PlayState) -> Vec<String> {
    let f = w.current_frame();
    let mut out = oot_game::play::DrawOut::default();
    w.draw(&f, &oot_game::play::ViewInfo::new(f.view.eye, glam::Mat4::IDENTITY), &mut out);
    out.opa.iter().chain(out.xlu.iter()).map(|c| c.mesh.name.clone()).collect()
}

/// Link on the floor at `(x, z)`, facing `yaw`.
fn place(w: &mut PlayState, x: f32, z: f32, yaw: i16) {
    let (y, _) = w.col.entity_raycast_down(Vec3::new(x, 200.0, z));
    w.place_player(Vec3::new(x, y, z), yaw);
}

#[test]
fn jabu_jabu_and_his_body() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, |_| {});
    idle(&mut w, 3);
    let all = jjs(&w);
    // The placed -1 (JABUJABU_MAIN) and the JABUJABU_COLLISION it spawns as its child: no
    // JABUJABU_UNUSED_COLLISION.
    assert_eq!(all.len(), 2);
    let h = main_jj(&w);
    let j = jj(&w, h);
    let body_h = j.body_collision_actor.expect("his body's collision");
    let body = jj(&w, body_h);
    assert_eq!((body.ty(), body.action), (JABUJABU_COLLISION, Action::StaticCollision));
    assert_eq!(body.actor.parent, Some(h));
    // Actor_SpawnAsChild at (x - 10, y, z), rot (0, world.rot.y, 0).
    assert_eq!(j.actor.world_pos, Vec3::new(-1445.0, 20.0, -48.0));
    assert_eq!(body.actor.world_pos, Vec3::new(-1455.0, 20.0, -48.0));
    assert_eq!(body.actor.shape_rot.y, -0x4000);
    // sInitChain: 0.087, the culling volume (4000, 3300, 1100), both; no shadow.
    for x in [j, body] {
        assert_eq!((x.actor.scale.x, x.actor.culling_volume_distance, x.actor.culling_volume_scale, x.actor.culling_volume_downward), (0.087, 4000.0, 3300.0, 1100.0));
        assert_eq!(x.actor.shape_shadow_draw, oot_game::shadow::ShadowDraw::None);
    }
    // gJabuJabuHeadCol and gJabuJabuBodyCol registered, the body's ceilings off, both colliding.
    let (bg, body_bg) = (j.dyna_bg_id().expect("his head's collision"), body.dyna_bg_id().expect("his body's"));
    assert!(w.col.dyna.is_bg_actor(bg) && w.col.dyna.is_bg_actor(body_bg));
    assert!(w.col.dyna.actors[body_bg as usize].ceiling_disabled && !w.col.dyna.actors[bg as usize].ceiling_disabled);
    assert!(!w.col.dyna.actors[body_bg as usize].collision_disabled && !w.col.dyna.actors[bg as usize].collision_disabled);
    // Without EVENTCHKINF_OPENED_JABU_JABU: waiting for the fish, the cylinder at (-1245, 20,
    // -48), immovable; gJabuJabuSkel's 21 limbs and the root (22 joints).
    assert_eq!(j.action, Action::WaitForFish);
    assert_eq!(j.collider.dim.pos, [-1245, 20, -48]);
    assert_eq!(j.actor.col_chk_info.mass, MASS_IMMOVABLE);
    assert_eq!(j.skel.as_ref().unwrap().limb_count, 22);
    // Drawn as the eye's bake; the body isn't drawn.
    let meshes = drawn(&w);
    assert!(meshes.iter().filter(|m| m.starts_with(&oot_game::pack::keys::bake("En_Jj/"))).count() == 1, "{meshes:?}");
}

#[test]
fn his_eyes_blink_and_he_groans_on_frame_41() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, |_| {});
    // From the update the room's actors are there.
    let mut n = 0;
    while jjs(&w).is_empty() && n < 10 {
        idle(&mut w, 1);
        n += 1;
    }
    let h = main_jj(&w);
    let mut eyes = vec![jj(&w, h).eye_index];
    let mut frames = Vec::new();
    for _ in 0..400 {
        idle(&mut w, 1);
        let j = jj(&w, h);
        eyes.push(j.eye_index);
        frames.push(j.skel.as_ref().unwrap().cur_frame);
    }
    // EnJj_Blink: from blinkTimer 0 (EnJj_Init's) the eye steps half, closed, open an update each;
    // back open, the timer Rand_S16Offset(20, 20) (20 to 39): open for 21 to 40 updates (the
    // reset's and the timer's), then the next blink.
    let start = eyes.iter().position(|&e| e == JABUJABU_EYE_HALF).unwrap();
    assert!(start <= 1 && eyes[..start].iter().all(|&e| e == JABUJABU_EYE_OPEN), "{:?}", &eyes[..4]);
    let eyes = eyes[start..].to_vec();
    assert_eq!(&eyes[..3], &[JABUJABU_EYE_HALF, JABUJABU_EYE_CLOSED, JABUJABU_EYE_OPEN]);
    let mut runs = Vec::new();
    let mut i = 2;
    while i < eyes.len() {
        let start = i;
        while i < eyes.len() && eyes[i] == JABUJABU_EYE_OPEN {
            i += 1;
        }
        if i >= eyes.len() {
            break;
        }
        runs.push(i - start);
        assert_eq!(&eyes[i..(i + 3).min(eyes.len())], &[JABUJABU_EYE_HALF, JABUJABU_EYE_CLOSED, JABUJABU_EYE_OPEN][..(eyes.len() - i).min(3)]);
        i += 2;
    }
    eprintln!("open runs {runs:?}");
    assert!(runs.len() >= 5 && runs.iter().all(|r| (21..=40).contains(r)), "{runs:?}");
    // The groan whenever the animation's frame is 41 before its update.
    let groans = sfx_frames(&w, NA_SE_EV_JABJAB_GROAN);
    let at_41 = frames.iter().filter(|&&f| f == 42.0).count();
    eprintln!("groans {groans:?}, frame 41 passed {at_41} times");
    assert!(!groans.is_empty() && groans.len() == at_41, "{groans:?}");
}

/// The fountain with a bottled fish on C-Left (the milk's bottle), Link `d` west of the feeding
/// spot facing +x (towards Jabu-Jabu). Returns the world settled.
fn with_fish_at(a: &Arc<GameAssets>, d: f32) -> PlayState {
    let mut w = fountain(a, |s| {
        let slot = oot_game::item::slot(ITEM_BOTTLE_MILK_FULL);
        s.inventory.items[slot] = ITEM_BOTTLE_FISH;
        s.equip_item_on_c_left(ITEM_BOTTLE_FISH);
    });
    idle(&mut w, 20);
    place(&mut w, FEEDING_SPOT.x - d, FEEDING_SPOT.z, 0x4000);
    idle(&mut w, 20);
    w
}

fn dropped_fish(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.actor(h).is_some_and(|x| x.id == ACTOR_EN_FISH && x.params == EN_FISH_TYPE_DROPPED && !x.killed)).collect()
}

#[test]
fn a_fish_let_out_by_jabu_jabu_opens_him() {
    let Some(a) = assets() else { return };
    // Link 20 west of the feeding spot (within 300 across of it).
    let mut w = with_fish_at(&a, 20.0);
    let h = main_jj(&w);
    let body_bg = jj(&w, jj(&w, h).body_collision_actor.unwrap()).dyna_bg_id().unwrap();
    assert_eq!(jj(&w, h).action, Action::WaitForFish);
    let mut prev = PadState::default();
    let mut c = PadState::default();
    c.button |= BTN_CLEFT;
    step(&mut w, &mut prev, c);
    // Each update: Player's (ACTORCAT_PLAYER) before his (ACTORCAT_ITEMACTION), so he sees the
    // release the update it starts.
    let mut t = 0;
    let mut release = None;
    let mut seen = None;
    let mut fish_at = None;
    let mut cs_at = None;
    let mut player_actions = Vec::new();
    while t < 400 && cs_at.is_none() {
        let p = w.player();
        if player_actions.last() != Some(&p.action.decomp_name()) {
            player_actions.push(p.action.decomp_name());
        }
        if release.is_none() && p.action.decomp_name() == "Player_Action_8084EFC0" {
            release = Some(t);
        }
        let j = jj(&w, h);
        if seen.is_none() && j.action == Action::BeginCutscene {
            seen = Some(t);
            // EnJj_WaitForFish: cutsceneCountdownTimer 100.
            assert_eq!(j.cutscene_countdown_timer, 100);
        }
        if fish_at.is_none() && !dropped_fish(&w).is_empty() {
            fish_at = Some(t);
        }
        if j.action == Action::RemoveDust {
            cs_at = Some(t);
            break;
        }
        step(&mut w, &mut prev, PadState::default());
        t += 1;
    }
    eprintln!("Player {player_actions:?}; release at {release:?}, seen at {seen:?}, the fish at {fish_at:?}, the cutscene at {cs_at:?}");
    let (release, seen, fish_at, cs_at) = (release.expect("the release"), seen.expect("seen"), fish_at.expect("the fish"), cs_at.expect("the cutscene"));
    assert_eq!(seen, release, "seen the update the release starts");
    // Player_Action_8084EFC0: the fish (EN_FISH_TYPE_DROPPED) on the animation's frame 76, the
    // bottle empty.
    assert!(fish_at > release && fish_at < cs_at);
    let slot = w.save.equips.c_button_slots[0] as usize;
    assert_eq!(w.save.inventory.items[slot], ITEM_BOTTLE_EMPTY);
    // EnJj_BeginCutscene: 100 updates counting down, the 101st starts gJabuInhalingCs.
    assert_eq!(cs_at - seen, 101);
    assert_eq!(w.cs_ctx.segment.as_ref().map(|s| s.name.as_str()), Some(JABU_INHALING_CS));
    assert!(w.save.get_event_chk_inf(EVENTCHKINF_OPENED_JABU_JABU));
    assert!(w.col.dyna.actors[body_bg as usize].collision_disabled, "his body's collision off");
    assert!(!sfx_frames(&w, NA_SE_SY_CORRECT_CHIME).is_empty());
    let fish = dropped_fish(&w)[0];

    // The script: run until its terminator (CS_DEST_JABU_JABU from frame 345).
    let mut by_frame = Vec::new();
    let mut fish_cs = None;
    let mut fish_max_x = f32::MIN;
    let mut fish_gone_at = None;
    let mut end = None;
    for _ in 0..600 {
        // The frame the actors see this update (Cutscene_UpdateScripted runs after Actor_UpdateAll).
        let frame = w.cs_ctx.frames;
        let running = w.cs_ctx.state != CS_STATE_IDLE;
        step(&mut w, &mut prev, PadState::default());
        let j = jj(&w, h);
        by_frame.push((frame, running, j.unk_30a, j.mouth_open_angle, j.dust, j.eye_index, j.extra_blink_counter));
        if fish_cs.is_none() {
            fish_cs = oot_actors::en_fish::statics(&mut w).jabu_cutscene_fish;
        }
        match w.actors.actor(fish) {
            Some(f) if !f.killed => fish_max_x = fish_max_x.max(f.world_pos.x),
            _ => {
                fish_gone_at.get_or_insert(frame);
            }
        }
        if w.transition.trigger == oot_game::transition::TRANS_TRIGGER_START {
            // The script's frame its update reached.
            end = Some(w.cs_ctx.frames);
            break;
        }
    }
    let end = end.expect("the terminator");
    eprintln!("the terminator at {end}; the fish gone at {fish_gone_at:?}, its furthest x {fish_max_x}");
    // CS_DEST_JABU_JABU (24): ENTR_JABU_JABU_0, a black fade.
    assert_eq!(end, 345);
    assert_eq!(Some(w.transition.next_entrance_index), w.entrance_by_name("ENTR_JABU_JABU_0"));
    // The fish let out took channel 1 (EnFish_Update), flew into his mouth on cue 2 (to x -549)
    // and was gone on cue 3 (from frame 281).
    assert_eq!(fish_cs, Some(fish));
    assert!(fish_max_x > -1000.0, "{fish_max_x}");
    assert_eq!(fish_gone_at, Some(281));
    // Cutscene_SetupScripted and CutsceneHandler_StartScript ran the script to its frame 1 on the
    // update he asked for it: from his next update a cue on channel 2 (cue 1), so his action
    // (EnJj_RemoveDust) never runs.
    assert_eq!((by_frame[0].0, by_frame[0].1), (1, true));
    assert!(by_frame.iter().all(|r| r.2 & UNK_30A_DUST_REMOVED == 0));
    // Channel 2: CS_CMD_ACTOR_CUE_2_5's list over 2_4's: cue 2 from frame 91 (curFrame > 90).
    let at = |f: u16| by_frame.iter().find(|r| r.0 == f).copied().unwrap();
    let before = at(90);
    assert_eq!((before.2 & (UNK_30A_BREATHING | UNK_30A_DUST_SPAWNED), before.3, before.4), (0, 0, None));
    let on = at(91);
    assert_eq!((on.2 & (UNK_30A_BREATHING | UNK_30A_DUST_SPAWNED), on.3), (UNK_30A_BREATHING | UNK_30A_DUST_SPAWNED, -102));
    let dust = on.4.expect("the dust");
    let d = w.actors.actor(dust).expect("the dust");
    assert_eq!((d.id, d.params, d.world_pos, d.parent), (ACTOR_EFF_DUST, EFF_DUST_TYPE_0, Vec3::new(-1100.0, 105.0, -27.0), Some(h)));
    // The mouth 102 a frame while at -5200 or more: -5202 from frame 141 (51 updates).
    assert_eq!(at(140).3, -5100);
    assert_eq!(at(141).3, -5202);
    assert_eq!(at(300).3, -5202);
    assert_eq!(jj(&w, h).skel.as_ref().unwrap().joint_table[HEAD_JOINT][2], -5202);
    let breaths = sfx_frames(&w, NA_SE_EV_JABJAB_BREATHE - SFX_FLAG);
    assert!(breaths.len() >= 250, "{}", breaths.len());
    // Cue 3 from frame 331: open, and one more blink after this one (bit 1).
    let c3 = at(331);
    assert!(c3.2 & UNK_30A_BLINKED != 0 && at(330).2 & UNK_30A_BLINKED == 0);
    // The dust lasts (EnJj_RemoveDust never ran).
    assert!(w.actors.actor(dust).is_some_and(|d| !d.killed));
}

#[test]
fn a_fish_let_out_too_far_from_the_feeding_spot_does_nothing() {
    let Some(a) = assets() else { return };
    // 320 west of it: Math_Vec3f_DistXZ(&feedingSpot, Link) >= 300.
    let mut w = with_fish_at(&a, 320.0);
    let h = main_jj(&w);
    let mut prev = PadState::default();
    let mut c = PadState::default();
    c.button |= BTN_CLEFT;
    step(&mut w, &mut prev, c);
    let mut released = false;
    for _ in 0..200 {
        step(&mut w, &mut prev, PadState::default());
        released |= w.player().action.decomp_name() == "Player_Action_8084EFC0";
        assert_eq!(jj(&w, h).action, Action::WaitForFish);
    }
    assert!(released && !dropped_fish(&w).is_empty(), "the fish let out");
    assert!(!w.save.get_event_chk_inf(EVENTCHKINF_OPENED_JABU_JABU));
}

#[test]
fn on_later_visits_his_mouth_opens_for_link() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, |s| s.set_event_chk_inf(EVENTCHKINF_OPENED_JABU_JABU));
    idle(&mut w, 3);
    let h = main_jj(&w);
    let body_bg = jj(&w, jj(&w, h).body_collision_actor.unwrap()).dyna_bg_id().unwrap();
    // EnJj_WaitToOpenMouth: Link (spawn 2, (-2361, 2, 674)) more than 300 across away.
    assert_eq!(jj(&w, h).action, Action::WaitToOpenMouth);
    idle(&mut w, 10);
    assert_eq!((jj(&w, h).action, jj(&w, h).mouth_open_angle), (Action::WaitToOpenMouth, 0));
    // Link on the feeding spot's platform, about 145 from him.
    place(&mut w, -1590.0, -44.0, 0x4000);
    let mut angles = Vec::new();
    for _ in 0..60 {
        idle(&mut w, 1);
        let j = jj(&w, h);
        angles.push((j.action, j.mouth_open_angle, w.col.dyna.actors[body_bg as usize].collision_disabled));
    }
    // The update after he sees Link within 300 (xzDistToPlayer is the last frame's), EnJj_OpenMouth:
    // 102 a frame while at -5200 or more; the body's collision off once below -2600.
    let first = angles.iter().position(|a| a.1 != 0).unwrap();
    assert_eq!(angles[first - 1].0, Action::OpenMouth);
    for (k, a) in angles[first..].iter().enumerate() {
        let want = (-102 * (k as i32 + 1)).max(-5202) as i16;
        assert_eq!(a.1, want, "{k}");
        assert_eq!(a.2, want < -2600, "{k}");
    }
    assert_eq!(jj(&w, h).skel.as_ref().unwrap().joint_table[HEAD_JOINT][2], -5202);
}
