//! `Obj_Syokudai` (`ovl_Obj_Syokudai/z_obj_syokudai.c`): the torches, with their flame, its point
//! light and the crackle (`NA_SE_EV_TORCH`).
//!
//! Params: the low 6 bits a switch flag, bits 6 to 9 a count (`torchCount`; 10 means 24), bit 10
//! always lit, and the high 4 bits the type: 0 golden (`gGoldenTorchDL`), 1 timed
//! (`gTimedTorchDL`), 2 wooden (`gWoodenTorchDL`). The Master Quest Deku Tree's:
//! - **golden**, lit by their switch flag (room 0's three 0x03E7 on 0x27; rooms 3, 5 and 7): the
//!   flag set lights one (`litTimer` -1, for good) with an attention cutscene
//!   (`OnePointCutscene_Attention`); the flag cleared again, it burns out over 20 frames;
//! - **timed**, lit by fire (Din's Fire, a fire arrow: `DMG_FIRE` on the flame's collider) or a
//!   burning Deku Stick, for `50 * count + 110` frames, counted in `sLitTorchCount`; the
//!   `count`th lit at once sets the flag (room 4's pair 0x1099 on 0x19, room 5's on 0x09, room
//!   10's 0x1053 on 0x13) with the attention cutscene, and all stay lit; else each burns out,
//!   its light and flame shrinking over its last 20 frames;
//! - **wooden** (room 10's 0x2400), always lit.
//!
//! A lit torch lights a Deku Stick held to its flame (Player's `unk_860` 210, or back up to 200
//! while it burns), and a normal arrow shot through its flame catches fire. Under water deeper
//! than 52 a torch goes out. A Keese flies to the nearest lit torch to catch fire
//! (`EnFirefly_ApproachLitTorch`).
//!
//! `torchType` is the params' high bits unshifted (`PARAMS_GET_NOSHIFT`: 0, 0x1000, 0x2000), so
//! its compares with 1 and 2 never hold (`@bug (game)`, kept): a wooden torch lights from its
//! switch flag too, lighting one sets its flag with the attention cutscene, and a timed torch put
//! out by water keeps its flag.
//!
//! The whole overlay is ported. Not reached yet: the Deku Stick (Link holds none until GAME-05
//! milestone 4b) and arrows (`En_Arrow` isn't ported: the id check stays, and the arrow's
//! collider is written through a marked hook). The light's glow sprite (`Lights_GlowCheck`) and
//! the cull zone (`cullingVolume*`) aren't ported for any actor.

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::binang_to_rad;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_EV_FLAME_IGNITION, NA_SE_EV_TORCH, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::lights::{LightInfo, LightNode};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::player::Player;

/// `ACTOR_OBJ_SYOKUDAI` (`actor_table.h`).
pub const ACTOR_OBJ_SYOKUDAI: i16 = 0x005E;
const OBJECT: &str = "object_syokudai";

/// `ACTOR_EN_ARROW` (`actor_table.h`).
const ACTOR_EN_ARROW: i16 = 0x0016;
/// `ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER` (`actor.h`).
const ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER: u32 = 1 << 10;

/// `Obj_Syokudai_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_OBJ_SYOKUDAI, name: "Obj_Syokudai", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER, object: OBJECT };

/// The types (the params' high 4 bits, `(u16)params >> 12`).
pub const TORCH_GOLDEN: usize = 0;
pub const TORCH_TIMED: usize = 1;
pub const TORCH_WOODEN: usize = 2;

/// `sCylInitStand`: the stand, metal (wood for the timed and wooden ones), 12 by 45; hit by
/// Player's attacks (`0xEE01FFFF`), hard, hookable; pushes everything.
pub const CYL_INIT_STAND: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit {
        col_type: COL_MATERIAL_METAL,
        at_flags: AT_NONE,
        ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER,
        oc_flags1: OC1_ON | OC1_TYPE_ALL,
        oc_flags2: OC2_TYPE_2,
        shape: COLSHAPE_CYLINDER,
    },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0010_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xEE01_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 12, height: 45, y_shift: 0, pos: [0, 0, 0] },
};

/// `sCylInitFlame`: the flame, 15 by 45 from 45 up; hit by arrows, fire and the Deku Stick
/// (`0x00020820`: `DMG_ARROW_NORMAL`, `DMG_ARROW_FIRE`, `DMG_MAGIC_FIRE`).
pub const CYL_INIT_FLAME: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0002_0820, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 15, height: 45, y_shift: 45, pos: [0, 0, 0] },
};

/// `sColMaterialsStand` (`ObjSyokudai_Init`'s static): the stand's material by type.
const COL_MATERIALS_STAND: [u8; 3] = [COL_MATERIAL_METAL, COL_MATERIAL_WOOD, COL_MATERIAL_WOOD];

/// The stands' bakes, by type (`displayLists`: `gGoldenTorchDL`, `gTimedTorchDL`,
/// `gWoodenTorchDL`), with `gameplay_dangeon_keep` on segment 5: the golden torch's flame guard
/// reads a texture there (`0x0500D0A0`).
const TORCH_BAKES: [&str; 3] = ["Obj_Syokudai/golden", "Obj_Syokudai/timed", "Obj_Syokudai/wooden"];
const TORCH_DLS: [&str; 3] = ["gGoldenTorchDL", "gTimedTorchDL", "gWoodenTorchDL"];
/// The flame: `Gfx_SetupDL_25Xlu`, the colours, `gEffFire1DL` under segment 8's scroll.
const FLAME_BAKE: &str = "Obj_Syokudai/flame";
const SEG_SCROLL: u8 = 0x08;
const SEG_COLORS: u8 = 0x0C;
const SEG_KEEP: u8 = 0x05;

/// The draw's scroll for `flameTexScroll`:
/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, 0, 0x20, 0x40, 1, 0, (flameTexScroll * -20) & 0x1FF, 0x20, 0x80)`.
fn flame_scroll(flame_tex_scroll: u8) -> Vec<(u32, u32)> {
    let y2 = (flame_tex_scroll as i32 * -20) as u32 & 0x1FF;
    gfx_two_tex_scroll(0, 0, 0, 0x20, 0x40, 1, 0, y2, 0x20, 0x80)
}

/// The stands, and the flame (`gDPSetPrimColor(0x80, 0x80, 255, 255, 0, 255)`,
/// `gDPSetEnvColor(255, 0, 0, 0)`, `gEffFire1DL`).
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = TORCH_BAKES
        .iter()
        .zip(TORCH_DLS)
        .map(|(name, dl)| MeshBake {
            name: (*name).into(),
            object: OBJECT.into(),
            segments: vec![(SEG_KEEP, BakeSegment::File("gameplay_dangeon_keep".into()))],
            prelude: vec![],
            body: BakeBody::DLists(vec![(OBJECT.into(), dl.into())]),
        })
        .collect();
    v.push(MeshBake {
        name: FLAME_BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_COLORS, BakeSegment::Commands(vec![(0xFA00_8080, 0xFFFF_00FF), (0xFB00_0000, 0xFF00_0000), (0xDF00_0000, 0)])), (SEG_SCROLL, BakeSegment::Dynamic(flame_scroll(0)))],
        prelude: vec![SEG_COLORS],
        body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffFire1DL".into())]),
    });
    v
}

/// The overlay's static: `sLitTorchCount`, the timed torches lit at once (every torch's init
/// zeroes it).
#[derive(Debug, Default)]
pub struct Statics {
    pub lit_torch_count: i32,
}

/// `sLitTorchCount`.
pub fn lit_torch_count(play: &mut PlayState) -> &mut i32 {
    &mut play.overlay_static::<Statics>(ACTOR_OBJ_SYOKUDAI).lit_torch_count
}

/// `PARAMS_GET_NOSHIFT(params, 12, 4)`: the type's bits, unshifted (0, 0x1000, 0x2000).
fn torch_type(params: i16) -> i32 {
    params as i32 & 0xF000
}

/// `PARAMS_GET_U(params, 6, 4)`: the count.
fn torch_count(params: i16) -> i32 {
    (params as i32 >> 6) & 0xF
}

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
fn switch_flag(params: i16) -> i32 {
    params as i32 & 0x3F
}

/// What the torch reads of Player (`GET_PLAYER(play)`): `heldItemAction`, the melee weapon's tip
/// (`MELEE_WEAPON_INFO_TIP(&meleeWeaponInfo[0])`) and `unk_860`.
struct PlayerSnap {
    held_item_action: i32,
    tip: Vec3,
    unk_860: i16,
}

fn player(play: &PlayState) -> Option<PlayerSnap> {
    let p = play.player.and_then(|h| play.actors.downcast::<Player>(h))?;
    Some(PlayerSnap { held_item_action: p.held_item_ap, tip: p.melee_weapon_info[0].tip, unk_860: p.unk_860 })
}

/// Player's `unk_860` (the burning Deku Stick's timer) written.
fn set_player_unk_860(play: &mut PlayState, v: i16) {
    if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
        p.unk_860 = v;
    }
}

/// `arrow->actor.params = 0; arrow->collider.elem.atDmgInfo.dmgFlags = DMG_ARROW_FIRE`: a normal
/// arrow through a lit flame becomes a fire arrow. `En_Arrow` isn't ported: no actor has its id,
/// so this isn't reached; its `collider` (a quad) would be its collider 0.
fn en_arrow_set_fire(play: &mut PlayState, h: ActorHandle) {
    let Some(a) = play.actors.get_mut(h) else { return };
    a.base_mut().params = 0;
    match a.collider_mut(0) {
        Some(ColliderMut::Quad(c)) => c.info.at_dmg_info.dmg_flags = DMG_ARROW_FIRE,
        _ => log::debug!("Obj_Syokudai: En_Arrow's collider isn't ported"),
    }
}

pub struct ObjSyokudai {
    pub actor: Actor,
    /// `standCollider`: collider id 0.
    pub stand_collider: ColliderCylinder,
    /// `flameCollider`: collider id 1.
    pub flame_collider: ColliderCylinder,
    /// `litTimer`: 0 unlit, -1 lit for good, else the frames left.
    pub lit_timer: i16,
    /// `flameTexScroll`: the flame's scroll.
    pub flame_tex_scroll: u8,
    /// `lightNode` and `lightInfo` (the context holds the info the node points at; this is the
    /// actor's copy it writes through the node).
    pub light_node: Option<LightNode>,
    pub light_info: LightInfo,
}

impl ObjSyokudai {
    /// `ObjSyokudai_Init`: the colliders (the stand's material by type), immovable, a glowing
    /// point light 70 up (radius -1 until the first update), lit from the start when always lit or
    /// its switch flag is set (`@bug (game)`: `torchType != 2` always holds, so a wooden torch
    /// too); the flame's scroll from `Rand`, `sLitTorchCount` reset, the focus 60 up.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let params = actor.params;
        let torch_type = torch_type(params);
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 1000); cullingVolumeDistance 4000,
        // cullingVolumeScale 800, cullingVolumeDownward 800 (the cull zone isn't ported).
        actor.scale = Vec3::ONE;
        // ActorShape_Init(&shape, 0, NULL, 0).
        actor.shape_y_offset = 0.0;
        let mut stand_collider = ColliderCylinder::new(&CYL_INIT_STAND);
        // sColMaterialsStand[PARAMS_GET_NOMASK(params, 12)].
        if let Some(&m) = COL_MATERIALS_STAND.get((params >> 12) as usize) {
            stand_collider.base.col_type = m;
        }
        let flame_collider = ColliderCylinder::new(&CYL_INIT_FLAME);
        actor.col_chk_info.mass = MASS_IMMOVABLE;
        let p = actor.world_pos;
        let light_info = LightInfo::point_glow(p.x as i16, (p.y + 70.0) as i16, p.z as i16, [255, 255, 180], -1);
        let light_node = play.light_ctx.insert_light(light_info);
        let mut t = ObjSyokudai { actor, stand_collider, flame_collider, lit_timer: 0, flame_tex_scroll: 0, light_node, light_info };
        if params & 0x400 != 0 || (torch_type != 2 && play.flags.get_switch(switch_flag(params))) {
            t.lit_timer = -1;
        }
        t.flame_tex_scroll = (play.rand.zero_one() * 20.0) as i32 as u8;
        *lit_torch_count(play) = 0;
        t.actor.set_focus(60.0);
        Box::new(t)
    }

    /// `OnePointCutscene_Attention(play, &this->actor)`.
    fn attention(&self, play: &mut PlayState) {
        if let Some(me) = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor)) {
            play.onepoint_attention(me);
        }
    }

    /// The type's index (`(u16)params >> 12`) for the draw.
    pub fn type_index(&self) -> usize {
        (self.actor.params as u16 >> 12) as usize
    }
}

impl ActorImpl for ObjSyokudai {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjSyokudai_Update`: under water deeper than 52, out; else lit (or burning out) by its
    /// switch flag, and lit by fire or a burning Deku Stick at its flame (or lighting the stick);
    /// the colliders; the timer down (a timed one burnt out leaves `sLitTorchCount`); while lit,
    /// the light (200, shrinking over the last 20 frames) at a brightness from `Rand`, and the
    /// crackle.
    fn update(&mut self, play: &mut PlayState) {
        let params = self.actor.params;
        let mut torch_count = torch_count(params);
        let switch_flag = switch_flag(params);
        let torch_type = torch_type(params);
        let mut light_radius: i32 = -1;
        let mut brightness: u8 = 0;
        let lit_time_scale = torch_count;
        if torch_count == 10 {
            torch_count = 24;
        }
        let p = self.actor.world_pos;
        let water_surface = play.col.water_surface(p.x, p.z, play.col.water_room);
        if let Some(water_surface) = water_surface
            && (water_surface - p.y) > 52.0
        {
            self.lit_timer = 0;
            // @bug (game): torchType is 0, 0x1000 or 0x2000, never 1.
            if torch_type == 1 {
                play.flags.unset_switch(switch_flag);
                if torch_count != 0 {
                    self.lit_timer = 1;
                }
            }
        } else {
            // (No Player: as one holding nothing.)
            let pl = player(play).unwrap_or(PlayerSnap { held_item_action: 0, tip: Vec3::ZERO, unk_860: 0 });
            let mut interaction_type = 0;
            if params & 0x400 != 0 {
                self.lit_timer = -1;
            }
            if torch_count != 0 {
                if play.flags.get_switch(switch_flag) {
                    if self.lit_timer == 0 {
                        self.lit_timer = -1;
                        if torch_type == 0 {
                            self.attention(play);
                        }
                    } else if self.lit_timer > 0 {
                        self.lit_timer = -1;
                    }
                } else if self.lit_timer < 0 {
                    self.lit_timer = 20;
                }
            }
            let mut dmg_flags = 0;
            if self.flame_collider.base.ac_flags & AC_HIT != 0 {
                dmg_flags = self.flame_collider.info.ac_hit_elem.map(|h| h.at_dmg_info.dmg_flags).unwrap_or(0);
                if dmg_flags & (DMG_FIRE | DMG_ARROW_NORMAL) != 0 {
                    interaction_type = 1;
                }
            } else if pl.held_item_action == play.data.items.ap("DEKU_STICK") {
                // Math_Vec3f_Diff(tip, &world.pos, &tipToFlame), 67 up: within 20 of the flame.
                let mut tip_to_flame = pl.tip - self.actor.world_pos;
                tip_to_flame.y -= 67.0;
                if tip_to_flame.x * tip_to_flame.x + tip_to_flame.y * tip_to_flame.y + tip_to_flame.z * tip_to_flame.z < 20.0 * 20.0 {
                    interaction_type = -1;
                }
            }
            if interaction_type != 0 {
                if self.lit_timer != 0 {
                    if interaction_type < 0 {
                        // The stick catches fire, or burns on.
                        if pl.unk_860 == 0 {
                            set_player_unk_860(play, 210);
                            // SFX_PLAY_AT_POS(&this->actor.projectedPos, NA_SE_EV_FLAME_IGNITION).
                            audio_play_actor_sfx2(play, NA_SE_EV_FLAME_IGNITION);
                        } else if pl.unk_860 < 200 {
                            set_player_unk_860(play, 200);
                        }
                    } else if dmg_flags & DMG_ARROW_NORMAL != 0 {
                        // arrow->actor.update != NULL && arrow->actor.id == ACTOR_EN_ARROW.
                        if let Some(h) = self.flame_collider.base.ac
                            && play.actors.actor(h).is_some_and(|a| !a.killed && a.id == ACTOR_EN_ARROW)
                        {
                            en_arrow_set_fire(play, h);
                        }
                    }
                    let lit_max = 50 * lit_time_scale + 100;
                    if 0 <= self.lit_timer && (self.lit_timer as i32) < lit_max && torch_type != 0 {
                        self.lit_timer = lit_max as i16;
                    }
                } else if torch_type != 0 && ((interaction_type > 0 && dmg_flags & DMG_FIRE != 0) || (interaction_type < 0 && pl.unk_860 != 0)) {
                    if interaction_type < 0 && pl.unk_860 < 200 {
                        set_player_unk_860(play, 200);
                    }
                    if torch_count == 0 {
                        self.lit_timer = -1;
                        // @bug (game): torchType != 2 always holds (a wooden torch sets its flag too).
                        if torch_type != 2 {
                            play.flags.set_switch(switch_flag);
                            self.attention(play);
                        }
                    } else {
                        *lit_torch_count(play) += 1;
                        if *lit_torch_count(play) >= torch_count {
                            play.flags.set_switch(switch_flag);
                            self.attention(play);
                            self.lit_timer = -1;
                        } else {
                            self.lit_timer = ((lit_time_scale * 50) + 110) as i16;
                        }
                    }
                    // SFX_PLAY_AT_POS(&this->actor.projectedPos, NA_SE_EV_FLAME_IGNITION).
                    audio_play_actor_sfx2(play, NA_SE_EV_FLAME_IGNITION);
                }
            }
        }
        self.stand_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.stand_collider);
        play.collision_check_set_ac(&self.actor, 0, &mut self.stand_collider);
        self.flame_collider.update(&self.actor);
        play.collision_check_set_ac(&self.actor, 1, &mut self.flame_collider);
        if self.lit_timer > 0 {
            self.lit_timer -= 1;
            if self.lit_timer == 0 && torch_type != 0 {
                *lit_torch_count(play) -= 1;
            }
        }
        if self.lit_timer != 0 {
            light_radius = if self.lit_timer < 0 || self.lit_timer >= 20 { 200 } else { ((self.lit_timer as f32 * 200.0) / 20.0) as i32 };
            brightness = ((play.rand.zero_one() * 127.0) as u8).wrapping_add(128);
            self.actor.play_sfx_flagged(NA_SE_EV_TORCH - SFX_FLAG);
        }
        // Lights_PointSetColorAndRadius(&lightInfo, brightness, brightness, 0, lightRadius).
        self.light_info.color = [brightness, brightness, 0];
        self.light_info.radius = light_radius as i16;
        play.light_ctx.set_info(self.light_node, self.light_info);
        self.flame_tex_scroll = self.flame_tex_scroll.wrapping_add(1);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.type_index() as u32, self.lit_timer as i32 as u32, self.flame_tex_scroll as u32];
        rs
    }

    /// `ObjSyokudai_Draw`: the stand by type; lit, the flame 52 up, turned to face the active
    /// camera (`Camera_GetCamDirYaw`), growing in over the 10 frames after it's lit by fire (its
    /// timer above `count * 50 + 100`) and shrinking over its last 20.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let [ty, lit_timer, scroll] = rs.switches.as_slice() else { return };
        let (lit_timer, scroll) = (*lit_timer as i32, *scroll as u8);
        let model = actor_draw_matrix(rs);
        let Some(bake) = TORCH_BAKES.get(*ty as usize) else { return };
        out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(bake)), model));
        if lit_timer == 0 {
            return;
        }
        let timer_max = torch_count(self.actor.params) * 50 + 100;
        let mut flame_scale = 1.0;
        if lit_timer > timer_max {
            flame_scale = (timer_max - lit_timer + 10) as f32 / 10.0;
        } else if lit_timer > 0 && lit_timer < 20 {
            flame_scale = lit_timer as f32 / 20.0;
        }
        flame_scale *= 0.0027;
        // Matrix_Translate(0, 52, 0), Matrix_RotateY((s16)(camDirYaw - shape.rot.y + 0x8000)),
        // Matrix_Scale(flameScale), each MTXMODE_APPLY.
        let angle = binang_to_rad(play.cam_dir_yaw().wrapping_sub(rs.rot[1]).wrapping_add(i16::MIN));
        let t = model * Mat4::from_translation(Vec3::new(0.0, 52.0, 0.0)) * Mat4::from_rotation_y(angle) * Mat4::from_scale(Vec3::splat(flame_scale));
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &flame_scroll(scroll));
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(FLAME_BAKE)), transform: t, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    /// `ObjSyokudai_Destroy`: the light out of the context.
    fn destroy(&mut self, play: &mut PlayState) {
        play.light_ctx.remove_light(self.light_node.take());
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Cylinder(&mut self.stand_collider)),
            1 => Some(ColliderMut::Cylinder(&mut self.flame_collider)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
