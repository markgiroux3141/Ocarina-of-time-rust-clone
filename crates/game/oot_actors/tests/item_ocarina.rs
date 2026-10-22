//! GAME-10 milestone 7 against the C: `Item_Ocarina` (`z_item_ocarina.c`). The Ocarina of Time in
//! Hyrule Field's moat (type 3: its init's kills, Navi's `Elf_Msg2`, its offer and bubbles, the
//! dive that takes it and the Song of Time's cutscene its text starts), Impa's throws (types 0 and
//! 1, driven by the cutscene's frame), type 2 and the others.
//!
//! Entered from `zora-sapphire` (the three stones, the milestone's start) at `ENTR_HYRULE_FIELD_0`
//! (layer 1, the child's day field). Expected values are worked out from the C in the comments.

mod common;

use std::sync::Arc;

use common::*;
use eng_input::pad::{BTN_A, PadState};
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::item_ocarina::{self, Action, ItemOcarina};
use oot_game::actor_ctx::ActorHandle;
use oot_game::item::ITEM_OCARINA_OF_TIME;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::{EVENTCHKINF_ZELDA_FLED_CASTLE, SaveContext};

/// The ocarina's placement (`spot00` layer 1 room 0: `Item_Ocarina` 0x0003).
const OCARINA_POS: Vec3 = Vec3::new(299.0, -136.0, 884.0);
/// The moat's water surface (its water box, `ootx`'s probe: -60) and floor (-140) there.
const MOAT_SURFACE: f32 = -60.0;
/// `ACTOR_ELF_MSG2` (`actor_table.h`: 0x0173).
const ACTOR_ELF_MSG2: i16 = 0x0173;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// Hyrule Field (`ENTR_HYRULE_FIELD_0`, layer 1) at 10:00 from `zora-sapphire`, `tweak` applied
/// last.
fn field(a: &Arc<GameAssets>, tweak: impl FnOnce(&mut SaveContext)) -> PlayState {
    let e = a.scenes.entrance_index("ENTR_HYRULE_FIELD_0").unwrap();
    let mut save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    save.apply_preset("zora-sapphire").unwrap();
    tweak(&mut save);
    oot_actors::play_entrance(a.clone(), data().unwrap(), rules().unwrap(), save).unwrap()
}

/// After Zelda's escape: `EVENTCHKINF_ZELDA_FLED_CASTLE` and `_82` set (the drawbridge's trigger).
fn fled(s: &mut SaveContext) {
    s.set_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE);
    s.set_event_chk_inf(0x82);
}

fn step(w: &mut PlayState, prev: &mut PadState, pad: PadState) {
    w.tick_with(scripted_input(*prev, pad));
    *prev = pad;
}

fn idle(w: &mut PlayState, n: u32) {
    let mut prev = PadState::default();
    for _ in 0..n {
        step(w, &mut prev, PadState::default());
    }
}

fn action(w: &PlayState) -> &'static str {
    w.player().action.decomp_name()
}

/// Every live `Item_Ocarina` in play.
fn ocarinas(w: &PlayState) -> Vec<ActorHandle> {
    w.actors.all().into_iter().filter(|&h| w.actors.downcast::<ItemOcarina>(h).is_some_and(|o| !o.actor.killed)).collect()
}

fn ocarina(w: &PlayState, h: ActorHandle) -> &ItemOcarina {
    w.actors.downcast::<ItemOcarina>(h).unwrap()
}

#[test]
fn the_ocarina_lies_in_the_moat_once_zelda_has_fled() {
    let Some(a) = assets() else { return };
    // Zelda not fled (the stones alone): killed at init.
    let mut w = field(&a, |_| {});
    idle(&mut w, 8);
    assert!(ocarinas(&w).is_empty());

    let mut w = field(&a, fled);
    idle(&mut w, 8);
    let v = ocarinas(&w);
    assert_eq!(v.len(), 1);
    let o = ocarina(&w, v[0]);
    // Type 3: ItemOcarina_WaitInWater, scale 0.2, spinRotOffset 0x400 (unused by type 3), drawn.
    assert_eq!(o.actor.world_pos, OCARINA_POS);
    assert_eq!(o.actor.scale, Vec3::splat(0.2));
    assert_eq!((o.action, o.spin_rot_offset, o.drawn), (Action::WaitInWater, 0x400, true));
    // Navi's Elf_Msg2 0x3800 at (299, -140, 884), rot (0, 4, 1).
    let navi: Vec<_> = w.actors.all().into_iter().filter_map(|h| w.actors.actor(h)).filter(|ac| ac.id == ACTOR_ELF_MSG2 && !ac.killed).map(|ac| (ac.params, ac.world_pos, ac.world_rot)).collect();
    assert!(navi.iter().any(|&(p, pos, rot)| p == 0x3800 && pos == Vec3::new(299.0, -140.0, 884.0) && (rot.x, rot.y, rot.z) == (0, 4, 1)), "{navi:x?}");
    // Its bubbles: EffectSsBubble_Spawn on the frames where gameplayFrames & 13 is 0.
    let bubbles = |w: &PlayState| w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == oot_game::effect::EFFECT_SS_BUBBLE && (e.pos.x - OCARINA_POS.x).abs() < 30.0).count();
    let mut seen = 0;
    for _ in 0..32 {
        let before = bubbles(&w);
        idle(&mut w, 1);
        if bubbles(&w) > before {
            seen += 1;
        }
    }
    // Two of every 16 frames (gameplayFrames 0, 2, 16, 18, ... mod 16): four in any 32.
    assert_eq!(seen, 4);

    // Taken once (EVENTCHKINF_43): gone at init.
    let mut w = field(&a, |s| {
        fled(s);
        s.set_event_chk_inf(item_ocarina::EVENTCHKINF_43);
    });
    idle(&mut w, 8);
    assert!(ocarinas(&w).is_empty());
}

#[test]
fn a_dive_takes_the_ocarina_and_its_text_starts_the_song_of_time() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, fled);
    idle(&mut w, 20);
    // Treading at the moat's surface (-60), 76 above the ocarina, facing it from 14 short.
    w.place_player(Vec3::new(OCARINA_POS.x, MOAT_SURFACE, OCARINA_POS.z - 14.0), 0);
    idle(&mut w, 60);
    assert!(w.player().state1 & oot_actors::player::STATE1_27 != 0, "swimming: {}", action(&w));
    let h = ocarinas(&w)[0];
    let mut prev = PadState::default();
    let a_pad = PadState { button: BTN_A, ..Default::default() };
    let mut actions: Vec<&'static str> = Vec::new();
    let mut taken = None;
    for f in 0..300 {
        step(&mut w, &mut prev, a_pad);
        if taken.is_none() && ocarina(&w, h).action == Action::StartSoTCutscene {
            taken = Some(f);
        }
        let s = action(&w);
        if actions.last() != Some(&s) {
            actions.push(s);
        }
    }
    eprintln!("taken at {taken:?}: {actions:?}");
    // The floor is at -140: any dive (D_80854784[0] = 120, to -180) reaches within the offer's 50.
    // Player_ActionHandler_2 sets its parent: ItemOcarina_WaitInWater sets EVENTCHKINF_43 and switch
    // 3, stops drawing it (it isn't killed) and goes on to ItemOcarina_StartSoTCutscene.
    assert!(taken.is_some(), "never taken: {actions:?}");
    assert!(w.save.get_event_chk_inf(item_ocarina::EVENTCHKINF_43));
    assert!(w.flags.get_switch(item_ocarina::OCARINA_SWITCH_FLAG));
    assert!(!ocarina(&w, h).drawn);
    assert!(!ocarina(&w, h).actor.killed);
    // Surfacing with it held up; its text (sGetItemTable's 0x3A) and Item_Give(ITEM_OCARINA_OF_TIME).
    let mut texts = Vec::new();
    let mut started = None;
    let mut prev = PadState::default();
    for f in 0..900 {
        if w.message_state() != oot_game::message::TEXT_STATE_NONE && texts.last() != Some(&w.msg_ctx.text_id) {
            texts.push(w.msg_ctx.text_id);
        }
        let pad = if f % 20 == 19 { a_pad } else { PadState::default() };
        step(&mut w, &mut prev, pad);
        // Actor_TextboxIsClosing: gHyruleFieldZeldaSongOfTimeCs and cutsceneTrigger 1, the
        // cutscene's start on the next frames (Cutscene_HandleConditionalTriggers).
        if started.is_none() && w.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE {
            started = Some(f);
            break;
        }
    }
    eprintln!("texts {texts:x?}, started at {started:?}");
    assert_eq!(texts.first(), Some(&0x3A));
    assert!(started.is_some(), "the Song of Time's cutscene never started");
    let script = w.cs_ctx.segment.as_ref().expect("a script");
    assert!(Arc::ptr_eq(script, &w.cutscene_script(item_ocarina::SONG_OF_TIME_CS).unwrap()));
    // Item_Give(ITEM_OCARINA_OF_TIME): the ocarina slot, and the Fairy Ocarina on C swapped for it.
    assert_eq!(w.save.inventory.items[oot_game::item::SLOT_OCARINA], ITEM_OCARINA_OF_TIME);
}

#[test]
fn impa_s_throws_follow_the_cutscene_s_frames() {
    let Some(a) = assets() else { return };
    let mut w = field(&a, fled);
    idle(&mut w, 8);
    w.object_ctx.spawn(0x00DE); // OBJECT_GI_OCARINA (in layer 1's list already)
    let start = Vec3::new(4.0, 81.0, 2600.0);
    // Type 0 (Impa's, layer 5 frame 845): ItemOcarina_GetThrown on its first update.
    let h = w.actor_spawn(item_ocarina::ACTOR_ITEM_OCARINA, start, [0, 0, 0], 0).unwrap();
    assert_eq!((ocarina(&w, h).action, ocarina(&w, h).actor.scale), (Action::GetThrown, Vec3::splat(0.1)));
    w.cs_ctx.frames = 845;
    idle(&mut w, 1);
    let o = ocarina(&w, h);
    // Gravity -0.3 to -5, thrown up at 6, on to ItemOcarina_Fly.
    assert_eq!((o.action, o.actor.gravity, o.actor.min_velocity_y, o.actor.velocity), (Action::Fly, -0.3, -5.0, Vec3::new(0.0, 6.0, 0.0)));
    let rot0 = o.actor.shape_rot;
    w.cs_ctx.frames = 846;
    idle(&mut w, 1);
    let o = ocarina(&w, h);
    // Actor_UpdatePos (6 up at R_UPDATE_RATE * 0.5 = 1.5: 9), then velocity.y += gravity (5.7); the
    // spin 0x800 about x, 0xC00 about y.
    assert_eq!(o.actor.world_pos, start + Vec3::new(0.0, 9.0, 0.0));
    assert!((o.actor.velocity.y - 5.7).abs() < 1e-5);
    assert_eq!((o.actor.shape_rot.x, o.actor.shape_rot.y), (rot0.x.wrapping_add(0x800), rot0.y.wrapping_add(0xC00)));
    // Frame 881: put at (250, 60, 1075) and thrown at (1, -5, -7) (after this frame's move).
    w.cs_ctx.frames = 881;
    idle(&mut w, 1);
    let o = ocarina(&w, h);
    assert_eq!((o.actor.world_pos, o.actor.velocity), (Vec3::new(250.0, 60.0, 1075.0), Vec3::new(1.0, -5.0, -7.0)));
    // Frame 897: into the water: a ripple and a splash, stopped, gravity -0.1 to -0.5, no spin.
    let ripples = |w: &PlayState| w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == oot_game::effect::EFFECT_SS_G_RIPPLE).count();
    let splashes = |w: &PlayState| w.effect_ss.table.iter().filter(|e| e.life > -1 && e.ty == oot_game::effect::EFFECT_SS_G_SPLASH).count();
    let (r0, s0) = (ripples(&w), splashes(&w));
    w.cs_ctx.frames = 897;
    idle(&mut w, 1);
    let o = ocarina(&w, h);
    assert_eq!((o.actor.velocity, o.actor.gravity, o.actor.min_velocity_y, o.spin_rot_offset), (Vec3::ZERO, -0.1, -0.5, 0));
    assert_eq!((ripples(&w), splashes(&w)), (r0 + 1, s0 + 1));
    // Frame 906: a second ripple at (274, -60, 907).
    w.cs_ctx.frames = 906;
    idle(&mut w, 1);
    assert!(w.effect_ss.table.iter().any(|e| e.life > -1 && e.ty == oot_game::effect::EFFECT_SS_G_RIPPLE && e.pos == Vec3::new(274.0, -60.0, 907.0)));
    // Sinking: velocity.y -0.1 a frame to -0.5.
    w.cs_ctx.frames = 907;
    idle(&mut w, 10);
    assert!((ocarina(&w, h).actor.velocity.y + 0.5).abs() < 1e-5);

    // Type 1 (Impa's outside layer 5, frame 195): func_80B865E0 tosses it at (0, 4, 6), gravity -0.3.
    let h = w.actor_spawn(item_ocarina::ACTOR_ITEM_OCARINA, start, [0, 0, 0], 1).unwrap();
    w.cs_ctx.frames = 195;
    idle(&mut w, 1);
    let o = ocarina(&w, h);
    assert_eq!((o.action, o.actor.gravity, o.actor.velocity), (Action::Fly1, -0.3, Vec3::new(0.0, 4.0, 6.0)));
    // Frame 220: put at (144, 80, 1686), thrown at (1, 2, -7), gravity -0.15 to -5.
    w.cs_ctx.frames = 220;
    idle(&mut w, 1);
    let o = ocarina(&w, h);
    assert_eq!((o.actor.world_pos, o.actor.velocity, o.actor.gravity, o.actor.min_velocity_y), (Vec3::new(144.0, 80.0, 1686.0), Vec3::new(1.0, 2.0, -7.0), -0.15, -5.0));

    // Type 2: nothing. Any other: killed at init.
    let h = w.actor_spawn(item_ocarina::ACTOR_ITEM_OCARINA, start, [0, 0, 0], 2).unwrap();
    idle(&mut w, 5);
    assert_eq!((ocarina(&w, h).action, ocarina(&w, h).actor.world_pos), (Action::DoNothing, start));
    let h = w.actor_spawn(item_ocarina::ACTOR_ITEM_OCARINA, start, [0, 0, 0], 4).unwrap();
    assert!(w.actors.actor(h).is_none_or(|o| o.killed));
}
