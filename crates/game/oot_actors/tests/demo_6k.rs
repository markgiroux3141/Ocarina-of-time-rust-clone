//! `Demo_6K` (GAME-11 milestone 2) against the C (`z_demo_6k.c`): the sages' lights, types 14 to
//! 19. The Temple of Time's room 0 places type 0x10 (cue channel 2, yellow-green): its init (scale
//! 0.2, hidden, the point light), `Demo6K_WaitForObject`, `func_80967410` following its cue
//! (`func_809691BC`), shown with a sparkle (`func_80967244`) while the cue is id 3, the draw
//! (`func_8096865C`), the destroy; the other types as placeholders chosen at init.
//!
//! A child enters the Temple at `ENTR_TEMPLE_OF_TIME_4` (room 0; Sheik is gone for a child, no
//! script runs). The light's update runs alone (`Actor_UpdateAll`'s call), its cue put on the
//! script's channel by hand (`play->csCtx.actorCues`). Expected values are worked out from the C
//! in the comments.

mod common;

use std::sync::Arc;

use common::*;
use glam::{IVec3, Vec3};
use oot_actors::demo_6k::*;
use oot_game::actor::ACTOR_FLAG_DRAW_CULLING_DISABLED;
use oot_game::actor_ctx::ActorHandle;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::effect::EFFECT_SS_KIRAKIRA;
use oot_game::play::{DrawOut, PlayState, ViewInfo};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `CS_STATE_RUN` (`cutscene.h`).
const CS_STATE_RUN: u8 = 2;
/// Room 0's light (`ootx scene-info --scene tokinoma --layer 0`).
const PLACED_AT: Vec3 = Vec3::new(697.0, 0.0, -168.0);

/// The Temple of Time's room 0 (`ENTR_TEMPLE_OF_TIME_4`), a child from `ocarina-of-time`; just
/// after `Play_Init`.
fn temple(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_TEMPLE_OF_TIME_4").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("ocarina-of-time").unwrap();
    save.adult = false;
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    settle(&mut w, ACTOR_DEMO_6K);
    assert_eq!(w.room_ctx.cur.num, 0);
    w
}

fn d6k(w: &PlayState, h: ActorHandle) -> &Demo6K {
    w.actors.downcast::<Demo6K>(h).expect("a Demo_6K")
}

/// The actor's update alone (`Actor_UpdateAll`'s call), as the updating actor.
fn update(w: &mut PlayState, h: ActorHandle) {
    let mut a = w.actors.take(h).expect("in the arena");
    w.cur_actor = Some(h);
    a.update(w);
    w.cur_actor = None;
    w.actors.put_back(h, a);
}

/// A cue `action` on channel 2 from `start` to `end` over frames 100 to 200, the script at
/// `frame`.
fn cue(w: &mut PlayState, action: u16, start: IVec3, end: IVec3, frame: u16) {
    w.cs_ctx.state = CS_STATE_RUN;
    w.cs_ctx.frames = frame;
    w.cs_ctx.npc_actions[2] = Some(CsCmdActorCue { action, start_frame: 100, end_frame: 200, rot: [0; 3], start_pos: start, end_pos: end, normal: IVec3::ZERO });
}

fn sparkles(w: &PlayState) -> Vec<oot_game::effect::EffectSs> {
    w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == EFFECT_SS_KIRAKIRA).cloned().collect()
}

/// The light's draw: how many translucent commands.
fn draws(w: &PlayState, h: ActorHandle) -> usize {
    let a = w.actors.get(h).unwrap();
    let rs = a.render_state();
    let mut out = DrawOut::default();
    a.draw(&rs, w, &ViewInfo { eye: Vec3::ZERO, billboard: glam::Mat4::IDENTITY }, &mut out);
    assert!(out.opa.is_empty());
    out.xlu.len()
}


/// Ticks until the room's actors of `id` are in and initialised (`Actor_UpdateAll` runs an init
/// once its object is loaded, without the update that frame); stops on that tick.
fn settle(w: &mut PlayState, id: i16) {
    for _ in 0..60 {
        let mine: Vec<&dyn oot_game::actor_ctx::ActorImpl> = w.actors.all().into_iter().filter_map(|h| w.actors.get(h)).filter(|a| a.base().id == id).collect();
        if !mine.is_empty() && mine.iter().all(|a| a.name() != "(not initialised)") {
            return;
        }
        w.tick_with(oot_game::play::scripted_input(eng_input::pad::PadState::default(), eng_input::pad::PadState::default()));
    }
    panic!("actor {id:#x} not initialised");
}

#[test]
fn the_temple_s_light_follows_its_cue() {
    let Some(a) = assets() else { return };
    let mut w = temple(&a);
    let h = w.actors.all().into_iter().find(|&h| w.actors.downcast::<Demo6K>(h).is_some()).expect("room 0's Demo_6K");
    // Demo6K_Init (0x10): scale 0.2, flags 1 (hidden), unk_293 2, not culled, the point light
    // (white, radius 100) at its placement. Its object (gameplay_keep) is in, so it was
    // initialised as the room loaded and the first frame's Demo6K_WaitForObject has run: drawn,
    // func_80967410 next.
    let l = d6k(&w, h);
    assert_eq!(l.actor.params, 0x10);
    assert_eq!(l.actor.world_pos, PLACED_AT);
    assert_eq!(l.actor.scale, Vec3::splat(0.2));
    assert_eq!((l.flags, l.unk_293, l.timer2), (1, 2, 0));
    assert!(l.actor.flags & ACTOR_FLAG_DRAW_CULLING_DISABLED != 0);
    assert_eq!(l.action, Demo6KAction::SageLight);
    assert!(l.drawn);
    assert!(l.light_node.is_some());
    assert_eq!((l.light_info.x, l.light_info.y, l.light_info.z, l.light_info.color, l.light_info.radius), (697, 0, -168, [255, 255, 255], 100));
    // A new one: waiting for its object (Demo6K_Init), then Demo6K_WaitForObject: drawn,
    // func_80967410 next (not run that update).
    let fresh = w.actor_spawn(ACTOR_DEMO_6K, PLACED_AT, [0; 3], 0x10).expect("spawned");
    assert_eq!((d6k(&w, fresh).action, d6k(&w, fresh).drawn), (Demo6KAction::WaitForObject, false));
    update(&mut w, fresh);
    assert_eq!((d6k(&w, fresh).action, d6k(&w, fresh).drawn, d6k(&w, fresh).timer2), (Demo6KAction::SageLight, true, 0));
    // No script: hidden, timer2 counting, nothing drawn.
    assert_eq!(w.cs_ctx.state, CS_STATE_IDLE);
    update(&mut w, h);
    assert_eq!((d6k(&w, h).flags, d6k(&w, h).timer2), (1, 1));
    assert_eq!(draws(&w, h), 0);
    // Cue 2 on channel 2, at frame 150 of 100..200: placed halfway (Environment_LerpWeight 0.5),
    // still hidden, no sparkle.
    let (start, end) = (IVec3::new(600, 100, -200), IVec3::new(700, 300, 0));
    let n = sparkles(&w).len();
    cue(&mut w, 2, start, end, 150);
    update(&mut w, h);
    assert_eq!(d6k(&w, h).actor.world_pos, Vec3::new(650.0, 200.0, -100.0));
    assert_eq!(d6k(&w, h).flags, 1);
    assert_eq!(sparkles(&w).len(), n);
    // Cue 3 at frame 175: three quarters on, shown, a sparkle at it: env sEnvColors[2] (200, 255,
    // 0), prim white, scale 6000 (the Temple of Time), 20 frames, 20 out.
    cue(&mut w, 3, start, end, 175);
    update(&mut w, h);
    let p = Vec3::new(675.0, 250.0, -50.0);
    assert_eq!(d6k(&w, h).actor.world_pos, p);
    assert_eq!(d6k(&w, h).flags & 1, 0);
    let s = sparkles(&w);
    assert_eq!(s.len(), n + 1);
    let k = s.iter().find(|e| e.pos == p).expect("the sparkle at the light");
    assert_eq!(k.life, 20);
    // regs: R_PRIM_COLOR_R..A 2..5, R_ENV_COLOR_R..B 6..8, R_SCALE 11 (`z_eff_ss_kirakira.c`).
    assert_eq!(&k.regs[2..9], &[255, 255, 255, 0, 200, 255, 0]);
    assert_eq!(k.regs[11], 6000);
    assert!((k.velocity.length() - 20.0).abs() < 0.01, "{}", k.velocity);
    // Drawn: two flashes (func_8096865C).
    assert_eq!(draws(&w, h), 2);
    // Cue 4: hidden again.
    cue(&mut w, 4, start, end, 190);
    update(&mut w, h);
    assert_eq!(d6k(&w, h).flags, 1);
    assert_eq!(draws(&w, h), 0);
    // No cue on its channel: hidden; the script over: hidden.
    w.cs_ctx.npc_actions[2] = None;
    update(&mut w, h);
    assert_eq!(d6k(&w, h).flags, 1);
    w.cs_ctx.state = CS_STATE_IDLE;
    update(&mut w, h);
    assert_eq!(d6k(&w, h).flags, 1);
    assert_eq!(d6k(&w, h).actor.scale, Vec3::splat(0.2));
    // Demo6K_Destroy: the light out.
    let node = d6k(&w, h).light_node;
    let mut boxed = w.actors.take(h).unwrap();
    boxed.destroy(&mut w);
    w.actors.put_back(h, boxed);
    assert!(d6k(&w, h).light_node.is_none());
    assert!(node.is_some());
}

#[test]
fn types_14_to_19_take_their_channels_and_colours() {
    let Some(a) = assets() else { return };
    let mut w = temple(&a);
    for ty in 14..=19i16 {
        let h = w.actor_spawn(ACTOR_DEMO_6K, Vec3::ZERO, [0; 3], ty).expect("spawned");
        assert_eq!(d6k(&w, h).unk_293 as i16, ty - 14);
        update(&mut w, h);
        // Its channel: cue 3 on channel ty - 14 shows it, at the cue's start (frame 100).
        let ch = (ty - 14) as usize;
        w.cs_ctx.state = CS_STATE_RUN;
        w.cs_ctx.frames = 100;
        w.cs_ctx.npc_actions = [None; 10];
        w.cs_ctx.npc_actions[ch] = Some(CsCmdActorCue { action: 3, start_frame: 100, end_frame: 200, rot: [0; 3], start_pos: IVec3::new(10 * ty as i32, 20, 30), end_pos: IVec3::ZERO, normal: IVec3::ZERO });
        update(&mut w, h);
        assert_eq!(d6k(&w, h).flags & 1, 0, "type {ty}");
        let at = Vec3::new(10.0 * ty as f32, 20.0, 30.0);
        assert_eq!(d6k(&w, h).actor.world_pos, at);
        let k = sparkles(&w).into_iter().find(|e| e.pos == at).expect("its sparkle");
        let c = ENV_COLORS[ch];
        assert_eq!(&k.regs[6..9], &[c[0] as i16, c[1] as i16, c[2] as i16]);
        w.actors.actor_mut(h).unwrap().kill();
    }
}

#[test]
fn the_other_types_are_placeholders() {
    let Some(a) = assets() else { return };
    let mut w = temple(&a);
    // Every type but 14 to 19 (5 is Rauru's light ball, 12 En_Viewer's).
    for ty in (0..14).chain([20]) {
        let h = w.actor_spawn(ACTOR_DEMO_6K, Vec3::ZERO, [0; 3], ty).expect("spawned");
        assert!(w.actors.downcast::<Demo6K>(h).is_none(), "type {ty} is a placeholder");
    }
}
