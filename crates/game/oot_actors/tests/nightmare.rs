//! The opening's nightmare (Hyrule Field's scene layer 4, GAME-04b milestone 6): the castle's
//! drawbridge (`Bg_Spot00_Hanebasi`), the riders (`En_Viewer`, with the horses' skins) and the
//! rain's draw. Expected values come from `z_bg_spot00_hanebasi.c`, `z_en_viewer.c`,
//! `z_skin_awb.c` and `Environment_DrawRain`.

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::bg_spot00_hanebasi::{BgSpot00Hanebasi, DT_CHAIN_1, DT_CHAIN_2, DT_DRAWBRIDGE};
use oot_actors::en_viewer::{ENVIEWER_TYPE_0_HORSE_ZELDA, ENVIEWER_TYPE_2_ZELDA, ENVIEWER_TYPE_3_GANONDORF, ENVIEWER_TYPE_4_HORSE_GANONDORF, EnViewer};
use oot_game::env::{PRECIP_RAIN_CUR, lerp_weight};
use oot_game::play::{PlayState, RenderState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Hyrule Field's layer 4, as the narration's terminator enters it.
fn nightmare() -> Option<PlayState> {
    let a = assets()?;
    let mut save = SaveContext::file_select_new();
    save.entrance_index = a.scenes.entrance_index("ENTR_SPOT00_0").unwrap();
    save.cutscene_index = 0xFFF0;
    let w = oot_actors::play_entrance(a.clone(), common::data()?, common::rules()?, save).expect("Play_Init");
    assert_eq!(w.save.scene_layer, 4);
    Some(w)
}

fn tick(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
}

fn bridge_parts(w: &PlayState) -> Vec<(i16, i16, Vec3)> {
    let mut v: Vec<(i16, i16, Vec3)> = w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<BgSpot00Hanebasi>(h)).map(|b| (b.actor.params, b.actor.shape_rot.x, b.actor.world_pos)).collect();
    v.sort_by_key(|p| p.0);
    v
}

fn viewer(w: &PlayState, ty: u8) -> &EnViewer {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<EnViewer>(h)).find(|v| v.ty == ty).expect("the viewer")
}

#[test]
fn the_drawbridge_lowers_on_the_scripts_flag() {
    let Some(mut w) = nightmare() else { return };
    // The bridge spawns once its object (object_spot00_objects) is loaded.
    let mut n = 0;
    while bridge_parts(&w).is_empty() {
        tick(&mut w);
        n += 1;
        assert!(n < 10);
    }
    // BgSpot00Hanebasi_Init in layer 4: raised (-0x4000), the first chain spawned with rot.x
    // 0xF020 and the second 316 across from it.
    let parts = bridge_parts(&w);
    assert_eq!(parts.iter().map(|p| (p.0, p.1)).collect::<Vec<_>>(), vec![(DT_DRAWBRIDGE, -0x4000), (DT_CHAIN_1, 0xF020u16 as i16), (DT_CHAIN_2, 0xF020u16 as i16)]);
    // The torches' lights: Lights_PointGlowSetInfo(±260, 168, 690, (255, 255, 0), 0).
    let torches: Vec<_> = w.light_ctx.lights().filter(|l| l.z == 690).collect();
    assert_eq!(torches.len(), 2);
    assert!(torches.iter().all(|l| l.radius == 0 && l.y == 168 && l.color == [255, 255, 0]));
    // Waits raised until the script sets env flag 0 (CS_MISC 3).
    let mut n = 0;
    while !w.flags_get_env(0) {
        tick(&mut w);
        assert_eq!(bridge_parts(&w)[0].1, -0x4000);
        n += 1;
        assert!(n < 600);
    }
    // In a cutscene layer the torches burn at 0.008: the lights' radius 0.008 * 37500 and
    // height 5000 * 0.008 + 128, their red and green 128 to 254 (Rand_ZeroOne).
    let torches: Vec<_> = w.light_ctx.lights().filter(|l| l.z == 690).collect();
    assert!(torches.iter().all(|l| l.radius == 300 && l.y == 168 && (128..=254).contains(&l.color[0]) && l.color[0] == l.color[1]));
    // The next update sees the flag (DrawbridgeWait switches to DrawbridgeRiseAndFall and
    // returns); the one after starts lowering it: Math_ScaledStepToS by 80 (120 a frame at
    // R_UPDATE_RATE 3); once past -0x27D8 the chains follow by (s16)(80 * 0.4f) = 32 (48 a
    // frame) to 0; it stops at 0.
    let mut bridge = -0x4000i32;
    let mut chain = 0xF020u16 as i16 as i32;
    tick(&mut w);
    assert_eq!(bridge_parts(&w)[0].1, -0x4000);
    loop {
        tick(&mut w);
        bridge = (bridge + 120).min(0);
        if bridge >= -0x27D8 {
            chain = (chain + 48).min(0);
        }
        let p = bridge_parts(&w);
        assert_eq!((p[0].1 as i32, p[1].1 as i32, p[2].1 as i32), (bridge, chain, chain));
        if bridge == 0 && chain == 0 {
            break;
        }
    }
    // The chains hang from the bridge's ends: (±158, 10, 400) through its draw matrix, set by
    // its draw (the frame before).
    let p = bridge_parts(&w);
    let m = oot_game::play::actor_draw_matrix(&RenderState::of(
        &w.actors.downcast::<BgSpot00Hanebasi>(w.actors.all().into_iter().find(|&h| w.actors.downcast::<BgSpot00Hanebasi>(h).is_some_and(|b| b.actor.params == DT_DRAWBRIDGE)).unwrap()).unwrap().actor,
    ));
    assert!((p[1].2 - m.transform_point3(Vec3::new(158.0, 10.0, 400.0))).length() < 1e-3);
    assert!((p[2].2 - m.transform_point3(Vec3::new(-158.0, 10.0, 400.0))).length() < 1e-3);
}

#[test]
fn the_riders_follow_their_cues() {
    let Some(mut w) = nightmare() else { return };
    let mut zelda_seen = false;
    let mut reared = false;
    tick(&mut w);
    for _ in 0..560 {
        // The actors update before the frame's cutscene step: they see the cue and the frame
        // as the last one left them.
        let (cue, frames, visible) = (w.cs_ctx.npc_actions[0], w.cs_ctx.frames, viewer(&w, ENVIEWER_TYPE_2_ZELDA).is_visible);
        let cue1 = if w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE { w.cs_ctx.npc_actions[1] } else { None };
        tick(&mut w);
        // EnViewer_UpdatePosition: Zelda (cue 0) along her cue by Environment_LerpWeight.
        if let Some(c) = cue
            && frames < c.end_frame
            && visible
            && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE
        {
            let (s, e) = (c.start_pos.as_vec3(), c.end_pos.as_vec3());
            let t = lerp_weight(c.end_frame, c.start_frame, frames);
            let want = Vec3::new((e.x - s.x) * t + s.x, (e.y - s.y) * t + s.y, (e.z - s.z) * t + s.z);
            for ty in [ENVIEWER_TYPE_0_HORSE_ZELDA, ENVIEWER_TYPE_2_ZELDA] {
                assert_eq!(viewer(&w, ty).actor.world_pos, want, "type {ty} at frame {frames}");
            }
            zelda_seen = true;
        }
        // Cue 1's action 1: Ganondorf and his horse rear (sTimer 100).
        if cue1.is_some_and(|c| c.action == 1) && viewer(&w, ENVIEWER_TYPE_3_GANONDORF).is_visible {
            let anim = |ty| viewer(&w, ty).skel.as_ref().and_then(|s| s.animation.as_ref()).map(|a| a.name.clone());
            assert_eq!(anim(ENVIEWER_TYPE_3_GANONDORF).as_deref(), Some("gYoungGanondorfHorsebackRearAnim"));
            assert_eq!(anim(ENVIEWER_TYPE_4_HORSE_GANONDORF).as_deref(), Some("gHorseGanonRearingAnim"));
            reared = true;
        }
    }
    assert!(zelda_seen, "Zelda's cue");
    assert!(reared, "Ganondorf's rear");
}

#[test]
fn the_horses_draw_their_skins() {
    let Some(mut w) = nightmare() else { return };
    let pack = oot_game::pack::GamePack::open_default().unwrap();
    let skin = pack.skin_skeleton("object_horse_zelda", "gHorseZeldaSkel").unwrap();
    // Until the horse's cue comes, nothing; then its skin, a bone per limb and per group, the
    // transform the skin's matrix (scale 0.01 at the actor).
    let mut drawn = None;
    for _ in 0..560 {
        tick(&mut w);
        let h = viewer(&w, ENVIEWER_TYPE_0_HORSE_ZELDA);
        let rs = oot_game::actor_ctx::ActorImpl::render_state(h);
        let mut out = oot_game::play::DrawOut::default();
        oot_game::actor_ctx::ActorImpl::draw(h, &rs, &w, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
        let cue = w.cs_ctx.npc_actions[0].is_some();
        assert_eq!(out.opa.len(), usize::from(cue && h.is_visible), "frame {}", w.cs_ctx.frames);
        if let Some(c) = out.opa.first() {
            assert_eq!(c.bones.len(), skin.bone_count());
            assert!((c.transform.x_axis.length() - 0.01).abs() < 1e-6);
            assert!((c.transform.w_axis.truncate() - h.actor.world_pos).length() < 1e-3);
            drawn = Some(w.cs_ctx.frames);
        }
    }
    assert!(drawn.is_some());
}

#[test]
fn the_rain_draws_its_drops() {
    let Some(mut w) = nightmare() else { return };
    // Environment_DrawRain: a drop per precipitation[PRECIP_RAIN_CUR], and as many rings while
    // Player is below the eye.
    let mut rained = false;
    for _ in 0..200 {
        tick(&mut w);
        let n = w.env_ctx.precipitation[PRECIP_RAIN_CUR] as usize;
        assert_eq!(w.rain.drops.len(), n);
        let below = w.player.and_then(|h| w.actors.actor(h)).is_some_and(|p| p.world_pos.y < w.view.eye.y);
        assert_eq!(w.rain.rings.len(), if below { n } else { 0 });
        // Each drop within 70 of the point 50 ahead of the eye on each axis (Rand_ZeroOne - 0.7,
        // times 100), scaled (0.4, 1.2, 0.4).
        let (eye, at) = (w.view.eye, w.view.at);
        let p50 = eye + (at - eye).normalize() * 50.0;
        for d in &w.rain.drops {
            let o = d.w_axis.truncate() - p50;
            assert!(o.x >= -70.0 && o.x < 30.0 && o.y >= -70.0 && o.y < 30.0 && o.z >= -70.0 && o.z < 30.0, "{o:?}");
            assert!((d.y_axis.length() - 1.2).abs() < 1e-4);
        }
        rained |= n > 0;
    }
    assert!(rained);
}
