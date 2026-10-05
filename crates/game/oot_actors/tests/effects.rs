//! The soft sprite effects (GAME-05 milestone 3a, `z_effect_soft_sprite.c`) against the C: a
//! hit's flash (`Effect_Ss_HitMark`) through its life, frame by frame, from
//! `EffectSs_UpdateAll` to the draw.
//!
//! Expected values are worked out from the C in the comments.

mod common;

use common::*;
use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_game::effect::EFFECT_SS_HITMARK;
use oot_game::effect::hitmark::EFFECT_HITMARK_WHITE;
use oot_game::play::{PlayState, scripted_input};

fn idle(w: &mut PlayState) {
    w.tick_with(scripted_input(PadState::default(), PadState::default()));
}

/// `EffectSs_LerpInv` as written: `a + (s32)((b - a) / (f32)weightInv)`, `b` at 0.
fn lerp_inv(a: i16, b: i16, weight_inv: i32) -> i16 {
    if weight_inv == 0 { b } else { (a as i32 + ((b as i32 - a as i32) as f32 / weight_inv as f32) as i32) as i16 }
}

#[test]
fn a_white_hit_mark_lives_eight_frames_fading_to_red() {
    let Some(mut w) = world() else { return };
    idle(&mut w);
    let pos = w.player().actor.world_pos + Vec3::new(0.0, 30.0, 40.0);
    // EffectSsHitMark_SpawnFixedScale(EFFECT_HITMARK_WHITE): scale 300.
    w.with_ss(|s| s.hit_mark_spawn_fixed_scale(EFFECT_HITMARK_WHITE, pos));
    let i = w.effect_ss.table.iter().position(|e| e.ty == EFFECT_SS_HITMARK && e.life >= 0).expect("the hit mark");
    // EffectSsHitMark_Init: life 8 (not dust), gEffHitMarkDL; rTexIndex 0, rType 0, the prim
    // colour sColors[0] (255, 255, 255), the env sColors[1] (255, 255, 0), rScale 300.
    let e = &w.effect_ss.table[i];
    assert_eq!(e.life, 8);
    assert_eq!(e.pos, pos);
    assert_eq!(e.regs[..9], [0, 0, 255, 255, 255, 255, 255, 0, 300]);
    // Each frame EffectSs_UpdateAll takes one off the life, then EffectSsHitMark_Update:
    // rTexIndex = 7 - life; past the first texture the colours step towards sColors[2] (255,
    // 255, 255) and sColors[3] (255, 0, 0) by EffectSs_LerpInv(.., life + 1).
    let (mut prim, mut env) = ([255i16, 255, 255], [255i16, 255, 0]);
    for life in (0..8).rev() {
        idle(&mut w);
        let e = &w.effect_ss.table[i];
        assert_eq!(e.life, life);
        let tex = 7 - life;
        if tex != 0 {
            let to_prim = [255, 255, 255];
            let to_env = [255, 0, 0];
            for c in 0..3 {
                prim[c] = lerp_inv(prim[c], to_prim[c], life as i32 + 1);
                env[c] = lerp_inv(env[c], to_env[c], life as i32 + 1);
            }
        }
        assert_eq!(e.regs[0], tex, "life {life}");
        assert_eq!(e.regs[2..5], prim, "life {life}");
        assert_eq!(e.regs[5..8], env, "life {life}");
        // EffectSsHitMark_Draw: sTextures[rType * 8 + rTexIndex] (gEffHitMark{tex + 1}Tex),
        // prim alpha 255, env alpha 0, on the transparent list.
        let name = oot_game::pack::keys::bake(&format!("EffectSs/hitmark/{}", tex + 1));
        let d = w.effect_draws.xlu.iter().find(|d| d.mesh.name == name).unwrap_or_else(|| panic!("drawn with {name} at life {life}"));
        let sv = d.params.segments.as_ref().unwrap();
        assert_eq!(sv.prim[0x0E], Some([prim[0] as u8, prim[1] as u8, prim[2] as u8, 255]));
        assert_eq!(sv.env[0x0E], Some([env[0] as u8, env[1] as u8, env[2] as u8, 0]));
        // rScale / 100 across (the billboard's scale).
        assert!((d.transform.x_axis.truncate().length() - 3.0).abs() < 1e-4);
        assert_eq!(d.transform.w_axis.truncate(), pos);
    }
    // The last frame's colours are the second pair's: LerpInv by 1 lands on them.
    assert_eq!((prim, env), ([255, 255, 255], [255, 0, 0]));
    // Then life -1: EffectSs_Delete, nothing drawn.
    idle(&mut w);
    assert_eq!(w.effect_ss.table[i].life, -1);
    assert!(!w.effect_draws.xlu.iter().any(|d| d.mesh.name.contains("EffectSs/hitmark")));
}
