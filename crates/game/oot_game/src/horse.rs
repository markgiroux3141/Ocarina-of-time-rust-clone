//! `z_horse.c` (the player's horse at a scene's start) and `z_actor.c`'s horse helpers
//! (`Actor_MountHorse`, `Actor_RequestHorseCameraSetting`, `Actor_IsMounted`, `Actor_NotMounted`;
//! `Actor_SetRideActor` is `PlayerIface::set_ride_actor`).
//!
//! `Horse_InitPlayerHorse` runs at the end of `Play_Init` (`Actor_InitPlayerHorse`) for an adult in
//! a scene a horse can be in (`Horse_CanSpawn`: Hyrule Field, Lake Hylia, Gerudo Valley, the
//! Gerudo's Fortress, Lon Lon Ranch):
//! - On a cutscene layer, after a jump over one of Lon Lon Ranch's fences (`ENTR_HYRULE_FIELD_11`,
//!   `_12`, `_13`, `_15`, not a respawn) or shut in the ranch with Epona won from Ingo
//!   (`INGO_RACE_STATE_TRAPPED_WIN_EPONA` without `EVENTCHKINF_EPONA_OBTAINED`):
//!   `Horse_SetupInCutscene`, Link on a horse spawned for the scene's script.
//! - Else `Horse_SetupInGameplay`: Link on Epona if he left the last scene riding her
//!   (`R_EXITED_SCENE_RIDING_HORSE`); Epona where he left her (`horseData`), or inactive at the
//!   scene's place for Epona's Song to call her; on the ranch without her, Epona herself (she runs
//!   from Link until the song); the stable by night without her, a horse standing in it.
//!
//! The Gerudo's Fortress's horseback archery (`minigameState` 3, its `CS_INDEX_0`) is Phase 18's:
//! its branches are here, and the horse's `HORSE_PTYPE_HORSEBACK_ARCHERY` part is logged.

use glam::Vec3;

use crate::actor::Actor;
use crate::actor_ctx::ActorHandle;
use crate::play::PlayState;
use crate::target::yaw_to;

/// `ACTOR_EN_HORSE` (`actor_table.h`: 0x0014).
pub const ACTOR_EN_HORSE: i16 = 0x0014;

// `SCENE_*` (`scene_table.h`).
pub const SCENE_STABLE: u16 = 0x36;
pub const SCENE_LON_LON_BUILDINGS: u16 = 0x4C;
pub const SCENE_HYRULE_FIELD: u16 = 0x51;
pub const SCENE_LAKE_HYLIA: u16 = 0x57;
pub const SCENE_GERUDO_VALLEY: u16 = 0x5A;
pub const SCENE_GERUDOS_FORTRESS: u16 = 0x5D;
pub const SCENE_LON_LON_RANCH: u16 = 0x63;

// `ENTR_*` (`entrance_table.h`).
pub const ENTR_HYRULE_FIELD_11: u16 = 0x28A;
pub const ENTR_HYRULE_FIELD_12: u16 = 0x28E;
pub const ENTR_HYRULE_FIELD_13: u16 = 0x292;
pub const ENTR_HYRULE_FIELD_15: u16 = 0x476;
pub const ENTR_LON_LON_RANCH_0: u16 = 0x157;
pub const ENTR_LON_LON_RANCH_7: u16 = 0x4CE;

// `CS_INDEX_*` (`save.h`).
pub const CS_INDEX_0: u16 = 0xFFF0;
pub const CS_INDEX_1: u16 = 0xFFF1;
pub const CS_INDEX_3: u16 = 0xFFF3;
pub const CS_INDEX_4: u16 = 0xFFF4;
pub const CS_INDEX_5: u16 = 0xFFF5;
pub const CS_INDEX_6: u16 = 0xFFF6;

/// `EVENTCHKINF_EPONA_OBTAINED` (`save.h`).
pub const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
/// `R_DEBUG_FORCE_EPONA_OBTAINED` (`regs.h`: `DREG(1)`): the debug registers start zeroed.
pub const R_DEBUG_FORCE_EPONA_OBTAINED: i16 = 0;

/// `INGO_RACE_STATE_TRAPPED_WIN_EPONA` (`save.h`): Epona won, Ingo shuts Link in the ranch.
pub const INGO_RACE_STATE_TRAPPED_WIN_EPONA: u16 = 6;
/// `EVENTINF_INGO_RACE_SECOND_RACE` (`save.h`): the race for Epona.
pub const EVENTINF_INGO_RACE_SECOND_RACE: u16 = 0x06;

/// `GET_EVENTINF_INGO_RACE_STATE()`: `eventInf[EVENTINF_INDEX_HORSES]`'s low four bits.
pub fn ingo_race_state(save: &crate::save::SaveContext) -> u16 {
    save.event_inf[0] & 0xF
}

/// `GET_EVENTINF_INGO_RACE_HORSETYPE()` (`EVENTINF_INGO_RACE_HORSETYPE`, bit 4): the horse Link
/// raced on (`HORSE_EPONA` 0, `HORSE_HNI` 1).
pub fn ingo_race_horsetype(save: &crate::save::SaveContext) -> u16 {
    (save.event_inf[0] >> 4) & 1
}

/// `HorseParamType` (`z_en_horse.h`) the spawns here give.
pub const HORSE_PTYPE_1: i16 = 1;
pub const HORSE_PTYPE_INACTIVE: i16 = 2;
pub const HORSE_PTYPE_5: i16 = 5;
pub const HORSE_PTYPE_6: i16 = 6;
pub const HORSE_PTYPE_7: i16 = 7;
pub const HORSE_PTYPE_HORSEBACK_ARCHERY: i16 = 8;
pub const HORSE_PTYPE_PLAYER_SPAWNED_RIDING: i16 = 9;
/// `HORSE_EPONA` (`z_en_horse.h`'s `HorseType`).
pub const HORSE_EPONA: u16 = 0;

/// `CAM_SET_HORSE` (`camera.h`: 0x06, "HORSE0").
pub const CAM_SET_HORSE: i16 = 0x06;
/// `ROOM_TYPE_4` (`room.h`).
const ROOM_TYPE_4: u8 = 4;

/// `Horse_CanSpawn`: whether the player's horse can be in `scene_id`.
pub fn horse_can_spawn(scene_id: i32) -> bool {
    [SCENE_HYRULE_FIELD, SCENE_LAKE_HYLIA, SCENE_GERUDO_VALLEY, SCENE_GERUDOS_FORTRESS, SCENE_LON_LON_RANCH].iter().any(|&s| s as i32 == scene_id)
}

/// `Horse_ResetHorseData`: Epona left at Hyrule Field's neutral place.
pub fn horse_reset_horse_data(play: &mut PlayState) {
    play.save.horse_data = crate::save::SRAM_HORSE_DATA;
}

/// `Horse_FixLakeHyliaPosition`: Epona left in Lake Hylia moved to a dry place (after the Water
/// Medallion the lake fills; called by `Door_Warp1`, Phase 16).
pub fn horse_fix_lake_hylia_position(play: &mut PlayState) {
    if play.save.horse_data.scene_id == SCENE_LAKE_HYLIA as i16 {
        play.save.horse_data = crate::save::HorseData { scene_id: SCENE_LAKE_HYLIA as i16, pos: [-2065, -863, 1839], angle: 0 };
    }
}

/// Whether Epona is had (`EVENTCHKINF_EPONA_OBTAINED`, or the debug `R_DEBUG_FORCE_EPONA_OBTAINED`).
fn epona_obtained(play: &PlayState) -> bool {
    play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) || R_DEBUG_FORCE_EPONA_OBTAINED != 0
}

/// `Actor_RequestHorseCameraSetting`: the main camera to `CAM_SET_HORSE`, unless the room is of
/// type 4 or the scene's camera is fixed.
pub fn actor_request_horse_camera_setting(play: &mut PlayState) {
    if play.room_ctx.cur.behavior_type1 != ROOM_TYPE_4 && play.cam_is_not_fixed() {
        let d = play.data.clone();
        play.game_camera.change_setting(&d.camera, CAM_SET_HORSE);
    }
}

/// `Actor_MountHorse`: Player's `rideActor` is `horse` and he rides (`PLAYER_STATE1_23`); the
/// horse's `child` is Player.
pub fn actor_mount_horse(play: &mut PlayState, horse: ActorHandle) {
    let Some(ph) = play.player else { return };
    if let Some(p) = play.actors.get_mut(ph).and_then(|p| p.as_player_mut()) {
        p.mount_horse(horse);
    }
    if let Some(h) = play.actors.actor_mut(horse) {
        h.child = Some(ph);
    }
}

/// `Actor_IsMounted`: someone rides the horse (its `child`).
pub fn actor_is_mounted(horse: &Actor) -> bool {
    horse.child.is_some()
}

/// `Actor_NotMounted`.
pub fn actor_not_mounted(horse: &Actor) -> bool {
    horse.child.is_none()
}

/// `Horse_SetupInGameplay`'s `HorseSpawn` table: each scene's place for an inactive Epona.
const HORSE_SPAWNS: [(u16, [i16; 3], i16, i16); 5] = [
    (SCENE_HYRULE_FIELD, [-460, 100, 6640], 0, HORSE_PTYPE_INACTIVE),
    (SCENE_LAKE_HYLIA, [-1929, -1025, 768], 0, HORSE_PTYPE_INACTIVE),
    (SCENE_GERUDO_VALLEY, [2566, -259, 767], 0, HORSE_PTYPE_INACTIVE),
    (SCENE_GERUDOS_FORTRESS, [-328, 10, 953], 0, HORSE_PTYPE_INACTIVE),
    (SCENE_LON_LON_RANCH, [928, 0, -2280], 0, HORSE_PTYPE_INACTIVE),
];

fn v3(p: [i16; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// `Actor_Spawn(&play->actorCtx, play, ACTOR_EN_HORSE, pos, 0, yaw, 0, params)`.
fn spawn_horse(play: &mut PlayState, pos: Vec3, yaw: i16, params: i16) -> Option<ActorHandle> {
    match play.actor_spawn(ACTOR_EN_HORSE, pos, [0, yaw, 0], params) {
        Ok(h) => Some(h),
        Err(e) => {
            log::error!("z_horse.c: no En_Horse spawned ({e:?})");
            None
        }
    }
}

/// `Horse_SetupInGameplay`.
pub fn horse_setup_in_gameplay(play: &mut PlayState) {
    let Some(ph) = play.player else { return };
    let Some((ppos, prot)) = play.actors.actor(ph).map(|a| (a.world_pos, a.shape_rot)) else { return };
    let scene = play.scene_id;
    if play.save.r_exited_scene_riding_horse && epona_obtained(play) {
        // Player entering the scene on top of the horse.
        let r = play.actor_spawn(ACTOR_EN_HORSE, ppos, [prot.x, prot.y, prot.z], HORSE_PTYPE_PLAYER_SPAWNED_RIDING);
        let Ok(h) = r else {
            log::error!("z_horse.c: no En_Horse to ride on ({r:?})");
            return;
        };
        actor_mount_horse(play, h);
        actor_request_horse_camera_setting(play);
        play.save.horse_data.scene_id = scene as i16;
        if scene == SCENE_GERUDOS_FORTRESS
            && let Some(a) = play.actors.actor_mut(h)
        {
            a.room = -1;
        }
    } else if scene == SCENE_GERUDOS_FORTRESS && play.save.minigame_state == 3 {
        // Horseback archery over (Phase 18).
        play.save.minigame_state = 0;
        if let Some(h) = spawn_horse(play, Vec3::new(3586.0, 1413.0, -402.0), 0x4000, HORSE_PTYPE_1)
            && let Some(a) = play.actors.actor_mut(h)
        {
            a.room = -1;
        }
    } else if play.save.entrance_index == ENTR_LON_LON_RANCH_7 && play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
        // Malon's obstacle course over.
        spawn_horse(play, Vec3::new(-25.0, 0.0, -1600.0), -0x4000, HORSE_PTYPE_1);
    } else if scene as i16 == play.save.horse_data.scene_id && epona_obtained(play) {
        // Where the horse was left.
        if horse_can_spawn(play.save.horse_data.scene_id as i32) {
            let hd = play.save.horse_data;
            if let Some(h) = spawn_horse(play, v3(hd.pos), hd.angle, HORSE_PTYPE_1)
                && scene == SCENE_GERUDOS_FORTRESS
                && let Some(a) = play.actors.actor_mut(h)
            {
                a.room = -1;
            }
        } else {
            log::error!("Horse_SetNormal(): {} set spot is no good", play.save.horse_data.scene_id);
            horse_reset_horse_data(play);
        }
    } else if scene == SCENE_LON_LON_RANCH && !epona_obtained(play) {
        // The ranch without Epona: Epona herself.
        spawn_horse(play, Vec3::new(0.0, 0.0, -500.0), 0, HORSE_PTYPE_1);
    } else if epona_obtained(play) {
        // Epona had but left elsewhere: inactive at the scene's place, for her song.
        if let Some(&(s, pos, angle, ty)) = HORSE_SPAWNS.iter().find(|s| s.0 == scene)
            && let Some(h) = spawn_horse(play, v3(pos), angle, ty)
            && s == SCENE_GERUDOS_FORTRESS
            && let Some(a) = play.actors.actor_mut(h)
        {
            a.room = -1;
        }
    } else if !epona_obtained(play) && scene == SCENE_LON_LON_BUILDINGS && !play.save.is_day() {
        spawn_horse(play, Vec3::new(0.0, 0.0, -60.0), 0x7360, HORSE_PTYPE_1);
    }
}

/// `HorseCutsceneSpawn`'s table: the horse a scene's script rides, by its `cutsceneIndex`.
const HORSE_CS_SPAWNS: [(u16, u16, [i16; 3], i16, i16); 8] = [
    (SCENE_GERUDOS_FORTRESS, CS_INDEX_0, [3600, 1413, 360], 0x8001u16 as i16, HORSE_PTYPE_HORSEBACK_ARCHERY),
    // The horse race.
    (SCENE_LON_LON_RANCH, CS_INDEX_0, [-250, 1, -1580], 0x4000, HORSE_PTYPE_6),
    // Epona's Song learned.
    (SCENE_LON_LON_RANCH, CS_INDEX_1, [0, 0, 0], 0, HORSE_PTYPE_5),
    // The credits.
    (SCENE_LON_LON_RANCH, CS_INDEX_5, [0, 0, 0], 0, HORSE_PTYPE_7),
    // The title screen.
    (SCENE_HYRULE_FIELD, CS_INDEX_3, [-2961, 313, 7700], 0, HORSE_PTYPE_7),
    (SCENE_HYRULE_FIELD, CS_INDEX_4, [-1900, 313, 7015], 0, HORSE_PTYPE_7),
    // The credits.
    (SCENE_HYRULE_FIELD, CS_INDEX_5, [-4043, 313, 6933], 0, HORSE_PTYPE_7),
    // Unused: hopping Lon Lon Ranch's north gate.
    (SCENE_HYRULE_FIELD, CS_INDEX_6, [-4043, 313, 6933], 0, HORSE_PTYPE_7),
];

/// Whether the entrance is one of the field's after a jump over a ranch fence.
fn is_fence_jump_entrance(entrance: u16) -> bool {
    matches!(entrance, ENTR_HYRULE_FIELD_11 | ENTR_HYRULE_FIELD_12 | ENTR_HYRULE_FIELD_13 | ENTR_HYRULE_FIELD_15)
}

/// `Horse_SetupInCutscene`.
pub fn horse_setup_in_cutscene(play: &mut PlayState) {
    let Some(ph) = play.player else { return };
    let Some((ppos, pyaw)) = play.actors.actor(ph).map(|a| (a.world_pos, a.world_rot.y)) else { return };
    let scene = play.scene_id;
    let entrance = play.save.entrance_index;
    if is_fence_jump_entrance(entrance) && play.save.respawn_flag == 0 {
        // Epona hopping over one of Lon Lon Ranch's fences. (The east and west places are paired
        // with the wrong entrances, as the C's comment notes: the script moves the horse before
        // it's seen.)
        let spawn_pos = match entrance {
            ENTR_HYRULE_FIELD_11 => [-2961, 313, 7700],
            ENTR_HYRULE_FIELD_12 => [-1900, 313, 7015],
            ENTR_HYRULE_FIELD_13 => [-4043, 313, 6933],
            _ => [-2313, 313, 5990],
        };
        let Some(h) = spawn_horse(play, v3(spawn_pos), pyaw, HORSE_PTYPE_7) else { return };
        actor_mount_horse(play, h);
        actor_request_horse_camera_setting(play);
        play.save.horse_data.scene_id = scene as i16;
    } else if scene == SCENE_LON_LON_RANCH && ingo_race_state(&play.save) == INGO_RACE_STATE_TRAPPED_WIN_EPONA && !epona_obtained(play) {
        let Some(h) = spawn_horse(play, Vec3::new(894.0, 0.0, -2084.0), -0x7FFF, HORSE_PTYPE_5) else { return };
        actor_mount_horse(play, h);
        actor_request_horse_camera_setting(play);
        play.save.horse_data.scene_id = scene as i16;
    } else {
        let cs_index = play.save.cutscene_index;
        let Some(&(_, _, mut pos, angle, ty)) = HORSE_CS_SPAWNS.iter().find(|s| s.0 == scene && s.1 == cs_index) else { return };
        if ty == HORSE_PTYPE_7 {
            if scene == SCENE_LON_LON_RANCH && cs_index == CS_INDEX_1 {
                // (Unreached: the ranch's CS_INDEX_1 row is HORSE_PTYPE_5.)
                pos = [ppos.x as i16, ppos.y as i16, ppos.z as i16];
            }
            let Some(h) = spawn_horse(play, v3(pos), pyaw, ty) else { return };
            actor_mount_horse(play, h);
            actor_request_horse_camera_setting(play);
        } else if ty == HORSE_PTYPE_5 || ty == HORSE_PTYPE_6 || ty == HORSE_PTYPE_HORSEBACK_ARCHERY {
            // HORSE_HNI (0x8000) for the race on the rental horse.
            let param_flag = if ingo_race_horsetype(&play.save) != HORSE_EPONA && ty == HORSE_PTYPE_6 { 0x8000u16 as i16 } else { 0 };
            let Some(h) = spawn_horse(play, v3(pos), angle, ty | param_flag) else { return };
            if let Some(a) = play.actors.actor_mut(ph) {
                a.world_pos = v3(pos);
                a.shape_rot.x = 0;
                a.shape_rot.z = 0;
                a.shape_rot.y = angle;
            }
            actor_mount_horse(play, h);
            actor_request_horse_camera_setting(play);
            let p = v3(pos);
            let eye = Vec3::new(p.x - 200.0, p.y + 100.0, p.z);
            // Play_SetCameraAtEye(play, play->activeCamId, &player->actor.world.pos, &eye).
            play.game_camera.set_at_eye(p, eye, p);
        } else {
            spawn_horse(play, v3(pos), angle, ty);
        }
    }
}

/// `Horse_InitPlayerHorse` (`Actor_InitPlayerHorse`, at the end of `Play_Init`): only for an adult.
pub fn horse_init_player_horse(play: &mut PlayState) {
    if !play.save.adult {
        return;
    }
    if !horse_can_spawn(play.save.horse_data.scene_id as i32) {
        log::error!("Horse_Set_Check(): {} set spot is no good", play.save.horse_data.scene_id);
        horse_reset_horse_data(play);
    }
    if !horse_can_spawn(play.scene_id as i32) {
        return;
    }
    let fence_jump = is_fence_jump_entrance(play.save.entrance_index) && play.save.respawn_flag == 0;
    let trapped = play.scene_id == SCENE_LON_LON_RANCH && ingo_race_state(&play.save) == INGO_RACE_STATE_TRAPPED_WIN_EPONA && !epona_obtained(play);
    if play.is_cutscene_layer() || fence_jump || trapped {
        horse_setup_in_cutscene(play);
    } else {
        horse_setup_in_gameplay(play);
    }
}

/// `Horse_RotateToPoint`: turns the actor's `world.rot.y` towards `pos` by at most `turn_amount`
/// (straight to it when nearer), and its shape with it.
pub fn horse_rotate_to_point(actor: &mut Actor, pos: Vec3, turn_amount: i16) {
    let x = yaw_to(actor.world_pos, pos).wrapping_sub(actor.world_rot.y);
    if x > turn_amount {
        actor.world_rot.y = actor.world_rot.y.wrapping_add(turn_amount);
    } else if x < turn_amount.wrapping_neg() {
        actor.world_rot.y = actor.world_rot.y.wrapping_sub(turn_amount);
    } else {
        actor.world_rot.y = actor.world_rot.y.wrapping_add(x);
    }
    actor.shape_rot.y = actor.world_rot.y;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_by_at_most_the_amount() {
        let mut a = Actor::new(Vec3::ZERO, 0);
        // The point due +x: Math_Vec3f_Yaw is atan2(dz, dx) = 0x4000.
        horse_rotate_to_point(&mut a, Vec3::new(100.0, 0.0, 0.0), 300);
        assert_eq!((a.world_rot.y, a.shape_rot.y), (300, 300));
        // Near enough: straight to it.
        a.world_rot.y = 0x4000 - 100;
        horse_rotate_to_point(&mut a, Vec3::new(100.0, 0.0, 0.0), 300);
        assert_eq!(a.world_rot.y, 0x4000);
        // The other way.
        a.world_rot.y = 0x4000 + 1000;
        horse_rotate_to_point(&mut a, Vec3::new(100.0, 0.0, 0.0), 300);
        assert_eq!(a.world_rot.y, 0x4000 + 700);
    }

    #[test]
    fn the_horse_scenes() {
        for s in [SCENE_HYRULE_FIELD, SCENE_LAKE_HYLIA, SCENE_GERUDO_VALLEY, SCENE_GERUDOS_FORTRESS, SCENE_LON_LON_RANCH] {
            assert!(horse_can_spawn(s as i32));
        }
        assert!(!horse_can_spawn(SCENE_STABLE as i32));
        assert!(!horse_can_spawn(SCENE_LON_LON_BUILDINGS as i32));
    }
}
