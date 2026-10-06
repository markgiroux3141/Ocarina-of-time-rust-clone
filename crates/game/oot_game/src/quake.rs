//! Quakes (`z_quake.c`): the cameras' shake. An actor, a cutscene or Player asks for one on a
//! camera (`Quake_Request`), sets its speed, its perturbations and its duration, and every
//! `Camera_Update` that gets as far as the view (`Quake_Update`) runs each request's callback
//! once, which counts its timer down and works out its offsets; the requests on the camera
//! being updated move its view's eye and at, its fov and its up vector (`ShakeInfo`), and their
//! eye offset is the camera's `quakeOffset`.
//!
//! The request table (`sQuakeRequests`, `sQuakeRequestCount`) is a static in `code`, so the play
//! state keeps it (`QuakeStatics`) and it carries over scene changes, as the code segment's
//! statics do; `Play_Init` clears it (`Quake_Init`). A request points at its camera
//! (`QuakeRequest.cam`); here it keeps the camera's id, which is its slot in `cameraPtrs` (a
//! sub camera's struct keeps its `camId` after `Play_ClearCamera`, so the C's pointer and the id
//! name the same camera). The camera functions read a request's camera's eye and at
//! (`Quake_UpdateShakeInfo`), so `Camera_Update` is given every camera's (`CamFrame::cameras`).
//!
//! The rumble that most callers ask for with a quake (`Rumble_Request`, `Rumble_Override`) isn't
//! ported.

use glam::Vec3;

use crate::camera::{CAM_ID_NONE, NUM_CAMS, VecSphGeo, diff_to_sph_geo, sph_geo_to_vec3};
use crate::play::{PlayState, Rand};

// QuakeType (quake.h).
pub const QUAKE_TYPE_NONE: u32 = 0;
/// Periodic, sustaining, random X perturbations.
pub const QUAKE_TYPE_1: u32 = 1;
/// Aperiodic, sustaining, random X perturbations.
pub const QUAKE_TYPE_2: u32 = 2;
/// Periodic, decaying.
pub const QUAKE_TYPE_3: u32 = 3;
/// Aperiodic, decaying, random X perturbations.
pub const QUAKE_TYPE_4: u32 = 4;
/// Periodic, sustaining.
pub const QUAKE_TYPE_5: u32 = 5;
/// Jump-periodic (the period restarts every 16 frames), sustaining, random X perturbations; it
/// never ends by its timer and must be removed (`Quake_RemoveRequest`).
pub const QUAKE_TYPE_6: u32 = 6;

// Quake_SetValue's value types (z_quake.c).
pub const QUAKE_SPEED: i16 = 1 << 0;
pub const QUAKE_Y_OFFSET: i16 = 1 << 1;
pub const QUAKE_X_OFFSET: i16 = 1 << 2;
pub const QUAKE_FOV: i16 = 1 << 3;
pub const QUAKE_ROLL: i16 = 1 << 4;
pub const QUAKE_ORIENTATION_PITCH: i16 = 1 << 5;
pub const QUAKE_ORIENTATION_YAW: i16 = 1 << 6;
pub const QUAKE_ORIENTATION_ROLL: i16 = 1 << 7;
pub const QUAKE_DURATION: i16 = 1 << 8;
pub const QUAKE_IS_RELATIVE_TO_SCREEN: i16 = 1 << 9;

/// `ARRAY_COUNT(sQuakeRequests)`.
pub const NUM_QUAKE_REQUESTS: usize = 4;

/// `QuakeRequest` (`z_quake.c`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QuakeRequest {
    /// `index`: the slot in the low 2 bits, a random tag above (`Quake_RequestImpl`).
    pub index: i16,
    pub duration: i16,
    /// `cam` and `camId`: the camera's id (see the module's doc).
    pub cam_id: i16,
    /// `type` (`QUAKE_TYPE_*`).
    pub type_: u32,
    /// `y`, `x`: the up/down and left/right shake; `fov` (binang); `upPitchOffset`: the roll,
    /// as an offset to the up vector's pitch.
    pub y: i16,
    pub x: i16,
    pub fov: i16,
    pub up_pitch_offset: i16,
    /// `orientation`: turns the xy shake (only x, the pitch, and y, the yaw, are read).
    pub orientation: [i16; 3],
    /// `speed`: the periodic types' angular frequency (binang per frame).
    pub speed: i16,
    /// `isRelativeToScreen`: the shake along the screen's axes, or the world's.
    pub is_relative_to_screen: i16,
    pub timer: i16,
}

/// `ShakeInfo` (`quake.h`): a request's offsets, or the ones `Quake_Update` merges for a camera.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShakeInfo {
    pub at_offset: Vec3,
    pub eye_offset: Vec3,
    /// `upPitchOffset`, `upYawOffset`: the roll, by turning the up vector.
    pub up_pitch_offset: i16,
    pub up_yaw_offset: i16,
    /// `fovOffset` (binang).
    pub fov_offset: i16,
    /// `maxOffset` (nothing reads it).
    pub max_offset: f32,
}

/// `z_quake.c`'s statics: `sQuakeRequests`, `sQuakeUnused`, `sQuakeRequestCount`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuakeStatics {
    pub requests: [QuakeRequest; NUM_QUAKE_REQUESTS],
    pub unused: i16,
    pub request_count: i16,
}

impl Default for QuakeStatics {
    /// As the ROM starts them: the table in `.bss`, `sQuakeUnused = 1`, `sQuakeRequestCount = 0`.
    fn default() -> QuakeStatics {
        QuakeStatics { requests: [QuakeRequest::default(); NUM_QUAKE_REQUESTS], unused: 1, request_count: 0 }
    }
}

/// `Quake_AddVecGeoToVec3f`.
pub fn quake_add_vec_geo_to_vec3f(a: Vec3, geo: VecSphGeo) -> Vec3 {
    let b = sph_geo_to_vec3(geo);
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

/// `Quake_UpdateShakeInfo`: the request's offsets for the shake's `y` and `x` this frame, with
/// `eye` and `at` its camera's (`req->cam`).
pub fn quake_update_shake_info(req: &QuakeRequest, shake: &mut ShakeInfo, y: f32, x: f32, eye: Vec3, at: Vec3) {
    let mut offset;
    if req.is_relative_to_screen != 0 {
        offset = Vec3::ZERO;
        let eye_to_at_geo = diff_to_sph_geo(eye, at);
        // The y shake: the unit vector pointed up, then turned by the orientation.
        let geo = VecSphGeo { r: req.y as f32 * y, pitch: eye_to_at_geo.pitch.wrapping_add(req.orientation[0]).wrapping_add(0x4000), yaw: eye_to_at_geo.yaw.wrapping_add(req.orientation[1]) };
        offset = quake_add_vec_geo_to_vec3f(offset, geo);
        // The x shake: pointed left, then turned by the orientation.
        let geo = VecSphGeo { r: req.x as f32 * x, pitch: eye_to_at_geo.pitch.wrapping_add(req.orientation[0]), yaw: eye_to_at_geo.yaw.wrapping_add(req.orientation[1]).wrapping_add(0x4000) };
        offset = quake_add_vec_geo_to_vec3f(offset, geo);
    } else {
        offset = Vec3::new(0.0, req.y as f32 * y, 0.0);
        let geo = VecSphGeo { r: req.x as f32 * x, pitch: req.orientation[0], yaw: req.orientation[1] };
        offset = quake_add_vec_geo_to_vec3f(offset, geo);
    }
    shake.at_offset = offset;
    shake.eye_offset = offset;
    // (f32 to s16, through s32 as the N64's cvt.w.s and the store do.)
    shake.up_yaw_offset = (0x8000 as f32 * y) as i32 as i16;
    shake.up_pitch_offset = (req.up_pitch_offset as f32 * y) as i32 as i16;
    shake.fov_offset = (req.fov as f32 * y) as i32 as i16;
}

/// `Math_SinS(req->speed * arg)`: the product is an `int`, passed as an `s16`.
fn sin_speed(req: &QuakeRequest, arg: i32) -> f32 {
    eng_math::sin_s((req.speed as i32).wrapping_mul(arg) as i16)
}

/// `Quake_CallbackType1`.
pub fn quake_callback_type1(req: &mut QuakeRequest, shake: &mut ShakeInfo, eye: Vec3, at: Vec3, rand: &mut Rand) -> i16 {
    if req.timer > 0 {
        let xy_offset = sin_speed(req, req.timer as i32);
        quake_update_shake_info(req, shake, xy_offset, rand.zero_one() * xy_offset, eye, at);
        req.timer = req.timer.wrapping_sub(1);
    }
    req.timer
}

/// `Quake_CallbackType5`.
pub fn quake_callback_type5(req: &mut QuakeRequest, shake: &mut ShakeInfo, eye: Vec3, at: Vec3) -> i16 {
    if req.timer > 0 {
        let xy_offset = sin_speed(req, req.timer as i32);
        quake_update_shake_info(req, shake, xy_offset, xy_offset, eye, at);
        req.timer = req.timer.wrapping_sub(1);
    }
    req.timer
}

/// `Quake_CallbackType6`: returns 1 whatever the timer, so the quake lasts until it's removed.
pub fn quake_callback_type6(req: &mut QuakeRequest, shake: &mut ShakeInfo, eye: Vec3, at: Vec3, rand: &mut Rand) -> i16 {
    req.timer = req.timer.wrapping_sub(1);
    let xy_offset = sin_speed(req, (req.timer as i32 & 0xF) + 500);
    quake_update_shake_info(req, shake, xy_offset, rand.zero_one() * xy_offset, eye, at);
    1
}

/// `Quake_CallbackType3`.
pub fn quake_callback_type3(req: &mut QuakeRequest, shake: &mut ShakeInfo, eye: Vec3, at: Vec3) -> i16 {
    if req.timer > 0 {
        let xy_offset = sin_speed(req, req.timer as i32) * (req.timer as f32 / req.duration as f32);
        quake_update_shake_info(req, shake, xy_offset, xy_offset, eye, at);
        req.timer = req.timer.wrapping_sub(1);
    }
    req.timer
}

/// `Quake_CallbackType2`.
pub fn quake_callback_type2(req: &mut QuakeRequest, shake: &mut ShakeInfo, eye: Vec3, at: Vec3, rand: &mut Rand) -> i16 {
    if req.timer > 0 {
        let xy_offset = rand.zero_one();
        quake_update_shake_info(req, shake, xy_offset, rand.zero_one() * xy_offset, eye, at);
        req.timer = req.timer.wrapping_sub(1);
    }
    req.timer
}

/// `Quake_CallbackType4`.
pub fn quake_callback_type4(req: &mut QuakeRequest, shake: &mut ShakeInfo, eye: Vec3, at: Vec3, rand: &mut Rand) -> i16 {
    if req.timer > 0 {
        let xy_offset = rand.zero_one() * (req.timer as f32 / req.duration as f32);
        quake_update_shake_info(req, shake, xy_offset, rand.zero_one() * xy_offset, eye, at);
        req.timer = req.timer.wrapping_sub(1);
    }
    req.timer
}

/// `OLib_Vec3fDist(v, &zeroVec)`.
fn dist_to_zero(v: Vec3) -> f32 {
    let (dx, dy, dz) = (v.x - 0.0, v.y - 0.0, v.z - 0.0);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

impl QuakeStatics {
    /// `Quake_GetFreeIndex`: the first unused slot, else the one with the least time left (the
    /// first of equals), which the new request replaces.
    pub fn quake_get_free_index(&self) -> i16 {
        let mut index = 0;
        // UINT16_MAX + 1; 0x20000 marks a free slot found.
        let mut timer_min: i32 = 0x10000;
        for (i, r) in self.requests.iter().enumerate() {
            if r.type_ == QUAKE_TYPE_NONE {
                index = i;
                timer_min = 0x20000;
                break;
            }
            if timer_min > r.timer as i32 {
                timer_min = r.timer as i32;
                index = i;
            }
        }
        if timer_min != 0x20000 {
            log::warn!("quake: too many request {index} is changed new one !!");
        }
        index as i16
    }

    /// `Quake_RequestImpl`: a cleared request in a free slot (or the one replaced) on camera
    /// `cam_id`, relative to the screen, its index tagged with a random number. Returns the slot.
    ///
    /// @bug (game): `sQuakeRequestCount` counts a request that replaced another too, so with more
    /// than four at once it stays above the requests left (`Quake_Update` then never returns
    /// early). Nothing else reads it.
    pub fn quake_request_impl(&mut self, cam_id: i16, type_: u32, rand: &mut Rand) -> usize {
        let index = self.quake_get_free_index();
        let slot = index as usize;
        let req = &mut self.requests[slot];
        *req = QuakeRequest::default();
        req.cam_id = cam_id;
        req.type_ = type_;
        req.is_relative_to_screen = 1;
        // A unique random tag in the upper bits (the ~3 assumes four requests).
        req.index = index.wrapping_add(((rand.zero_one() * 0x10000 as f32) as i32 as i16) & !3);
        self.request_count = self.request_count.wrapping_add(1);
        slot
    }

    /// `Quake_Remove`.
    pub fn quake_remove(&mut self, slot: usize) {
        let req = &mut self.requests[slot];
        req.type_ = QUAKE_TYPE_NONE;
        req.timer = -1;
        self.request_count = self.request_count.wrapping_sub(1);
    }

    /// `Quake_GetRequest`: the slot of the request `index` names, if it's still that one.
    pub fn quake_get_request(&self, index: i16) -> Option<usize> {
        let slot = (index & 3) as usize;
        let req = &self.requests[slot];
        if req.type_ == QUAKE_TYPE_NONE {
            return None;
        }
        if index != req.index {
            return None;
        }
        Some(slot)
    }

    /// `Quake_SetValue` (nothing calls it). Returns the request's slot.
    ///
    /// @bug (game): the C has no return statement; it returns the request because that's left in
    /// `v0`, as here.
    pub fn quake_set_value(&mut self, index: i16, value_type: i16, value: i16) -> Option<usize> {
        let slot = self.quake_get_request(index)?;
        let req = &mut self.requests[slot];
        match value_type {
            QUAKE_SPEED => req.speed = value,
            QUAKE_Y_OFFSET => req.y = value,
            QUAKE_X_OFFSET => req.x = value,
            QUAKE_FOV => req.fov = value,
            QUAKE_ROLL => req.up_pitch_offset = value,
            QUAKE_ORIENTATION_PITCH => req.orientation[0] = value,
            QUAKE_ORIENTATION_YAW => req.orientation[1] = value,
            QUAKE_ORIENTATION_ROLL => req.orientation[2] = value,
            QUAKE_DURATION => {
                req.timer = value;
                req.duration = req.timer;
            }
            QUAKE_IS_RELATIVE_TO_SCREEN => req.is_relative_to_screen = value,
            _ => {}
        }
        Some(slot)
    }

    /// `Quake_SetSpeed`: false if the request is gone.
    pub fn quake_set_speed(&mut self, index: i16, speed: i16) -> bool {
        let Some(slot) = self.quake_get_request(index) else { return false };
        self.requests[slot].speed = speed;
        true
    }

    /// `Quake_SetDuration`: the timer and the duration (the decaying types' scale).
    pub fn quake_set_duration(&mut self, index: i16, duration: i16) -> bool {
        let Some(slot) = self.quake_get_request(index) else { return false };
        let req = &mut self.requests[slot];
        req.timer = duration;
        req.duration = duration;
        true
    }

    /// `Quake_GetTimeLeft`: 0 if the request is gone.
    pub fn quake_get_time_left(&self, index: i16) -> i16 {
        self.quake_get_request(index).map(|slot| self.requests[slot].timer).unwrap_or(0)
    }

    /// `Quake_SetPerturbations`: the up/down and left/right shake, the fov's and the roll's
    /// (the up vector's pitch).
    pub fn quake_set_perturbations(&mut self, index: i16, y: i16, x: i16, fov: i16, roll: i16) -> bool {
        let Some(slot) = self.quake_get_request(index) else { return false };
        let req = &mut self.requests[slot];
        req.y = y;
        req.x = x;
        req.fov = fov;
        req.up_pitch_offset = roll;
        true
    }

    /// `Quake_SetOrientation`.
    pub fn quake_set_orientation(&mut self, index: i16, is_relative_to_screen: i16, orientation: [i16; 3]) -> bool {
        let Some(slot) = self.quake_get_request(index) else { return false };
        let req = &mut self.requests[slot];
        req.is_relative_to_screen = is_relative_to_screen;
        req.orientation = orientation;
        true
    }

    /// `Quake_Init` (`Play_Init`): every slot unused, its timer 0 (the rest of each request is
    /// left as it was).
    pub fn quake_init(&mut self) {
        for r in &mut self.requests {
            r.type_ = QUAKE_TYPE_NONE;
            r.timer = 0;
        }
        self.unused = 1;
        self.request_count = 0;
    }

    /// `Quake_Request`: a request of `type_` on camera `cam_id`; returns its index.
    pub fn quake_request(&mut self, cam_id: i16, type_: u32, rand: &mut Rand) -> i16 {
        let slot = self.quake_request_impl(cam_id, type_, rand);
        self.requests[slot].index
    }

    /// `Quake_RemoveRequest`: false if the request is gone already.
    pub fn quake_remove_request(&mut self, index: i16) -> bool {
        let Some(slot) = self.quake_get_request(index) else { return false };
        self.quake_remove(slot);
        true
    }

    /// `Quake_Update` from camera `cam_id`'s `Camera_Update`: every request's callback runs (its
    /// timer counts down, whichever camera it's on), a request whose timer reached 0 or whose
    /// camera is gone is removed, and the offsets of the ones on this camera merge into
    /// `cam_shake` (the largest of each, by magnitude). `cams` is `play->cameraPtrs`: each
    /// camera's eye and at (`None`: NULL), this one's as its mode function left them. Returns the
    /// number of requests merged.
    ///
    /// @bug (game): a request of types 1 to 5 whose timer is negative (a negative duration) is
    /// never removed, and its callback doesn't write the shake, so it merges the shake the
    /// request before it left (the C's uninitialised local; zeros here for the first).
    pub fn quake_update(&mut self, cam_id: i16, cams: &[Option<(Vec3, Vec3)>; NUM_CAMS], rand: &mut Rand, cam_shake: &mut ShakeInfo) -> i16 {
        *cam_shake = ShakeInfo::default();
        if self.request_count == 0 {
            return 0;
        }
        let mut shake = ShakeInfo::default();
        let mut num_quakes_applied = 0;
        for index in 0..NUM_QUAKE_REQUESTS {
            if self.requests[index].type_ == QUAKE_TYPE_NONE {
                continue;
            }
            let req_cam = self.requests[index].cam_id;
            let Some((eye, at)) = usize::try_from(req_cam).ok().and_then(|i| cams.get(i).copied().flatten()) else {
                log::warn!("quake: stopped! 'coz camera [{req_cam}] killed!!");
                self.quake_remove(index);
                continue;
            };
            let is_different_cam_id = req_cam != cam_id;
            // ABS on the s16 promoted to int: -0x8000 is 0x8000.
            let abs_speed_div = (self.requests[index].speed as i32).abs() as f32 / 0x8000 as f32;
            let req = &mut self.requests[index];
            // sQuakeCallbacks[req->type].
            let left = match req.type_ {
                QUAKE_TYPE_1 => quake_callback_type1(req, &mut shake, eye, at, rand),
                QUAKE_TYPE_2 => quake_callback_type2(req, &mut shake, eye, at, rand),
                QUAKE_TYPE_3 => quake_callback_type3(req, &mut shake, eye, at),
                QUAKE_TYPE_4 => quake_callback_type4(req, &mut shake, eye, at, rand),
                QUAKE_TYPE_5 => quake_callback_type5(req, &mut shake, eye, at),
                QUAKE_TYPE_6 => quake_callback_type6(req, &mut shake, eye, at, rand),
                t => {
                    // Past the end of sQuakeCallbacks: the C jumps to whatever follows it.
                    log::error!("quake: no callback for type {t}");
                    0
                }
            };
            if left == 0 {
                // The quake's timer has run out.
                self.quake_remove(index);
                continue;
            }
            if is_different_cam_id {
                // The quake is on another camera.
                continue;
            }
            let pick = |cur: &mut f32, new: f32| {
                if cur.abs() < new.abs() {
                    *cur = new;
                }
            };
            pick(&mut cam_shake.at_offset.x, shake.at_offset.x);
            pick(&mut cam_shake.at_offset.y, shake.at_offset.y);
            pick(&mut cam_shake.at_offset.z, shake.at_offset.z);
            pick(&mut cam_shake.eye_offset.x, shake.eye_offset.x);
            pick(&mut cam_shake.eye_offset.y, shake.eye_offset.y);
            pick(&mut cam_shake.eye_offset.z, shake.eye_offset.z);
            if cam_shake.up_pitch_offset < shake.up_pitch_offset {
                cam_shake.up_pitch_offset = shake.up_pitch_offset;
                cam_shake.up_yaw_offset = shake.up_yaw_offset;
            }
            if cam_shake.fov_offset < shake.fov_offset {
                cam_shake.fov_offset = shake.fov_offset;
            }
            // CLAMP_MIN.
            let mut max_curr = dist_to_zero(shake.at_offset) * abs_speed_div;
            let mut max_next = dist_to_zero(shake.eye_offset) * abs_speed_div;
            max_curr = if max_curr < max_next { max_next } else { max_curr };
            max_next = cam_shake.up_pitch_offset as f32 * (1.0 / 200.0) * abs_speed_div;
            max_curr = if max_curr < max_next { max_next } else { max_curr };
            max_next = cam_shake.fov_offset as f32 * (1.0 / 200.0) * abs_speed_div;
            max_curr = if max_curr < max_next { max_next } else { max_curr };
            if cam_shake.max_offset < max_curr {
                cam_shake.max_offset = max_curr;
            }
            num_quakes_applied += 1;
        }
        num_quakes_applied
    }
}

impl PlayState {
    /// `Quake_Request(Play_GetCamera(play, cam_id), quake_type)`: a quake on camera `cam_id`
    /// (`CAM_ID_NONE`: the active one, `GET_ACTIVE_CAM`); returns its index, for the setters.
    /// A camera that doesn't exist (the C's NULL, which it would read through) keeps the id, and
    /// the next `Quake_Update` removes the request.
    pub fn quake_request(&mut self, cam_id: i16, quake_type: u32) -> i16 {
        let id = if cam_id == CAM_ID_NONE { self.active_cam_id } else { cam_id };
        if self.camera(id).is_none() {
            log::error!("quake: Quake_Request on camera {id}, which doesn't exist");
        }
        self.quake.quake_request(id, quake_type, &mut self.rand)
    }

    /// `Quake_SetSpeed`.
    pub fn quake_set_speed(&mut self, index: i16, speed: i16) -> bool {
        self.quake.quake_set_speed(index, speed)
    }

    /// `Quake_SetPerturbations`.
    pub fn quake_set_perturbations(&mut self, index: i16, y: i16, x: i16, fov: i16, roll: i16) -> bool {
        self.quake.quake_set_perturbations(index, y, x, fov, roll)
    }

    /// `Quake_SetDuration`.
    pub fn quake_set_duration(&mut self, index: i16, duration: i16) -> bool {
        self.quake.quake_set_duration(index, duration)
    }

    /// `Quake_SetOrientation`.
    pub fn quake_set_orientation(&mut self, index: i16, is_relative_to_screen: i16, orientation: [i16; 3]) -> bool {
        self.quake.quake_set_orientation(index, is_relative_to_screen, orientation)
    }

    /// `Quake_GetTimeLeft`.
    pub fn quake_get_time_left(&self, index: i16) -> i16 {
        self.quake.quake_get_time_left(index)
    }

    /// `Quake_RemoveRequest`.
    pub fn quake_remove_request(&mut self, index: i16) -> bool {
        self.quake.quake_remove_request(index)
    }

    /// `Actor_RequestQuake` (`z_actor.c`): a decaying quake (type 3) on the main camera at speed
    /// 20000.
    pub fn actor_request_quake(&mut self, y: i16, duration: i16) {
        self.actor_request_quake_with_speed(y, duration, 20000);
    }

    /// `Actor_RequestQuakeWithSpeed` (`z_actor.c`).
    pub fn actor_request_quake_with_speed(&mut self, y: i16, duration: i16, speed: i16) {
        let quake_index = self.quake_request(crate::camera::CAM_ID_MAIN, QUAKE_TYPE_3);
        self.quake_set_speed(quake_index, speed);
        self.quake_set_perturbations(quake_index, y, 0, 0, 0);
        self.quake_set_duration(quake_index, duration);
    }

    /// `Actor_RequestQuakeAndRumble` (`z_actor.c`): the rumble (`Rumble_Request`, 255 or 180 by
    /// `quake_y`) isn't ported.
    pub fn actor_request_quake_and_rumble(&mut self, quake_y: i16, quake_duration: i16) {
        log::debug!("Rumble_Request (quake y {quake_y}): not ported");
        self.actor_request_quake(quake_y, quake_duration);
    }

    /// `Camera_RequestQuake` (`z_camera.c`): a decaying quake (type 3) on camera `cam_id` at
    /// speed 0x61A8. Returns false when the request's index is 0 (a tag of 0 in slot 0), as the
    /// C does, without setting it up.
    pub fn camera_request_quake(&mut self, cam_id: i16, y: i16, duration: i16) -> bool {
        let quake_index = self.quake_request(cam_id, QUAKE_TYPE_3);
        if quake_index == 0 {
            return false;
        }
        self.quake_set_speed(quake_index, 0x61A8);
        self.quake_set_perturbations(quake_index, y, 0, 0, 0);
        self.quake_set_duration(quake_index, duration);
        true
    }
}

#[cfg(test)]
mod tests;
