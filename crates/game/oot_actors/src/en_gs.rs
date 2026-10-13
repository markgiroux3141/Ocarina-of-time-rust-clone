//! `En_Gs` (`ovl_En_Gs/z_en_gs.c`): a gossip stone. `params & 0xFF` is its hint (text 0x400 +
//! it), `(params >> 8) & 0x3F` the switch flag a song sets.
//!
//! It talks (`func_80A4E648`): "a strange stone" (0x2053), or with the Mask of Truth on, its hint
//! (0x2054, then the hint). Played a song beside it (`func_80A4E470`, the ocarina's free play) it
//! gives a fairy: a timed one for Saria's, Epona's, Zelda's Lullaby, the Sun's and Time's songs,
//! a big one for the Song of Storms; then its switch flag is set and it gives no more.
//!
//! Struck, it reacts by the damage table's reaction (`EnGs_Update`): the sword's wobble
//! (`func_80A4E910`, then "it doesn't seem to want to talk" 0x71B1), the slingshot's, arrows' and
//! hookshot's twist (`func_80A4EA08`), the hammer's squash (`func_80A4EB3C`), magic's spin and
//! stretch (`func_80A4F13C`); a bomb's countdown, flashing blue then red, and launch into the sky
//! (`func_80A4ED34`) until it hits something and explodes (`func_80A4F700`).
//!
//! Drawn as `EnGs_Draw` does: turned and scaled by the reactions, the flash's prim colour, and
//! the launch's flame (`gEffFire1DL`); from baked meshes (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the launch's explosion (`EffectSsBomb2_SpawnLayered`: the effect isn't
//! ported; its sound plays).

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::smooth_step_to_f;
use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor, BGCHECKFLAG_CEILING, BGCHECKFLAG_WALL, UPDBGCHECKINFO_FLAG_0,
    UPDBGCHECKINFO_FLAG_1,
};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_EN_STALKID_ATTACK, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::message::TEXT_STATE_DONE;
use oot_game::npc::{offer_talk, process_talk_request};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::gfx_two_tex_scroll;

use crate::en_elf::{ACTOR_EN_ELF, FAIRY_HEAL_BIG, FAIRY_HEAL_TIMED};

/// `ACTOR_EN_GS` (`actor_table.h`: 0x01B9).
pub const ACTOR_EN_GS: i16 = 0x01B9;
pub const OBJECT: &str = "object_gs";

/// `En_Gs_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_GS, name: "En_Gs", category: ACTORCAT_PROP, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_DURING_OCARINA, object: OBJECT };

// Sound effects (`sfx/*_table.h`).
const NA_SE_EV_BUTTERFRY_TO_FAIRY: u16 = 0x28E7;
const NA_SE_SY_WARNING_COUNT_N: u16 = 0x4819;
const NA_SE_SY_WARNING_COUNT_E: u16 = 0x481A;
const NA_SE_EV_FIRE_PILLAR: u16 = 0x2833;
const NA_SE_IT_BOMB_EXPLOSION: u16 = 0x180E;
const NA_SE_EV_STONE_GROW_UP: u16 = 0x28E9;
const NA_SE_EV_STONE_LAUNCH: u16 = 0x28EA;
const NA_SE_EV_STONE_ROLLING: u16 = 0x28EB;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `PLAYER_STATE2_23` (near an ocarina listener), `PLAYER_STATE2_24` (the ocarina out).
const PLAYER_STATE2_23: u32 = 1 << 23;
const PLAYER_STATE2_24: u32 = 1 << 24;
/// `PLAYER_MASK_TRUTH` (`player.h`).
const PLAYER_MASK_TRUTH: u8 = 8;
/// `OcarinaMode` (`ocarina.h`).
const OCARINA_MODE_01: u16 = 0x01;
const OCARINA_MODE_04: u16 = 0x04;
// `OcarinaSongId` (`ocarina.h`).
const OCARINA_SONG_SARIAS: u16 = 6;
const OCARINA_SONG_EPONAS: u16 = 7;
const OCARINA_SONG_LULLABY: u16 = 8;
const OCARINA_SONG_SUNS: u16 = 9;
const OCARINA_SONG_TIME: u16 = 10;
const OCARINA_SONG_STORMS: u16 = 11;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit {
        col_type: COL_MATERIAL_HARD,
        at_flags: AT_NONE,
        ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER,
        oc_flags1: OC1_ON | OC1_TYPE_ALL,
        oc_flags2: OC2_TYPE_2,
        shape: COLSHAPE_CYLINDER,
    },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 21, height: 48, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `sDamageTable`: no damage; the reactions pick the stone's response (`EnGs_Update`).
pub static DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x0), // Deku nut
        dmg_entry(0, 0x0), // Deku stick
        dmg_entry(0, 0xE), // Slingshot
        dmg_entry(0, 0xC), // Explosive
        dmg_entry(0, 0xE), // Boomerang
        dmg_entry(0, 0xE), // Normal arrow
        dmg_entry(0, 0xD), // Hammer swing
        dmg_entry(0, 0xE), // Hookshot
        dmg_entry(0, 0xF), // Kokiri sword
        dmg_entry(0, 0xF), // Master sword
        dmg_entry(0, 0xF), // Giant's Knife
        dmg_entry(0, 0xB), // Fire arrow
        dmg_entry(0, 0xB), // Ice arrow
        dmg_entry(0, 0xB), // Light arrow
        dmg_entry(0, 0x0), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(0, 0xB), // Fire magic
        dmg_entry(0, 0xB), // Ice magic
        dmg_entry(0, 0xB), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(0, 0x0), // Kokiri spin
        dmg_entry(0, 0x0), // Giant spin
        dmg_entry(0, 0x0), // Master spin
        dmg_entry(0, 0x0), // Kokiri jump
        dmg_entry(0, 0x0), // Giant jump
        dmg_entry(0, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(0, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// The stone: `gGossipStoneMaterialDL` (bound on segment 9 and run first), the draw's prim
/// colour (segment 8), then `gGossipStoneDL` and `gGossipStoneSquishedDL`.
pub const BAKE: &str = "En_Gs/stone";
/// The launch's flame: the colours, `gEffFire1DL` under segment 8's scroll.
pub const FIRE_BAKE: &str = "En_Gs/fire";
const SEG_PRIM: u8 = 0x08;
const SEG_MATERIAL: u8 = 0x09;
const SEG_SCROLL: u8 = 0x08;
const SEG_COLORS: u8 = 0x0C;

/// `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, 0, 0, 0x20, 0x40, 1, 0, -frames * 0x14, 0x20, 0x80)`.
fn fire_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 0x20, 0x40, 1, 0, frames.wrapping_mul(0x14).wrapping_neg(), 0x20, 0x80)
}

/// The meshes `EnGs_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake {
            name: BAKE.into(),
            object: OBJECT.into(),
            segments: vec![
                // The material list as data on segment 9, run as the prelude, so the draw's prim
                // colour (segment 8, after it) is the one the stone's lists take.
                (SEG_MATERIAL, BakeSegment::Texture { file: OBJECT.into(), symbol: "gGossipStoneMaterialDL".into() }),
                (SEG_PRIM, BakeSegment::DynamicColor { env: false, prim: true }),
            ],
            prelude: vec![SEG_MATERIAL, SEG_PRIM],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gGossipStoneDL".into()), (OBJECT.into(), "gGossipStoneSquishedDL".into())]),
        },
        MeshBake {
            name: FIRE_BAKE.into(),
            object: OBJECT.into(),
            // gDPSetPrimColor(128, 128, 255, 255, 0, 255), gDPSetEnvColor(255, 0, 0, 0).
            segments: vec![(SEG_COLORS, BakeSegment::Commands(vec![(0xFA00_8080, 0xFFFF_00FF), (0xFB00_0000, 0xFF00_0000), (0xDF00_0000, 0)])), (SEG_SCROLL, BakeSegment::Dynamic(fire_scroll(0)))],
            prelude: vec![SEG_COLORS],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gEffFire1DL".into())]),
        },
    ]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The sword's wobble.
    Func80A4E910,
    /// The twist.
    Func80A4EA08,
    /// The hammer's squash.
    Func80A4EB3C,
    /// The bomb's countdown and launch.
    Func80A4ED34,
    /// Magic's spin.
    Func80A4F13C,
    /// Exploded, gone in 10 frames.
    Func80A4F700,
    /// Waiting for a song.
    Func80A4F734,
}

pub struct EnGs {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub action: Action,
    /// `unk_19C`: the talk's state (0 none, 1 continuing to the hint, 2 talking, 3 struck).
    pub unk_19c: u8,
    /// `unk_19D`: bit 0, the ocarina started.
    pub unk_19d: u8,
    /// `unk_19E`: 1 turned, 2 the flame, 4 flashing, 8 exploded (not drawn), 0x10 launched.
    pub unk_19e: u8,
    /// `unk_19F`: the reaction's step.
    pub unk_19f: u8,
    pub unk_1a0: [[i16; 3]; 3],
    pub unk_1b4: [Vec3; 2],
    pub unk_1d8: Vec3,
    pub flash_color: [u8; 4],
    pub unk_1e8: f32,
    pub unk_1ec: f32,
    pub unk_1f0: f32,
    pub unk_1f4: f32,
    pub unk_1f8: f32,
    pub unk_1fc: f32,
    pub unk_200: u16,
}

/// `DEG_TO_BINANG2` (`z_math.h`, not `PLATFORM_N64`): `(s32)(degrees * (0x8000 / 180.0f))`.
fn deg_to_binang2(degrees: f32) -> i32 {
    (degrees * (32768.0 / 180.0)) as i32
}

/// `sinf(DEG_TO_RAD(x))`: `DEG_TO_RAD` multiplies by `M_PI / 180.0f` as a double.
fn sin_deg(x: f32) -> f32 {
    ((x as f64 * (std::f64::consts::PI / 180.0f32 as f64)) as f32).sin()
}

impl EnGs {
    /// `EnGs_Init`.
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnGs {
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            action: Action::Func80A4F734,
            unk_19c: 0,
            unk_19d: 0,
            unk_19e: 0,
            unk_19f: 0,
            unk_1a0: [[0; 3]; 3],
            unk_1b4: [Vec3::ONE; 2],
            unk_1d8: actor.world_pos,
            flash_color: [0; 4],
            unk_1e8: 0.0,
            unk_1ec: 0.0,
            unk_1f0: 0.0,
            unk_1f4: 0.0,
            unk_1f8: 0.0,
            unk_1fc: 0.0,
            unk_200: 0,
            actor,
        };
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100).
        this.actor.scale = Vec3::splat(0.1);
        this.actor.col_chk_info.set_info2(Some(&DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        this.actor.target_mode = ATTENTION_RANGE_6;
        Box::new(this)
    }

    fn switch_flag(&self) -> i32 {
        ((self.actor.params >> 8) & 0x3F) as i32
    }

    /// `func_80A4E3EC`: the talk's next state: 0 done, 1 on to the hint (the Mask of Truth's
    /// text ended), 2 still talking.
    fn func_80a4e3ec(&mut self, play: &mut PlayState) -> u8 {
        let mut ret = 2;
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            match self.actor.text_id {
                0x2054 => {
                    self.actor.text_id = (self.actor.params & 0xFF) as u16 + 0x400;
                    ret = 1;
                }
                _ => ret = 0,
            }
        }
        ret
    }

    fn player_state2(play: &PlayState) -> u32 {
        play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags2()).unwrap_or(0)
    }

    fn set_player_state2(play: &mut PlayState, set: u32) {
        if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            p.change_state_flags2(set, 0);
        }
    }

    /// `func_80A4E470`: within 100, the ocarina's free play; a song gives a fairy.
    fn func_80a4e470(&mut self, play: &mut PlayState) {
        if self.actor.xz_dist_to_player > 100.0 {
            return;
        }
        if self.unk_19d == 0 {
            Self::set_player_state2(play, PLAYER_STATE2_23);
            if Self::player_state2(play) & PLAYER_STATE2_24 != 0 {
                play.message_start_ocarina(oot_game::message::OCARINA_ACTION_FREE_PLAY);
                self.unk_19d |= 1;
            }
        } else if self.unk_19d & 1 != 0 {
            if play.msg_ctx.ocarina_mode == OCARINA_MODE_04 {
                // msgCtx->unk_E3F2: the song played (lastPlayedSong is reset by the message box).
                let song = play.msg_ctx.unk_e3f2;
                let pos = self.actor.world_pos + Vec3::Y * 40.0;
                if matches!(song, OCARINA_SONG_SARIAS | OCARINA_SONG_EPONAS | OCARINA_SONG_LULLABY | OCARINA_SONG_SUNS | OCARINA_SONG_TIME) {
                    let _ = play.actor_spawn(ACTOR_EN_ELF, pos, [0; 3], FAIRY_HEAL_TIMED);
                    audio_play_actor_sfx2(play, NA_SE_EV_BUTTERFRY_TO_FAIRY);
                } else if song == OCARINA_SONG_STORMS {
                    let _ = play.actor_spawn(ACTOR_EN_ELF, pos, [0; 3], FAIRY_HEAL_BIG);
                    audio_play_actor_sfx2(play, NA_SE_EV_BUTTERFRY_TO_FAIRY);
                }
                self.unk_19d = 0;
                let flag = self.switch_flag();
                play.flags.set_switch(flag);
            } else if play.msg_ctx.ocarina_mode == OCARINA_MODE_01 {
                Self::set_player_state2(play, PLAYER_STATE2_23);
            }
        }
    }

    /// `func_80A4E648`: the talk.
    fn func_80a4e648(&mut self, play: &mut PlayState) {
        if self.unk_19c == 1 {
            play.continue_textbox(self.actor.text_id);
            self.unk_19c = 2;
        } else if self.unk_19c == 2 {
            self.unk_19c = self.func_80a4e3ec(play);
        } else if process_talk_request(&mut self.actor) {
            self.unk_19c = 2;
        } else {
            let (x, y) = oot_game::target::actor_screen_pos(play.view_proj, &self.actor);
            if (0..=320).contains(&x) && (0..=240).contains(&y) && self.unk_19c != 3 {
                let a = self.actor.clone();
                if offer_talk(play, &a, 40.0) {
                    let mask = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.current_mask()).unwrap_or(0);
                    self.actor.text_id = if mask == PLAYER_MASK_TRUTH { 0x2054 } else { 0x2053 };
                }
            }
        }
    }

    /// `func_80A4E754` (`arg2`, `arg3` always `&unk_1E8`, `&unk_1EC`; `arg4` `&unk_200`): `unk_1E8` smoothed to `unk_1EC`, the stone squashed and stretched by it in
    /// `arg8`-frame waves (unless `arg9`). Returns the step left.
    #[allow(clippy::too_many_arguments)]
    fn func_80a4e754(&mut self, arg5: f32, arg6: f32, arg7: f32, arg8: i32, arg9: i32) -> f32 {
        let target = self.unk_1ec;
        let mut sp2c = smooth_step_to_f(&mut self.unk_1e8, target, arg5, arg6, arg7);
        if arg9 == 0 {
            sp2c = smooth_step_to_f(&mut self.unk_1e8, target, arg5, arg6, arg7);
            let phase = ((self.unk_200 as i32 % arg8) as f32 * (1.0 / arg8 as f32)) * 360.0;
            self.unk_1b4[0].x = 1.0 + (sin_deg(phase) * self.unk_1e8);
            self.unk_1b4[0].y = 1.0 - (sin_deg(phase) * self.unk_1e8);
            self.unk_200 = self.unk_200.wrapping_add(1);
        }
        sp2c
    }

    /// `func_80A4E910`: the sword's wobble, then "it doesn't seem to want to talk".
    fn func_80a4e910(&mut self, play: &mut PlayState) {
        if self.unk_19f == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_STALKID_ATTACK);
            self.unk_200 = 0;
            self.unk_19f = 1;
            self.unk_1e8 = 0.5;
            self.unk_1ec = 0.0;
        } else if self.unk_19f == 1 && self.func_80a4e754(0.8, 0.007, 0.001, 7, 0) == 0.0 {
            if !play.play_in_cs_mode() {
                play.start_textbox(0x71B1, None);
            }
            self.unk_19c = 0;
            self.action = Action::Func80A4F734;
        }
    }

    /// `func_80A4EA08`: the twist.
    fn func_80a4ea08(&mut self, play: &mut PlayState) {
        if self.unk_19f == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_STALKID_ATTACK);
            self.unk_1e8 = 0.3;
            self.unk_1ec = 0.0;
            self.unk_200 = 0;
            self.unk_19f = 1;
        } else if self.unk_19f == 1 {
            // DEG_TO_BINANG(((unk_200 % 8) / 8.0f) * 360.0f).
            self.unk_1a0[0][2] = (((self.unk_200 % 8) as f32 / 8.0 * 360.0) * (32768.0 / 180.0)) as i32 as i16;
            self.unk_1a0[1][2] = self.unk_1a0[0][2].wrapping_neg();
            if self.func_80a4e754(0.8, 0.005, 0.001, 7, 0) == 0.0 {
                self.unk_19c = 0;
                self.action = Action::Func80A4F734;
            }
        }
    }

    /// `func_80A4EB3C`: the hammer's squash, held 100 frames, then back with a wobble.
    fn func_80a4eb3c(&mut self, play: &mut PlayState) {
        if self.unk_19f == 0 {
            self.unk_1e8 = self.unk_1b4[0].y - 1.0;
            self.unk_1ec = -0.8;
            self.unk_19f += 1;
        } else if self.unk_19f == 1 {
            let ret = smooth_step_to_f(&mut self.unk_1e8, self.unk_1ec, 1.0, 0.4, 0.001);
            self.unk_1b4[0].y = self.unk_1e8 + 1.0;
            if ret == 0.0 {
                self.unk_200 = 0;
                self.unk_19f += 1;
            }
        } else if self.unk_19f == 2 {
            self.unk_200 = self.unk_200.wrapping_add(1);
            if self.unk_200 >= 100 {
                self.unk_19f += 1;
                self.unk_1e8 = self.unk_1b4[0].y - 1.0;
                self.unk_1ec = 0.0;
            }
        } else if self.unk_19f == 3 {
            let ret = smooth_step_to_f(&mut self.unk_1e8, self.unk_1ec, 1.0, 0.5, 0.001);
            self.unk_1b4[0].y = self.unk_1e8 + 1.0;
            if ret == 0.0 {
                self.unk_1e8 = 0.5;
                self.unk_1ec = 0.0;
                self.unk_200 = 0;
                audio_play_actor_sfx2(play, NA_SE_EN_STALKID_ATTACK);
                self.unk_19f += 1;
            }
        } else if self.unk_19f == 4 && self.func_80a4e754(1.0, 0.03, 0.001, 5, 0) == 0.0 {
            self.unk_19c = 0;
            self.action = Action::Func80A4F734;
        }
    }

    /// `func_80A4ED34`: the bomb: 40 frames, an 80-frame countdown flashing blue then red, smoke
    /// and the launch, up until it hits something.
    fn func_80a4ed34(&mut self, play: &mut PlayState) {
        /// `flashRed`, `flashBlue`, `baseWhite`.
        const FLASH_RED: [u8; 4] = [255, 50, 50, 0];
        const FLASH_BLUE: [u8; 4] = [50, 50, 255, 0];
        const BASE_WHITE: [u8; 4] = [255, 255, 255, 0];

        if self.unk_19f == 0 {
            self.unk_200 = 40;
            self.unk_19f += 1;
        }

        if self.unk_19f == 1 {
            // (u32)unk_200-- <= 0.
            let old = self.unk_200;
            self.unk_200 = self.unk_200.wrapping_sub(1);
            if old as u32 == 0 {
                self.unk_200 = 80;
                self.unk_19e |= 4;
                self.unk_19f += 1;
            }
        }

        if self.unk_19f == 2 {
            self.unk_200 = self.unk_200.wrapping_sub(1);
            self.flash_color = BASE_WHITE;
            if self.unk_200 < 80 && (self.unk_200 % 20) < 8 {
                if self.unk_200 < 20 {
                    self.flash_color = FLASH_RED;
                    if self.unk_200 % 20 == 7 {
                        play.audio.play_sfx_centered(NA_SE_SY_WARNING_COUNT_E);
                    }
                } else {
                    self.flash_color = FLASH_BLUE;
                    if self.unk_200 % 20 == 7 {
                        play.audio.play_sfx_centered(NA_SE_SY_WARNING_COUNT_N);
                    }
                }
            }
            if self.unk_200 == 0 {
                self.unk_200 = 0;
                self.unk_19e &= !4;
                self.unk_19f += 1;
            }
        }

        if self.unk_19f == 3 {
            /// `dustAccel`, `dustPrim`, `dustEnv`.
            const DUST_ACCEL: Vec3 = Vec3::new(0.0, -0.3, 0.0);
            const DUST_PRIM: [u8; 4] = [200, 200, 200, 128];
            const DUST_ENV: [u8; 4] = [100, 100, 100, 0];
            for _ in 0..3 {
                let vx = play.rand.centered_float(15.0);
                let vy = play.rand.zero_float(-1.0);
                let vz = play.rand.centered_float(15.0);
                let w = self.actor.world_pos;
                let dust_pos = Vec3::new(w.x + (vx + vx), w.y + 7.0, w.z + (vz + vz));
                let scale = (play.rand.zero_float(50.0) as i16).wrapping_add(200);
                play.with_ss(|s| s.func_8002836c(dust_pos, Vec3::new(vx, vy, vz), DUST_ACCEL, DUST_PRIM, DUST_ENV, scale, 40, 15));
            }
            self.actor.play_sfx_flagged(NA_SE_EV_FIRE_PILLAR - SFX_FLAG);
            let old = self.unk_200;
            self.unk_200 = self.unk_200.wrapping_add(1);
            if old >= 40 {
                self.unk_19e |= 0x10;
                self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                self.actor.culling_volume_distance = 12000.0;
                self.actor.gravity = 0.3;
                self.unk_19f += 1;
            }
        }

        if self.unk_19f == 4 {
            self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 60.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1);
            if self.actor.bg_check_flags & (BGCHECKFLAG_WALL | BGCHECKFLAG_CEILING) != 0 {
                audio_play_actor_sfx2(play, NA_SE_IT_BOMB_EXPLOSION);
                // EffectSsBomb2_SpawnLayered(play, &pos, &zero, &zero, 100, 20): the effect isn't
                // ported.
                log::info!("En_Gs: EffectSsBomb2 (the launched stone's explosion) isn't ported");
                self.unk_200 = 10;
                self.unk_19e |= 8;
                self.action = Action::Func80A4F700;
            } else {
                self.actor.play_sfx_flagged(NA_SE_EV_STONE_LAUNCH - SFX_FLAG);
            }
            self.actor.move_forward();
            if self.actor.y_dist_to_player < -12000.0 {
                self.actor.kill();
            }
        }
    }

    /// `func_80A4F13C`: magic's spin, up to speed and down, a stretch, and back with a wobble.
    fn func_80a4f13c(&mut self, play: &mut PlayState) {
        let tmp2 = self.unk_1a0[0][1];
        let mut tmp: i32;

        if self.unk_19f == 0 {
            self.unk_1e8 = 0.0;
            self.unk_1ec = 90.0;
            self.unk_1f0 = 0.1;
            self.unk_1f4 = 2.0;
            self.unk_1f8 = 0.0;
            self.unk_19f = 1;
        }
        if self.unk_19f == 1 {
            smooth_step_to_f(&mut self.unk_1f0, self.unk_1f4, 1.0, 0.1, 0.001);
            let tmpf1 = smooth_step_to_f(&mut self.unk_1e8, self.unk_1ec, 1.0, self.unk_1f0, 0.001);
            self.unk_1a0[0][1] = self.unk_1a0[0][1].wrapping_add(deg_to_binang2(self.unk_1e8) as i16);
            if tmpf1 == 0.0 {
                self.unk_200 = 0;
                self.unk_19f = 2;
            }
        }
        if self.unk_19f == 2 {
            self.unk_1a0[0][1] = self.unk_1a0[0][1].wrapping_add(deg_to_binang2(self.unk_1e8) as i16);
            let old = self.unk_200;
            self.unk_200 = self.unk_200.wrapping_add(1);
            if old > 40 {
                self.unk_1e8 = self.unk_1b4[0].y - 1.0;
                self.unk_1ec = 1.5;
                self.unk_1f0 = self.unk_1b4[1].y - 1.0;
                self.unk_1f4 = -0.3;
                audio_play_actor_sfx2(play, NA_SE_EV_STONE_GROW_UP);
                self.unk_19f = 3;
            }
        }
        if self.unk_19f == 3 {
            self.unk_1a0[0][1] = self.unk_1a0[0][1].wrapping_add(deg_to_binang2(90.0) as i16);
            let tmpf1 = smooth_step_to_f(&mut self.unk_1e8, self.unk_1ec, 0.8, 0.2, 0.001);
            smooth_step_to_f(&mut self.unk_1f0, self.unk_1f4, 0.8, 0.2, 0.001);
            self.unk_1b4[0].x = self.unk_1f0 + 1.0;
            self.unk_1b4[0].y = self.unk_1e8 + 1.0;
            if tmpf1 == 0.0 {
                self.unk_1e8 = (2.0 * std::f64::consts::PI / 9.0000002) as f32;
                self.unk_1ec = (std::f64::consts::PI / 9.0000002) as f32;
                self.unk_19f = 4;
            }
        }
        if self.unk_19f == 4 {
            let tmpf1 = smooth_step_to_f(&mut self.unk_1e8, self.unk_1ec, 0.8, deg_to_binang2(90.0) as f32, deg_to_binang2(20.0) as f32);
            self.unk_1a0[0][1] = self.unk_1a0[0][1].wrapping_add(self.unk_1e8 as i32 as i16);
            if tmpf1 == 0.0 {
                tmp = self.unk_1a0[0][1] as i32;
                if tmp > 0 {
                    tmp -= 0x10000;
                }
                self.unk_1e8 = tmp as f32;
                self.unk_1ec = 0.0;
                self.unk_19f = 5;
            }
        }
        if self.unk_19f == 5 {
            tmp = self.unk_1a0[0][1] as i32;
            if tmp > 0 {
                // @bug (game): 0xFFFF, where step 4 takes 0x10000.
                tmp -= 0xFFFF;
            }
            self.unk_1e8 = tmp as f32;
            let tmpf1 = smooth_step_to_f(&mut self.unk_1e8, self.unk_1ec, 0.8, deg_to_binang2(20.0) as f32, 0.001);
            self.unk_1a0[0][1] = self.unk_1e8 as i32 as i16;
            if tmpf1 == 0.0 {
                self.unk_1e8 = self.unk_1b4[0].y - 1.0;
                self.unk_1ec = 0.0;
                self.unk_1a0[0][1] = 0;
                self.unk_200 = 0;
                self.unk_1f0 = self.unk_1b4[0].x - 1.0;
                self.unk_1f4 = 0.0;
                self.unk_1f8 = 0.5;
                self.unk_1fc = 0.0;
                audio_play_actor_sfx2(play, NA_SE_EN_STALKID_ATTACK);
                self.unk_19f = 6;
            }
        }
        if self.unk_19f == 6 {
            let tmpf1 = smooth_step_to_f(&mut self.unk_1e8, self.unk_1ec, 0.8, 0.1, 0.001);
            let tmpf2 = smooth_step_to_f(&mut self.unk_1f0, self.unk_1f4, 0.8, 0.1, 0.001);
            let tmpf3 = smooth_step_to_f(&mut self.unk_1f8, self.unk_1fc, 0.8, 0.02, 0.001);
            self.unk_1b4[0].x = self.unk_1f0 + 1.0;
            self.unk_1b4[0].y = self.unk_1e8 + 1.0;
            let wave = sin_deg(((self.unk_200 % 10) as f32 * 0.1) * 360.0) * self.unk_1f8;
            self.unk_1b4[0].x += wave;
            self.unk_1b4[0].y += wave;
            self.unk_200 = self.unk_200.wrapping_add(1);
            if tmpf1 == 0.0 && tmpf2 == 0.0 && tmpf3 == 0.0 {
                self.unk_19c = 0;
                self.action = Action::Func80A4F734;
            }
        }
        if (self.unk_1a0[0][1] as u16) < (tmp2 as u16) {
            audio_play_actor_sfx2(play, NA_SE_EV_STONE_ROLLING);
        }
    }

    /// `func_80A4F700`.
    fn func_80a4f700(&mut self) {
        let old = self.unk_200;
        self.unk_200 = self.unk_200.wrapping_sub(1);
        if old == 0 {
            self.actor.kill();
        }
    }

    /// `func_80A4F734`: waiting for a song, until its switch flag is set.
    fn func_80a4f734(&mut self, play: &mut PlayState) {
        if !play.flags.get_switch(self.switch_flag()) {
            self.func_80a4e470(play);
        }
    }

    /// `func_80A4F77C`: the turns and scales reset; the talk stopped.
    fn func_80a4f77c(&mut self) {
        self.unk_1a0 = [[0; 3]; 3];
        self.unk_1b4 = [Vec3::ONE; 2];
        self.unk_19c = 3;
    }

    /// The stone's matrix (`EnGs_Draw`'s, under `unk_19E & 1`).
    fn stone_matrix(model: Mat4, r: &[i16], s: Vec3) -> Mat4 {
        use eng_math::binang_to_rad;
        model
            * Mat4::from_rotation_y(binang_to_rad(r[1]))
            * Mat4::from_rotation_x(binang_to_rad(r[0]))
            * Mat4::from_rotation_z(binang_to_rad(r[2]))
            * Mat4::from_scale(s)
            * Mat4::from_rotation_y(binang_to_rad(r[4]))
            * Mat4::from_rotation_x(binang_to_rad(r[3]))
            * Mat4::from_rotation_z(binang_to_rad(r[5]))
    }
}

impl ActorImpl for EnGs {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnGs_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.set_focus(23.0);
        if self.unk_19e & 0x10 == 0 {
            if self.collider.base.ac_flags & AC_HIT != 0 {
                self.unk_19f = 0;
                self.collider.base.ac_flags &= !AC_HIT;
                let next = match self.actor.col_chk_info.damage_reaction {
                    15 => Some((1, Action::Func80A4E910)),
                    14 => Some((1, Action::Func80A4EA08)),
                    13 => Some((1, Action::Func80A4EB3C)),
                    12 => Some((2, Action::Func80A4ED34)),
                    11 => Some((1, Action::Func80A4F13C)),
                    _ => None,
                };
                if let Some((bit, action)) = next {
                    self.unk_19e |= bit;
                    self.func_80a4f77c();
                    self.action = action;
                }
            }
            self.collider.update(&self.actor);
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
        match self.action {
            Action::Func80A4E910 => self.func_80a4e910(play),
            Action::Func80A4EA08 => self.func_80a4ea08(play),
            Action::Func80A4EB3C => self.func_80a4eb3c(play),
            Action::Func80A4ED34 => self.func_80a4ed34(play),
            Action::Func80A4F13C => self.func_80a4f13c(play),
            Action::Func80A4F700 => self.func_80a4f700(),
            Action::Func80A4F734 => self.func_80a4f734(play),
        }
        self.func_80a4e648(play);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        let r = &self.unk_1a0;
        rs.angles = vec![r[0][0], r[0][1], r[0][2], r[1][0], r[1][1], r[1][2]];
        let s = self.unk_1b4[0];
        rs.switches = vec![self.unk_19e as u32, u32::from_be_bytes(self.flash_color), s.x.to_bits(), s.y.to_bits(), s.z.to_bits()];
        rs
    }
    /// `EnGs_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let [flags, flash, sx, sy, sz] = rs.switches.as_slice() else { return };
        if rs.angles.len() < 6 {
            return;
        }
        let flags = *flags as u8;
        if flags & 8 != 0 {
            return;
        }
        let model = oot_game::play::actor_draw_matrix(rs);
        let m = if flags & 1 != 0 { Self::stone_matrix(model, &rs.angles, Vec3::new(f32::from_bits(*sx), f32::from_bits(*sy), f32::from_bits(*sz))) } else { model };
        // gDPSetPrimColor(0, 0, flashColor) while it flashes, else (255, 255, 255, 255).
        let prim = if flags & 4 != 0 { flash.to_be_bytes() } else { [255, 255, 255, 255] };
        let mut sv = SegmentValues::default();
        sv.prim[SEG_PRIM as usize] = Some(prim);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        if flags & 2 != 0 {
            // Matrix_ReplaceRotation(billboard), Matrix_Scale(0.05, -0.05, 1).
            let t = oot_game::draw::replace_rotation(model, view.billboard) * Mat4::from_scale(Vec3::new(0.05, -0.05, 1.0));
            let mut sv = SegmentValues::default();
            sv.read(SEG_SCROLL, &fire_scroll(play.gameplay_frames));
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(FIRE_BAKE)), transform: t, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deg_to_binang2_truncates_to_s32() {
        // 20 * (0x8000 / 180.0f) = 3640.89 -> 3640; 90 -> 16384.
        assert_eq!(deg_to_binang2(20.0), 3640);
        assert_eq!(deg_to_binang2(90.0), 16384);
    }

    #[test]
    fn the_damage_table_picks_reactions() {
        // The sword 0xF, the slingshot 0xE, bombs 0xC, the hammer 0xD, fire arrows 0xB.
        assert_eq!(DAMAGE_TABLE.table[8] >> 4, 0xF);
        assert_eq!(DAMAGE_TABLE.table[2] >> 4, 0xE);
        assert_eq!(DAMAGE_TABLE.table[3] >> 4, 0xC);
        assert_eq!(DAMAGE_TABLE.table[6] >> 4, 0xD);
        assert_eq!(DAMAGE_TABLE.table[11] >> 4, 0xB);
    }
}
