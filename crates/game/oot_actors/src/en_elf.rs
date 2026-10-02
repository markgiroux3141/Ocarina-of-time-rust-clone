//! `En_Elf` (`ovl_En_Elf/z_en_elf.c`): the fairies. `params` is the type (`FairyType`):
//! - `FAIRY_NAVI` (0), Link's fairy, spawned by `Player_Init` (`Player_SpawnFairy`). She
//!   follows Link's hat, flies to what Z would lock on to (the target context's `naviRefPos`,
//!   in its category's colours), fades into Link's hat when nothing is around, and dashes out
//!   when something is. She says her C-Up text (`naviTextId`, from `QuestHint_GetNaviTextId`
//!   while `naviTimer` is between 600 and 3000) when Player talks to her. In a cutscene she
//!   follows her cue in `npcActions[8]` (the opening, the Deku Tree's talk);
//! - `FAIRY_KOKIRI` (3), a Kokiri child's fairy, in a colour of `sColorFlags`, bobbing above
//!   its parent;
//! - the healing fairies (`FAIRY_HEAL`, `_TIMED`, `_BIG`), the revival ones (`FAIRY_REVIVE_*`)
//!   and the spawner (`FAIRY_SPAWNER`): ported, their `Magic_Fill` aside.
//!
//! Drawn as `EnElf_Draw` does: `gFairySkel` (`gameplay_keep`) after `Gfx_SetupDL_27Xlu`, with
//! segment 8's prim colour and render mode (Navi's without the z-buffer) and the outer colour as
//! the env colour, in two bakes. `EnElf_OverrideLimbDraw` puts limb 8 (the glow) at its
//! parent's origin, unrotated, at its pulsing scale.
//!
//! Not ported: the sparkles (`EffectSsKiraKira`: their spawn's `Rand` calls are made), the
//! lights' effect on the draw (`crate::lights` keeps them, the renderer can't draw
//! them), `Environment_AdjustLights` while Navi talks, and `Elf_Msg` (no ported scene has one).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_gfx::{DrawCmd, MeshKey, SegmentValues};
use eng_math::{atan2_s, cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_ITEMACTION, ACTORCAT_NPC, ActorHandle, ActorImpl, ActorProfile, PLAYER_BODYPART_HAT, PLAYER_BODYPART_HEAD, PLAYER_BODYPART_WAIST, audio_play_actor_sfx2};
use oot_game::audio::sfx::*;
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::lights::{LightInfo, LightNode};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, VIEWPOINT_PIVOT, ViewInfo};
use oot_game::skelanime_std::SkelAnimeStd;

pub const ACTOR_EN_ELF: i16 = 0x0018;

/// `En_Elf_Profile`: `ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA`, `ACTORCAT_ITEMACTION`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_ELF, name: "En_Elf", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: "gameplay_keep" };

// `FairyType`.
pub const FAIRY_NAVI: i16 = 0;
pub const FAIRY_REVIVE_BOTTLE: i16 = 1;
pub const FAIRY_HEAL_TIMED: i16 = 2;
pub const FAIRY_KOKIRI: i16 = 3;
pub const FAIRY_SPAWNER: i16 = 4;
pub const FAIRY_REVIVE_DEATH: i16 = 5;
pub const FAIRY_HEAL: i16 = 6;
pub const FAIRY_HEAL_BIG: i16 = 7;

/// `FAIRY_FLAG_TIMED`, `FAIRY_FLAG_BIG`.
pub const FAIRY_FLAG_TIMED: u16 = 1 << 8;
pub const FAIRY_FLAG_BIG: u16 = 1 << 9;

/// `PLAYER_STATE1_10` (holding an item up, a first-person item), `PLAYER_STATE1_20` (first
/// person), `PLAYER_STATE2_NAVI_ACTIVE` (Navi out of Link's hat) (`player.h`).
const PLAYER_STATE1_10: u32 = 1 << 10;
const PLAYER_STATE1_20: u32 = 1 << 20;
pub const PLAYER_STATE2_NAVI_ACTIVE: u32 = 1 << 20;

/// `SCENE_LINKS_HOUSE`.
const SCENE_LINKS_HOUSE: u16 = 0x34;
/// `GI_MAX`: a healing fairy's offer, which only a bottle takes.
const GI_MAX: i16 = 0x7E;

/// `sInnerColors`, `sOuterColors`.
const INNER_COLORS: [[f32; 4]; 2] = [[255.0, 255.0, 255.0, 255.0], [255.0, 220.0, 220.0, 255.0]];
const OUTER_COLORS: [[f32; 4]; 2] = [[255.0, 255.0, 255.0, 255.0], [255.0, 50.0, 100.0, 255.0]];
/// `sColorFlags`: each Kokiri fairy colour's r, g, b (1: 200..255, 2: 0..255, 0: none).
const COLOR_FLAGS: [[u8; 3]; 13] = [[0, 0, 0], [1, 0, 0], [1, 2, 0], [1, 0, 2], [0, 1, 0], [2, 1, 0], [0, 1, 2], [0, 0, 1], [2, 0, 1], [0, 2, 1], [1, 1, 0], [1, 0, 1], [0, 1, 1]];

/// The bakes: Navi's (`G_RM_PASS, G_RM_CLD_SURF2`, `fairyFlags & 4`) and the others'
/// (`G_RM_PASS, G_RM_ZB_CLD_SURF2`).
pub const BAKE_NAVI: &str = "En_Elf/navi";
pub const BAKE_FAIRY: &str = "En_Elf/fairy";
/// The segments: 8 the draw's list (`gSPSegment(POLY_XLU_DISP++, 0x08, dListHead)`), 0x0D
/// `Gfx_SetupDL_27Xlu`'s `sSetupDL[SETUPDL_27]`, 0x0E the env colour it sets before
/// `SkelAnime_Draw`, and 1 `play->billboardMtx`, which the glow's list (`gGlowCircleSmallDL`,
/// limb 8) multiplies in: the bake takes the identity, and the draw applies the billboard to
/// that limb (`ViewInfo::billboard`), as `En_Item00`'s do.
const SEG_BILLBOARD: u8 = 0x01;
const SEG_LIST: u8 = 0x08;
const SEG_SETUP_DL: u8 = 0x0D;
const SEG_ENV: u8 = 0x0E;

/// The fairies' meshes (docs/adr/0012-actor-bakes.md). `sSetupDL` (`z_rcp.c`) is data in `code`,
/// so SETUPDL_27 is written out from its macros (`gbi.h` values), as `target::bakes` does.
pub fn bakes() -> Vec<MeshBake> {
    const PIPE_SYNC: (u32, u32) = (0xE700_0000, 0);
    const END: (u32, u32) = (0xDF00_0000, 0);
    // SETUPDL_27: gsSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, G_ON),
    // gsDPSetCombineMode(G_CC_MODULATEI_PRIM, G_CC_MODULATEI_PRIM),
    // gsDPSetOtherMode(G_AD_NOTPATTERN | G_TC_FILT | G_TF_BILERP | G_TP_PERSP (| 0 fields),
    //                  G_AC_NONE | G_ZS_PIXEL | G_RM_AA_ZB_XLU_SURF | G_RM_AA_ZB_XLU_SURF2),
    // gsSPLoadGeometryMode(G_ZBUFFER | G_SHADE | G_CULL_BACK | G_SHADING_SMOOTH).
    let setup_dl_27 = vec![PIPE_SYNC, (0xD700_0002, 0xFFFF_FFFF), (0xFC11_FE23, 0xFFFF_F7FB), (0xEF08_2C10, 0x0050_49D8), (0xD900_0000, 0x0020_0405)];
    // EnElf_Draw's segment 8: gDPPipeSync, gDPSetPrimColor(0, 0x01, inner), gDPSetRenderMode
    // (G_SETOTHERMODE_L, shift 3, length 29): G_RM_PASS (GBL_c1(G_BL_CLR_IN, G_BL_0,
    // G_BL_CLR_IN, G_BL_1)) with G_RM_CLD_SURF2 (IM_RD | CVG_DST_SAVE | FORCE_BL | ZMODE_OPA |
    // GBL_c2(G_BL_CLR_IN, G_BL_A_IN, G_BL_CLR_MEM, G_BL_1MA)) or G_RM_ZB_CLD_SURF2 (also
    // Z_CMP, ZMODE_XLU), gSPEndDisplayList.
    let list = |render_mode: u32| vec![PIPE_SYNC, (0xFA00_0001, 0x0000_00FF), (0xE200_001C, render_mode), END];
    let bake = |name: &str, render_mode: u32| MeshBake {
        name: name.into(),
        object: "gameplay_keep".into(),
        segments: vec![
            (SEG_BILLBOARD, BakeSegment::Bytes(crate::en_item00::identity_mtx())),
            (SEG_SETUP_DL, BakeSegment::Commands(setup_dl_27.clone())),
            (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false }),
            (SEG_LIST, BakeSegment::Dynamic(list(render_mode))),
        ],
        prelude: vec![SEG_SETUP_DL, SEG_ENV],
        body: BakeBody::Skeleton { file: "gameplay_keep".into(), symbol: "gFairySkel".into(), limbs: Vec::new() },
    };
    vec![bake(BAKE_NAVI, 0x0C18_4340), bake(BAKE_FAIRY, 0x0C18_4B50)]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80A03CF8`: Navi.
    Navi,
    /// `func_80A0329C`: a healing fairy flying round its home.
    Heal,
    /// `func_80A03610`: healing Link, circling up and away.
    Healing,
    /// `func_80A03990`, `func_80A03814`: reviving Link.
    Revive,
    ReviveCircle,
    /// `func_80A0353C`: a Kokiri's fairy.
    Kokiri,
    /// `func_80A03604`: the spawner, which does nothing.
    Spawner,
}

/// `func_2C8`: the drift that `func_80A01C38` picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    None,
    /// `func_80A02A20`: a circle and a bob.
    Circle,
    /// `func_80A02AA4`: a pulsing circle.
    Pulse,
    /// `func_80A02B38`: a figure of eight across Link's facing.
    Eight,
    /// `func_80A0214C`, `func_80A01FE0`, `func_80A020A4`: the healing fairy's wander.
    Wander,
    Follow,
    Flee,
}

/// The update `actor.update` holds: `EnElf_Update`, or Navi's `func_80A053F0` and her talk's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Update {
    Normal,
    /// `func_80A053F0`.
    Navi,
    /// `func_80A052F4`: her text; its end, or its choice to talk to Saria.
    Talk,
    /// `func_80A05208`: "talk to Navi?" after no to Saria.
    AskNavi,
    /// `func_80A05188`, `func_80A05114`, `func_80A05040`: Saria's texts.
    SariaText,
    SariaMore,
    SariaAgain,
}

pub struct EnElf {
    pub actor: Actor,
    pub skel: Option<SkelAnimeStd>,
    pub skeleton: Option<Arc<Skeleton>>,
    pub inner_color: [f32; 4],
    pub outer_color: [f32; 4],
    pub light_glow: Option<LightNode>,
    pub light_no_glow: Option<LightNode>,
    pub unk_28c: Vec3,
    pub unk_29c: f32,
    pub unk_2a0: f32,
    pub unk_2a4: f32,
    pub unk_2a8: i16,
    pub unk_2aa: i16,
    pub unk_2ac: i16,
    pub unk_2ae: i16,
    pub unk_2b0: i16,
    pub unk_2b4: f32,
    pub unk_2b8: f32,
    pub unk_2bc: i16,
    pub timer: u16,
    pub unk_2c0: i16,
    pub disappear_timer: i16,
    pub fairy_flags: u16,
    pub unk_2c6: u8,
    pub unk_2c7: u8,
    pub drift: Drift,
    pub action: Action,
    pub update_fn: Update,
    /// `shape.shadowAlpha` (`func_80A04D90` sets 50; only the big fairy has a shadow).
    pub shadow_alpha: u8,
}

/// What the fairy reads of Player (`GET_PLAYER(play)`), at the start of its update.
#[derive(Debug, Clone, Copy)]
struct PlayerSnap {
    handle: ActorHandle,
    pos: Vec3,
    shape_yaw: i16,
    hat: Vec3,
    head: Vec3,
    waist: Vec3,
    target: Option<ActorHandle>,
    state1: u32,
    state2: u32,
    navi_text_id: i16,
}

fn player(play: &PlayState) -> Option<PlayerSnap> {
    let h = play.player?;
    let p = play.actors.get(h)?;
    let pi = p.as_player()?;
    Some(PlayerSnap {
        handle: h,
        pos: p.base().world_pos,
        shape_yaw: p.base().shape_rot.y,
        hat: pi.body_part(PLAYER_BODYPART_HAT),
        head: pi.body_part(PLAYER_BODYPART_HEAD),
        waist: pi.body_part(PLAYER_BODYPART_WAIST),
        target: pi.target(),
        state1: pi.state_flags1(),
        state2: pi.state_flags2(),
        navi_text_id: pi.navi_text_id(),
    })
}

/// `player->stateFlags2` bits on, then off.
fn change_player_state2(play: &mut PlayState, set: u32, clear: u32) {
    if let Some(pi) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        pi.change_state_flags2(set, clear);
    }
}

fn set_navi_text_id(play: &mut PlayState, id: i16) {
    if let Some(pi) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        pi.set_navi_text_id(id);
    }
}

/// `func_80A01F90`: `a` farther than `dist` from `b` in x and z.
fn func_80a01f90(a: Vec3, b: Vec3, dist: f32) -> bool {
    dist * dist < (b.x - a.x) * (b.x - a.x) + (b.z - a.z) * (b.z - a.z)
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    ((b.x - a.x) * (b.x - a.x) + (b.z - a.z) * (b.z - a.z)).sqrt()
}

impl EnElf {
    fn new(actor: Actor) -> EnElf {
        EnElf {
            actor,
            skel: None,
            skeleton: None,
            inner_color: INNER_COLORS[0],
            outer_color: OUTER_COLORS[0],
            light_glow: None,
            light_no_glow: None,
            unk_28c: Vec3::ZERO,
            unk_29c: 0.0,
            unk_2a0: 0.0,
            unk_2a4: 0.0,
            unk_2a8: 0,
            unk_2aa: 0,
            unk_2ac: 0,
            unk_2ae: 0,
            unk_2b0: 0,
            unk_2b4: 0.0,
            unk_2b8: 0.0,
            unk_2bc: 0,
            timer: 0,
            unk_2c0: 0,
            disappear_timer: 0,
            fairy_flags: 0,
            unk_2c6: 0,
            unk_2c7: 0,
            drift: Drift::None,
            action: Action::Spawner,
            update_fn: Update::Normal,
            shadow_alpha: 0xFF,
        }
    }

    /// `EnElf_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut e = EnElf::new(actor);
        // ICHAIN_VEC3F_DIV1000(scale, 8).
        e.actor.scale = Vec3::splat(0.008);
        match play.assets.clone() {
            Some(assets) => match (assets.skeleton("gameplay_keep", "gFairySkel"), assets.animation("gameplay_keep", "gFairyAnim")) {
                (Ok(s), Ok(a)) => {
                    // SkelAnime_Init(..., 15): the skeleton's limbs and the root.
                    e.skel = Some(SkelAnimeStd::init_flex(s.limbs.len(), Some(a)));
                    e.skeleton = Some(s);
                }
                (s, a) => log::error!("En_Elf: {:?} {:?}", s.err(), a.err()),
            },
            None => {}
        }
        // ActorShape_Init(&shape, 0, NULL, 15), shadowAlpha 0xFF.
        e.actor.shape_y_offset = 0.0;
        e.shadow_alpha = 0xFF;
        let p = e.actor.world_pos;
        let (x, y, z) = (p.x as i16, p.y as i16, p.z as i16);
        e.light_glow = play.light_ctx.insert_light(LightInfo::point_glow(x, y, z, [255, 255, 255], 0));
        e.light_no_glow = play.light_ctx.insert_light(LightInfo::point_no_glow(x, y, z, [255, 255, 255], 0));
        e.fairy_flags = 0;
        e.disappear_timer = 600;
        e.unk_2a4 = 0.0;
        let mut color_config: i32 = 0;
        let pl = player(play);
        match e.actor.params {
            FAIRY_NAVI => {
                e.actor.room = -1;
                e.action = Action::Navi;
                e.func_80a01c38(0);
                e.fairy_flags |= 4;
                e.update_fn = Update::Navi;
                e.unk_2c7 = 0x14;
                if play.save.navi_timer >= 25800 || play.save.navi_timer < 3000 {
                    play.save.navi_timer = 0;
                }
            }
            FAIRY_REVIVE_BOTTLE => {
                color_config = -1;
                e.action = Action::Healing;
                if let Some(pl) = pl {
                    e.unk_2b8 = dist_xz(e.actor.world_pos, pl.pos);
                    e.unk_2ac = pl.shape_yaw;
                    e.unk_28c.y = e.actor.world_pos.y - pl.pos.y;
                }
                e.unk_2b0 = -0x1000;
                e.unk_2aa = 0;
                e.unk_2b4 = 0.0;
            }
            FAIRY_REVIVE_DEATH => {
                color_config = -1;
                e.action = Action::Revive;
                e.unk_2b8 = 0.0;
                if let Some(pl) = pl {
                    e.unk_2ac = pl.shape_yaw;
                    e.unk_28c.y = e.actor.world_pos.y - pl.pos.y;
                }
                e.unk_2b0 = 0;
                e.unk_2aa = 0;
                e.unk_2b4 = 7.0;
            }
            FAIRY_HEAL_BIG | FAIRY_HEAL_TIMED | FAIRY_HEAL => {
                if e.actor.params == FAIRY_HEAL_BIG {
                    // (shape.shadowDraw = ActorShadow_DrawWhiteCircle: shadows aren't drawn.)
                    e.fairy_flags |= FAIRY_FLAG_BIG;
                }
                if e.actor.params != FAIRY_HEAL {
                    e.fairy_flags |= FAIRY_FLAG_TIMED;
                }
                color_config = -1;
                e.action = Action::Heal;
                e.unk_2b4 = play.rand.zero_float(10.0) + 10.0;
                e.unk_2aa = 0;
                e.unk_2ae = (play.rand.zero_float(1048.0) as i16).wrapping_add(0x200);
                e.unk_28c = e.actor.world_pos;
                e.unk_2bc = play.rand.centered_float(32767.0) as i16;
                e.drift = Drift::Wander;
                // The C's actor is zeroed memory before Actor_UpdateAll first measures it.
                e.actor.xz_dist_to_player = 0.0;
                e.func_80a0232c(play);
                e.unk_2c0 = 0;
                e.disappear_timer = 240;
            }
            FAIRY_KOKIRI => {
                color_config = (play.rand.zero_float(11.99) + 1.0) as i32;
                e.action = Action::Kokiri;
                e.func_80a01c38(0);
            }
            FAIRY_SPAWNER => {
                e.action = Action::Spawner;
                e.func_80a01c38(8);
                let p = e.actor.world_pos;
                for _ in 0..8 {
                    let _ = play.actor_spawn(ACTOR_EN_ELF, Vec3::new(p.x, p.y - 30.0, p.z), [0; 3], FAIRY_HEAL);
                }
            }
            _ => log::error!("En_Elf: unknown type {} (ASSERT)", e.actor.params),
        }
        e.unk_2a0 = 3.0;
        e.inner_color = INNER_COLORS[0];
        if color_config > 0 {
            let f = COLOR_FLAGS[color_config as usize];
            e.outer_color = [color_value(play, f[0]), color_value(play, f[1]), color_value(play, f[2]), 0.0];
        } else {
            e.inner_color = INNER_COLORS[(-color_config) as usize];
            e.outer_color = OUTER_COLORS[(-color_config) as usize];
        }
        Box::new(e)
    }

    fn play_speed(&mut self, s: f32) {
        if let Some(sk) = &mut self.skel {
            sk.play_speed = s;
        }
    }

    fn skel_update(&mut self) {
        if let Some(sk) = &mut self.skel {
            sk.update();
        }
    }

    /// `func_80A01C38`: the drift and its speeds for mode `arg1` (`unk_2A8`).
    fn func_80a01c38(&mut self, arg1: i16) {
        self.unk_2a8 = arg1;
        let (ae, b0, c0, b4, b8, speed, drift): (Option<i16>, Option<i16>, Option<i16>, f32, f32, f32, Drift) = match arg1 {
            0 => (Some(0x400), Some(0x200), Some(100), 5.0, 20.0, 1.0, Drift::Circle),
            12 => (Some(0x400), Some(0x200), Some(100), 1.0, 5.0, 1.0, Drift::Circle),
            10 => (Some(0x400), Some(0), None, 5.0, 0.0, 1.0, Drift::Circle),
            9 => (Some(0x1000), Some(0x200), None, 3.0, 10.0, 1.0, Drift::Circle),
            7 => (Some(0x1E), None, Some(1), 0.0, 0.0, 1.0, Drift::Circle),
            8 => (Some(0x1000), Some(0x200), None, 0.0, 0.0, 1.0, Drift::Circle),
            1 => (Some(0x1000), Some(0x800), None, 5.0, 7.5, 2.0, Drift::Circle),
            2 => (Some(0x400), Some(0x1000), None, 10.0, 20.0, 1.0, Drift::Pulse),
            3 => (None, Some(0x600), None, 1.0, 1.0, 1.0, Drift::Eight),
            4 => (None, Some(0x800), None, 20.0, 10.0, 2.0, Drift::Eight),
            5 => (None, Some(0x200), None, 10.0, 10.0, 0.5, Drift::Eight),
            6 => (Some(0x1000), Some(0x800), None, 60.0, 20.0, 2.0, Drift::Circle),
            11 => (Some(0x400), Some(0x2000), Some(42), 5.0, 1.0, 1.0, Drift::Circle),
            _ => return,
        };
        if let Some(v) = ae {
            self.unk_2ae = v;
        }
        if let Some(v) = b0 {
            self.unk_2b0 = v;
        }
        if let Some(v) = c0 {
            self.unk_2c0 = v;
        }
        self.drift = drift;
        self.unk_2b4 = b4;
        self.unk_2b8 = b8;
        self.play_speed(speed);
    }

    /// `this->func_2C8(this, play)`.
    fn drift(&mut self, play: &mut PlayState) {
        match self.drift {
            Drift::None => log::error!("En_Elf: no func_2C8 (ASSERT)"),
            Drift::Circle => self.func_80a02a20(),
            Drift::Pulse => self.func_80a02aa4(),
            Drift::Eight => self.func_80a02b38(play),
            Drift::Wander => self.func_80a0214c(play),
            Drift::Follow => self.func_80a01fe0(play),
            Drift::Flee => self.func_80a020a4(play),
        }
    }

    /// `func_80A01FE0`: after Link, slower near him, until `unk_2C0` runs out.
    fn func_80a01fe0(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        self.unk_2b8 = if !func_80a01f90(self.actor.world_pos, pl.pos, 30.0) { 0.5 } else { 2.0 };
        if self.unk_2c0 > 0 {
            self.unk_2c0 -= 1;
        } else {
            self.unk_2a8 = 1;
            self.unk_2ac = 0x80;
            self.unk_2b8 = play.rand.zero_float(1.0) + 0.5;
            self.unk_2b0 = play.rand.centered_float(32767.0) as i16;
            self.drift = Drift::Wander;
        }
    }

    /// `func_80A020A4`: away from Link, until 50 off and `unk_2C0` runs out.
    fn func_80a020a4(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        if func_80a01f90(self.actor.world_pos, pl.pos, 50.0) {
            if self.unk_2c0 > 0 {
                self.unk_2c0 -= 1;
            } else {
                self.unk_2a8 = 1;
                self.unk_2ac = 0x80;
                self.unk_2b8 = play.rand.zero_float(1.0) + 0.5;
                self.unk_2b0 = play.rand.centered_float(32767.0) as i16;
                self.drift = Drift::Wander;
            }
        }
    }

    /// `func_80A0214C`: the wander's choice every `unk_2C0` frames: away from Link when close,
    /// to him more likely the farther he is, and now and then a new heading.
    fn func_80a0214c(&mut self, play: &mut PlayState) {
        if self.unk_2c0 > 0 {
            self.unk_2c0 -= 1;
        } else {
            let mut d = self.actor.xz_dist_to_player;
            if d < 50.0 {
                if play.rand.zero_one() < 0.2 {
                    self.unk_2a8 = 2;
                    self.unk_2ac = 0x400;
                    self.unk_2b8 = 2.0;
                    self.drift = Drift::Flee;
                    self.actor.speed_xz = 1.5;
                    self.unk_2c0 = (play.rand.zero_float(8.0) as i16) + 4;
                } else {
                    self.unk_2c0 = 10;
                }
            } else {
                if d > 150.0 {
                    d = 150.0;
                }
                d = ((d - 50.0) * 0.95) + 0.05;
                if play.rand.zero_one() < d {
                    self.unk_2a8 = 3;
                    self.unk_2ac = 0x200;
                    self.unk_2b8 = (d * 2.0) + 1.0;
                    self.drift = Drift::Follow;
                    self.unk_2c0 = (play.rand.zero_float(16.0) as i16) + 0x10;
                } else {
                    self.unk_2c0 = 10;
                }
            }
        }
        if play.rand.zero_one() < 0.1 {
            self.unk_2a8 = 1;
            self.unk_2ac = 0x80;
            self.unk_2b8 = play.rand.zero_float(0.5) + 0.5;
            self.unk_2b0 = play.rand.centered_float(32767.0) as i16;
        }
    }

    /// `func_80A0232C`: back towards home once more than 100 from it.
    fn func_80a0232c(&mut self, play: &mut PlayState) {
        if func_80a01f90(self.unk_28c, self.actor.world_pos, 100.0) {
            self.unk_2a8 = 0;
            self.unk_2ac = 0x200;
            self.drift = Drift::Wander;
            self.unk_2b8 = 1.5;
        } else {
            self.drift(play);
        }
    }

    /// `func_80A02A20`.
    fn func_80a02a20(&mut self) {
        self.unk_28c.x = sin_s(self.unk_2ac) * self.unk_2b8;
        self.unk_28c.y = sin_s(self.unk_2aa) * self.unk_2b4;
        self.unk_28c.z = cos_s(self.unk_2ac) * self.unk_2b8;
        self.unk_2ac = self.unk_2ac.wrapping_add(self.unk_2b0);
        self.unk_2aa = self.unk_2aa.wrapping_add(self.unk_2ae);
    }

    /// `func_80A02AA4`.
    fn func_80a02aa4(&mut self) {
        let xz_scale = (cos_s(self.unk_2aa) * self.unk_2b4) + self.unk_2b8;
        self.unk_28c.x = sin_s(self.unk_2ac) * xz_scale;
        self.unk_28c.y = 0.0;
        self.unk_28c.z = cos_s(self.unk_2ac) * xz_scale;
        self.unk_2ac = self.unk_2ac.wrapping_add(self.unk_2b0);
        self.unk_2aa = self.unk_2aa.wrapping_add(self.unk_2ae);
    }

    /// `func_80A02B38`.
    fn func_80a02b38(&mut self, play: &PlayState) {
        let yaw = player(play).map(|p| p.shape_yaw).unwrap_or(0);
        self.unk_2aa = (self.unk_2ac as i32 * 2) as i16;
        self.unk_28c.x = sin_s(self.unk_2ac) * self.unk_2b8;
        self.unk_28c.y = sin_s(self.unk_2aa) * self.unk_2b4;
        self.unk_28c.z = -sin_s(yaw) * self.unk_28c.x;
        self.unk_28c.x = cos_s(yaw) * self.unk_28c.x;
        self.unk_2ac = self.unk_2ac.wrapping_add(self.unk_2b0);
    }

    /// `func_80A02BD8`: the vertical velocity towards `target.y` plus the drift.
    fn func_80a02bd8(&mut self, target: Vec3, arg2: f32) {
        let t = ((target.y + self.unk_28c.y) - self.actor.world_pos.y) * arg2;
        let dir = if t >= 0.0 { 1.0 } else { -1.0 };
        let t = t.abs().clamp(0.0, 20.0) * dir;
        step_to_f(&mut self.actor.velocity.y, t, 32.0);
    }

    /// `func_80A02C98`: towards `target` plus the drift, by `arg2` of the way, at most 20.
    fn func_80a02c98(&mut self, target: Vec3, arg2: f32) {
        let xt = ((target.x + self.unk_28c.x) - self.actor.world_pos.x) * arg2;
        let zt = ((target.z + self.unk_28c.z) - self.actor.world_pos.z) * arg2;
        let xd = if xt >= 0.0 { 1.0 } else { -1.0 };
        let zd = if zt >= 0.0 { 1.0 } else { -1.0 };
        let xt = xt.abs().clamp(0.0, 20.0) * xd;
        let zt = zt.abs().clamp(0.0, 20.0) * zd;
        self.func_80a02bd8(target, arg2);
        step_to_f(&mut self.actor.velocity.x, xt, 1.5);
        step_to_f(&mut self.actor.velocity.z, zt, 1.5);
        self.actor.update_pos();
    }

    /// `func_80A02E30`: onto `target` plus the drift in x and z at once.
    fn func_80a02e30(&mut self, target: Vec3) {
        self.func_80a02bd8(target, 0.2);
        self.actor.velocity.x = (target.x + self.unk_28c.x) - self.actor.world_pos.x;
        self.actor.velocity.z = (target.z + self.unk_28c.z) - self.actor.world_pos.z;
        self.actor.update_pos();
        self.actor.world_pos.x = target.x + self.unk_28c.x;
        self.actor.world_pos.z = target.z + self.unk_28c.z;
    }

    /// `func_80A02EC0`: onto `target` in x and z with no x/z velocity.
    fn func_80a02ec0(&mut self, target: Vec3) {
        self.func_80a02bd8(target, 0.2);
        self.actor.velocity.x = 0.0;
        self.actor.velocity.z = 0.0;
        self.actor.update_pos();
        self.actor.world_pos.x = target.x + self.unk_28c.x;
        self.actor.world_pos.z = target.z + self.unk_28c.z;
    }

    /// `func_80A02F2C`: the bob's vertical velocity.
    fn func_80a02f2c(&mut self, target: Vec3) {
        let t = (((sin_s(self.unk_2aa) * self.unk_2b4) + target.y) - self.actor.world_pos.y) * 0.2;
        let dir = if t >= 0.0 { 1.0 } else { -1.0 };
        self.unk_2aa = self.unk_2aa.wrapping_add(self.unk_2ae);
        let t = t.abs().clamp(0.0, 20.0) * dir;
        step_to_f(&mut self.actor.velocity.y, t, 1.5);
    }

    /// `func_80A03018`: the healing fairy's heading and forward move.
    fn func_80a03018(&mut self, play: &PlayState) {
        let Some(pl) = player(play) else { return };
        smooth_step_to_f(&mut self.actor.speed_xz, self.unk_2b8, 0.2, 0.5, 0.01);
        let p = self.actor.world_pos;
        let target_yaw = match self.unk_2a8 {
            0 => atan2_s(-(p.z - self.unk_28c.z), -(p.x - self.unk_28c.x)),
            3 => atan2_s(-(p.z - pl.pos.z), -(p.x - pl.pos.x)),
            2 => atan2_s(p.z - pl.pos.z, p.x - pl.pos.x),
            _ => self.unk_2b0,
        };
        smooth_step_to_s(&mut self.unk_2bc, target_yaw, 10, self.unk_2ac, 0x20);
        self.actor.world_rot.y = self.unk_2bc;
        self.actor.move_forward();
    }

    /// `func_80A03148`: towards `arg1` plus the drift by `arg4`, the x/z speed between `arg2` and
    /// `arg3` + 30.
    fn func_80a03148(&mut self, arg1: Vec3, arg2: f32, arg3: f32, arg4: f32) {
        let mut xt = ((arg1.x + self.unk_28c.x) - self.actor.world_pos.x) * arg4;
        let mut zt = ((arg1.z + self.unk_28c.z) - self.actor.world_pos.z) * arg4;
        let arg4 = arg4 + 0.3;
        let arg3 = arg3 + 30.0;
        self.func_80a02bd8(arg1, arg4);
        let mut xz = (xt * xt + zt * zt).sqrt();
        let clamped = xz.clamp(arg2, arg3);
        self.actor.speed_xz = clamped;
        if xz != clamped && xz != 0.0 {
            xz = clamped / xz;
            xt *= xz;
            zt *= xz;
        }
        step_to_f(&mut self.actor.velocity.x, xt, 5.0);
        step_to_f(&mut self.actor.velocity.z, zt, 5.0);
        self.actor.update_pos();
    }

    /// `func_80A0329C`: a healing fairy: wander, bob at Link's waist height, heal him when he's
    /// under it (`Health_ChangeBy(128)`), offer itself to a bottle, and vanish when timed.
    fn func_80a0329c(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        self.skel_update();
        if play.rand.zero_one() < 0.05 {
            self.unk_2b4 = play.rand.zero_float(10.0) + 10.0;
            self.unk_2ae = (play.rand.zero_float(1024.0) as i16).wrapping_add(0x200);
        }
        self.func_80a0232c(play);
        self.unk_28c.y = pl.waist.y;
        self.func_80a02f2c(self.unk_28c);
        self.func_80a03018(play);
        if self.unk_2a8 == 2 || self.unk_2a8 == 3 {
            self.spawn_sparkles(play, 16);
        }
        if self.actor.parent.is_some() {
            // Actor_HasParent: a bottle caught it.
            self.actor.kill();
            return;
        }
        if play.player_in_cs_mode() {
            return;
        }
        let height_diff = self.actor.world_pos.y - pl.pos.y;
        if height_diff > 0.0 && height_diff < 60.0 && !func_80a01f90(self.actor.world_pos, pl.pos, 10.0) {
            oot_game::item::health_change_by(&mut play.save, Some(&mut play.audio), 128);
            if self.fairy_flags & FAIRY_FLAG_BIG != 0 {
                log::warn!("En_Elf: Magic_Fill not ported (no magic meter)");
            }
            self.unk_2b8 = 50.0;
            self.unk_2ac = pl.shape_yaw;
            self.unk_2b0 = -0x1000;
            self.unk_28c.y = 30.0;
            self.unk_2b4 = 0.0;
            self.unk_2aa = 0;
            self.action = Action::Healing;
            return;
        }
        if self.fairy_flags & FAIRY_FLAG_TIMED != 0 {
            if self.disappear_timer > 0 {
                self.disappear_timer -= 1;
            } else {
                self.disappear_timer -= 1;
                if self.disappear_timer > -10 {
                    self.actor.scale = Vec3::splat(((self.disappear_timer + 10) as f32 * 0.008) * 0.1);
                } else {
                    self.actor.kill();
                    return;
                }
            }
        }
        if self.fairy_flags & FAIRY_FLAG_BIG == 0 {
            // GI_MAX: only a bottle can take it.
            let me = self.actor.clone();
            oot_game::get_item::offer_get_item_range(play, &me, GI_MAX, 80.0, 60.0);
        }
    }

    /// `func_80A0353C`: a Kokiri's fairy bobbing above its parent (killed with it).
    fn func_80a0353c(&mut self, play: &mut PlayState) {
        self.skel_update();
        self.func_80a02a20();
        match self.actor.parent.and_then(|h| play.actors.actor(h)).filter(|a| !a.killed) {
            Some(parent) => {
                let mut pos = parent.world_pos;
                pos.y += (1500.0 * self.actor.scale.y) + 40.0;
                self.func_80a02c98(pos, 0.2);
            }
            None => self.actor.kill(),
        }
        self.unk_2bc = atan2_s(self.actor.velocity.z, self.actor.velocity.x);
    }

    /// `func_80A03610`: after healing, circling Link and rising, then shrinking away.
    fn func_80a03610(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        self.skel_update();
        smooth_step_to_f(&mut self.unk_2b8, 30.0, 0.1, 4.0, 1.0);
        self.unk_28c.x = cos_s(self.unk_2ac) * self.unk_2b8;
        self.unk_28c.y += self.unk_2b4;
        match self.unk_2aa {
            0 => {
                if self.unk_2b4 < 2.0 {
                    self.unk_2b4 += 0.1;
                } else {
                    self.unk_2aa += 1;
                }
            }
            1 => {
                if self.unk_2b4 > -1.0 {
                    self.unk_2b4 -= 0.2;
                }
            }
            _ => {}
        }
        self.unk_28c.z = sin_s(self.unk_2ac) * -self.unk_2b8;
        self.unk_2ac = self.unk_2ac.wrapping_add(self.unk_2b0);
        self.func_80a02c98(pl.pos, 0.2);
        if self.unk_2b4 < 0.0 && self.unk_28c.y < 20.0 && self.unk_28c.y > 0.0 {
            self.actor.scale = Vec3::splat((self.unk_28c.y * 0.008) * 0.05);
        }
        if self.unk_28c.y < -10.0 {
            self.actor.kill();
            return;
        }
        self.unk_2bc = atan2_s(self.actor.velocity.z, self.actor.velocity.x);
        self.spawn_sparkles(play, 32);
        audio_play_actor_sfx2(play, NA_SE_EV_FIATY_HEAL - SFX_FLAG);
    }

    /// `func_80A03814`: a revival fairy circling Link's waist, then up and away.
    fn func_80a03814(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        self.skel_update();
        if self.unk_28c.y > 200.0 {
            self.actor.kill();
            return;
        }
        if self.unk_2ae >= 0x7E {
            self.unk_2b8 += 0.1;
            self.unk_2b4 += 0.5;
            self.unk_28c.y += self.unk_2b4;
        } else {
            self.unk_2ae += 1;
            if self.unk_2b8 < 30.0 {
                self.unk_2b8 += 0.5;
            }
            if self.unk_28c.y > 0.0 {
                self.unk_28c.y -= 0.7;
            }
        }
        self.unk_28c.x = cos_s(self.unk_2ac) * self.unk_2b8;
        self.unk_28c.z = sin_s(self.unk_2ac) * -self.unk_2b8;
        self.unk_2ac = self.unk_2ac.wrapping_add(self.unk_2b0);
        self.func_80a02e30(pl.waist);
        self.unk_2bc = atan2_s(self.actor.velocity.z, self.actor.velocity.x);
        self.spawn_sparkles(play, 32);
        audio_play_actor_sfx2(play, NA_SE_EV_FIATY_HEAL - SFX_FLAG);
    }

    /// `func_80A03990`: a revival fairy popping up out of Link, growing.
    fn func_80a03990(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        self.skel_update();
        self.unk_28c.z = 0.0;
        self.unk_28c.x = 0.0;
        self.unk_28c.y += self.unk_2b4;
        self.unk_2b4 -= 0.35;
        if self.unk_2b4 <= 0.0 {
            self.action = Action::ReviveCircle;
            self.unk_2b0 = 0x800;
            self.unk_2ae = 0;
            self.unk_2b4 = 0.0;
            self.unk_2b8 = 1.0;
        }
        self.func_80a02e30(pl.waist);
        self.actor.scale = Vec3::splat((1.0 - (self.unk_2b4 * self.unk_2b4 * ((1.0 / 9.0) * (1.0 / 9.0)))) * 0.008);
        self.unk_2bc = atan2_s(self.actor.velocity.z, self.actor.velocity.x);
        self.spawn_sparkles(play, 32);
        audio_play_actor_sfx2(play, NA_SE_EV_FIATY_HEAL - SFX_FLAG);
    }

    /// `func_80A03AB0`: Navi's colours (`func_80A04414`), the animation, and the drift.
    fn func_80a03ab0(&mut self, play: &mut PlayState) {
        if self.fairy_flags & 4 != 0 {
            self.func_80a04414(play);
        }
        self.skel_update();
        self.drift(play);
    }

    /// `EnElf_UpdateLights`: the glow light at the fairy (radius 100, 0 while in Link's hat),
    /// the other at her or, while she talks, above Link.
    fn update_lights(&mut self, play: &mut PlayState) {
        let glow_radius = if self.unk_2a8 == 8 { 0 } else { 100 };
        let p = self.actor.world_pos;
        let no_glow = if self.fairy_flags & 0x20 != 0 {
            let pp = player(play).map(|pl| pl.pos).unwrap_or(p);
            LightInfo::point_no_glow(pp.x as i16, ((pp.y as i16) as f32 + 60.0) as i16, pp.z as i16, [255, 255, 255], 200)
        } else {
            LightInfo::point_no_glow(p.x as i16, p.y as i16, p.z as i16, [255, 255, 255], -1)
        };
        play.light_ctx.set_info(self.light_no_glow, no_glow);
        play.light_ctx.set_info(self.light_glow, LightInfo::point_glow(p.x as i16, p.y as i16, p.z as i16, [255, 255, 255], glow_radius));
        self.unk_2bc = atan2_s(self.actor.velocity.z, self.actor.velocity.x);
        // Actor_SetScale(&actor, actor.scale.x).
        self.actor.scale = Vec3::splat(self.actor.scale.x);
    }

    /// `EnElf_GetCuePos`: the cue in `npcActions[action]` at this frame.
    fn cutscene_next_pos(play: &PlayState, slot: usize) -> Option<Vec3> {
        let a = play.cs_ctx.npc_actions[slot]?;
        let (s, e) = (a.start_pos.as_vec3(), a.end_pos.as_vec3());
        let lerp = oot_game::env::lerp_weight(a.end_frame, a.start_frame, play.cs_ctx.frames);
        Some(Vec3::new(((e.x - s.x) * lerp) + s.x, ((e.y - s.y) * lerp) + s.y, ((e.z - s.z) * lerp) + s.z))
    }

    /// `func_80A03CF8`: Navi.
    fn func_80a03cf8(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        self.func_80a0461c(play);
        self.func_80a03ab0(play);
        let mut x_scale = 0.0;
        if play.cs_ctx.state != CS_STATE_IDLE && play.cs_ctx.npc_actions[8].is_some() {
            let next = Self::cutscene_next_pos(play, 8).unwrap_or(self.actor.world_pos);
            if play.cs_ctx.npc_actions[8].is_some_and(|a| a.action == 5) {
                self.spawn_sparkles(play, 16);
            }
            let prev = self.actor.world_pos;
            if self.unk_2a8 == 0xA {
                self.func_80a02ec0(next);
            } else {
                self.func_80a02c98(next, 0.2);
            }
            if play.scene_id == SCENE_LINKS_HOUSE && play.save.scene_layer == 4 {
                // The dash as she comes into Link's house in the opening, and each time she
                // swoops on him.
                if play.cs_ctx.frames == 55 {
                    audio_play_actor_sfx2(play, NA_SE_EV_FAIRY_DASH);
                }
                if self.unk_2a8 == 6 {
                    if self.fairy_flags & 0x40 != 0 {
                        if prev.y < self.actor.world_pos.y {
                            self.fairy_flags &= !0x40;
                        }
                    } else if self.actor.world_pos.y < prev.y {
                        self.fairy_flags |= 0x40;
                        audio_play_actor_sfx2(play, NA_SE_EV_FAIRY_DASH);
                    }
                }
            }
        } else {
            let hat = pl.hat;
            let dist_hat = hat.distance(self.actor.world_pos);
            match self.unk_2a8 {
                7 => {
                    self.func_80a02c98(hat, 1.0 - self.unk_2ae as f32 * (1.0 / 30.0));
                    x_scale = hat.distance(self.actor.world_pos);
                    if dist_hat < 7.0 {
                        self.unk_2c0 = 0;
                        x_scale = 0.0;
                    } else if dist_hat < 25.0 {
                        x_scale = (x_scale - 5.0) * 0.05;
                        x_scale = 1.0 - x_scale;
                        x_scale = (1.0 - x_scale * x_scale) * 0.008;
                    } else {
                        x_scale = 0.008;
                    }
                    self.spawn_sparkles(play, 16);
                }
                8 => {
                    self.func_80a02c98(hat, 0.2);
                    self.actor.world_pos = hat;
                    self.func_80a029a8(1);
                }
                11 => {
                    let mut next = hat;
                    next.y += 1500.0 * self.actor.scale.y;
                    self.func_80a02e30(next);
                    self.spawn_sparkles(play, 16);
                    if self.unk_2b8 <= 19.0 {
                        self.unk_2b8 += 1.0;
                    }
                    if self.unk_2b8 >= 21.0 {
                        self.unk_2b8 -= 1.0;
                    }
                    if self.unk_2c0 < 0x20 {
                        self.unk_2b0 = (self.unk_2c0 as i32 * 0xF0 + 0x200) as i16;
                    }
                }
                12 => {
                    let mut next = play.active_camera().eye;
                    next.y += -2000.0 * self.actor.scale.y;
                    self.func_80a03148(next, 0.0, 20.0, 0.2);
                }
                _ => {
                    self.func_80a029a8(1);
                    let mut next = play.target_ctx.navi_ref_pos;
                    next.y += 1500.0 * self.actor.scale.y;
                    if play.target_ctx.arrow_pointed.is_some() {
                        self.func_80a03148(next, 0.0, 20.0, 0.2);
                        if self.actor.speed_xz >= 5.0 {
                            self.spawn_sparkles(play, 16);
                        }
                    } else {
                        if self.timer % 32 == 0 {
                            self.unk_2a0 = play.rand.zero_float(7.0) + 3.0;
                        }
                        if self.fairy_flags & 2 != 0 {
                            if dist_hat < 30.0 {
                                self.fairy_flags ^= 2;
                            }
                            self.func_80a03148(next, 0.0, 20.0, 0.2);
                            self.spawn_sparkles(play, 16);
                        } else {
                            if dist_hat > 100.0 {
                                self.fairy_flags |= 2;
                                if self.unk_2c7 == 0 {
                                    audio_play_actor_sfx2(play, NA_SE_EV_FAIRY_DASH);
                                }
                                self.unk_2c0 = 0x64;
                            }
                            self.func_80a03148(next, 0.0, self.unk_2a0, 0.2);
                        }
                    }
                }
            }
        }
        if self.unk_2a8 == 7 {
            self.actor.scale.x = x_scale;
        } else if self.unk_2a8 == 8 {
            self.actor.scale.x = 0.0;
        } else {
            smooth_step_to_f(&mut self.actor.scale.x, 0.008, 0.3, 0.000_800_000_04, 0.000_080_000_005);
        }
        self.update_lights(play);
    }

    /// `func_80A029A8`: `disappearTimer` back up towards 600.
    fn func_80a029a8(&mut self, increment: i16) {
        if self.disappear_timer < 600 {
            self.disappear_timer += increment;
        }
    }

    /// `func_80A04414`: Navi's colours: at once to white when the pointed actor changes
    /// (`naviMoveProgressFactor`), then over four frames to the target context's once she's there.
    fn func_80a04414(&mut self, play: &mut PlayState) {
        let arrow = play.target_ctx.arrow_pointed;
        let (inner, outer, navi_ref) = (play.target_ctx.navi_inner, play.target_ctx.navi_outer, play.target_ctx.navi_ref_pos);
        if play.target_ctx.navi_move_progress_factor != 0.0 {
            self.unk_2c6 = 0;
            self.unk_29c = 1.0;
            if self.unk_2c7 == 0 {
                audio_play_actor_sfx2(play, NA_SE_EV_FAIRY_DASH);
            }
        } else if self.unk_2c6 == 0 {
            if arrow.is_none() || self.actor.world_pos.distance(navi_ref) < 50.0 {
                self.unk_2c6 = 1;
            }
        } else if self.unk_29c != 0.0 {
            if step_to_f(&mut self.unk_29c, 0.0, 0.25) {
                self.inner_color = inner;
                self.outer_color = outer;
            } else {
                let rate = 0.25 / self.unk_29c;
                change_color(&mut self.inner_color, inner, rate);
                change_color(&mut self.outer_color, outer, rate);
            }
        }
        let target = player(play).and_then(|p| p.target);
        if self.fairy_flags & 1 != 0 {
            if arrow.is_none() || target.is_none() {
                self.fairy_flags ^= 1;
            }
        } else if let Some(a) = arrow.and_then(|h| play.actors.actor(h))
            && target.is_some()
        {
            let sfx_id = if a.category == ACTORCAT_NPC {
                NA_SE_VO_NAVY_HELLO
            } else if a.category == ACTORCAT_ENEMY {
                NA_SE_VO_NAVY_ENEMY
            } else {
                NA_SE_VO_NAVY_HEAR
            };
            if self.unk_2c7 == 0 {
                audio_play_actor_sfx2(play, sfx_id);
            }
            self.fairy_flags |= 1;
        }
    }

    /// `func_80A0461C`: which mode Navi is in (`unk_2A8`): her cue's in a cutscene; otherwise
    /// out after the pointed actor, into Link's hat when nothing's around (7, then 8), out again
    /// (11), or at the camera (12) in first person and in a shop.
    fn func_80a0461c(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        let mut temp: i16;
        if play.cs_ctx.state != CS_STATE_IDLE {
            match play.cs_ctx.npc_actions[8] {
                Some(a) => {
                    temp = match a.action {
                        4 => 9,
                        3 => 6,
                        1 => 10,
                        _ => 0,
                    };
                }
                None => {
                    temp = 0;
                    self.unk_2c0 = 100;
                }
            }
        } else {
            let arrow = play.target_ctx.arrow_pointed.and_then(|h| play.actors.actor(h).map(|a| a.category));
            // R_SCENE_CAM_TYPE & 0x10 with VIEWPOINT_PIVOT: only the shop kind is used with it.
            if pl.state1 & PLAYER_STATE1_10 != 0 || ((play.scene_cam_type & 0x10) != 0 && play.viewpoint == VIEWPOINT_PIVOT) {
                temp = 12;
                self.unk_2c0 = 100;
            } else if arrow.is_none() || arrow == Some(ACTORCAT_NPC) {
                if arrow.is_some() {
                    self.unk_2c0 = 100;
                    change_player_state2(play, PLAYER_STATE2_NAVI_ACTIVE, 0);
                    temp = 0;
                } else {
                    temp = match self.unk_2a8 {
                        0 => {
                            if self.unk_2c0 != 0 {
                                self.unk_2c0 -= 1;
                                0
                            } else {
                                if self.unk_2c7 == 0 {
                                    audio_play_actor_sfx2(play, NA_SE_EV_NAVY_VANISH);
                                }
                                7
                            }
                        }
                        7 => {
                            if self.unk_2c0 != 0 {
                                if self.unk_2ae > 0 {
                                    self.unk_2ae -= 1;
                                    7
                                } else {
                                    change_player_state2(play, PLAYER_STATE2_NAVI_ACTIVE, 0);
                                    0
                                }
                            } else {
                                self.func_80a029a8(10);
                                8
                            }
                        }
                        8 => 8,
                        11 => {
                            if self.unk_2c0 > 0 {
                                self.unk_2c0 -= 1;
                                11
                            } else {
                                0
                            }
                        }
                        _ => 0,
                    };
                }
            } else {
                temp = 1;
            }
            let state2 = player(play).map(|p| p.state2).unwrap_or(0);
            match temp {
                0 => {
                    if state2 & PLAYER_STATE2_NAVI_ACTIVE == 0 {
                        temp = 7;
                        if self.unk_2c7 == 0 {
                            audio_play_actor_sfx2(play, NA_SE_EV_NAVY_VANISH);
                        }
                    }
                }
                8 => {
                    if state2 & PLAYER_STATE2_NAVI_ACTIVE != 0 {
                        self.unk_2c0 = 42;
                        temp = 11;
                        if self.unk_2c7 == 0 {
                            audio_play_actor_sfx2(play, NA_SE_EV_FAIRY_DASH);
                        }
                    }
                }
                7 => change_player_state2(play, 0, PLAYER_STATE2_NAVI_ACTIVE),
                _ => change_player_state2(play, PLAYER_STATE2_NAVI_ACTIVE, 0),
            }
        }
        if temp != self.unk_2a8 {
            self.func_80a01c38(temp);
            if temp == 11 {
                self.unk_2b8 = dist_xz(pl.hat, self.actor.world_pos);
                self.unk_2ac = oot_game::target::yaw_to(self.actor.world_pos, pl.hat);
            }
        }
    }

    /// `EnElf_SpawnSparkles`: `EffectSsKiraKira_SpawnDispersed` isn't ported; its and this
    /// function's `Rand` calls are made.
    fn spawn_sparkles(&mut self, play: &mut PlayState, _life: i32) {
        let r = &mut play.rand;
        let _pos = Vec3::new(r.centered_float(6.0), r.zero_one() * 6.0, r.centered_float(6.0)) + self.actor.world_pos;
        // EffectSsKiraKira_SpawnDispersed: the velocity's and the acceleration's y, the yaw.
        let (_vy, _ay, _yaw) = (r.zero_one(), r.zero_one(), r.zero_one());
    }

    /// `func_80A04D90`: the floor below (`BgCheck_EntityRaycastDown5`), and `shadowAlpha` 50.
    fn func_80a04d90(&mut self, play: &PlayState) {
        let (floor, poly) = play.col.entity_raycast_down(self.actor.world_pos);
        self.actor.floor_height = floor;
        self.actor.floor_poly = poly;
        self.shadow_alpha = 50;
    }

    /// `func_80A04DE4`: Navi flies to talk to Link: in front of his head, unless she has a
    /// target to talk about.
    fn func_80a04de4(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        if self.fairy_flags & 0x10 != 0 {
            let mut navi_ref = play.target_ctx.navi_ref_pos;
            let me = play.cur_actor;
            if pl.target.is_none() || pl.target == Some(pl.handle) || pl.target == me {
                navi_ref.x = pl.head.x + (sin_s(pl.shape_yaw) * 20.0);
                navi_ref.y = pl.head.y + 5.0;
                navi_ref.z = pl.head.z + (cos_s(pl.shape_yaw) * 20.0);
            }
            self.actor.focus_pos = navi_ref;
            self.fairy_flags &= !0x10;
        }
        self.func_80a03ab0(play);
        let head_copy = self.actor.focus_pos;
        self.func_80a03148(head_copy, 0.0, 20.0, 0.2);
        if self.actor.speed_xz >= 5.0 {
            self.spawn_sparkles(play, 16);
        }
        smooth_step_to_f(&mut self.actor.scale.x, 0.008, 0.3, 0.000_800_000_04, 0.000_080_000_005);
        self.update_lights(play);
    }

    /// `func_80A04F94`: turning to face her heading, and the scene darkening
    /// (`Environment_AdjustLights`, not ported).
    fn func_80a04f94(&mut self) {
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.unk_2bc, 5, 0x1000, 0x400);
        self.timer = self.timer.wrapping_add(1);
        step_to_f(&mut self.unk_2a4, 1.0, 0.05);
    }

    /// Her talk's end: back to `func_80A053F0`, mode 0, the light off Link.
    fn end_talk(&mut self) {
        self.update_fn = Update::Navi;
        self.func_80a01c38(0);
        self.fairy_flags &= !0x20;
    }

    /// `func_80A052F4`: her text; a choice to talk to Saria (0xE2 on yes, 0xE1 on no), or the
    /// text's end.
    fn func_80a052f4(&mut self, play: &mut PlayState) {
        use oot_game::message::{TEXT_STATE_CHOICE};
        self.func_80a04de4(play);
        if play.message_state() == TEXT_STATE_CHOICE {
            if play.message_should_advance() {
                // (msgCtx.unk_E3F2 = 0xFF: the ocarina's, not ported.)
                if play.msg_ctx.choice_index == 0 {
                    self.update_fn = Update::SariaText;
                    play.continue_textbox(0xE2);
                } else {
                    self.update_fn = Update::AskNavi;
                    play.continue_textbox(0xE1);
                }
            }
        } else if oot_game::npc::textbox_is_closing(play) {
            self.end_talk();
        }
        self.func_80a04f94();
    }

    /// `func_80A05208`: "talk to Navi?": yes, her C-Up text (0x15F without one); no, the end.
    fn func_80a05208(&mut self, play: &mut PlayState) {
        use oot_game::message::{TEXT_STATE_CHOICE};
        self.func_80a04de4(play);
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            if play.msg_ctx.choice_index == 0 {
                let t = play.elf_message_get_c_up_text();
                play.continue_textbox(if t != 0 { t } else { 0x15F });
                self.update_fn = Update::Talk;
            } else {
                play.with_msg(|m, f| m.close_textbox(f.audio));
                self.end_talk();
            }
        }
        self.func_80a04f94();
    }

    /// `func_80A05188`: after 0xE2, Saria's text.
    fn func_80a05188(&mut self, play: &mut PlayState) {
        use oot_game::message::{TEXT_STATE_EVENT};
        self.func_80a04de4(play);
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            let t = play.elf_message_get_saria_text();
            play.continue_textbox(t);
            self.update_fn = Update::SariaMore;
        }
        self.func_80a04f94();
    }

    /// `func_80A05114`: after Saria's text, 0xE3 ("talk to her again?").
    fn func_80a05114(&mut self, play: &mut PlayState) {
        use oot_game::message::{TEXT_STATE_EVENT};
        self.func_80a04de4(play);
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.continue_textbox(0xE3);
            self.update_fn = Update::SariaAgain;
        }
        self.func_80a04f94();
    }

    /// `func_80A05040`: talk to Saria again: yes, her text again; no, the end.
    fn func_80a05040(&mut self, play: &mut PlayState) {
        use oot_game::message::{TEXT_STATE_CHOICE};
        self.func_80a04de4(play);
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            if play.msg_ctx.choice_index == 0 {
                let t = play.elf_message_get_saria_text();
                play.continue_textbox(t);
                self.update_fn = Update::SariaMore;
            } else {
                play.with_msg(|m, f| m.close_textbox(f.audio));
                self.end_talk();
            }
        }
        self.func_80a04f94();
    }

    /// `func_80A053F0`: Navi's update. Her C-Up text while `naviTimer` allows it; Player's talk
    /// to her; else her action, and `naviTimer` counting outside cutscenes.
    fn func_80a053f0(&mut self, play: &mut PlayState) {
        let Some(pl) = player(play) else { return };
        if pl.navi_text_id == 0 {
            if pl.target.is_none() && play.save.navi_timer >= 600 && play.save.navi_timer <= 3000 {
                // (Or nREG(89), a debug register: 0.)
                let mut t = play.elf_message_get_c_up_text() as i16;
                if t == 0x15F {
                    t = 0;
                }
                set_navi_text_id(play, t);
            }
        } else if pl.navi_text_id < 0 {
            // A negative id: the talk starts at once.
            self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
        }
        if oot_game::npc::process_talk_request(&mut self.actor) {
            play.audio.func_800f4524(SfxPos::Default, NA_SE_VO_SK_LAUGH, 0x20);
            self.actor.focus_pos = self.actor.world_pos;
            if self.actor.text_id == play.elf_message_get_c_up_text() {
                self.fairy_flags |= 0x80;
                play.save.navi_timer = 3001;
            }
            self.fairy_flags |= 0x10;
            self.fairy_flags |= 0x20;
            self.update_fn = Update::Talk;
            self.func_80a01c38(3);
            // (elfMsg->actor.flags |= ACTOR_FLAG_TALK: no Elf_Msg.)
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
        } else {
            self.run_action(play);
            self.actor.shape_rot.y = self.unk_2bc;
            // (nREG(80) = HIGH_SCORE(HS_HBA): a debug register.)
            if !play.play_in_cs_mode() {
                if play.save.navi_timer < 25800 {
                    play.save.navi_timer += 1;
                } else if self.fairy_flags & 0x80 == 0 {
                    play.save.navi_timer = 0;
                }
            }
        }
        self.timer = self.timer.wrapping_add(1);
        if self.unk_2a4 > 0.0 {
            step_to_f(&mut self.unk_2a4, 0.0, 0.05);
            // Environment_AdjustLights: not ported.
        }
        if self.unk_2c7 > 0 {
            self.unk_2c7 -= 1;
        }
        if self.unk_2c7 == 0 && play.cs_ctx.state != CS_STATE_IDLE {
            self.unk_2c7 = 1;
        }
        self.func_80a04d90(play);
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Navi => self.func_80a03cf8(play),
            Action::Heal => self.func_80a0329c(play),
            Action::Healing => self.func_80a03610(play),
            Action::Revive => self.func_80a03990(play),
            Action::ReviveCircle => self.func_80a03814(play),
            Action::Kokiri => self.func_80a0353c(play),
            Action::Spawner => {}
        }
    }

    /// `EnElf_Update`.
    fn en_elf_update(&mut self, play: &mut PlayState) {
        self.run_action(play);
        self.actor.shape_rot.y = self.unk_2bc;
        self.timer = self.timer.wrapping_add(1);
        if self.fairy_flags & FAIRY_FLAG_BIG != 0 {
            self.func_80a04d90(play);
        }
    }

    /// The limb matrices `EnElf_Draw` draws with: the pose, with limb 8 put at its parent's
    /// origin in the world, unrotated and scaled by `scale` (`EnElf_OverrideLimbDraw`:
    /// `Matrix_Translate(MTXMODE_NEW)` and `Matrix_Scale`, before the limb's own transform), and
    /// for a big fairy no wings (limbs 4, 7, 11, 14: their lists set to NULL; here, collapsed).
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], model: Mat4, scale: f32, big: bool) -> Vec<Mat4> {
        let plain = skeleton.pose_override(joints, |_, _, _| Mat4::IDENTITY);
        // limbIndex 8 is the 0-based limb 7.
        let glow = 7usize;
        let parent = skeleton.parents.get(glow).copied().flatten().map(|p| plain[p as usize]).unwrap_or(Mat4::IDENTITY);
        let world = (model * parent).transform_point3(Vec3::ZERO);
        let replaced = model.inverse() * Mat4::from_translation(world) * Mat4::from_scale(Vec3::splat(scale));
        let pre = parent.inverse() * replaced;
        let mut bones = skeleton.pose_override(joints, |limb, _, _| if limb == 8 { pre } else { Mat4::IDENTITY });
        if big {
            for l in [4usize, 7, 11, 14] {
                if let Some(b) = bones.get_mut(l - 1) {
                    *b = Mat4::ZERO;
                }
            }
        }
        bones
    }
}

/// `EnElf_GetColorValue`.
fn color_value(play: &mut PlayState, flag: u8) -> f32 {
    match flag {
        1 => play.rand.zero_float(55.0) + 200.0,
        2 => play.rand.zero_float(255.0),
        _ => 0.0,
    }
}

/// `EnElf_ChangeColor(dest, new, dest, rate)`.
fn change_color(dest: &mut [f32; 4], new: [f32; 4], rate: f32) {
    let diff = [new[0] - dest[0], new[1] - dest[1], new[2] - dest[2], new[3] - dest[3]];
    for i in 0..4 {
        dest[i] += diff[i] * rate;
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `values`: the inner colour (4), the outer colour (4), `scale.x`.
    pub const INNER: usize = 0;
    pub const OUTER: usize = 4;
    pub const SCALE_X: usize = 8;
    /// `switches`: `timer`, `disappearTimer` (as u16), `fairyFlags`, drawn (the mode isn't 8 and
    /// `fairyFlags & 8` is clear).
    pub const TIMER: usize = 0;
    pub const DISAPPEAR: usize = 1;
    pub const FLAGS: usize = 2;
    pub const DRAWN: usize = 3;
}

impl ActorImpl for EnElf {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, play: &mut PlayState) {
        match self.update_fn {
            Update::Normal => self.en_elf_update(play),
            Update::Navi => self.func_80a053f0(play),
            Update::Talk => self.func_80a052f4(play),
            Update::AskNavi => self.func_80a05208(play),
            Update::SariaText => self.func_80a05188(play),
            Update::SariaMore => self.func_80a05114(play),
            Update::SariaAgain => self.func_80a05040(play),
        }
    }
    /// `EnElf_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.light_ctx.remove_light(self.light_glow.take());
        play.light_ctx.remove_light(self.light_no_glow.take());
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if let Some(s) = &self.skel {
            rs.joints = Some(eng_anim::anim::JointTable { rot: s.joint_table.clone(), face: 0 });
        }
        rs.values = self.inner_color.iter().chain(self.outer_color.iter()).copied().chain([self.actor.scale.x]).collect();
        rs.switches = vec![self.timer as u32, self.disappear_timer as u16 as u32, self.fairy_flags as u32, (self.unk_2a8 != 8 && self.fairy_flags & 8 == 0) as u32];
        rs
    }
    /// `EnElf_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) else { return };
        if rs.values.len() < 9 || rs.switches.len() < 4 || rs.switches[rs::DRAWN] == 0 {
            return;
        }
        // In first person (PLAYER_STATE1_20) only in front of the eye (kREG(90) is 0).
        let first_person = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).is_some_and(|p| p.state_flags1() & PLAYER_STATE1_20 != 0);
        let projected_z = (play.view_proj * rs.pos.extend(1.0)).z;
        if first_person && !(0.0 < projected_z) {
            return;
        }
        let (timer, flags) = (rs.switches[rs::TIMER] as u16, rs.switches[rs::FLAGS] as u16);
        let disappear = rs.switches[rs::DISAPPEAR] as u16 as i16;
        let mut env_alpha = ((timer as i32) * 50) & 0x1FF;
        if env_alpha > 255 {
            env_alpha = 511 - env_alpha;
        }
        let alpha_scale = if disappear < 0 { (disappear as f32 * (7.0 / 6000.0)) + 1.0 } else { 1.0 };
        let v = &rs.values;
        let inner = [v[rs::INNER] as u8, v[rs::INNER + 1] as u8, v[rs::INNER + 2] as u8, (v[rs::INNER + 3] * alpha_scale) as u8];
        let outer = [v[rs::OUTER] as u8, v[rs::OUTER + 1] as u8, v[rs::OUTER + 2] as u8, (env_alpha as f32 * alpha_scale) as u8];
        let mut sv = SegmentValues::default();
        sv.prim[SEG_LIST as usize] = Some(inner);
        sv.env[SEG_ENV as usize] = Some(outer);
        // EnElf_OverrideLimbDraw's limb 8 scale.
        let big = flags & FAIRY_FLAG_BIG != 0;
        let mut scale = ((sin_s((timer as i32).wrapping_mul(4096) as i16) * 0.1) + 1.0) * 0.012;
        if big {
            scale *= 2.0;
        }
        scale *= v[rs::SCALE_X] * 124.99999;
        let model = oot_game::play::actor_draw_matrix(rs);
        let mut bones = Self::pose(skeleton, &joints.rot, model, scale, big);
        // gGlowCircleSmallDL's gSPMatrix(0x01000000, G_MTX_MUL): the billboard after limb 8's.
        if let Some(b) = bones.get_mut(7) {
            *b *= view.billboard;
        }
        let bake = if flags & 4 != 0 { BAKE_NAVI } else { BAKE_FAIRY };
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: model, bones, params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } });
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
