//! GAME-10 milestone 2 against the C: Zora's Domain's waterfalls (`z_bg_spot07_taki.c`) and the
//! Domain's draw config (`Scene_DrawConfigZorasDomain`, `z_scene_table.c`).
//!
//! The placements (`ootx scene-info --scene spot07_scene`): every layer places `Bg_Spot07_Taki` 0
//! at the origin in room 1 and 1 at (445, 1008, -1742) in room 0. Link enters at
//! `ENTR_ZORAS_DOMAIN_0` (room 1) a child (layer 0) or an adult (layer 2), the intro seen.

mod common;

use std::sync::Arc;

use common::*;
use eng_collision::dyna::BG_ACTOR_MAX;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::bg_spot07_taki::*;
use oot_game::actor_ctx::ActorHandle;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::scene_table::{DrawConfigState, gfx_two_tex_scroll, segment_values};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

const EVENTCHKINF_A4: u16 = 0xA4;

fn domain(a: &Arc<GameAssets>, adult: bool) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_ZORAS_DOMAIN_0").expect("entrance");
    let mut save = SaveContext::new(e, adult, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("fairy-magic").unwrap();
    save.set_event_chk_inf(EVENTCHKINF_A4);
    let mut w = oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).expect("Play_Init");
    for _ in 0..2 {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
    assert_eq!(w.room_ctx.cur.num, 1);
    w
}

fn falls(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<BgSpot07Taki>(h).is_some_and(|t| !t.actor.killed)).collect()
}

#[test]
fn a_child_s_falls_are_only_drawn() {
    let Some(a) = assets() else { return };
    let w = domain(&a, false);
    let hs = falls(&w);
    assert_eq!(hs.len(), 1, "room 1's");
    let t = w.actors.downcast::<BgSpot07Taki>(hs[0]).unwrap();
    // sInitChain: scale 0.1; a child's: no collision.
    assert_eq!((t.actor.params, t.actor.scale, t.actor.world_pos, t.bg), (0, Vec3::splat(0.1), Vec3::ZERO, BG_ACTOR_MAX));
}

#[test]
fn an_adult_s_ice_collides() {
    let Some(a) = assets() else { return };
    let w = domain(&a, true);
    let hs = falls(&w);
    assert_eq!(hs.len(), 1);
    let t = w.actors.downcast::<BgSpot07Taki>(hs[0]).unwrap();
    // LINK_IS_ADULT: object_spot07_object_Col_002590 set as a DynaPoly actor.
    assert_ne!(t.bg, BG_ACTOR_MAX);
}

#[test]
fn the_falls_scroll_as_the_c_does() {
    // BgSpot07Taki_Draw at gameplayFrames 5: segment 8 ((-5) & 0x7F, 5, 32, 32; 5, 5), the big
    // fall's 9 ((-5) & 0x7F, (-15) & 0xFF, 64, 64; 5, (-15) & 0xFF), 0xA (0, 15, 32, 128).
    assert_eq!(scroll_8(5), gfx_two_tex_scroll(0, 123, 5, 32, 32, 1, 5, 5, 32, 32));
    assert_eq!(scroll_9(0, 5), gfx_two_tex_scroll(0, 123, 241, 64, 64, 1, 5, 241, 64, 64));
    assert_eq!(scroll_9(1, 5), gfx_two_tex_scroll(0, 0, 123, 32, 32, 1, 0, 123, 32, 32));
    assert_eq!(scroll_a(5), gfx_two_tex_scroll(0, 0, 15, 32, 128, 1, 0, 15, 32, 128));
    // (frames * 3) & 0x1FF wraps at 512: frame 171's is 1.
    assert_eq!(scroll_a(171), gfx_two_tex_scroll(0, 0, 1, 32, 128, 1, 0, 1, 32, 128));
}

#[test]
fn the_domain_s_draw_config_scrolls_its_second_tile_for_a_child_only() {
    // Scene_DrawConfigZorasDomain: segment 0xC (OPA) Gfx_TwoTexScroll(0, 0, 0, 64, 32, 1, 0,
    // 127 - frames % 128, 64, 32); 0 for an adult.
    let mut child = DrawConfigState { child: true, gameplay_frames: 5, ..Default::default() };
    let mut adult = DrawConfigState { child: false, gameplay_frames: 5, ..Default::default() };
    let c = segment_values("SDC_ZORAS_DOMAIN", &mut child).expect("ported");
    let ad = segment_values("SDC_ZORAS_DOMAIN", &mut adult).expect("ported");
    assert_ne!(c, ad);
    let mut want = eng_gfx::SegmentValues::default();
    want.read(0x0C, &gfx_two_tex_scroll(0, 0, 0, 64, 32, 1, 0, 122, 64, 32));
    assert_eq!(c[0], want);
    let mut want = eng_gfx::SegmentValues::default();
    want.read(0x0C, &gfx_two_tex_scroll(0, 0, 0, 64, 32, 1, 0, 0, 64, 32));
    assert_eq!(ad[0], want);
}
