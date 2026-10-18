//! Actor shadows against the C (`z_actor.c`'s `ActorShape_Init`, `ActorShadow_Draw`, the actors'
//! inits): the shapes a few actors' inits give them (`En_Item00`'s circle at alpha 180, Player's
//! feet, the big healing fairy's white circle), and `ActorShadow_Draw` on a sloped floor of Death
//! Mountain Trail (the circle on the floor's plane, `func_80038A28`; its size and alpha by the
//! height above the floor; none from 50 below or from 500 up, nor without a floor), then the same
//! through `Actor_DrawAll` (`PlayState::draw`) for a rupee lying on the trail.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_elf::{ACTOR_EN_ELF, EnElf, FAIRY_HEAL_BIG};
use oot_actors::en_item00::{ACTOR_EN_ITEM00, EnItem00, ITEM00_RUPEE_GREEN};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, ViewInfo, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;
use oot_game::shadow::{self, BAKE_CIRCLE, BAKE_HORSE, ShadowDraw, ShadowRs};

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn idle(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

/// Link by Dodongo's Cavern's mouth on Death Mountain Trail (`Demo_Go`'s tests' place).
const LINK_AT: Vec3 = Vec3::new(-1656.0, 688.0, -519.0);
/// The shadow's prim colour's segment (`crate::shadow`'s `SEG_COLOR`).
const SEG_COLOR: usize = 0x0B;

/// Death Mountain Trail (`ENTR_DEATH_MOUNTAIN_TRAIL_0`), the child by the cavern's mouth.
fn trail(a: &Arc<GameAssets>) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_DEATH_MOUNTAIN_TRAIL_0").expect("entrance");
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("cavern-boss").unwrap();
    let audio = oot_game::audio::GameAudio::boot_logged(a.audio.clone(), a.audio_tables.clone(), false);
    let mut w = PlayState::play_init_with(a.clone(), data().unwrap(), rules().unwrap(), save, audio).expect("Play_Init");
    idle(&mut w, 3);
    w.place_player(LINK_AT, 0);
    idle(&mut w, 1);
    w
}

/// `CollisionPoly_GetNormalF`.
fn normal_f(n: [i16; 3]) -> Vec3 {
    Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32) * (1.0 / 32767.0)
}

/// The first floor (`BgCheck_EntityRaycastDown`, from 100 up) on a 40-unit grid round Link whose
/// normal's y is in `ny`: its point, height and polygon.
fn find_floor(w: &PlayState, ny: std::ops::Range<f32>) -> (Vec3, eng_collision::bgcheck::PolyId) {
    for r in 0i32..20 {
        for i in -r..=r {
            for j in -r..=r {
                if i.abs() != r && j.abs() != r {
                    continue;
                }
                let p = LINK_AT + Vec3::new(i as f32 * 40.0, 100.0, j as f32 * 40.0);
                let (y, poly) = w.col.entity_raycast_down(p);
                if let Some(poly) = poly
                    && ny.contains(&normal_f(w.col.poly(poly).normal).y)
                {
                    return (Vec3::new(p.x, y, p.z), poly);
                }
            }
        }
    }
    panic!("no floor with a normal's y in {ny:?} round Link");
}

#[test]
fn the_inits_give_their_shapes_shadows() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    // Player_InitCommon: ActorShape_Init(&shape, 0, ActorShadow_DrawFeet, ageProperties->unk_04):
    // the child's sAgeProperties[1].unk_04 is 60 (the adult's 90); shadowAlpha 255. His feet
    // (Actor_SetFeetPos in Player_PostLimbDrawGameplay) are by him, near the floor he stands on.
    let p = w.player();
    assert!(!p.adult);
    assert_eq!((p.actor.shape_shadow_draw, p.actor.shape_shadow_scale, p.actor.shape_shadow_alpha, p.actor.shape_y_offset), (ShadowDraw::Feet, 60.0, 255, 0.0));
    for f in p.actor.shape_feet_pos {
        let d = f - p.actor.world_pos;
        assert!(Vec3::new(d.x, 0.0, d.z).length() < 30.0 && d.y.abs() < 20.0, "a foot at {f}, Link at {}", p.actor.world_pos);
    }
    assert!(p.actor.shape_feet_pos[0] != p.actor.shape_feet_pos[1]);
    // EnItem00_Init, a green rupee: scale 0.015, yOffset 750 (its case's; 980 the default),
    // ActorShape_Init(.., ActorShadow_DrawCircle, 6), then shadowAlpha 180.
    let h = w.actor_spawn(ACTOR_EN_ITEM00, LINK_AT + Vec3::new(0.0, 50.0, 0.0), [0; 3], ITEM00_RUPEE_GREEN).expect("En_Item00");
    let r = &w.actors.downcast::<EnItem00>(h).unwrap().actor;
    assert_eq!((r.shape_shadow_draw, r.shape_shadow_scale, r.shape_shadow_alpha, r.shape_y_offset), (ShadowDraw::Circle, 6.0, 180, 750.0));
    // EnElf_Init: ActorShape_Init(&shape, 0, NULL, 15), shadowAlpha 255; FAIRY_HEAL_BIG then
    // shadowDraw = ActorShadow_DrawWhiteCircle.
    let h = w.actor_spawn(ACTOR_EN_ELF, LINK_AT + Vec3::new(0.0, 50.0, 0.0), [0; 3], FAIRY_HEAL_BIG).expect("En_Elf");
    let e = &w.actors.downcast::<EnElf>(h).unwrap().actor;
    assert_eq!((e.shape_shadow_draw, e.shape_shadow_scale, e.shape_shadow_alpha), (ShadowDraw::WhiteCircle, 15.0, 255));
    // The render state carries the shape's shadow (none for NULL: Actor_Draw skips it).
    let rs = w.actors.get(h).unwrap().render_state();
    assert_eq!(rs.shadow.map(|s| (s.draw, s.scale)), Some((ShadowDraw::WhiteCircle, 15.0)));
}

#[test]
fn the_circle_lies_on_a_sloped_floor_shrinking_and_fading_with_height() {
    let Some(a) = assets() else { return };
    let w = trail(&a);
    let (at, poly) = find_floor(&w, 0.6..0.97);
    let n = normal_f(w.col.poly(poly).normal);
    let scale = Vec3::splat(0.01);
    let s = ShadowRs { draw: ShadowDraw::Circle, scale: 30.0, alpha: 255, floor_height: at.y, floor_poly: Some(poly), feet_pos: [at; 2] };
    let draw = |s: &ShadowRs, h: f32, yaw: i16| {
        let mut out = Vec::new();
        shadow::draw(s, at + Vec3::Y * h, scale, yaw, &w.col, &[], &mut out);
        out
    };
    for h in [-49.0, 0.0, 70.0, 150.0, 300.0, 499.0] {
        let out = draw(&s, h, 0x1234);
        assert_eq!(out.len(), 1, "a circle {h} above the floor");
        let c = &out[0];
        assert_eq!(c.mesh, eng_gfx::MeshKey::named(keys::bake(BAKE_CIRCLE)));
        // ActorShadow_Draw: temp1 = the height clamped to 0..150, temp2 = 1 - temp1 / 350; the
        // alpha shadowAlpha × temp2 (truncated), black; the scale temp2 × shadowScale × scale.
        let t = h.clamp(0.0, 150.0);
        let k = 1.0 - t * (1.0 / 350.0);
        assert_eq!(c.params.segments.as_ref().unwrap().prim[SEG_COLOR], Some([0, 0, 0, (255.0 * k) as u32 as u8]));
        let m = c.transform;
        // func_80038A28: at the floor below the actor, its y axis the floor's normal; the circle's
        // x and z (not turned to the actor's yaw) on the floor's plane, each k × 30 × 0.01 long.
        assert!((m.w_axis.truncate() - at).length() < 1e-3, "{}", m.w_axis);
        assert!((m.transform_vector3(Vec3::Y) - n).length() < 1e-3);
        let (x, z) = (m.transform_vector3(Vec3::X), m.transform_vector3(Vec3::Z));
        for v in [x, z] {
            assert!((v.length() - k * 30.0 * 0.01).abs() < 1e-4, "{v} for {h}");
            assert!(v.dot(n).abs() < 1e-4);
        }
        assert!(x.dot(z).abs() < 1e-5);
        assert!(x.x > 0.0, "the circle isn't turned to the yaw");
    }
    // Nothing from 50 below or from 500 up, nor without a floor.
    assert!(draw(&s, -50.5, 0).is_empty());
    assert!(draw(&s, 500.5, 0).is_empty());
    assert!(draw(&ShadowRs { floor_poly: None, ..s }, 0.0, 0).is_empty());
    // The white circle: the same, prim white. The horse's: gHorseShadowDL turned to the yaw
    // (Matrix_RotateY(shape.rot.y) after the floor's matrix): at 0x4000 its x is the circle's -z.
    let white = draw(&ShadowRs { draw: ShadowDraw::WhiteCircle, ..s }, 0.0, 0);
    assert_eq!(white[0].params.segments.as_ref().unwrap().prim[SEG_COLOR], Some([255, 255, 255, 255]));
    let circle = draw(&s, 0.0, 0)[0].transform;
    let horse = &draw(&ShadowRs { draw: ShadowDraw::Horse, ..s }, 0.0, 0x4000)[0];
    assert_eq!(horse.mesh, eng_gfx::MeshKey::named(keys::bake(BAKE_HORSE)));
    let (hx, cz) = (horse.transform.transform_vector3(Vec3::X), circle.transform_vector3(Vec3::Z));
    assert!((hx + cz).length() < 1e-4, "{hx} {cz}");
}

#[test]
fn actor_draw_puts_a_lying_rupee_s_circle_in_the_opaque_list() {
    let Some(a) = assets() else { return };
    let mut w = trail(&a);
    let (at, poly) = find_floor(&w, 0.6..0.97);
    // A green rupee dropped 5 above it (EnItem00_Init without 0x8000: at rest; then gravity -0.9,
    // as Item_DropCollectible gives its drops): its update falls and finds its floor
    // (Actor_UpdateBgCheckInfo while gravity isn't 0). (A placed rupee, gravity 0, never looks
    // for a floor: no shadow.)
    let h = w.actor_spawn(ACTOR_EN_ITEM00, at + Vec3::Y * 5.0, [0; 3], ITEM00_RUPEE_GREEN).expect("En_Item00");
    w.actors.downcast_mut::<EnItem00>(h).unwrap().actor.gravity = -0.9;
    idle(&mut w, 10);
    let r = w.actors.downcast::<EnItem00>(h).expect("still there").actor.clone();
    assert_eq!(r.floor_poly, Some(poly));
    // Actor_Draw after EnItem00_Draw: the circle at shadowAlpha 180 faded by the height above the
    // floor (at most 150), on the floor's plane under the rupee.
    let frame = w.current_frame();
    let mut out = DrawOut::default();
    w.draw(&frame, &ViewInfo::new(Vec3::ZERO, glam::Mat4::IDENTITY), &mut out);
    let key = eng_gfx::MeshKey::named(keys::bake(BAKE_CIRCLE));
    let c = out.opa.iter().find(|c| c.mesh == key && (c.transform.w_axis.truncate() - Vec3::new(r.world_pos.x, r.floor_height, r.world_pos.z)).length() < 1e-3).expect("the rupee's circle");
    let k = 1.0 - (r.world_pos.y - r.floor_height).clamp(0.0, 150.0) * (1.0 / 350.0);
    assert_eq!(c.params.segments.as_ref().unwrap().prim[SEG_COLOR], Some([0, 0, 0, (180.0 * k) as u32 as u8]));
    assert!((c.transform.transform_vector3(Vec3::Y) - normal_f(w.col.poly(poly).normal)).length() < 1e-3);
    assert!((c.transform.transform_vector3(Vec3::X).length() - k * 6.0 * r.scale.x).abs() < 1e-4);
    assert!(!out.xlu.iter().any(|c| c.mesh == key), "no shadow in the translucent list");
}
