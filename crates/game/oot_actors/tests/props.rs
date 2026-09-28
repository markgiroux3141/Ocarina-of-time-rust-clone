//! Kokiri Forest's props with their real inits (`Obj_Hana`, `En_Ishi`, `En_Kusa`): what they
//! become after `Play_Init`, their colliders, and the sword cutting a bush. Expected values
//! come from the actors' C (params, init chains, collider inits).

mod common;

use std::sync::Arc;

use eng_input::pad::PadState;
use glam::Vec3;
use oot_actors::PlayExt;
use oot_actors::en_ishi::EnIshi;
use oot_actors::en_kusa::{self, EnKusa};
use oot_actors::obj_hana::ObjHana;
use oot_game::collision_check::*;
use oot_game::play::{PlayState, scripted_input};
use oot_game::play_scene::GameAssets;
use oot_game::save::SaveContext;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

fn enter(entrance: &str) -> Option<PlayState> {
    let a = assets()?;
    let e = a.scenes.entrance_index(entrance).expect("entrance");
    let save = SaveContext::new(e, false, oot_game::env::clock_time(10, 0) as u16);
    Some(oot_actors::play_entrance(a, common::data()?, common::rules()?, save).expect("Play_Init"))
}

fn frames(w: &mut PlayState, n: usize) {
    for _ in 0..n {
        w.tick_with(scripted_input(PadState::default(), PadState::default()));
    }
}

fn all<T: 'static + oot_game::actor_ctx::ActorImpl>(w: &PlayState) -> Vec<&T> {
    w.actors.all().into_iter().filter_map(|h| w.actors.downcast::<T>(h)).collect()
}

#[test]
fn kokiri_forest_props_initialise_as_their_c_does() {
    let Some(mut w) = enter("ENTR_SPOT04_3") else { return };
    // Frame 1 spawns room 0's list; the room objects load by frame 2 (Object_UpdateBank twice),
    // so the props on object_kanban wait; the ones on the keeps initialise at once.
    frames(&mut w, 4);
    let room0 = w.scene.as_ref().unwrap().rooms[0].clone();
    let placed = |id: i16| room0.actors.iter().filter(|e| e.id == id).count();

    // Obj_Hana: 4 small rocks (params 1: gFieldKakeraDL, scale 0.1, yOffset 58, a 10 x 18 body)
    // and a bush (params 2: gFieldBushDL, scale 0.4, 12 x 44). EVENTCHKINF_40 isn't set, so the
    // bush stays.
    let hana = all::<ObjHana>(&w);
    assert_eq!(hana.len(), placed(oot_actors::obj_hana::ACTOR_OBJ_HANA));
    for h in &hana {
        let (scale, y_off, r, ht) = if h.actor.params & 3 == 1 { (0.1, 58.0, 10, 18) } else { (0.4, 0.0, 12, 44) };
        assert_eq!((h.actor.scale.x, h.actor.shape_y_offset, h.collider.dim.radius, h.collider.dim.height), (scale, y_off, r, ht));
        assert_eq!(h.actor.col_chk_info.mass, MASS_IMMOVABLE);
    }

    // En_Ishi: 4 small rocks (params 0x0200: type 0, snapped to the floor, yOffset 58, scale 0.1,
    // gravity -1.2), with the hard body the sword bounces off.
    let ishi = all::<EnIshi>(&w);
    assert_eq!(ishi.len(), placed(oot_actors::en_ishi::ACTOR_EN_ISHI));
    for i in &ishi {
        assert!(!i.actor.killed);
        assert_eq!((i.actor.scale.x, i.actor.shape_y_offset, i.actor.gravity), (0.1, 58.0, -1.2));
        assert_eq!(i.collider.base.ac_flags & AC_HARD, AC_HARD);
        assert_eq!(i.actor.home_pos, i.actor.world_pos, "snapped to the floor and home");
        assert_ne!(i.actor.shape_rot.y, 0, "rotation 0 in the placement: a random yaw");
    }

    // En_Kusa: the 12 village bushes (0x0200: type 0 on gameplay_field_keep, already loaded) are
    // past EnKusa_WaitObject and waiting to be cut.
    let kusa = all::<EnKusa>(&w);
    assert_eq!(kusa.len(), placed(en_kusa::ACTOR_EN_KUSA));
    for k in &kusa {
        assert_eq!(k.action, en_kusa::Action::Main);
        assert_eq!(k.actor.scale, Vec3::splat(0.4));
        assert_eq!(k.actor.flags & en_kusa::ACTOR_FLAG_ENKUSA_CUT, 0);
    }
}

#[test]
fn the_sword_cuts_a_village_bush() {
    let Some(mut w) = enter("ENTR_SPOT04_3") else { return };
    frames(&mut w, 4);
    // The bush at (385, 0, 643): stand 30 in front of it, facing it, sword out, and slash.
    let bush = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKusa>(h).is_some_and(|k| k.actor.home_pos.x == 385.0)).expect("the bush");
    let bush_pos = w.actors.actor(bush).unwrap().world_pos;
    let pos = bush_pos + Vec3::new(0.0, 0.0, -35.0);
    w.place_player(pos, 0);
    let mut prev = PadState::default();
    let b = PadState { button: eng_input::pad::BTN_B, ..Default::default() };
    // B draws the sword (the change animation), then B again slashes.
    let mut cut_at = None;
    for f in 0..60 {
        let pad = if f % 12 < 2 { b } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        if !w.actors.exists(bush) {
            cut_at = Some(f);
            break;
        }
    }
    assert!(cut_at.is_some(), "the bush is cut (killed: a type-0 bush goes) within 60 frames; player {:?}", w.player().action);
}

#[test]
fn a_slash_cuts_a_piece_off_a_sign() {
    use oot_actors::en_kanban::{self, EnKanban};
    let Some(mut w) = enter("ENTR_SPOT04_3") else { return };
    frames(&mut w, 4);
    // The sign at (49, -80, 967), facing -z (rotation 0x8000): stand 35 in front, facing it.
    let sign = w.actors.all().into_iter().find(|&h| w.actors.downcast::<EnKanban>(h).is_some_and(|k| k.actor.home_pos.x == 49.0 && k.actor.home_pos.z == 967.0)).expect("the sign");
    {
        let k = w.actors.downcast::<EnKanban>(sign).unwrap();
        // EnKanban_Init: whole (partFlags 0xFFFF), textId params | 0x300, child Link: 15 lower.
        assert_eq!((k.part_flags, k.actor.text_id), (0xFFFF, (k.actor.params as u16) | 0x300));
        assert_eq!(k.actor.world_pos.y, k.actor.home_pos.y - 15.0);
        assert_eq!(k.actor.target_mode, 0);
    }
    w.place_player(Vec3::new(49.0, -80.0, 967.0 - 35.0), 0);
    let mut prev = PadState::default();
    let b = PadState { button: eng_input::pad::BTN_B, ..Default::default() };
    let mut piece = None;
    for f in 0..60 {
        let pad = if f % 12 < 2 { b } else { PadState::default() };
        w.tick_with(scripted_input(prev, pad));
        prev = pad;
        let k = w.actors.downcast::<EnKanban>(sign).unwrap();
        if let Some(c) = k.actor.child {
            piece = Some((c, k.part_flags, (k.cut_mark_timer, k.cut_mark_alpha)));
            break;
        }
    }
    let (ph, flags, mark) = piece.expect("a piece is cut off within 60 frames");
    // The piece is the parts the cut took (sCutFlags[cutType] & partFlags), and the sign keeps
    // the rest; the cut mark starts: cutMarkTimer 5, and the same update's step takes it to 4
    // with the alpha at 255.
    let p = w.actors.downcast::<EnKanban>(ph).expect("the piece is an En_Kanban");
    assert_eq!(p.actor.params, en_kanban::ENKANBAN_PIECE);
    assert_eq!(p.actor.parent, Some(sign));
    assert_ne!(p.part_flags, 0);
    assert_eq!(p.part_flags & flags, 0, "the piece's parts left the sign");
    assert_eq!((p.part_flags | flags) & 0x3FF, 0x3FF, "together they're the whole sign");
    assert_eq!(p.actor.gravity, -1.0);
    assert_eq!(mark, (4, 255));
    // Walking 500 away puts the sign back (partFlags 0xFFFF) and the pieces go.
    w.place_player(Vec3::new(49.0, -80.0, 367.0), 0);
    frames(&mut w, 3);
    assert_eq!(w.actors.downcast::<EnKanban>(sign).unwrap().part_flags, 0xFFFF);
    assert!(!w.actors.exists(ph), "the piece is gone");
}
