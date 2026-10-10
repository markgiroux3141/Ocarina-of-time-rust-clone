//! The depth probe (`eng_gfx::DrawLists::depth_probe`) through the game's frame: the pixel
//! `Environment_DrawLensFlare` asks for (`sSunDepthTestX`, `sSunDepthTestY`) read back after the
//! frame, as `Environment_GraphCallback` reads the z-buffer: the far plane where only the sky is
//! drawn, nearer where the field is. Skips without a GPU or a current pack.

use eng_input::pad::PadState;
use oot::Options;
use oot_game::play::scripted_input;

#[test]
fn the_probe_reads_the_far_plane_in_the_sky_and_the_field_below() {
    if !oot_game::pack::default_pack_path().is_ok_and(|p| oot_game::pack::is_current(&p, None)) {
        return;
    }
    let Ok((device, queue)) = eng_render::headless_device() else { return };
    // Hyrule Field by day, in front of the drawbridge, facing away from the castle.
    let o = Options { entrance: Some("ENTR_HYRULE_FIELD_0".into()), preset: Some("deku-tree-dead".into()), time: "12:00".into(), child: true, ..Default::default() };
    let mut a = oot::load_assets(&o).expect("the assets");
    let mut w = oot::new_play(&a, true);
    let none = PadState::default();
    for _ in 0..40 {
        w.tick_with(scripted_input(none, none));
    }
    let mut r = eng_render::Renderer::new(&device, &queue);
    let mut scene = oot::SceneGfx::new(&a);
    let t = eng_render::Target::new(&device, 320, 240);
    let frame = w.current_frame();
    let mut depth_at = |w: &mut oot_game::play::PlayState, x: i16, y: i16| {
        w.env_statics.sun_depth_test = [x, y];
        oot::draw_frame(&mut r, &device, &queue, &t, &mut a, &mut scene, w, &frame, false).expect("a probe")
    };
    // The top of the screen is sky (no depth written: 1), the bottom the field before Link.
    assert_eq!(depth_at(&mut w, 160, 2), 1.0);
    let ground = depth_at(&mut w, 160, 235);
    assert!(ground < 1.0, "{ground}");
    // Environment_GraphCallback: sSunScreenDepth the far plane's (GPACK_ZDZ(G_MAXFBZ, 0)), or not.
    w.environment_graph_callback(Some(1.0));
    assert_eq!(w.env_statics.sun_screen_depth, oot_game::env_draw::ZBUF_MAX);
    w.environment_graph_callback(Some(ground));
    assert_eq!(w.env_statics.sun_screen_depth, 0);
}
