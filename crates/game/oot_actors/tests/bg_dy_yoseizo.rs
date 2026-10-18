//! `Bg_Dy_Yoseizo` and `En_Dy_Extra` (GAME-09 milestone 6) against the C (`z_bg_dy_yoseizo.c`,
//! `z_en_dy_extra.c`), with `Door_Warp1`'s orange warp (`z_door_warp1.c`): the Great Fairy's init
//! by spawn and fountain, her wait for switch 0x38, the new-acquire choice per reward, the reward
//! cutscene's start, the reward cues' effects on the save (the magic fountain's and a spell
//! fountain's), her effects and draw, the beam, the orange warp's growth and fade; then the magic
//! cutscene run whole from the lullaby's switch to her exit, and a revisit (one-point 8603, the
//! refill, the beam) run whole.
//!
//! The cues are put on her channel by hand (`play->csCtx.actorCues[0]`) for the unit cases, the
//! fairy updated alone as `Actor_UpdateAll` would. Expected values are worked out from the C in
//! the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::{IVec3, Vec3};
use oot_actors::PlayExt;
use oot_actors::bg_dy_yoseizo::*;
use oot_actors::door_warp1::{self, DoorWarp1, WARP_ORANGE};
use oot_actors::en_dy_extra::{self, EnDyExtra};
use oot_actors::en_ex_item::{EXITEM_MAGIC_WIND, EnExItem};
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::message::*;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// `CS_STATE_RUN` (`cutscene.h`).
const CS_STATE_RUN: u8 = 2;
/// Her placement in both fountains (`daiyousei_izumi_room_0`, `yousei_izumi_yoko_room_0`).
const FAIRY_AT: Vec3 = Vec3::new(-19.0, -2.0, -1000.0);

/// The fountain at `entrance` (its spawn the reward) on the `goron-ruby` preset (no magic) with
/// `edit` applied, the sound log on, a few frames in.
fn fountain(a: &Arc<GameAssets>, entrance: &str, edit: impl Fn(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("goron-ruby").unwrap();
    edit(&mut save);
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    w
}

fn find(w: &PlayState) -> Option<ActorHandle> {
    w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgDyYoseizo>(h).is_some())
}

fn fairy(w: &PlayState, h: ActorHandle) -> &BgDyYoseizo {
    w.actors.downcast::<BgDyYoseizo>(h).expect("the Great Fairy")
}

fn fairy_mut(w: &mut PlayState, h: ActorHandle) -> &mut BgDyYoseizo {
    w.actors.downcast_mut::<BgDyYoseizo>(h).expect("the Great Fairy")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Her cue `action` on channel 0, the script at `frame`.
fn cue(w: &mut PlayState, action: u16, frame: u16) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.frames = frame;
    let p = IVec3::new(-47, 15, -986);
    w.cs_ctx.npc_actions[0] = Some(CsCmdActorCue { action, start_frame: frame, end_frame: frame + 10, rot: [0; 3], start_pos: p, end_pos: p, normal: IVec3::ZERO });
}

fn of_kind<T: ActorImpl + 'static>(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<T>(h).is_some() && !w.actors.actor(h).unwrap().killed).collect()
}

/// She's in `Reward_GiveReward` (cue 3 seen), grown, as the cutscene leaves her.
fn giving(w: &mut PlayState, h: ActorHandle) {
    let f = fairy_mut(w, h);
    f.action = Action::RewardGiveReward;
    f.scale = 0.035;
    f.drawn = true;
}

#[test]
fn her_reward_is_the_spawn_and_her_pose_the_fountains() {
    let Some(a) = assets() else { return };
    // The magic fountain from Death Mountain (spawn 0): reward 0 (magic); her home -2, 40 above it
    // 38; the focus at her position; SkelAnime_InitFlex with gGreatFairySittingTransitionAnim
    // (looped); BgDyYoseizo_WaitSong (NTSC 1.1 and later); no draw (the profile has none).
    let w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    assert_eq!(w.scene_id, SCENE_GREAT_FAIRYS_FOUNTAIN_MAGIC);
    let h = find(&w).expect("placed");
    let f = fairy(&w, h);
    assert_eq!((f.reward, f.home_pos_y, f.above_fountain_y, f.action, f.drawn, f.scale), (0, -2.0, 38.0, Action::WaitSong, false, 0.0));
    assert!(f.skel.is("gGreatFairySittingTransitionAnim"));
    assert_eq!(f.skel.mode, oot_game::skelanime_std::ANIMMODE_LOOP);
    // The crater's spawn (1, through entrance list 1): double magic; Ganon's Castle's (2): double
    // defence.
    for (e, r) in [("ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_1", 1), ("ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_2", 2)] {
        let w = fountain(&a, e, |_| {});
        assert_eq!(fairy(&w, find(&w).unwrap()).reward, r, "{e}");
    }
    // A spell fountain: lying down (gGreatFairyLayingDownTransitionAnim); Hyrule Castle's spawn 1
    // is Din's Fire.
    let w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_SPELLS_1", |_| {});
    assert_eq!(w.scene_id, SCENE_GREAT_FAIRYS_FOUNTAIN_SPELLS);
    let f = fairy(&w, find(&w).unwrap());
    assert_eq!(f.reward, BG_DY_YOSEIZO_REWARD_SPELL_DINS_FIRE);
    assert!(f.skel.is("gGreatFairyLayingDownTransitionAnim"));
    assert_eq!(f.actor.world_pos, FAIRY_AT);
}

#[test]
fn she_waits_for_the_lullabys_switch_and_without_magic_only_the_first_comes() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    let h = find(&w).unwrap();
    // No switch 0x38: she waits.
    for _ in 0..5 {
        update(&mut w, h);
    }
    assert_eq!(fairy(&w, h).action, Action::WaitSong);
    // Zelda's Lullaby played (En_Okarina_Tag 0x1CB8 sets switch 0x38): OCARINA_MODE_04, Link
    // held (PLAYER_CSACTION_1); without magic the magic fountain's spawn 0 still comes.
    w.flags.set_switch(0x38);
    w.msg_ctx.ocarina_mode = OCARINA_MODE_03;
    update(&mut w, h);
    assert_eq!((fairy(&w, h).action, w.msg_ctx.ocarina_mode, w.player().cs_mode), (Action::SetupCutscene, OCARINA_MODE_04, 1));
    assert!(!fairy(&w, h).actor.killed);
    // The crater's fountain (double magic) without magic: killed.
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_1", |_| {});
    let h = find(&w).unwrap();
    w.flags.set_switch(0x38);
    update(&mut w, h);
    assert!(fairy(&w, h).actor.killed);
    assert_eq!(fairy(&w, h).action, Action::WaitSong);
    // ... and with it she comes.
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_1", |s| s.is_magic_acquired = true);
    let h = find(&w).unwrap();
    w.flags.set_switch(0x38);
    update(&mut w, h);
    assert_eq!((fairy(&w, h).action, fairy(&w, h).actor.killed), (Action::SetupCutscene, false));
    // A spell fountain without magic: killed, whatever the spawn.
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_SPELLS_0", |_| {});
    let h = find(&w).unwrap();
    w.flags.set_switch(0x38);
    update(&mut w, h);
    assert!(fairy(&w, h).actor.killed);
}

#[test]
fn a_new_reward_is_chosen_by_fountain_spawn_and_save() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    let h = find(&w).unwrap();
    // The magic fountain: magic new without isMagicAcquired (BREG(2) 0); double magic without
    // isDoubleMagicAcquired; double defence without isDoubleDefenseAcquired; each sets
    // isNewAcquireMagic.
    let cases: [(i16, fn(&mut SaveContext), bool); 6] = [
        (0, |_| {}, true),
        (0, |s| s.is_magic_acquired = true, false),
        (1, |s| s.is_magic_acquired = true, true),
        (1, |s| s.is_double_magic_acquired = true, false),
        (2, |_| {}, true),
        (2, |s| s.is_double_defense_acquired = true, false),
    ];
    for (reward, edit, new) in cases {
        let save = w.save.clone();
        edit(&mut w.save);
        let f = fairy_mut(&mut w, h);
        f.reward = reward;
        f.is_new_acquire_magic = false;
        let mut taken = w.actors.take(h).unwrap();
        let f = taken.as_any_mut().downcast_mut::<BgDyYoseizo>().unwrap();
        assert_eq!((f.is_new_acquire(&w), f.is_new_acquire_magic), (new, new), "magic fountain reward {reward}");
        w.actors.put_back(h, taken);
        w.save = save;
    }
    // A spell fountain: new while its ITEMGETINF is clear; isNewAcquireMagic stays false.
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_SPELLS_0", |s| s.is_magic_acquired = true);
    let h = find(&w).unwrap();
    for (reward, flag) in [(0, ITEMGETINF_FARORES_WIND), (1, ITEMGETINF_DINS_FIRE), (2, ITEMGETINF_NAYRUS_LOVE)] {
        for given in [false, true] {
            let save = w.save.clone();
            if given {
                w.save.set_item_get_inf(flag);
            }
            fairy_mut(&mut w, h).reward = reward;
            let mut taken = w.actors.take(h).unwrap();
            let f = taken.as_any_mut().downcast_mut::<BgDyYoseizo>().unwrap();
            assert_eq!((f.is_new_acquire(&w), f.is_new_acquire_magic), (!given, false), "spell {reward}, given {given}");
            w.actors.put_back(h, taken);
            w.save = save;
        }
    }
    // ITEMGETINF_INDEX_SPELLS (1): the flags are bits 8 to 10 of itemGetInf[1].
    w.save.set_item_get_inf(ITEMGETINF_NAYRUS_LOVE);
    assert_ne!(w.save.item_get_inf[1] & (1 << 10), 0);
}

#[test]
fn a_new_reward_starts_its_cutscene_but_not_on_a_cutscene_layer() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    let h = find(&w).unwrap();
    w.flags.set_switch(0x38);
    update(&mut w, h);
    // BgDyYoseizo_SetupCutscene: magic new: play->csCtx.script = gGreatFairyMagicCs,
    // cutsceneTrigger 1, BgDyYoseizo_Reward_WaitCutscene; still not drawn.
    update(&mut w, h);
    let f = fairy(&w, h);
    assert_eq!((f.action, f.drawn, f.is_new_acquire_magic), (Action::RewardWaitCutscene, false, true));
    assert_eq!((w.cs_ctx.segment.as_ref().unwrap().name.as_str(), w.save.cutscene_trigger), ("gGreatFairyMagicCs", 1));
    // Her cutscene by reward and fountain.
    assert_eq!(BgDyYoseizo::reward_cutscene(&w, 1), Some("gGreatFairyDoubleMagicCs"));
    assert_eq!(BgDyYoseizo::reward_cutscene(&w, 2), Some("gGreatFairyDoubleDefenseCs"));
    let ws = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_SPELLS_0", |_| {});
    assert_eq!([0, 1, 2].map(|r| BgDyYoseizo::reward_cutscene(&ws, r)), [Some("gGreatFairyFaroresWindCs"), Some("gGreatFairyDinsFireCs"), Some("gGreatFairyNayrusLoveCs")]);
    // Waiting: cue 2 (with a script running) shows her, Link held, the transition once,
    // NA_SE_EV_GREAT_FAIRY_APPEAR.
    w.save.cutscene_trigger = 0;
    let n = w.audio.log.as_ref().unwrap().sfx.len();
    cue(&mut w, 1, 50);
    update(&mut w, h);
    assert_eq!(fairy(&w, h).action, Action::RewardWaitCutscene);
    cue(&mut w, 2, 90);
    update(&mut w, h);
    let f = fairy(&w, h);
    assert_eq!((f.action, f.drawn, f.is_done_appearing), (Action::RewardAppear, true, false));
    assert!(f.skel.is("gGreatFairySittingTransitionAnim"));
    assert_eq!(f.skel.mode, oot_game::skelanime_std::ANIMMODE_ONCE);
    assert!(w.audio.log.as_ref().unwrap().sfx[n..].iter().any(|&(_, s, _)| s == NA_SE_EV_GREAT_FAIRY_APPEAR));
    // A cutscene layer (IS_CUTSCENE_LAYER, a debug feature): no script started, but she waits all
    // the same.
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    let h = find(&w).unwrap();
    w.save.scene_layer = 4;
    w.cs_ctx.segment = None;
    w.flags.set_switch(0x38);
    update(&mut w, h);
    update(&mut w, h);
    assert_eq!(fairy(&w, h).action, Action::RewardWaitCutscene);
    assert!(w.cs_ctx.segment.is_none() && w.save.cutscene_trigger == 0);
}

#[test]
fn she_appears_spinning_up_to_her_size_and_spawns_two_sparkles_a_frame() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    let h = find(&w).unwrap();
    {
        let f = fairy_mut(&mut w, h);
        f.action = Action::RewardAppear;
        f.drawn = true;
    }
    cue(&mut w, 2, 100);
    // Math_ApproachF(&scale, 0.035, scaleApproachFraction (0 at first), 0.005): nothing the first
    // frame; the fractions grow (0.02 a frame for y to 0.8, 0.05 at most for the scale to 0.2);
    // shape.rot.y += 0xBB8 while under 0.034.
    update(&mut w, h);
    assert_eq!((fairy(&w, h).scale, fairy(&w, h).actor.shape_rot.y), (0.0, 0xBB8));
    let mut frames = 1;
    while fairy(&w, h).scale < 0.01 {
        update(&mut w, h);
        frames += 1;
    }
    // From 0.01 big (SpawnEffectsType checks before the next spawn): two type 0 sparkles a frame,
    // 0.4 big, 90 frames, white inside, (155, 255, 255) outside, alpha 30 after their first update.
    let count = |w: &PlayState| fairy(w, h).effects.iter().filter(|e| e.active != 0).count();
    let before = count(&w);
    update(&mut w, h);
    assert_eq!(count(&w), before + 2);
    let e = fairy(&w, h).effects.iter().rev().find(|e| e.active != 0).copied().unwrap();
    assert_eq!((e.ty, e.scale, e.life, e.inner_color, e.outer_color, e.alpha), (0, 0.4, 89, [255, 255, 255], [155, 255, 255], 30));
    // Grown (0.034) and spinning to face front (0x3E8 a frame, Math_ApproachS by a fifth), then
    // done appearing; the transition then plays out to the idle loop (gGreatFairySittingAnim).
    for _ in 0..200 {
        if fairy(&w, h).is_done_appearing {
            break;
        }
        update(&mut w, h);
        frames += 1;
    }
    let f = fairy(&w, h);
    assert!(f.is_done_appearing && f.scale >= 0.034, "after {frames} frames: scale {} rot {}", f.scale, f.actor.shape_rot.y);
    assert!(f.actor.shape_rot.y.abs() < 50);
    for _ in 0..200 {
        update(&mut w, h);
        if fairy(&w, h).has_switched_anim {
            break;
        }
    }
    assert!(fairy(&w, h).skel.is("gGreatFairySittingAnim"));
    // Cue 3: the giving animation once, the mouth open, Reward_GiveReward.
    cue(&mut w, 3, 409);
    update(&mut w, h);
    let f = fairy(&w, h);
    assert_eq!((f.action, f.mouth_tex_index, f.has_switched_anim), (Action::RewardGiveReward, 1, false));
    assert!(f.skel.is("gGreatFairyGivingUpgradeAnim"));
    // The draw: the skeleton with eye and mouth (sEyeTextures[eyeTexIndex] on 8, mouth 1 on 10),
    // then a billboard per active sparkle.
    let view = ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let mut out = DrawOut::default();
    f.draw(&f.render_state(), &w, &view, &mut out);
    assert_eq!(out.opa.len(), 1);
    assert_eq!(out.opa[0].mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake(&bake_name(f.eye_tex_index as usize, 1))));
    assert_eq!(out.xlu.len(), f.effects.iter().filter(|e| e.active == 1).count());
    assert!(out.xlu.iter().all(|c| c.mesh == eng_gfx::MeshKey::named(oot_game::pack::keys::bake(PARTICLE_BAKE))));
}

#[test]
fn the_magic_fountains_reward_cues() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |s| s.health = 0x10);
    let h = find(&w).unwrap();
    giving(&mut w, h);
    // Cue 4 (434..499): her light, effects of type n + 1 = 1 (yellow, 0.2 big, 50 frames) over her
    // head, homing on Link, each with NA_SE_EV_HEALING.
    cue(&mut w, 4, 440);
    update(&mut w, h);
    let lights: Vec<_> = fairy(&w, h).effects.iter().filter(|e| e.active != 0 && e.ty == 1).copied().collect();
    assert_eq!(lights.len(), 2);
    assert_eq!((lights[0].scale, lights[0].life, lights[0].inner_color, lights[0].outer_color), (0.2, 49, [255, 255, 100], [255, 255, 100]));
    let n = w.audio.log.as_ref().unwrap().sfx.len();
    update(&mut w, h);
    assert!(w.audio.log.as_ref().unwrap().sfx[n..].iter().any(|&(_, s, _)| s == NA_SE_EV_HEALING - oot_game::audio::sfx::SFX_FLAG));
    // Cue 0x13 (499..569): an orange Door_Warp1 (19 - (19 - WARP_ORANGE)) at Link, once.
    let link = w.player().actor.world_pos;
    cue(&mut w, 0x13, 499);
    update(&mut w, h);
    update(&mut w, h);
    let warps = of_kind::<DoorWarp1>(&w);
    assert_eq!(warps.len(), 1);
    let warp = w.actors.downcast::<DoorWarp1>(warps[0]).unwrap();
    assert_eq!((door_warp1::TYPE.get(warp.actor.params), warp.actor.world_pos), (WARP_ORANGE, link));
    assert_eq!(fairy(&w, h).unk304, 1);
    // Cue 0x0A (569..720): the magic: isMagicAcquired, magicFillTarget 0x30, the HUD's hearts and
    // magic (HUD_VISIBILITY_HEARTS_MAGIC); once, healthAccumulator 0x140 (no Magic_Fill: only double
    // defence's).
    assert!(!w.save.is_magic_acquired);
    cue(&mut w, 0x0A, 569);
    update(&mut w, h);
    assert!(w.save.is_magic_acquired);
    assert_eq!((w.save.magic_fill_target, w.save.hud_visibility_mode, w.save.health_accumulator, fairy(&w, h).energy_refilled), (0x30, HUD_VISIBILITY_HEARTS_MAGIC, 0x140, true));
    assert_eq!(w.save.magic_state, oot_game::magic::MAGIC_STATE_IDLE);
    w.save.health_accumulator = 0;
    update(&mut w, h);
    assert_eq!(w.save.health_accumulator, 0);
    // Cue 0x0D: the disappear.
    cue(&mut w, 0x0D, 890);
    update(&mut w, h);
    assert_eq!(fairy(&w, h).action, Action::SetupDisappear);

    // Double magic (cue 11): acquired, the double meter (0x60), magicLevel 0 (Interface_Update grows
    // the meter again).
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_1", |s| {
        s.is_magic_acquired = true;
        s.magic_level = 1;
    });
    let h = find(&w).unwrap();
    giving(&mut w, h);
    cue(&mut w, 0x0B, 600);
    update(&mut w, h);
    assert!(w.save.is_double_magic_acquired);
    assert_eq!((w.save.magic_fill_target, w.save.magic_level), (0x60, 0));
    // Double defence (cue 12): acquired, and with the refill Magic_Fill (MAGIC_STATE_FILL to the
    // whole meter); cue 18 fills the defence hearts one a frame, to 20.
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_2", |s| {
        s.is_magic_acquired = true;
        s.magic_level = 1;
    });
    let h = find(&w).unwrap();
    giving(&mut w, h);
    cue(&mut w, 0x0C, 600);
    update(&mut w, h);
    assert!(w.save.is_double_defense_acquired);
    assert_eq!((w.save.magic_state, w.save.magic_fill_target), (oot_game::magic::MAGIC_STATE_FILL, 0x30));
    assert_eq!(w.save.inventory.defense_hearts, 0);
    cue(&mut w, 18, 700);
    update(&mut w, h);
    assert_eq!(w.save.inventory.defense_hearts, 1);
    cue(&mut w, 7, 720);
    for _ in 0..30 {
        update(&mut w, h);
    }
    assert_eq!(w.save.inventory.defense_hearts, 20);
}

#[test]
fn a_spell_fountains_reward_cues() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_SPELLS_0", |s| s.is_magic_acquired = true);
    let h = find(&w).unwrap();
    giving(&mut w, h);
    // Cue 4 in a spell fountain: a Demo_Effect light, once, at her, sSpellRewardsDemoEffectLightColor[0]
    // (green, 2) << 12 | DEMO_EFFECT_LIGHT (0x12).
    cue(&mut w, 4, 300);
    update(&mut w, h);
    update(&mut w, h);
    let lights: Vec<_> = w.actors.all().into_iter().filter(|&hh| w.actors.actor(hh).is_some_and(|x| x.id == oot_actors::demo_effect::ACTOR_DEMO_EFFECT)).collect();
    assert_eq!(lights.len(), 1);
    assert_eq!(w.actors.actor(lights[0]).unwrap().params, 0x2012);
    assert!(fairy(&w, h).has_spawned_demo_effect);
    // Cue 14: Farore's Wind (EXITEM_MAGIC_WIND) as her child 53 over a child's head; magic filled
    // (already acquired), healthAccumulator 0x140, ITEMGETINF_FARORES_WIND, Item_Give.
    let link = w.player().actor.world_pos;
    cue(&mut w, 14, 400);
    update(&mut w, h);
    let item = fairy(&w, h).item.expect("spawned");
    let e = w.actors.downcast::<EnExItem>(item).unwrap();
    assert_eq!((e.ty, e.actor.world_pos), (EXITEM_MAGIC_WIND, link + Vec3::Y * 53.0));
    assert_eq!(e.actor.parent, Some(h));
    assert!(w.save.get_item_get_inf(ITEMGETINF_FARORES_WIND));
    assert_eq!((w.save.health_accumulator, w.save.magic_state), (0x140, oot_game::magic::MAGIC_STATE_FILL));
    assert_eq!(w.save.inv_content(oot_game::item::ITEM_FARORES_WIND), oot_game::item::ITEM_FARORES_WIND);
    // On it: kept over Link's head at 0.3.
    w.player_mut().actor.world_pos.x += 10.0;
    update(&mut w, h);
    if let Some(e) = w.actors.downcast::<EnExItem>(item) {
        assert_eq!((e.actor.world_pos.x, e.scale), (link.x + 10.0, 0.3));
    }
    // Cue 17: put away.
    cue(&mut w, 17, 500);
    update(&mut w, h);
    assert!(fairy(&w, h).item.is_none());
    // Without magic, the spell gives it (no fill).
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_SPELLS_2", |s| s.is_magic_acquired = false);
    let h = find(&w).unwrap();
    giving(&mut w, h);
    cue(&mut w, 16, 400);
    update(&mut w, h);
    assert!(w.save.is_magic_acquired && w.save.get_item_get_inf(ITEMGETINF_NAYRUS_LOVE));
    assert_eq!(w.save.magic_state, oot_game::magic::MAGIC_STATE_IDLE);
}

#[test]
fn the_orange_warp_grows_then_fades() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    let at = Vec3::new(-20.0, 10.0, -790.0);
    let h = w.actor_spawn(oot_actors::boss_goma::ACTOR_DOOR_WARP1, at, [0; 3], WARP_ORANGE).unwrap();
    let warp = |w: &PlayState| w.actors.downcast::<DoorWarp1>(h).unwrap().clone_state();
    // DoorWarp1_SetupWarp for the coloured warps: scale 0, unk_1AE -140, unk_1B0 -80, unk_194
    // 0.23, unk_198 0.6, both alphas 0; DoorWarp1_WarpAppear.
    let s = warp(&w);
    assert_eq!((s.0, s.1, s.2, s.3, s.4, s.5, s.6), (door_warp1::Action::WarpAppear, 0, -140, -80, 0.23, 0.6, (0.0, 0.0)));
    // The coloured warps' growth: unk_1AE by 4 while under -50 (23 frames to -48), unk_1B0 by 4
    // while under 70 (38 frames to 72), the alphas Math_SmoothStepToF(255, 0.4, 10, 0.01); the
    // scale stays 0. The 39th frame: func_809998A4.
    let (mut ray, mut alpha) = (0.0f32, 0.0f32);
    for k in 1..=38 {
        update(&mut w, h);
        eng_math::smooth_step_to_f(&mut ray, 255.0, 0.4, 10.0, 0.01);
        eng_math::smooth_step_to_f(&mut alpha, 255.0, 0.4, 10.0, 0.01);
        let s = warp(&w);
        assert_eq!((s.0, s.2, s.3, s.6), (door_warp1::Action::WarpAppear, (-140 + 4 * k.min(23)) as i16, (-80 + 4 * k) as i16, (ray, alpha)), "frame {k}");
    }
    update(&mut w, h);
    eng_math::smooth_step_to_f(&mut ray, 255.0, 0.4, 10.0, 0.01);
    eng_math::smooth_step_to_f(&mut alpha, 255.0, 0.4, 10.0, 0.01);
    let s = warp(&w);
    assert_eq!((s.0, s.1, s.2, s.3), (door_warp1::Action::Func809998A4, 0, -48, 72));
    // func_809998A4: both alphas down by Math_SmoothStepToF(0, 0.1, 2, 0.01): 2 a frame from near
    // 255, NA_SE_EV_WARP_HOLE while the rays show.
    let n = w.audio.log.as_ref().unwrap().sfx.len();
    update(&mut w, h);
    eng_math::smooth_step_to_f(&mut ray, 0.0, 0.1, 2.0, 0.01);
    eng_math::smooth_step_to_f(&mut alpha, 0.0, 0.1, 2.0, 0.01);
    assert_eq!(warp(&w).6, (ray, alpha));
    assert!(w.audio.log.as_ref().unwrap().sfx[n..].iter().any(|&(_, s, _)| s == oot_game::audio::sfx::NA_SE_EV_WARP_HOLE - oot_game::audio::sfx::SFX_FLAG));
    for _ in 0..400 {
        update(&mut w, h);
    }
    let s = warp(&w);
    assert_eq!((s.0, s.6), (door_warp1::Action::Func809998A4, (0.0, 0.0)));
    // Gone dark: no more sound.
    let n = w.audio.log.as_ref().unwrap().sfx.len();
    update(&mut w, h);
    assert!(!w.audio.log.as_ref().unwrap().sfx[n..].iter().any(|&(_, s, _)| s == oot_game::audio::sfx::NA_SE_EV_WARP_HOLE - oot_game::audio::sfx::SFX_FLAG));
}

/// What the orange warp's test reads of a `Door_Warp1`.
trait WarpState {
    #[allow(clippy::type_complexity)]
    fn clone_state(&self) -> (door_warp1::Action, i16, i16, i16, f32, f32, (f32, f32));
}

impl WarpState for DoorWarp1 {
    fn clone_state(&self) -> (door_warp1::Action, i16, i16, i16, f32, f32, (f32, f32)) {
        (self.action, self.scale, self.unk_1ae, self.unk_1b0, self.unk_194, self.unk_198, (self.light_ray_alpha, self.warp_alpha))
    }
}

#[test]
fn the_beam_sinks_waits_its_60_frames_then_fades_when_told() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    let at = Vec3::new(-20.0, 210.0, -790.0);
    let h = w.actor_spawn(en_dy_extra::ACTOR_EN_DY_EXTRA, at, [0; 3], 0).unwrap();
    fn beam(w: &PlayState, h: ActorHandle) -> &EnDyExtra {
        w.actors.downcast::<EnDyExtra>(h).unwrap()
    }
    // EnDyExtra_Init: scale (0.025, 0.039, 0.025), gravity -0.2, alphaFac 1, timer 60.
    let b = beam(&w, h);
    assert_eq!((b.ty, b.scale, b.actor.gravity, b.alpha_fac, b.timer, b.home_pos), (0, Vec3::new(0.025, 0.039, 0.025), -0.2, 1.0, 60, at));
    // The update: timer down, the scale on the actor, the hum, then the action
    // (Math_ApproachF(&gravity, 0, 0.1, 0.005): -0.195), then Actor_MoveXZGravity (velocity.y
    // -0.195).
    let n = w.audio.log.as_ref().unwrap().sfx.len();
    update(&mut w, h);
    let b = beam(&w, h);
    assert_eq!((b.timer, b.actor.scale), (59, b.scale));
    assert!((b.actor.gravity + 0.195).abs() < 1e-6);
    assert!((b.actor.velocity.y + 0.195).abs() < 1e-6);
    assert!(w.audio.log.as_ref().unwrap().sfx[n..].iter().any(|&(_, s, _)| s == en_dy_extra::NA_SE_PL_SPIRAL_HEAL_BEAM - oot_game::audio::sfx::SFX_FLAG));
    // Told to go before its 60 frames: it waits them out.
    w.actors.downcast_mut::<EnDyExtra>(h).unwrap().start_disappearing = 1;
    for _ in 0..58 {
        update(&mut w, h);
    }
    assert_eq!(beam(&w, h).action, en_dy_extra::Action::Wait);
    update(&mut w, h);
    assert_eq!((beam(&w, h).action, beam(&w, h).timer), (en_dy_extra::Action::Disappear, 200));
    // Fading: Math_ApproachZeroF(&alphaFac, 0.03, 0.05) (0.03 of it a frame), gone under 0.02.
    update(&mut w, h);
    assert!((beam(&w, h).alpha_fac - 0.97).abs() < 1e-6);
    // The draw's alphas: 0, (s8)(alphaFac × 240), (s8)(alphaFac × 255) as bytes.
    assert_eq!(EnDyExtra::alpha_vals(1.0), [0, 240, 255]);
    assert_eq!(EnDyExtra::alpha_vals(0.5), [0, 120, 127]);
    let mut k = 0;
    while !beam(&w, h).actor.killed {
        update(&mut w, h);
        k += 1;
        assert!(k < 200);
    }
    // 0.97^n < 0.02: n = 129 (log 0.02 / log 0.97 = 128.4), seen the update after.
    assert!((128..=131).contains(&k), "{k}");
    // Under y -55 its fall stops.
    let h = w.actor_spawn(en_dy_extra::ACTOR_EN_DY_EXTRA, Vec3::new(0.0, -60.0, 0.0), [0; 3], 1).unwrap();
    update(&mut w, h);
    assert!((w.actors.downcast::<EnDyExtra>(h).unwrap().actor.velocity.y + 0.195).abs() < 1e-6);
    update(&mut w, h);
    let b = w.actors.downcast::<EnDyExtra>(h).unwrap();
    assert!((b.actor.velocity.y - b.actor.gravity).abs() < 1e-6, "velocity {} gravity {}", b.actor.velocity.y, b.actor.gravity);
}

/// The world's run with A pressed on its own frame whenever a box waits; each frame `each` sees
/// the state. Stops when `done` says so (or after `limit` frames).
fn run_cutscene(w: &mut PlayState, limit: usize, mut each: impl FnMut(&PlayState) -> bool) -> usize {
    let mut prev = PadState::default();
    for k in 0..limit {
        let st = w.message_state();
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let mut pad = PadState::default();
        if waits && prev.button == 0 {
            pad.button = BTN_A;
        }
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if each(w) {
            return k + 1;
        }
    }
    limit
}

#[test]
fn the_magic_cutscene_runs_from_the_lullabys_switch_to_her_exit() {
    let Some(a) = assets() else { return };
    // Entered from the summit (ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0) after the Ruby: no magic.
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |_| {});
    assert!(!w.save.is_magic_acquired);
    idle(&mut w, 30);
    let h = find(&w).unwrap();
    let tag = w.actors.all().into_iter().find(|&t| w.actors.actor(t).is_some_and(|x| x.id == oot_actors::en_okarina_tag::ACTOR_EN_OKARINA_TAG)).expect("the lullaby's spot");
    // The lullaby played on the Triforce: En_Okarina_Tag 0x1CB8 sets switch 0x38.
    w.flags.set_switch(0x38);
    let notes = w.player().notes.len();
    let mut changes: Vec<(u16, Action)> = Vec::new();
    let mut texts: Vec<u16> = Vec::new();
    let (mut started, mut warp_at, mut magic_at, mut drawn_at, mut stopped_at) = (false, None, None, None, None);
    let mut last = None;
    let frames = run_cutscene(&mut w, 2000, |w| {
        let frame = w.cs_ctx.frames;
        if w.cs_ctx.segment.as_ref().is_some_and(|s| s.name == "gGreatFairyMagicCs") && w.cs_ctx.state != CS_STATE_IDLE {
            started = true;
        }
        if w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if started && stopped_at.is_none() && w.cs_ctx.state == CS_STATE_IDLE {
            stopped_at = Some(texts.len());
        }
        if warp_at.is_none() && !of_kind::<DoorWarp1>(w).is_empty() {
            warp_at = Some(frame);
        }
        if magic_at.is_none() && w.save.is_magic_acquired {
            magic_at = Some(frame);
        }
        let Some(f) = w.actors.downcast::<BgDyYoseizo>(h) else { return true };
        if drawn_at.is_none() && f.drawn {
            drawn_at = Some(frame);
        }
        if last != Some(f.action) {
            changes.push((frame, f.action));
            last = Some(f.action);
        }
        f.actor.killed
    });
    for c in &changes {
        eprintln!("frame {:5} {:?}", c.0, c.1);
    }
    eprintln!("texts {texts:x?}, {frames} frames; warp at {warp_at:?}, magic at {magic_at:?}, drawn at {drawn_at:?}");
    assert!(started, "the script never ran");
    // Player's cues 0x05, 0x4B, 0x4C and 0x13 (PLAYER_CSACTION_8, 77, 19 and 28) and the fairy's
    // PLAYER_CSACTION_1: nothing Player doesn't do.
    assert_eq!(w.player().notes[notes..], [] as [String; 0], "Player's notes");
    let actions: Vec<Action> = changes.iter().map(|c| c.1).collect();
    assert_eq!(
        actions,
        vec![Action::SetupCutscene, Action::RewardWaitCutscene, Action::RewardAppear, Action::RewardGiveReward, Action::SetupDisappear, Action::Disappear, Action::Kill]
    );
    let at = |a: Action| changes.iter().find(|c| c.1 == a).map(|c| c.0).unwrap();
    // Her cue 2 at 90 shows her, cue 3 at 409 the giving, cue 13 at 890 the disappear (each seen
    // the update after the script sets it, a frame or two late).
    assert!((90..=92).contains(&drawn_at.unwrap()), "{drawn_at:?}");
    assert!((409..=411).contains(&at(Action::RewardGiveReward)));
    assert!((890..=892).contains(&at(Action::SetupDisappear)));
    // The orange warp on cue 0x13 (499) and the magic on cue 0x0A (569).
    assert!((499..=501).contains(&warp_at.unwrap()), "{warp_at:?}");
    assert!((569..=571).contains(&magic_at.unwrap()), "{magic_at:?}");
    // The script's texts, in order.
    for t in [0x00D7, 0x00DD, 0x00E7, 0x00EF] {
        assert!(texts.contains(&t), "text {t:#x} in {texts:x?}");
    }
    // CS_START_SEQ 0x25 at 700: Audio_PlaySequenceInCutscene(0x24), NA_BGM_HEART_GET, a fanfare
    // (SEQ_FLAG_FANFARE): Audio_PlayFanfare's sequence. (Its start, Audio_UpdateFanfare, waits in
    // the test on the scene's audio reset, which the logged audio never finishes.)
    assert_eq!(w.audio.fanfare_seq_id, 0x24);
    // Her exit (BgDyYoseizo_Kill): Link let go (PLAYER_CSACTION_7, then 0), the light setting back,
    // the lullaby's spot killed, switch 0x38 cleared.
    assert!(w.actors.actor(h).is_none_or(|x| x.killed));
    assert!(w.actors.actor(tag).is_none_or(|x| x.killed));
    assert!(!w.flags.get_switch(0x38));
    assert_eq!(w.env_ctx.light_setting_override, 0);
    // The script stops (CS_MISC_STOP_CUTSCENE at 960) as she goes: Player's PLAYER_CSACTION_7
    // ends his cutscene mode once it's idle.
    let mut k = 0;
    while w.cs_ctx.state != CS_STATE_IDLE {
        idle(&mut w, 1);
        k += 1;
        assert!(k < 100);
    }
    idle(&mut w, 2);
    assert_eq!(w.player().cs_mode, 0);
    // The meter: magic acquired, magicFillTarget 0x30; out of the cutscene Interface_Update grows it
    // (magicLevel 1, the capacity to 0x30) and fills it.
    assert_eq!(w.save.magic_fill_target, 0x30);
    idle(&mut w, 40);
    assert_eq!((w.save.magic_level, w.save.magic_capacity, w.save.magic), (1, 0x30, 0x30));
}

#[test]
fn a_revisit_shows_one_point_8603_refills_link_under_the_beam_and_she_goes() {
    let Some(a) = assets() else { return };
    let mut w = fountain(&a, "ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0", |s| {
        s.is_magic_acquired = true;
        s.magic_level = 1;
        s.magic_capacity = 0x30;
        s.magic = 0x10;
        s.health = 0x10;
    });
    idle(&mut w, 30);
    let h = find(&w).unwrap();
    w.flags.set_switch(0x38);
    let mut changes: Vec<(u32, Action)> = Vec::new();
    let mut texts: Vec<u16> = Vec::new();
    let (mut cam_8603, mut beam_seen, mut refill_seen, mut last, mut k) = (false, None, false, None, 0u32);
    let n = w.audio.log.as_ref().unwrap().sfx.len();
    run_cutscene(&mut w, 3000, |w| {
        k += 1;
        if w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        if let Some(c) = w.camera(w.active_cam_id)
            && c.cs_id == 8603
        {
            cam_8603 = true;
            assert_eq!(c.cs_info.key_frame_cnt, 5);
        }
        if let Some(&b) = of_kind::<EnDyExtra>(w).first() {
            let e = w.actors.downcast::<EnDyExtra>(b).unwrap();
            beam_seen.get_or_insert((e.ty, e.actor.parent));
        }
        // Magic_Fill (healthAccumulator 0x140 the interface takes the same frame).
        refill_seen |= w.save.magic_state == oot_game::magic::MAGIC_STATE_FILL;
        let Some(f) = w.actors.downcast::<BgDyYoseizo>(h) else { return true };
        if last != Some(f.action) {
            changes.push((k, f.action));
            last = Some(f.action);
        }
        f.actor.killed
    });
    for c in &changes {
        eprintln!("update {:5} {:?}", c.0, c.1);
    }
    eprintln!("texts {texts:x?}");
    let actions: Vec<Action> = changes.iter().map(|c| c.1).collect();
    assert_eq!(
        actions,
        vec![
            Action::SetupCutscene,
            Action::RevisitSetupAppear,
            Action::RevisitAppear,
            Action::RevisitFinishAppear,
            Action::RevisitSetupTalk,
            Action::RevisitTalk,
            Action::RevisitSetupRefillPlayerEnergy,
            Action::RevisitRefillPlayerEnergy,
            Action::RevisitWaitTalkEnd,
            Action::SetupDisappear,
            Action::Disappear,
            Action::Kill
        ]
    );
    // One-point 8603 (D_801232CC's five keyframes) held the camera; NA_SE_EV_GREAT_FAIRY_APPEAR.
    assert!(cam_8603);
    // 8604's (a spell fountain's) table is there too, five rows: the first from the fountain's
    // side at (-16, 87, -829) towards (98, 24, -714), its timer 45 (z_onepointdemo.c's D_80123394).
    for t in ["D_801232CC", "D_80123394"] {
        assert_eq!(w.onepoint.len(w.onepoint.table(t)), 5, "{t}");
    }
    let k = *w.onepoint.kf(w.onepoint.table("D_80123394"), 0);
    assert_eq!(k.timer_init, 45);
    assert!(w.audio.log.as_ref().unwrap().sfx[n..].iter().any(|&(_, s, _)| s == NA_SE_EV_GREAT_FAIRY_APPEAR));
    // Texts 0xDB then 0xDA; the beam (type 0, her child) and the refill (healthAccumulator 0x140,
    // Magic_Fill): full health and magic.
    assert_eq!(texts.iter().filter(|&&t| t == 0xDB || t == 0xDA).copied().collect::<Vec<_>>(), vec![0xDB, 0xDA]);
    assert_eq!(beam_seen, Some((0, Some(h))));
    assert!(refill_seen);
    assert_eq!((w.save.health, w.save.magic), (w.save.health_capacity, 0x30));
    // The same exit as the reward's.
    assert!(!w.flags.get_switch(0x38));
    assert_eq!(w.env_ctx.light_setting_override, 0);
    idle(&mut w, 2);
    assert_eq!(w.player().cs_mode, 0);
}
