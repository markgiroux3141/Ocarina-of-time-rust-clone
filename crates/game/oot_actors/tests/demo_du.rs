//! `Demo_Du` (GAME-09 milestone 4) against the C (`z_demo_du.c`): the init for each params (the
//! Goron's Ruby ported, the other three placeholders), the Ruby's cues driving each state (the
//! fall and its sounds, the landing's dust, the stand, the chest, the slap's dust and sounds, the
//! Ruby given, the last idle and Link's cry), the face by the script's frame, the draws; then the
//! Ruby's cutscene run whole on Death Mountain Trail's layer 5 with `Demo_Go`'s three Gorons.
//!
//! The cues are put on the cutscene's channels by hand (`play->csCtx.actorCues`), Darunia spawned
//! on the trail's normal layer and updated alone, as `Actor_UpdateAll` would. Expected values are
//! worked out from the C in the comments.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::{IVec3, Vec3};
use oot_actors::PlayExt;
use oot_actors::demo_du::*;
use oot_actors::demo_go::DemoGo;
use oot_game::actor_ctx::{ActorHandle, ActorImpl};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::effect::EFFECT_SS_DUST;
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
/// `OBJECT_DU` (`object_table.h`: 0x009D).
const OBJECT_DU: i16 = 0x009D;
/// Link by the cavern's mouth, where the cutscene holds him (its Player cues' position).
const LINK_AT: Vec3 = Vec3::new(-1656.0, 688.0, -519.0);
/// Darunia's placement on layer 5.
const DARUNIA_AT: Vec3 = Vec3::new(-1799.0, 705.0, -466.0);

/// The cue positions of `gDMTGoronRubyCs`' channel 2: the fall's start (cue 1, 2) and its end (2
/// to 7).
const HIGH: IVec3 = IVec3::new(-1647, 910, -557);
const LOW: IVec3 = IVec3::new(-1652, 677, -564);

/// Death Mountain Trail (`ENTR_DEATH_MOUNTAIN_TRAIL_0`, its normal layer) after the cavern, Link by
/// the cavern's mouth, Darunia's object in, the sound log on.
fn trail(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-boss").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    w.place_player(LINK_AT, 0);
    idle(&mut w, 1);
    if w.object_ctx.get_index(OBJECT_DU).is_none() {
        w.object_ctx.spawn(OBJECT_DU);
    }
    w
}

fn spawn(w: &mut PlayState, params: i16) -> ActorHandle {
    w.actor_spawn(ACTOR_DEMO_DU, DARUNIA_AT, [0; 3], params).expect("Demo_Du spawned")
}

fn du(w: &PlayState, h: ActorHandle) -> &DemoDu {
    w.actors.downcast::<DemoDu>(h).expect("a Demo_Du")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// Darunia's cue on channel 2, from `start` to `end` over `from..to`, at script frame `frame`.
fn cue(w: &mut PlayState, action: u16, from: u16, to: u16, start: IVec3, end: IVec3, frame: u16) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.frames = frame;
    w.cs_ctx.npc_actions[2] = Some(CsCmdActorCue { action, start_frame: from, end_frame: to, rot: [0; 3], start_pos: start, end_pos: end, normal: IVec3::ZERO });
}

/// The script's cue `action` on channel 2 (its frames and positions), at `frame`.
fn script_cue(w: &mut PlayState, action: u16, frame: u16) {
    match action {
        1 => cue(w, 1, 0, 173, HIGH, HIGH, frame),
        2 => cue(w, 2, 173, 183, HIGH, LOW, frame),
        3 => cue(w, 3, 183, 288, LOW, LOW, frame),
        4 => cue(w, 4, 288, 312, LOW, LOW, frame),
        5 => cue(w, 5, 312, 420, LOW, LOW, frame),
        6 => cue(w, 6, 420, 1100, LOW, LOW, frame),
        _ => cue(w, 7, 1100, 1198, LOW, LOW, frame),
    }
}

fn sfx_count(w: &PlayState) -> usize {
    w.audio.log.as_ref().unwrap().sfx.len()
}

/// The sounds asked for since the log held `n`.
fn sfx_since(w: &PlayState, n: usize) -> Vec<u16> {
    w.audio.log.as_ref().unwrap().sfx[n..].iter().map(|&(_, s, _)| s).collect()
}

fn dust(w: &PlayState) -> Vec<oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_DUST).cloned().collect()
}

/// Updates until `update_index` changes (at most `limit` updates); returns how many it took.
fn until_next(w: &mut PlayState, h: ActorHandle, limit: usize) -> usize {
    let from = du(w, h).update_index;
    for k in 1..=limit {
        update(w, h);
        if du(w, h).update_index != from {
            return k;
        }
    }
    panic!("still in {from} after {limit} updates");
}

/// Darunia landed and standing up (GR_04, index 11): spawned, through cues 1 to 3.
fn landed(w: &mut PlayState) -> ActorHandle {
    let h = spawn(w, 1);
    update(w, h);
    script_cue(w, 2, 175);
    update(w, h);
    script_cue(w, 2, 183);
    update(w, h);
    script_cue(w, 3, 184);
    update(w, h);
    assert_eq!(du(w, h).update_index, cs_goronsruby_subscene(4));
    h
}

#[test]
fn the_rubys_darunia_starts_hidden_without_an_animation() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = spawn(&mut w, DEMO_DU_CS_GORONS_RUBY);
    let d = du(&w, h);
    // DemoDu_Init: ActorShape_Init (yOffset 0, the circle shadow 30, shadowAlpha 255), Actor_Init's
    // scale 0.01; DemoDu_InitCs_GoronsRuby: SkelAnime_InitFlex without an animation, updateIndex
    // CS_GORONSRUBY_SUBSCENE(0) = 7, drawIndex 0 (nothing), the eyes and mouth 0.
    assert_eq!((d.actor.scale, d.actor.shape_y_offset, d.actor.shape_shadow_alpha), (Vec3::splat(0.01), 0.0, 255));
    assert_eq!((d.actor.shape_shadow_draw, d.actor.shape_shadow_scale), (oot_game::shadow::ShadowDraw::Circle, 30.0));
    assert!(d.skel.animation.is_none());
    assert_eq!((d.update_index, d.draw_index, d.eye_tex_index, d.mouth_tex_index), (7, 0, 0, 0));
    // DemoDu_UpdateCs_GR_00: on to GR_01 (8) at once, a script or not.
    update(&mut w, h);
    assert_eq!(du(&w, h).update_index, cs_goronsruby_subscene(1));
    // Hidden: DemoDu_Draw_NoDraw; no floor yet (no Actor_UpdateBgCheckInfo): Actor_Draw's shadow
    // has none to lie on.
    let mut out = DrawOut::default();
    let d = du(&w, h);
    let rs = d.render_state();
    d.draw(&rs, &w, &ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY }, &mut out);
    assert!(out.opa.is_empty() && out.xlu.is_empty());
    assert_eq!(rs.shadow.map(|s| s.floor_poly), Some(None));
}

#[test]
fn the_other_cutscenes_are_placeholders_chosen_at_init() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    // DemoDu_Init's switch: 2 the Chamber after Ganon, 3 the credits, 0 and any other the Fire
    // Medallion: not ported, placeholders.
    for params in [DEMO_DU_CS_FIREMEDALLION, DEMO_DU_CS_CHAMBER_AFTER_GANON, DEMO_DU_CS_CREDITS, 4, -1] {
        let h = spawn(&mut w, params);
        assert_eq!(w.actors.get(h).unwrap().name(), "Placeholder", "params {params}");
        assert!(w.actors.downcast::<DemoDu>(h).is_none());
    }
    let h = spawn(&mut w, DEMO_DU_CS_GORONS_RUBY);
    assert_eq!(w.actors.get(h).unwrap().name(), "Demo_Du");
}

#[test]
fn cue_2_drops_him_along_it_and_at_its_end_he_lands_in_dust() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = spawn(&mut w, 1);
    update(&mut w, h);
    // GR_01 on cue 1: frame 160 the fall's whistle (Sfx_PlaySfxCentered2(NA_SE_EV_OBJECT_FALL)).
    let n = sfx_count(&w);
    script_cue(&mut w, 1, 160);
    update(&mut w, h);
    assert!(sfx_since(&w, n).contains(&NA_SE_EV_OBJECT_FALL));
    assert_eq!((du(&w, h).update_index, du(&w, h).draw_index), (8, 0));
    let n = sfx_count(&w);
    script_cue(&mut w, 1, 161);
    update(&mut w, h);
    assert!(!sfx_since(&w, n).contains(&NA_SE_EV_OBJECT_FALL));
    // Frame 174, cue 2 (173..183): GR_01's Link surprised (NA_SE_VO_LI_SURPRISE_KID at Link), then
    // DemoDu_CsGoronsRuby_AdvanceTo02: gDaruniaStandUpAfterFallingAnim set once from 0, GR_02 (9),
    // drawn (1), placed by the lerp: Environment_LerpWeight(183, 173, 174) = 0.1.
    let n = sfx_count(&w);
    script_cue(&mut w, 2, 174);
    update(&mut w, h);
    assert!(sfx_since(&w, n).contains(&NA_SE_VO_LI_SURPRISE_KID));
    let d = du(&w, h);
    assert_eq!((d.update_index, d.draw_index), (9, 1));
    assert!(d.skel.is(ANIM_STAND_UP_AFTER_FALLING));
    assert_eq!((d.skel.cur_frame, d.skel.mode), (0.0, oot_game::skelanime_std::ANIMMODE_ONCE));
    let lerp = |t: f32| Vec3::new(-1647.0 + (-5.0) * t, 910.0 + (-233.0) * t, -557.0 + (-7.0) * t);
    assert!(d.actor.world_pos.distance(lerp(0.1)) < 1e-3, "{}", d.actor.world_pos);
    // GR_02 at 178: halfway (0.5), still falling; the animation held (GR_02 doesn't update it).
    script_cue(&mut w, 2, 178);
    update(&mut w, h);
    let d = du(&w, h);
    assert_eq!(d.update_index, 9);
    assert!(d.actor.world_pos.distance(lerp(0.5)) < 1e-3);
    assert_eq!(d.skel.cur_frame, 0.0);
    // At 183 (curFrame >= endFrame): at the cue's end, landed: func_8096A630's func_80033480 (10 + 1
    // puffs), NA_SE_EN_GOLON_LAND_BIG; GR_03 (10).
    let (n, before) = (sfx_count(&w), dust(&w).len());
    script_cue(&mut w, 2, 183);
    update(&mut w, h);
    let d = du(&w, h);
    assert_eq!(d.update_index, 10);
    assert_eq!((d.actor.world_pos.x, d.actor.world_pos.z), (-1652.0, -564.0));
    assert_eq!(dust(&w).len() - before, 11);
    assert!(sfx_since(&w, n).contains(&NA_SE_EN_GOLON_LAND_BIG));
    assert!(d.actor.floor_poly.is_some());
    // GR_03 holds through cue 2; cue 3: GR_04 (11).
    script_cue(&mut w, 2, 183);
    update(&mut w, h);
    assert_eq!(du(&w, h).update_index, 10);
    script_cue(&mut w, 3, 184);
    update(&mut w, h);
    assert_eq!(du(&w, h).update_index, 11);
}

#[test]
fn he_stands_beats_his_chest_slaps_link_and_gives_the_ruby() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = landed(&mut w);
    // GR_04: the stand-up played once; done: gDaruniaIdleAnim looped, GR_05 (12).
    let last = du(&w, h).skel.end_frame;
    script_cue(&mut w, 3, 200);
    let k = until_next(&mut w, h, 400);
    assert_eq!(k as f32, last + 1.0);
    assert!(du(&w, h).skel.is(ANIM_IDLE));
    assert_eq!((du(&w, h).update_index, du(&w, h).skel.mode), (12, oot_game::skelanime_std::ANIMMODE_LOOP));
    // Idle through cue 3; cue 4: gDaruniaHitBreastAnim once with a 4-frame morph, GR_06 (13).
    update(&mut w, h);
    assert_eq!(du(&w, h).update_index, 12);
    script_cue(&mut w, 4, 288);
    update(&mut w, h);
    assert!(du(&w, h).skel.is(ANIM_HIT_BREAST));
    assert_eq!(du(&w, h).update_index, 13);
    // GR_06: NA_SE_EN_DARUNIA_HIT_BREAST - SFX_FLAG every update of it; done: idle, GR_07 (14).
    loop {
        let n = sfx_count(&w);
        update(&mut w, h);
        assert!(sfx_since(&w, n).contains(&(NA_SE_EN_DARUNIA_HIT_BREAST - SFX_FLAG)));
        if du(&w, h).update_index != 13 {
            break;
        }
    }
    assert_eq!(du(&w, h).update_index, 14);
    assert!(du(&w, h).skel.is(ANIM_IDLE));
    // Cue 5: gDaruniaHitLinkAnim once, GR_08 (15).
    script_cue(&mut w, 5, 312);
    update(&mut w, h);
    assert!(du(&w, h).skel.is(ANIM_HIT_LINK));
    assert_eq!(du(&w, h).update_index, 15);
    // GR_08: as the slap's frame passes 31 and 41, five puffs each about Link's left forearm
    // (bodyPartsPos[PLAYER_BODYPART_L_FOREARM] plus dustPosOffsets[5..10], then [0..5]), brown
    // ((190, 150, 110) and (120, 80, 40), each moved by one delta of -10 to 9), scale 200 to 239,
    // 10 frames; NA_SE_EN_DARUNIA_HIT_LINK and Link's NA_SE_VO_LI_DAMAGE_S_KID.
    let forearm = w.player().body_parts_pos[11];
    let offsets = [[11.0, -11.0, -6.0], [0.0, 14.0, -13.0], [14.0, -2.0, -10.0], [10.0, -6.0, -8.0], [8.0, 6.0, 8.0], [13.0, 8.0, -10.0], [-14.0, 1.0, -14.0], [5.0, 12.0, -9.0], [11.0, 6.0, -7.0], [14.0, 14.0, -14.0]].map(Vec3::from_array);
    let mut slaps = Vec::new();
    while du(&w, h).update_index == 15 {
        let (n, before) = (sfx_count(&w), dust(&w));
        update(&mut w, h);
        let f = du(&w, h).skel.cur_frame;
        // (The effects aren't updated here: the new ones are those at new places.)
        let new: Vec<_> = dust(&w).into_iter().filter(|e| !before.iter().any(|b| b.pos == e.pos)).collect();
        let heard = sfx_since(&w, n);
        if f == 31.0 || f == 41.0 {
            slaps.push(f);
            assert_eq!(new.len(), 5, "frame {f}");
            assert!(heard.contains(&NA_SE_EN_DARUNIA_HIT_LINK) && heard.contains(&NA_SE_VO_LI_DAMAGE_S_KID));
            let set = if f == 31.0 { &offsets[5..10] } else { &offsets[0..5] };
            for e in &new {
                assert!(set.iter().any(|o| (e.pos - (forearm + *o)).length() < 1e-3), "{} not about {forearm}", e.pos);
                let r = &e.regs;
                let delta = r[0] as i32 - 190;
                assert!((-10..=9).contains(&delta));
                assert_eq!((r[1] as i32 - 150, r[2] as i32 - 110, r[4] as i32 - 120, r[5] as i32 - 80, r[6] as i32 - 40), (delta, delta, delta, delta, delta));
                assert!((200..=239).contains(&r[oot_game::effect::dust::R_SCALE]));
            }
        } else if du(&w, h).update_index == 15 {
            assert!(new.is_empty() && !heard.contains(&NA_SE_EN_DARUNIA_HIT_LINK), "frame {f}");
        }
    }
    assert_eq!(slaps, vec![31.0, 41.0]);
    // Done: gDaruniaHitBreastAnim (no morph), GR_09 (16), its sound; done: idle, GR_10 (17).
    assert!(du(&w, h).skel.is(ANIM_HIT_BREAST));
    assert_eq!(du(&w, h).update_index, 16);
    let n = sfx_count(&w);
    until_next(&mut w, h, 200);
    assert!(sfx_since(&w, n).contains(&(NA_SE_EN_DARUNIA_HIT_BREAST - SFX_FLAG)));
    assert_eq!(du(&w, h).update_index, 17);
    // Cue 6: gDaruniaItemGiveAnim once, GR_11 (18); done: gDaruniaItemGiveIdleAnim looped, GR_12
    // (19).
    script_cue(&mut w, 6, 420);
    update(&mut w, h);
    assert!(du(&w, h).skel.is(ANIM_ITEM_GIVE));
    assert_eq!(du(&w, h).update_index, 18);
    until_next(&mut w, h, 200);
    assert!(du(&w, h).skel.is(ANIM_ITEM_GIVE_IDLE));
    assert_eq!((du(&w, h).update_index, du(&w, h).skel.mode), (19, oot_game::skelanime_std::ANIMMODE_LOOP));
    // Cue 7: idle, GR_13 (20), to the end; frame 1400 Link's cry (NA_SE_VO_LI_FALL_L_KID), only then.
    script_cue(&mut w, 7, 1100);
    update(&mut w, h);
    assert!(du(&w, h).skel.is(ANIM_IDLE));
    assert_eq!(du(&w, h).update_index, 20);
    let n = sfx_count(&w);
    script_cue(&mut w, 7, 1399);
    update(&mut w, h);
    assert!(!sfx_since(&w, n).contains(&NA_SE_VO_LI_FALL_L_KID));
    script_cue(&mut w, 7, 1400);
    update(&mut w, h);
    assert!(sfx_since(&w, n).contains(&NA_SE_VO_LI_FALL_L_KID));
    assert_eq!(du(&w, h).update_index, 20);
}

#[test]
fn his_face_follows_the_scripts_frame() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = landed(&mut w);
    // DemoDu_CsGoronsRuby_UpdateFaceTextures from GR_04 on: (frame, eyes, mouth); None blinking
    // (DemoDu_UpdateEyes: 0 to 2).
    for (frame, eye, mouth) in [(200, None, 0), (259, None, 0), (260, None, 3), (334, None, 3), (335, Some(3), 1), (364, Some(3), 1), (365, Some(0), 3), (394, Some(0), 3), (395, None, 0), (409, None, 0), (410, None, 3), (1300, None, 3)] {
        script_cue(&mut w, 3, frame);
        update(&mut w, h);
        let d = du(&w, h);
        assert_eq!(d.mouth_tex_index, mouth, "frame {frame}");
        match eye {
            Some(e) => assert_eq!(d.eye_tex_index, e, "frame {frame}"),
            None => assert!((0..3).contains(&d.eye_tex_index), "frame {frame}"),
        }
    }
    // The blink: blinkTimer down by 1 an update; at 0 Rand_S16Offset(60, 60); the eyes the timer
    // while it's under 3 (2 shut, 1 opening, 0 open), else open.
    let mut seen = Vec::new();
    script_cue(&mut w, 3, 200);
    for _ in 0..300 {
        update(&mut w, h);
        let d = du(&w, h);
        assert_eq!(d.eye_tex_index, if d.blink_timer < 3 { d.blink_timer } else { 0 });
        assert!((0..120).contains(&d.blink_timer));
        seen.push(d.eye_tex_index);
    }
    assert!(seen.windows(3).any(|s| s == [2, 1, 0]));
}

#[test]
fn draw_01_is_en_dus_skeleton_opaque_and_draw_02_translucent() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let h = landed(&mut w);
    let view = ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY };
    let draws = |w: &PlayState| {
        let d = du(w, h);
        let mut out = DrawOut::default();
        d.draw(&d.render_state(), w, &view, &mut out);
        out
    };
    let image = |sym: &str| {
        let t = a.pack.texture(OBJECT, sym).unwrap();
        (t.width, t.height, t.rgba.clone())
    };
    // DemoDu_Draw_01: sEyeTextures[eye] on 8, sMouthTextures[mouth] on 9, gDaruniaNoseSeriousTex on
    // 0xA, env (0, 0, 0, 255), the skeleton opaque: En_Du's bake. The circle shadow is Actor_Draw's
    // (the render state's, with the floor found).
    {
        let d = w.actors.downcast_mut::<DemoDu>(h).unwrap();
        d.eye_tex_index = 3;
        d.mouth_tex_index = 1;
    }
    let out = draws(&w);
    assert_eq!(out.opa.len(), 1);
    assert!(out.xlu.is_empty());
    let s = du(&w, h).render_state().shadow.expect("the shape's shadow");
    assert_eq!((s.draw, s.scale, s.alpha, s.floor_poly.is_some()), (oot_game::shadow::ShadowDraw::Circle, 30.0, 255, true));
    let c = &out.opa[0];
    assert_eq!(c.mesh, eng_gfx::MeshKey::named(oot_game::pack::keys::bake(oot_actors::en_du::BAKE)));
    let imgs = &c.params.texture_images;
    let got: Vec<_> = imgs.iter().map(|i| (i.source, i.image.width, i.image.height, i.image.rgba.to_vec())).collect();
    let (e, m, n) = (image("gDaruniaEyeClosingTex"), image("gDaruniaMouthGrinningTex"), image("gDaruniaNoseSeriousTex"));
    assert_eq!(got, vec![(0x0800_0000, e.0, e.1, e.2.to_vec()), (0x0900_0000, m.0, m.1, m.2.to_vec()), (0x0A00_0000, n.0, n.1, n.2.to_vec())]);
    assert_eq!(c.params.segments.as_ref().unwrap().env[0x0E], Some([0, 0, 0, 255]));
    // DemoDu_Draw_02: the same translucent (this module's bake), env alpha shadowAlpha.
    {
        let d = w.actors.downcast_mut::<DemoDu>(h).unwrap();
        d.draw_index = 2;
        d.shadow_alpha = 128;
    }
    let out = draws(&w);
    assert!(out.opa.is_empty());
    let c = out.xlu.iter().find(|c| c.mesh == eng_gfx::MeshKey::named(oot_game::pack::keys::bake(BAKE_XLU))).expect("Draw_02");
    assert_eq!(c.params.segments.as_ref().unwrap().env[0x0E], Some([0, 0, 0, 128]));
    // The bake: the whole skeleton, nothing unresolved.
    let d: eng_gfx::DrawList = a.pack.assets.get(&oot_game::pack::keys::bake(BAKE_XLU)).expect("Demo_Du/xlu baked");
    assert!(d.stats.unresolved_addresses.is_empty());
    assert!(d.triangle_count() > 0);
}

/// The log records (level, target, message) of this thread (the tests run on their own threads).
struct ThreadLog;
static RECORDS: Mutex<Vec<(std::thread::ThreadId, log::Level, String, String)>> = Mutex::new(Vec::new());

impl log::Log for ThreadLog {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, r: &log::Record) {
        if r.level() <= log::Level::Info {
            RECORDS.lock().unwrap().push((std::thread::current().id(), r.level(), r.target().to_string(), r.args().to_string()));
        }
    }
    fn flush(&self) {}
}
static LOGGER: ThreadLog = ThreadLog;

#[test]
fn the_rubys_cutscene_runs_whole_to_its_terminator() {
    let Some(a) = assets() else { return };
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Info);
    // The logger is ours: a probe is recorded.
    log::warn!(target: "probe", "probe");
    assert!(RECORDS.lock().unwrap().iter().any(|r| r.0 == std::thread::current().id() && r.2 == "probe"));
    // King Dodongo's warp's arrival: ENTR_DEATH_MOUNTAIN_TRAIL_0 with CS_INDEX_1 (0xFFF1, layer 5),
    // after the cavern: EVENTCHKINF_25 and the Ruby given (as Door_Warp1 would).
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-boss").unwrap();
    save.set_event_chk_inf(0x25);
    oot_game::item::item_give(&mut save, None, 0x6D);
    save.cutscene_index = 0xFFF1;
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), true);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    assert_eq!(w.save.scene_layer, 5);
    let me = std::thread::current().id();
    let find = |w: &PlayState| {
        let all = w.actors.all();
        let darunia = all.iter().copied().find(|&h| w.actors.downcast::<DemoDu>(h).is_some());
        let mut gorons: Vec<ActorHandle> = all.iter().copied().filter(|&h| w.actors.downcast::<DemoGo>(h).is_some()).collect();
        gorons.sort_by_key(|&h| w.actors.actor(h).unwrap().params);
        (darunia, gorons)
    };
    // (frame, Darunia's updateIndex, the Gorons' actions) at each change; positions at frames.
    let mut changes: Vec<(u16, String, i32)> = Vec::new();
    let mut last: Vec<i32> = vec![-1; 4];
    let mut at: std::collections::BTreeMap<u16, (Vec3, Vec<Vec3>)> = Default::default();
    let mut heard_fall = false;
    let mut end_frame = None;
    let mut texts = Vec::new();
    let mut prev = PadState::default();
    let mut ticks = 0;
    for _ in 0..4000 {
        ticks += 1;
        // The script's frame the actors see this update (Cutscene_UpdateScripted runs after
        // Actor_UpdateAll and moves it on).
        let frame = w.cs_ctx.frames;
        // A pressed (on its own frame) when a box waits: the script's texts read.
        let st = w.message_state();
        let waits = matches!(st, TEXT_STATE_AWAITING_NEXT | TEXT_STATE_DONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_EVENT) || w.msg_ctx.msg_mode == MSGMODE_TEXT_AWAIT_INPUT;
        let mut pad = PadState::default();
        if waits && prev.button == 0 {
            pad.button = BTN_A;
        }
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if w.msg_ctx.text_id != 0 && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        let (Some(dh), gorons) = find(&w) else { continue };
        assert_eq!(gorons.len(), 3);
        let state = [du(&w, dh).update_index, w.actors.downcast::<DemoGo>(gorons[0]).unwrap().action, w.actors.downcast::<DemoGo>(gorons[1]).unwrap().action, w.actors.downcast::<DemoGo>(gorons[2]).unwrap().action];
        for (i, s) in state.iter().enumerate() {
            if *s != last[i] {
                changes.push((frame, ["Demo_Du", "Demo_Go 0", "Demo_Go 1", "Demo_Go 2"][i].to_string(), *s));
                last[i] = *s;
            }
        }
        if [100, 178, 200, 400, 1280, 1300, 1400, 1440, 1500].contains(&frame) && w.cs_ctx.state != CS_STATE_IDLE {
            at.entry(frame).or_insert((w.actors.actor(dh).unwrap().world_pos, gorons.iter().map(|&g| w.actors.actor(g).unwrap().world_pos).collect()));
        }
        heard_fall |= w.audio.log.as_ref().unwrap().sfx.iter().any(|&(_, s, _)| s == NA_SE_EV_OBJECT_FALL);
        if w.transition.trigger == oot_game::transition::TRANS_TRIGGER_START {
            end_frame = Some(frame);
            break;
        }
    }
    for c in &changes {
        eprintln!("frame {:5} {:10} -> {}", c.0, c.1, c.2);
    }
    for (f, (d, g)) in &at {
        eprintln!("frame {f}: Darunia {d}, Gorons {g:?}");
    }
    // The terminator (CS_DEST 12, from frame 1515): on to ENTR_DEATH_MOUNTAIN_TRAIL_5.
    eprintln!("texts {texts:x?}");
    let end_frame = end_frame.expect("the terminator");
    eprintln!("the terminator at frame {end_frame}, after {ticks} updates");
    assert_eq!(Some(w.transition.next_entrance_index), w.entrance_by_name("ENTR_DEATH_MOUNTAIN_TRAIL_5"));
    assert!(heard_fall);
    // Each reached its last state: Darunia GR_13 (20); the Gorons walking (6).
    assert_eq!(last, vec![cs_goronsruby_subscene(13), 6, 6, 6]);
    // Darunia's states in the script's order, each once (7, GR_00, for its one update).
    let du_states: Vec<i32> = changes.iter().filter(|c| c.1 == "Demo_Du").map(|c| c.2).collect();
    assert_eq!(du_states, (7..=20).collect::<Vec<_>>());
    let state_at = |who: &str, s: i32| changes.iter().find(|c| c.1 == who && c.2 == s).map(|c| c.0).unwrap();
    // Cue 2 from 173 (seen the frame after the script sets it), landed at its end (183), standing
    // up on cue 3, the chest on cue 4 (288), the slap on cue 5 (312), the Ruby on cue 6 (420), idle on
    // cue 7 (1100).
    assert!((173..=175).contains(&state_at("Demo_Du", 9)));
    assert!((183..=185).contains(&state_at("Demo_Du", 10)));
    assert!(state_at("Demo_Du", 13) >= 288 && state_at("Demo_Du", 15) >= 312 && state_at("Demo_Du", 18) >= 420);
    assert!((1100..=1102).contains(&state_at("Demo_Du", 20)));
    // The Gorons fall on their cue 2 (1270, 1250, 1425) and land at its end (1286, 1272, 1438).
    for (who, fall, land) in [("Demo_Go 0", 1270, 1286), ("Demo_Go 1", 1250, 1272), ("Demo_Go 2", 1425, 1438)] {
        assert!((fall..=fall + 2).contains(&state_at(who, 2)), "{who}");
        assert!((land..=land + 2).contains(&state_at(who, 3)), "{who}");
    }
    // Where they are: Darunia hidden at his placement before his fall, at the cue's end after it;
    // the Gorons at their cue's end once landed (on the floor: y at or above it).
    let (d100, _) = &at[&100];
    assert_eq!(*d100, DARUNIA_AT);
    let (d400, _) = &at[&400];
    assert_eq!((d400.x, d400.z), (-1652.0, -564.0));
    let (_, g1280) = &at[&1280];
    assert_eq!((g1280[1].x, g1280[1].z), (-1731.0, -574.0));
    let (_, g1300) = &at[&1300];
    assert_eq!((g1300[0].x, g1300[0].z), (-1546.0, -591.0));
    // Nothing unexpected logged: no warning or error from this run (the placeholders' info aside).
    let records: Vec<_> = RECORDS.lock().unwrap().iter().filter(|r| r.0 == me && r.1 <= log::Level::Warn && r.2 != "probe").map(|r| format!("{} {}: {}", r.1, r.2, r.3)).collect();
    let infos: Vec<_> = RECORDS.lock().unwrap().iter().filter(|r| r.0 == me && r.1 == log::Level::Info).map(|r| format!("{}: {}", r.2, r.3)).collect();
    eprintln!("info: {infos:?}");
    assert!(records.is_empty(), "{records:?}");
}
