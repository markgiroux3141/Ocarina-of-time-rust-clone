//! `En_Kanban` (`ovl_En_Kanban/z_en_kanban.c`): a square sign on a post. A sword slash cuts it:
//! the cut-off part flies off as a piece (another `En_Kanban`, `params` `ENKANBAN_PIECE`, the
//! sign's child) that bounces and settles, and a white mark flashes along the cut. Walking more
//! than 500 away puts the sign back whole and the pieces go.
//!
//! The sign offers to talk (`func_8002F2CC`, 68 units, facing it); its text is `params | 0x300`.
//! Once the box closes it waits 20 frames before offering again.
//!
//! Not ported: the ocarina repair (Zelda's Lullaby: no
//! ocarina), the hammer's quake (`actorCtx.unk_02` is never set), the shadow (a texture the
//! draw builds from the parts every frame), the dust and water effects, and the sounds.

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey, SegmentValues};
use eng_math::{approach_f, approach_s, approach_zero_f, binang_to_rad, cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_0, ACTOR_FLAG_3, ACTOR_FLAG_4, ACTOR_FLAG_25, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2, BGCHECKFLAG_GROUND, BGCHECKFLAG_WALL, BGCHECKFLAG_WATER_TOUCH};
use oot_game::actor_ctx::{ACTORCAT_EXPLOSIVE, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile};
use oot_game::camera::f_atan2f;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const ACTOR_EN_KANBAN: i16 = 0x0141;

/// `En_Kanban_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_KANBAN, name: "En_Kanban", category: ACTORCAT_PROP, flags: ACTOR_FLAG_0 | ACTOR_FLAG_3 | ACTOR_FLAG_4, object: "object_kanban" };

/// `ENKANBAN_PIECE`, `ENKANBAN_FISHING`.
pub const ENKANBAN_PIECE: i16 = 0xFFDD_u16 as i16;
const ENKANBAN_FISHING: i16 = 0x300;

// The sign's parts (`PART_*`).
const PART_UPPER_LEFT: u16 = 1 << 0;
const PART_LEFT_UPPER: u16 = 1 << 1;
const PART_LEFT_LOWER: u16 = 1 << 2;
const PART_RIGHT_UPPER: u16 = 1 << 3;
const PART_RIGHT_LOWER: u16 = 1 << 4;
const PART_LOWER_LEFT: u16 = 1 << 5;
const PART_UPPER_RIGHT: u16 = 1 << 6;
const PART_LOWER_RIGHT: u16 = 1 << 7;
const PART_POST_UPPER: u16 = 1 << 8;
const PART_POST_LOWER: u16 = 1 << 9;
const PART_POST_STAND: u16 = 1 << 10;
const LEFT_HALF: u16 = PART_UPPER_LEFT | PART_LEFT_UPPER | PART_LEFT_LOWER | PART_LOWER_LEFT;
const RIGHT_HALF: u16 = PART_UPPER_RIGHT | PART_RIGHT_UPPER | PART_RIGHT_LOWER | PART_LOWER_RIGHT;
const UPPER_HALF: u16 = PART_POST_UPPER | PART_UPPER_RIGHT | PART_RIGHT_UPPER | PART_UPPER_LEFT | PART_LEFT_UPPER;
const UPPERLEFT_HALF: u16 = PART_POST_UPPER | PART_UPPER_RIGHT | PART_LEFT_LOWER | PART_UPPER_LEFT | PART_LEFT_UPPER;
const UPPERRIGHT_HALF: u16 = PART_POST_UPPER | PART_UPPER_RIGHT | PART_RIGHT_UPPER | PART_UPPER_LEFT | PART_RIGHT_LOWER;
const ALL_PARTS: u16 = LEFT_HALF | RIGHT_HALF | PART_POST_UPPER | PART_POST_LOWER;

// `EnKanbanActionState`.
const ENKANBAN_SIGN: u8 = 0;
const ENKANBAN_AIR: u8 = 1;
const ENKANBAN_UNUSED: u8 = 2;
const ENKANBAN_GROUND: u8 = 3;
const ENKANBAN_WATER: u8 = 4;
const ENKANBAN_REPAIR: u8 = 5;

// `EnKanbanPiece`.
const PIECE_WHOLE_SIGN: u8 = 0;
const PIECE_OTHER: u8 = 100;

// `EnKanbanCutType`.
const CUT_POST: u8 = 0;
const CUT_VERT_L: u8 = 1;
const CUT_HORIZ: u8 = 2;
const CUT_DIAG_L: u8 = 3;
const CUT_DIAG_R: u8 = 4;
const CUT_VERT_R: u8 = 5;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COLTYPE_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderInfoInit {
        elem_type: ELEMTYPE_UNK0,
        toucher: ColliderTouch { dmg_flags: 0xFFCF_FFFF, effect: 0, damage: 0 },
        bumper: ColliderBumpInit { dmg_flags: 0xFFCF_FFFF, effect: 0, defense: 0 },
        toucher_flags: TOUCH_ON | TOUCH_SFX_NORMAL,
        bumper_flags: BUMP_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 50, y_shift: 5, pos: [0; 3] },
};

/// `sPartFlags`, and `sDisplayLists` (each part's list in `object_kanban`).
const PART_FLAGS: [u16; 11] =
    [PART_UPPER_LEFT, PART_LEFT_UPPER, PART_LEFT_LOWER, PART_RIGHT_UPPER, PART_RIGHT_LOWER, PART_LOWER_LEFT, PART_UPPER_RIGHT, PART_LOWER_RIGHT, PART_POST_UPPER, PART_POST_LOWER, PART_POST_STAND];
const PART_DLISTS: [&str; 11] = [
    "object_kanban_DL_000CB0",
    "object_kanban_DL_000DB8",
    "object_kanban_DL_000E78",
    "object_kanban_DL_000F38",
    "object_kanban_DL_000FF8",
    "object_kanban_DL_0010B8",
    "object_kanban_DL_0011C0",
    "object_kanban_DL_0012C8",
    "object_kanban_DL_0013D0",
    "object_kanban_DL_001488",
    "object_kanban_DL_001540",
];

/// `sPieceOffsets` (by `EnKanbanPiece`).
const PIECE_OFFSETS: [Vec3; 19] = [
    Vec3::new(0.0, 44.0, 0.0),
    Vec3::new(0.0, 50.0, 0.0),
    Vec3::new(0.0, 38.0, 0.0),
    Vec3::new(10.0, 44.0, 0.0),
    Vec3::new(-10.0, 44.0, 0.0),
    Vec3::new(-10.0, 50.0, 0.0),
    Vec3::new(10.0, 50.0, 0.0),
    Vec3::new(-10.0, 38.0, 0.0),
    Vec3::new(10.0, 38.0, 0.0),
    Vec3::new(-7.5, 51.0, 0.0),
    Vec3::new(-12.5, 48.0, 0.0),
    Vec3::new(-12.5, 40.0, 0.0),
    Vec3::new(-7.5, 37.0, 0.0),
    Vec3::new(7.5, 51.0, 0.0),
    Vec3::new(12.5, 48.0, 0.0),
    Vec3::new(12.5, 40.0, 0.0),
    Vec3::new(7.5, 37.0, 0.0),
    Vec3::new(0.0, 50.0, 0.0),
    Vec3::new(0.0, 38.0, 0.0),
];

/// `sPieceSizes` (width, height).
const PIECE_SIZES: [(f32, f32); 19] = [
    (1500.0, 1000.0),
    (1500.0, 500.0),
    (1500.0, 500.0),
    (700.0, 1000.0),
    (700.0, 1000.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (700.0, 500.0),
    (200.0, 500.0),
    (200.0, 500.0),
];

/// `sCutTypes[PLAYER_MWA_*]`.
const CUT_TYPES: [u8; 28] = [
    CUT_VERT_L, CUT_VERT_L, CUT_DIAG_R, CUT_DIAG_R, CUT_HORIZ, CUT_HORIZ, CUT_HORIZ, CUT_HORIZ, CUT_HORIZ, CUT_HORIZ, CUT_HORIZ, CUT_HORIZ, CUT_POST, CUT_POST, CUT_POST, CUT_POST, CUT_VERT_L, CUT_VERT_L, CUT_VERT_L,
    CUT_VERT_L, CUT_HORIZ, CUT_HORIZ, CUT_POST, CUT_POST, CUT_POST, CUT_POST, CUT_POST, CUT_POST,
];

/// `sCutFlags[EnKanbanCutType]`.
const CUT_FLAGS: [u16; 6] = [ALL_PARTS, LEFT_HALF, UPPER_HALF, UPPERLEFT_HALF, UPPERRIGHT_HALF, RIGHT_HALF];

/// `sCutAngles` (radians).
const CUT_ANGLES: [f32; 6] = [0.50 * std::f32::consts::PI, 0.0, 0.50 * std::f32::consts::PI, 0.66 * std::f32::consts::PI, 0.34 * std::f32::consts::PI, 0.0];

/// The segment of the cut mark's colours (the draw's `gDPSetPrimColor` / `gDPSetEnvColor`).
const CUT_MARK_COLOR_SEGMENT: u8 = 0x0E;

/// The meshes the draw needs (docs/adr/0012-actor-bakes.md): `EnKanban_Draw` runs
/// `object_kanban_DL_000C30` (the material) before the whole sign or its parts, and the cut
/// mark with its colours set by the draw.
pub fn bakes() -> Vec<MeshBake> {
    let dl = |f: &str, s: &str| (f.to_string(), s.to_string());
    let mut v = vec![MeshBake {
        name: "En_Kanban/sign".into(),
        object: "object_kanban".into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![dl("object_kanban", "object_kanban_DL_000C30"), dl("gameplay_keep", "gSignRectangularDL")]),
    }];
    for (i, part) in PART_DLISTS.iter().enumerate() {
        v.push(MeshBake {
            name: format!("En_Kanban/part{i}"),
            object: "object_kanban".into(),
            segments: Vec::new(),
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![dl("object_kanban", "object_kanban_DL_000C30"), dl("object_kanban", part)]),
        });
    }
    v.push(MeshBake {
        name: "En_Kanban/cut_mark".into(),
        object: "object_kanban".into(),
        segments: vec![(CUT_MARK_COLOR_SEGMENT, BakeSegment::DynamicColor { env: true, prim: true })],
        prelude: vec![CUT_MARK_COLOR_SEGMENT],
        body: BakeBody::DLists(vec![dl("object_kanban", "object_kanban_DL_001630")]),
    });
    v
}

pub struct EnKanban {
    pub actor: Actor,
    pub frame_count: u8,
    pub air_timer: i16,
    pub action_state: u8,
    pub part_flags: u16,
    pub part_count: u8,
    pub invincibility_timer: i16,
    pub offset: Vec3,
    pub spin_rot: [i16; 3],
    pub spin_vel: [i16; 3],
    pub spin_x_flag: bool,
    pub spin_z_flag: bool,
    pub bounce_x: i16,
    pub bounce_z: i16,
    pub bounce_count: u8,
    pub piece_width: f32,
    pub piece_height: f32,
    pub direction: i16,
    /// Radians.
    pub floor_rot: Vec3,
    pub cut_type: u8,
    pub piece_type: u8,
    pub cut_mark_timer: i16,
    pub cut_mark_alpha: i16,
    pub z_target_timer: i16,
    pub msg_flag: bool,
    pub msg_timer: u8,
    pub collider: ColliderCylinder,
}

/// Indices into the render state's extras.
mod rs {
    pub const SPIN_X: usize = 0;
    pub const SPIN_Z: usize = 1;
    pub const FLOOR_X: usize = 0;
    pub const FLOOR_Z: usize = 1;
    pub const CUT_MARK_ALPHA: usize = 2;
    pub const PART_FLAGS: usize = 0;
    pub const ACTION_STATE: usize = 1;
    pub const CUT_TYPE: usize = 2;
}

impl EnKanban {
    fn new(actor: Actor) -> EnKanban {
        EnKanban {
            actor,
            frame_count: 0,
            air_timer: 0,
            action_state: ENKANBAN_SIGN,
            part_flags: 0,
            part_count: 0,
            invincibility_timer: 0,
            offset: Vec3::ZERO,
            spin_rot: [0; 3],
            spin_vel: [0; 3],
            spin_x_flag: false,
            spin_z_flag: false,
            bounce_x: 0,
            bounce_z: 0,
            bounce_count: 0,
            piece_width: 0.0,
            piece_height: 0.0,
            direction: 0,
            floor_rot: Vec3::ZERO,
            cut_type: 0,
            piece_type: 0,
            cut_mark_timer: 0,
            cut_mark_alpha: 0,
            z_target_timer: 0,
            msg_flag: false,
            msg_timer: 0,
            collider: ColliderCylinder::default(),
        }
    }

    /// `EnKanban_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.scale = Vec3::splat(0.01);
        let mut k = EnKanban::new(actor);
        if k.actor.params != ENKANBAN_PIECE {
            k.actor.target_mode = 0;
            k.actor.flags |= ACTOR_FLAG_0;
            k.collider = ColliderCylinder::new(&CYLINDER_INIT);
            k.actor.text_id = if k.actor.params == ENKANBAN_FISHING {
                if play.save.adult { 0x4090 } else { 0x409D }
            } else {
                (k.actor.params as u16) | 0x300
            };
            k.bounce_x = 1;
            k.part_flags = 0xFFFF;
            k.actor.update_bg_check_info(&play.col, 10.0, 10.0, 50.0, UPDBGCHECKINFO_FLAG_2);
            k.set_floor_rot(play);
            if !play.save.adult {
                k.actor.world_pos.y -= 15.0;
            }
        }
        Box::new(k)
    }

    /// `EnKanban_SetFloorRot`.
    fn set_floor_rot(&mut self, play: &PlayState) {
        if let Some(fp) = self.actor.floor_poly {
            let n = play.col.poly_normal(fp);
            self.floor_rot.x = -f_atan2f(-n.z * n.y, 1.0);
            self.floor_rot.z = f_atan2f(-n.x * n.y, 1.0);
        }
    }

    /// `EnKanban_Message`: offer to talk when facing it; after the talk, 20 frames' pause.
    fn message(&mut self, play: &mut PlayState) {
        if !self.msg_flag {
            if self.msg_timer == 0 {
                let d = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
                if (d as i32).abs() < 0x2800 {
                    if oot_game::npc::process_talk_request(&mut self.actor) {
                        self.msg_flag = true;
                    } else {
                        oot_game::npc::offer_talk(play, &self.actor, 68.0);
                    }
                }
            } else {
                self.msg_timer -= 1;
            }
        } else if oot_game::npc::textbox_is_closing(play) {
            self.msg_flag = false;
            self.msg_timer = 20;
        }
    }

    /// The sign's part of `EnKanban_Update` (`ENKANBAN_SIGN`).
    fn update_sign(&mut self, play: &mut PlayState) {
        if self.invincibility_timer != 0 {
            self.invincibility_timer -= 1;
        }
        if self.z_target_timer != 0 {
            self.z_target_timer -= 1;
        }
        if self.z_target_timer == 1 {
            self.actor.flags &= !ACTOR_FLAG_0;
        }
        if self.part_flags == 0xFFFF {
            self.message(play);
        }
        if self.invincibility_timer == 0 && self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            self.invincibility_timer = 6;
            if self.cut(play) {
                return;
            }
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 44.0;
        self.collider.update(&self.actor);
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        if self.actor.xz_dist_to_player > 500.0 {
            self.actor.flags |= ACTOR_FLAG_0;
            self.part_flags = 0xFFFF;
        }
        if self.cut_mark_timer != 0 {
            if self.cut_mark_timer >= 5 {
                self.cut_mark_alpha = (self.cut_mark_alpha + 255).min(255);
            } else {
                self.cut_mark_alpha = (self.cut_mark_alpha - 65).max(0);
            }
            self.cut_mark_timer -= 1;
        }
    }

    /// The hit: a piece for the part the cut takes off. Returns true when the update ends here
    /// (the cut took nothing: the new piece is killed).
    fn cut(&mut self, play: &mut PlayState) -> bool {
        let (r, pos) = (self.actor.shape_rot, self.actor.world_pos);
        let Ok(ph) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_KANBAN, pos, [r.x, r.y, r.z], ENKANBAN_PIECE) else { return false };
        let hit_slash = self.collider.info.ac_hit_info.is_some_and(|h| h.toucher.dmg_flags & DMG_SLASH != 0);
        let mwa = play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).map(|p| p.melee_weapon_animation).unwrap_or(0);
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        self.cut_type = if hit_slash { CUT_TYPES.get(mwa).copied().unwrap_or(CUT_POST) } else { CUT_POST };
        if (yaw_diff as i32).abs() > 0x4000 {
            if self.cut_type == CUT_DIAG_R {
                self.cut_type = CUT_DIAG_L;
            } else if self.cut_type == CUT_VERT_L {
                self.cut_type = CUT_VERT_R;
            }
        }
        let cut = CUT_FLAGS[self.cut_type as usize];
        let piece_flags = cut & self.part_flags;
        let Some(piece) = play.actors.downcast_mut::<EnKanban>(ph) else { return false };
        piece.part_flags = piece_flags;
        if piece.part_flags == 0 {
            piece.actor.kill();
            return true;
        }
        piece.part_count = PART_FLAGS.iter().filter(|&&f| f & piece.part_flags != 0).count() as u8;
        let pf = piece.part_flags;
        self.part_flags &= !cut;
        if self.part_flags & ALL_PARTS == 0 {
            self.z_target_timer = 10;
        }
        let has = |f: u16| pf & f != 0;
        let mut ty = if has(PART_UPPER_LEFT) && has(PART_LOWER_RIGHT) {
            PIECE_WHOLE_SIGN
        } else if has(PART_LEFT_UPPER) && has(PART_RIGHT_UPPER) {
            1
        } else if has(PART_LEFT_LOWER) && has(PART_RIGHT_LOWER) {
            2
        } else if has(PART_UPPER_RIGHT) && has(PART_LOWER_RIGHT) {
            3
        } else if has(PART_UPPER_LEFT) && has(PART_LOWER_LEFT) {
            4
        } else if has(PART_UPPER_LEFT) && has(PART_LEFT_UPPER) {
            5
        } else if has(PART_UPPER_RIGHT) && has(PART_RIGHT_UPPER) {
            6
        } else if has(PART_LEFT_LOWER) && has(PART_LOWER_LEFT) {
            7
        } else if has(PART_RIGHT_LOWER) && has(PART_LOWER_RIGHT) {
            8
        } else if has(PART_UPPER_LEFT) {
            9
        } else if has(PART_LEFT_UPPER) {
            10
        } else if has(PART_LEFT_LOWER) {
            11
        } else if has(PART_LOWER_LEFT) {
            12
        } else if has(PART_UPPER_RIGHT) {
            13
        } else if has(PART_RIGHT_UPPER) {
            14
        } else if has(PART_RIGHT_LOWER) {
            15
        } else if has(PART_LOWER_RIGHT) {
            16
        } else if has(PART_POST_UPPER) {
            17
        } else if has(PART_POST_LOWER) {
            18
        } else {
            PIECE_OTHER
        };
        if ty == PIECE_OTHER {
            ty = PIECE_WHOLE_SIGN;
        }
        piece.piece_type = ty;
        // Matrix_RotateY(shape.rot.y), Matrix_MultVec3f(sPieceOffsets[type]).
        let o = PIECE_OFFSETS[ty as usize];
        let offset = Mat4::from_rotation_y(binang_to_rad(self.actor.shape_rot.y)).transform_point3(o);
        piece.actor.world_pos += offset;
        piece.offset = -o / self.actor.scale.x;
        (piece.piece_width, piece.piece_height) = PIECE_SIZES[ty as usize];
        piece.actor.gravity = -1.0;
        piece.action_state = ENKANBAN_AIR;
        let yaw_towards = self.actor.yaw_towards_player;
        let many = piece.part_count >= 4;
        let _ = piece;
        let rand = &mut play.rand;
        let world_rot_y = (rand.centered_float(0x3000 as f32) as i16).wrapping_add(yaw_towards).wrapping_add(0x8000_u16 as i16);
        let vel_y = rand.zero_float(2.0) + 3.0;
        let speed = rand.zero_float(2.0) + 3.0;
        let (bx, bz) = if many { ((rand.zero_float(10.0) as i16) + 6, (rand.zero_float(10.0) as i16) + 6) } else { ((rand.zero_float(7.0) as i16) + 3, (rand.zero_float(7.0) as i16) + 3) };
        let spin_y = rand.centered_float(0x1800 as f32) as i16;
        let direction = if rand.zero_one() < 0.5 { 1 } else { -1 };
        let Some(piece) = play.actors.downcast_mut::<EnKanban>(ph) else { return false };
        piece.actor.world_rot.y = world_rot_y;
        piece.actor.velocity.y = vel_y;
        piece.actor.speed_xz = speed;
        piece.bounce_x = bx;
        piece.bounce_z = bz;
        piece.spin_vel[1] = spin_y;
        piece.direction = direction;
        piece.air_timer = 100;
        piece.actor.flags &= !ACTOR_FLAG_0;
        piece.actor.flags |= ACTOR_FLAG_25;
        self.cut_mark_timer = 5;
        false
    }

    /// A piece in the air (`ENKANBAN_AIR`): flying, spinning, bouncing.
    fn update_air(&mut self, play: &mut PlayState) {
        let mut bounced = false;
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 30.0, 30.0, 50.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        // The floor rotation from a point pushed back by the height (the shadow's).
        let (tx, ty, tz, flags, ydw) = (self.actor.world_pos.x, self.actor.world_pos.y, self.actor.world_pos.z, self.actor.bg_check_flags, self.actor.y_dist_to_water);
        self.actor.world_pos.z += ((self.actor.world_pos.y - self.actor.floor_height) * -50.0) / 100.0;
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 50.0, UPDBGCHECKINFO_FLAG_2);
        self.set_floor_rot(play);
        self.actor.world_pos = Vec3::new(tx, ty, tz);
        self.actor.bg_check_flags = flags;
        self.actor.y_dist_to_water = ydw;
        let on_ground = self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0;
        for (k, flag) in [(0usize, self.spin_x_flag), (2usize, self.spin_z_flag)] {
            if flag {
                self.spin_rot[k] = self.spin_rot[k].wrapping_add(self.spin_vel[k]);
                self.spin_vel[k] = self.spin_vel[k].wrapping_sub(0x800);
                if self.spin_rot[k] <= 0 && on_ground {
                    self.spin_rot[k] = 0;
                    self.spin_vel[k] = 0;
                }
            } else {
                self.spin_rot[k] = self.spin_rot[k].wrapping_sub(self.spin_vel[k]);
                self.spin_vel[k] = self.spin_vel[k].wrapping_sub(0x800);
                if self.spin_rot[k] >= 0 && on_ground {
                    self.spin_rot[k] = 0;
                    self.spin_vel[k] = 0;
                }
            }
            if self.spin_vel[k] < -0xC00 {
                self.spin_vel[k] = -0xC00;
            }
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.actor.speed_xz *= -0.5;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
            self.action_state = ENKANBAN_WATER;
            self.bounce_x = 0;
            self.bounce_z = 0;
            self.actor.world_pos.y += self.actor.y_dist_to_water;
            // The splash and ripples: effects, not ported.
            self.actor.velocity.y = 0.0;
            self.actor.gravity = 0.0;
            return;
        }
        if on_ground {
            if self.bounce_count == 0 {
                self.bounce_count += 1;
                self.actor.velocity.y *= -0.3;
                let r = play.rand.centered_float(16384.0) as i16;
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(r);
            } else {
                self.actor.velocity.y = 0.0;
            }
            self.actor.speed_xz *= 0.7;
            if self.spin_rot[0] == 0 && self.bounce_x != 0 {
                self.spin_vel[0] = self.bounce_x.wrapping_mul(0x200);
                self.bounce_x = (self.bounce_x - 5).max(0);
                self.spin_x_flag = play.rand.zero_one() < 0.5;
                bounced = true;
            }
            if self.spin_rot[2] == 0 && self.bounce_z != 0 {
                self.spin_vel[2] = self.bounce_z.wrapping_mul(0x200);
                self.bounce_z = (self.bounce_z - 5).max(0);
                self.spin_z_flag = play.rand.zero_one() < 0.5;
                bounced = true;
            }
            approach_s(&mut self.actor.shape_rot.x, self.direction.wrapping_mul(0x4000), 1, 0x2000);
        } else {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.spin_vel[1]);
            self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(self.direction.wrapping_mul(0x7D0));
        }
        if bounced {
            // The sound and the dust (func_800286CC): not ported. The dust's random positions
            // are drawn from the game's sequence all the same.
            let n = (self.part_count as f32 * 0.5) as i16 + 3;
            for _ in 0..n {
                play.rand.centered_float((self.part_count as f32 * 0.5) + 20.0);
                play.rand.centered_float((self.part_count as f32 * 0.5) + 20.0);
            }
        }
        // DECR(airTimer).
        if self.air_timer != 0 {
            self.air_timer -= 1;
            if self.air_timer == 0 {
                self.action_state = ENKANBAN_GROUND;
            }
        } else {
            self.action_state = ENKANBAN_GROUND;
        }
    }

    /// A piece on the ground or in water (`ENKANBAN_GROUND`, `ENKANBAN_WATER`, and the end of
    /// `ENKANBAN_AIR`).
    fn update_rest(&mut self, play: &mut PlayState) {
        let signpost_flags = self.actor.parent.and_then(|h| play.actors.downcast::<EnKanban>(h)).map(|s| s.part_flags);
        if signpost_flags.is_none_or(|f| f == 0xFFFF) {
            self.actor.kill();
        }
        approach_f(&mut self.actor.shape_y_offset, 100.0, 1.0, 5.0);
        if self.action_state == ENKANBAN_WATER {
            self.update_water(play);
        }
        // play->actorCtx.unk_02 (a hammer's quake nearby): never set.
        if self.bounce_x == 0 {
            // A bomb (params 1) within 100 throws the piece up again.
            let bombs: Vec<ActorHandle> = play.actors.category(ACTORCAT_EXPLOSIVE).to_vec();
            for b in bombs {
                let Some(bomb) = play.actors.actor(b) else { continue };
                if bomb.params != 1 {
                    continue;
                }
                let d = self.actor.world_pos - bomb.world_pos;
                let dist = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
                if dist < 100.0 {
                    let strength = (100.0 - dist) * 0.05;
                    self.action_state = ENKANBAN_AIR;
                    self.actor.gravity = -1.0;
                    self.actor.world_rot.y = eng_math::rad_to_binang(f_atan2f(d.x, d.z));
                    let rand = &mut play.rand;
                    if self.part_count >= 4 {
                        self.bounce_x = (rand.zero_float(10.0) as i16) + 6;
                        self.bounce_z = (rand.zero_float(10.0) as i16) + 6;
                        self.actor.velocity.y = 2.5 + strength;
                        self.actor.speed_xz = 3.0 + strength;
                    } else {
                        self.bounce_x = (rand.zero_float(7.0) as i16) + 3;
                        self.bounce_z = (rand.zero_float(7.0) as i16) + 3;
                        self.actor.velocity.y = 5.0 + strength;
                        self.actor.speed_xz = 4.0 + strength;
                    }
                    self.spin_vel[1] = rand.centered_float(0x1800 as f32) as i16;
                    self.direction = if rand.zero_one() < 0.5 { 1 } else { -1 };
                    self.air_timer = 70;
                }
            }
        }
        // The ocarina (ocarinaFlag, Zelda's Lullaby → ENKANBAN_REPAIR): no ocarina yet.
    }

    /// `ENKANBAN_WATER`: floating, pushed along by Player wading past.
    fn update_water(&mut self, play: &mut PlayState) {
        let (pspeed, py) = play.player.and_then(|h| play.actors.actor(h)).map(|p| (p.speed_xz, p.world_pos.y)).unwrap_or((0.0, 0.0));
        if pspeed > 0.0 && py < self.actor.world_pos.y && self.actor.xyz_dist_to_player_sq < 50.0 * 50.0 {
            approach_f(&mut self.actor.speed_xz, pspeed, 1.0, 0.2);
            if self.actor.speed_xz > 1.0 {
                self.actor.speed_xz = 1.0;
            }
            let target = self.actor.yaw_towards_player.wrapping_add(0x8000_u16 as i16);
            if eng_math::smooth_step_to_s(&mut self.actor.world_rot.y, target, 1, 0x1000, 0) > 0 {
                self.spin_vel[1] = (self.actor.speed_xz * 1000.0) as i16;
            } else {
                self.spin_vel[1] = (self.actor.speed_xz * -1000.0) as i16;
            }
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.speed_xz = 0.0;
        }
        self.actor.move_forward();
        if self.actor.speed_xz != 0.0 {
            self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 50.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                self.actor.speed_xz *= -0.5;
                self.spin_vel[1] = if self.spin_vel[1] > 0 { -0x7D0 } else { 0x7D0 };
            }
            approach_zero_f(&mut self.actor.speed_xz, 1.0, 0.15);
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.spin_vel[1]);
        approach_s(&mut self.spin_vel[1], 0, 1, 0x3A);
        approach_s(&mut self.actor.shape_rot.x, self.direction.wrapping_mul(0x4000), 2, 0x1000);
        let fc = self.frame_count as i16;
        approach_s(&mut self.spin_rot[0], (sin_s(2500i16.wrapping_mul(fc)) * 500.0) as i16, 2, 0x1000);
        approach_s(&mut self.spin_rot[2], (cos_s(3000i16.wrapping_mul(fc)) * 500.0) as i16, 2, 0x1000);
        approach_zero_f(&mut self.floor_rot.x, 0.5, 0.2);
        approach_zero_f(&mut self.floor_rot.z, 0.5, 0.2);
        // The ripples: effects, not ported.
    }

    /// The piece's model matrix (`EnKanban_Draw`, `actionState != ENKANBAN_SIGN`).
    fn piece_matrix(&self, rs: &RenderState) -> Mat4 {
        let r = binang_to_rad;
        let spin = [rs.angles[rs::SPIN_X], rs.angles[rs::SPIN_Z]];
        let z1 = (sin_s(spin[0]) * self.piece_height).abs();
        let z2 = (sin_s(spin[1]) * self.piece_width).abs();
        let z_shift = z2.max(z1) * -(self.direction as f32);
        Mat4::from_translation(rs.pos)
            * Mat4::from_scale(rs.scale)
            * Mat4::from_rotation_x(rs.values[rs::FLOOR_X])
            * Mat4::from_rotation_z(rs.values[rs::FLOOR_Z])
            * Mat4::from_translation(Vec3::new(0.0, rs.y_offset, 0.0))
            * Mat4::from_rotation_y(r(rs.rot[1]))
            * Mat4::from_rotation_x(r(rs.rot[0]))
            * Mat4::from_translation(Vec3::new(0.0, 0.0, z_shift))
            * Mat4::from_rotation_x(r(spin[0]))
            * Mat4::from_rotation_y(r(spin[1]))
            * Mat4::from_translation(Vec3::new(self.offset.x, self.offset.y, self.offset.z - 100.0))
    }
}

impl ActorImpl for EnKanban {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnKanban_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.frame_count = self.frame_count.wrapping_add(1);
        match self.action_state {
            ENKANBAN_SIGN => self.update_sign(play),
            ENKANBAN_AIR | ENKANBAN_UNUSED => {
                let before = self.action_state;
                self.update_air(play);
                // The C breaks out of the switch on landing in water; otherwise it falls
                // through into the ground and water case.
                if !(self.action_state == ENKANBAN_WATER && before != ENKANBAN_WATER) {
                    self.update_rest(play);
                }
            }
            ENKANBAN_GROUND | ENKANBAN_WATER => self.update_rest(play),
            ENKANBAN_REPAIR => {}
            _ => {}
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.angles = vec![self.spin_rot[0], self.spin_rot[2]];
        rs.values = vec![self.floor_rot.x, self.floor_rot.z, self.cut_mark_alpha as f32];
        rs.switches = vec![self.part_flags as u32, self.action_state as u32, self.cut_type as u32];
        rs
    }
    /// `EnKanban_Draw` (without the shadow).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < 3 || rs.values.len() < 3 || rs.angles.len() < 2 {
            // A render state from before the init (`Uninit`).
            return;
        }
        let part_flags = rs.switches[rs::PART_FLAGS] as u16;
        let bake = |n: &str| MeshKey::named(keys::bake(n));
        let parts = |m: Mat4, out: &mut DrawOut| {
            for (i, f) in PART_FLAGS.iter().enumerate() {
                if f & part_flags != 0 {
                    out.opa.push(DrawCmd::new(bake(&format!("En_Kanban/part{i}")), m));
                }
            }
        };
        if rs.switches[rs::ACTION_STATE] as u8 != ENKANBAN_SIGN {
            parts(self.piece_matrix(rs), out);
            return;
        }
        let m = oot_game::play::actor_draw_matrix(rs) * Mat4::from_translation(Vec3::new(0.0, 0.0, -100.0));
        if part_flags == 0xFFFF {
            out.opa.push(DrawCmd::new(bake("En_Kanban/sign"), m));
        } else {
            parts(m, out);
        }
        let alpha = rs.values[rs::CUT_MARK_ALPHA] as i16;
        if alpha != 0 {
            let cut_type = rs.switches[rs::CUT_TYPE] as usize;
            let cut_offset = if cut_type as u8 == CUT_POST { -1200.0 } else { 0.0 };
            let mm = m
                * Mat4::from_translation(Vec3::new(0.0, 4400.0 + cut_offset, 200.0))
                * Mat4::from_rotation_z(CUT_ANGLES[cut_type])
                * Mat4::from_scale(Vec3::new(0.0, 10.0, 2.0));
            let mut sv = SegmentValues::default();
            sv.prim[CUT_MARK_COLOR_SEGMENT as usize] = Some([255, 255, 255, alpha.clamp(0, 255) as u8]);
            sv.env[CUT_MARK_COLOR_SEGMENT as usize] = Some([255, 255, 150, 0]);
            let mut cmd = DrawCmd::new(bake("En_Kanban/cut_mark"), mm);
            cmd.params.segments = Some(sv);
            out.xlu.push(cmd);
        }
    }
    /// `EnKanban_Destroy`: the sign's collider.
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0 && self.action_state == ENKANBAN_SIGN && self.actor.params != ENKANBAN_PIECE).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
