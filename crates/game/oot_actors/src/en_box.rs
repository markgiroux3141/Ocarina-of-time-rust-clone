//! `En_Box` (`ovl_En_Box/z_en_box.c`): the treasure chests.
//!
//! `params`: bits 12..15 the type (`EnBoxType`), bits 5..11 the get-item id, bits 0..4 the
//! scene's treasure flag; `world.rot.z` is the switch flag (for the types that appear or fall
//! on one). A chest is a DynaPoly actor (`gTreasureChestCol`, no ceiling) with a skeleton whose
//! front (limb 1) and side-and-lid (limb 3) lists `EnBox_PostLimbDraw` adds.
//!
//! Waiting (`EnBox_WaitOpen`), it offers its item to a Link standing in front of it and facing
//! it (`func_8002F554` with the get-item id negated: a chest's). Player's get-item interrupt
//! opens it on A (`func_8083E5A8`): it sets `unk_1F4` to 1 for a new major item (the long
//! opening, with the light `Demo_Tre_Lgt` for a big chest) or -1 for the kick, and the chest
//! plays the opening for Link's age and sets its treasure flag. An opened chest (its flag set)
//! starts open.
//!
//! Also ported: the chests that appear on a switch flag, on the room's clear flag, or fall on a
//! switch flag. Not ported: the chests the Zelda's Lullaby and Sun's Song make appear (types 9
//! and 10: `func_809C9700` waits on the ocarina, which isn't ported, so they stay hidden), the
//! lens-hidden chests' drawing (types 4 and 6 with `ACTOR_FLAG_7`: the Lens of Truth isn't
//! ported, so they aren't drawn until opened), the one-point cutscene cameras
//! (`OnePointCutscene_Init`, `_Attention`), the fanfare and the sounds, and the effects (the
//! falling chests' dust, the ice trap's smoke: their `Rand_ZeroOne` calls are made). The light
//! (`Demo_Tre_Lgt`) and the sparkles (`Demo_Kankyo`) spawn as placeholders.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_CHEST, ActorImpl, ActorProfile};
use oot_game::get_item::offer_get_item;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_ONCE, Anim, SkelAnimeStd};

/// `ACTOR_EN_BOX` (`actor_table.h`: 0x000A).
pub const ACTOR_EN_BOX: i16 = 0x000A;
/// `ACTOR_DEMO_KANKYO` (0x008C) with `DEMOKANKYO_SPARKLES`, `ACTOR_DEMO_TRE_LGT` (0x00AA).
const ACTOR_DEMO_KANKYO: i16 = 0x008C;
const DEMOKANKYO_SPARKLES: i16 = 0x11;
const ACTOR_DEMO_TRE_LGT: i16 = 0x00AA;

pub const OBJECT: &str = "object_box";
const COLLISION: &str = "gTreasureChestCol";
const SKELETON: &str = "gTreasureChestSkel";

/// `En_Box_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_BOX, name: "En_Box", category: ACTORCAT_CHEST, flags: 0, object: OBJECT };

/// `EnBoxType`.
pub const ENBOX_TYPE_BIG_DEFAULT: u8 = 0;
pub const ENBOX_TYPE_ROOM_CLEAR_BIG: u8 = 1;
pub const ENBOX_TYPE_DECORATED_BIG: u8 = 2;
pub const ENBOX_TYPE_SWITCH_FLAG_FALL_BIG: u8 = 3;
pub const ENBOX_TYPE_4: u8 = 4;
pub const ENBOX_TYPE_SMALL: u8 = 5;
pub const ENBOX_TYPE_6: u8 = 6;
pub const ENBOX_TYPE_ROOM_CLEAR_SMALL: u8 = 7;
pub const ENBOX_TYPE_SWITCH_FLAG_FALL_SMALL: u8 = 8;
pub const ENBOX_TYPE_9: u8 = 9;
pub const ENBOX_TYPE_10: u8 = 10;
pub const ENBOX_TYPE_SWITCH_FLAG_BIG: u8 = 11;

/// `movementFlags` (`z_en_box.c`).
const ENBOX_MOVE_IMMOBILE: u8 = 1 << 0;
const ENBOX_MOVE_UNUSED: u8 = 1 << 1;
const ENBOX_MOVE_FALL_ANGLE_SIDE: u8 = 1 << 2;
const ENBOX_MOVE_STICK_TO_GROUND: u8 = 1 << 4;

/// `ENBOX_TREASURE_FLAG_UNK_MIN`, `_MAX`: the treasure chest shop's chests (`func_8002F5F0`,
/// which tells Player the nearest: the chest game isn't ported).
const ENBOX_TREASURE_FLAG_UNK_MIN: i16 = 20;
const ENBOX_TREASURE_FLAG_UNK_MAX: i16 = 32;

/// `GI_ICE_TRAP`.
const GI_ICE_TRAP: i16 = 0x7C;

/// `sAnimations`: the long opening (adult, child), then the short one (both). Indexed by
/// `linkAge` (`LINK_AGE_ADULT` 0, `LINK_AGE_CHILD` 1).
const ANIMATIONS: [&str; 4] = ["gTreasureChestAnim_00024C", "gTreasureChestAnim_000128", "gTreasureChestAnim_00043C", "gTreasureChestAnim_00043C"];

/// The segment the lists call for a render mode (an empty list, or `func_809CA4A0` /
/// `func_809CA518`'s), and the one the bakes take the env colour from.
const SEG_RENDER_MODE: u8 = 0x08;
const SEG_ENV: u8 = 0x0B;

/// The chest's bakes: opaque (`EnBox_EmptyDList` on segment 8, env (0, 0, 0, 255)), the boss
/// key chest's opaque one, and the translucent ones (`func_809CA4A0` for the fading types,
/// `func_809CA518` for types 4 and 6) with the env alpha a dynamic colour.
const BAKE_OPA: &str = "En_Box/opa";
const BAKE_OPA_BOSS_KEY: &str = "En_Box/opa_boss_key";
const BAKE_XLU: &str = "En_Box/xlu";
const BAKE_XLU_46: &str = "En_Box/xlu_46";

pub fn bakes() -> Vec<MeshBake> {
    let limbs = |front: &str, side: &str| vec![LimbOverride { limb: 0, file: OBJECT.into(), symbol: front.into() }, LimbOverride { limb: 2, file: OBJECT.into(), symbol: side.into() }];
    let body = |front: &str, side: &str| BakeBody::Skeleton { file: OBJECT.into(), symbol: SKELETON.into(), limbs: limbs(front, side) };
    let end = (0xDF00_0000u32, 0u32);
    // gDPSetEnvColor(0, 0, 0, 255).
    let env_opaque = BakeSegment::Commands(vec![(0xFB00_0000, 0x0000_00FF), end]);
    // func_809CA4A0: gDPSetRenderMode(AA_EN | Z_CMP | Z_UPD | IM_RD | CLR_ON_CVG | CVG_DST_WRAP |
    // ZMODE_XLU | FORCE_BL | GBL_c1(G_BL_CLR_FOG, G_BL_A_SHADE, G_BL_CLR_IN, G_BL_1MA), the same
    // with GBL_c2(G_BL_CLR_IN, G_BL_A_IN, G_BL_CLR_MEM, G_BL_1MA)): G_SETOTHERMODE_L, shift 3, 29 bits
    // (w0 0xE2 << 24 | (32 - 3 - 29) << 8 | (29 - 1)).
    let rm_xlu = (0xE200_001Cu32, 0xC810_49F8u32);
    // func_809CA518: AA_EN | Z_CMP | Z_UPD | IM_RD | CVG_DST_CLAMP | ZMODE_OPA | ALPHA_CVG_SEL |
    // GBL_c1(G_BL_CLR_FOG, G_BL_A_SHADE, G_BL_CLR_IN, G_BL_1MA), G_RM_AA_ZB_OPA_SURF2.
    let rm_46 = (0xE200_001Cu32, 0xC811_2078u32);
    let opa = |name: &str, front: &str, side: &str| MeshBake {
        name: name.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_ENV, env_opaque.clone()), (SEG_RENDER_MODE, BakeSegment::Commands(vec![end]))],
        prelude: vec![SEG_ENV],
        body: body(front, side),
    };
    let xlu = |name: &str, rm: (u32, u32)| MeshBake {
        name: name.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false }), (SEG_RENDER_MODE, BakeSegment::Commands(vec![rm, end]))],
        prelude: vec![SEG_ENV],
        body: body("gTreasureChestChestFrontDL", "gTreasureChestChestSideAndLidDL"),
    };
    vec![
        opa(BAKE_OPA, "gTreasureChestChestFrontDL", "gTreasureChestChestSideAndLidDL"),
        opa(BAKE_OPA_BOSS_KEY, "gTreasureChestBossKeyChestFrontDL", "gTreasureChestBossKeyChestSideAndTopDL"),
        xlu(BAKE_XLU, rm_xlu),
        xlu(BAKE_XLU_46, rm_46),
    ]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnBox_FallOnSwitchFlag`.
    FallOnSwitchFlag,
    /// `EnBox_Fall`.
    Fall,
    /// `func_809C9700`: waiting for a song (types 9 and 10).
    WaitSong,
    /// `EnBox_AppearOnSwitchFlag`.
    AppearOnSwitchFlag,
    /// `EnBox_AppearOnRoomClear`.
    AppearOnRoomClear,
    /// `EnBox_AppearInit`.
    AppearInit,
    /// `EnBox_AppearAnimation`.
    AppearAnimation,
    /// `EnBox_WaitOpen`.
    WaitOpen,
    /// `EnBox_Open`.
    Open,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::FallOnSwitchFlag => "EnBox_FallOnSwitchFlag",
            Action::Fall => "EnBox_Fall",
            Action::WaitSong => "func_809C9700",
            Action::AppearOnSwitchFlag => "EnBox_AppearOnSwitchFlag",
            Action::AppearOnRoomClear => "EnBox_AppearOnRoomClear",
            Action::AppearInit => "EnBox_AppearInit",
            Action::AppearAnimation => "EnBox_AppearAnimation",
            Action::WaitOpen => "EnBox_WaitOpen",
            Action::Open => "EnBox_Open",
        }
    }
}

pub struct EnBox {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub skel: Option<SkelAnimeStd>,
    skeleton: Option<Arc<Skeleton>>,
    /// `unk_1A8`: the appearing and falling delays.
    pub unk_1a8: i32,
    /// `unk_1B0`: the lid's opening, 0 to 1 (read by nothing ported).
    pub unk_1b0: f32,
    pub action: Action,
    /// `unk_1F4`: set by Player to open (1 long, -1 short), then counts the frames open.
    pub unk_1f4: i16,
    pub movement_flags: u8,
    pub alpha: u8,
    pub switch_flag: u8,
    /// `type`.
    pub ty: u8,
    pub ice_smoke_timer: u8,
    /// `unk_1FB`: the song chests' state.
    pub unk_1fb: u8,
}

impl EnBox {
    fn treasure_flag(&self) -> i32 {
        (self.actor.params & 0x1F) as i32
    }

    /// The get-item id the chest holds (`params >> 5 & 0x7F`).
    pub fn get_item_id(&self) -> i16 {
        (self.actor.params >> 5) & 0x7F
    }

    fn is_small(&self) -> bool {
        matches!(self.ty, ENBOX_TYPE_SMALL | ENBOX_TYPE_6 | ENBOX_TYPE_ROOM_CLEAR_SMALL | ENBOX_TYPE_SWITCH_FLAG_FALL_SMALL)
    }

    fn animation(play: &PlayState, name: &str) -> Option<Anim> {
        play.assets.as_ref()?.animation(OBJECT, name).map_err(|e| log::error!("En_Box: {e:#}")).ok()
    }

    /// `EnBox_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let adult = play.save.adult;
        let mut anim_frame_start = 0.0;
        let anim = Self::animation(play, ANIMATIONS[if adult { 0 } else { 1 }]);
        let end_frame = anim.as_ref().map(|a| a.last_frame()).unwrap_or(0.0);
        // sInitChain: targetMode 0.
        actor.target_mode = 0;
        // DynaPolyActor_Init(DPM_UNK), gTreasureChestCol, DynaPoly_SetBgActor,
        // DynaPoly_DisableCeilingCollision.
        let bg = match play.assets.as_ref().map(|a| a.pack.collision(&keys::collision(OBJECT, COLLISION))) {
            Some(Ok(h)) => play.col.dyna.set_bg_actor(Arc::new(h), source(&actor), 0),
            other => {
                if let Some(Err(e)) = other {
                    log::error!("En_Box: {e:#}");
                }
                BG_ACTOR_MAX
            }
        };
        play.col.dyna.disable_ceiling_collision(bg);
        let mut b = EnBox {
            ty: ((actor.params >> 12) & 0xF) as u8,
            switch_flag: actor.world_rot.z as u8,
            actor,
            bg,
            skel: None,
            skeleton: None,
            unk_1a8: 0,
            unk_1b0: 0.0,
            action: Action::WaitOpen,
            unk_1f4: 0,
            movement_flags: 0,
            alpha: 0,
            ice_smoke_timer: 0,
            unk_1fb: 0,
        };
        b.actor.gravity = -5.5;
        b.actor.min_velocity_y = -50.0;
        let ty = b.ty;
        if play.flags.get_treasure(b.treasure_flag()) {
            b.alpha = 255;
            b.ice_smoke_timer = 100;
            b.action = Action::Open;
            b.movement_flags |= ENBOX_MOVE_STICK_TO_GROUND;
            anim_frame_start = end_frame;
        } else if (ty == ENBOX_TYPE_SWITCH_FLAG_FALL_BIG || ty == ENBOX_TYPE_SWITCH_FLAG_FALL_SMALL) && !play.flags.get_switch(b.switch_flag as i32) {
            play.col.dyna.set_collision_disabled(bg, true);
            if play.rand.zero_one() < 0.5 {
                b.movement_flags |= ENBOX_MOVE_FALL_ANGLE_SIDE;
            }
            b.unk_1a8 = -12;
            b.action = Action::FallOnSwitchFlag;
            b.alpha = 0;
            b.movement_flags |= ENBOX_MOVE_IMMOBILE;
            b.actor.flags |= ACTOR_FLAG_4;
        } else if (ty == ENBOX_TYPE_ROOM_CLEAR_BIG || ty == ENBOX_TYPE_ROOM_CLEAR_SMALL) && !play.flags.get_clear(b.actor.room) {
            b.action = Action::AppearOnRoomClear;
            play.col.dyna.set_collision_disabled(bg, true);
            b.hide();
        } else if ty == ENBOX_TYPE_9 || ty == ENBOX_TYPE_10 {
            b.action = Action::WaitSong;
            b.actor.flags |= ACTOR_FLAG_25;
            play.col.dyna.set_collision_disabled(bg, true);
            b.hide();
        } else if ty == ENBOX_TYPE_SWITCH_FLAG_BIG && !play.flags.get_switch(b.switch_flag as i32) {
            b.action = Action::AppearOnSwitchFlag;
            play.col.dyna.set_collision_disabled(bg, true);
            b.hide();
        } else {
            if ty == ENBOX_TYPE_4 || ty == ENBOX_TYPE_6 {
                b.actor.flags |= ACTOR_FLAG_7;
            }
            b.action = Action::WaitOpen;
            b.movement_flags |= ENBOX_MOVE_IMMOBILE | ENBOX_MOVE_STICK_TO_GROUND;
        }
        b.actor.world_rot.y = b.actor.world_rot.y.wrapping_add(i16::MIN);
        b.actor.home_rot.z = 0;
        b.actor.world_rot.z = 0;
        b.actor.shape_rot.z = 0;
        // SkelAnime_Init(&gTreasureChestSkel, anim), then Animation_Change(anim, 1.5,
        // animFrameStart, endFrame, ANIMMODE_ONCE, 0).
        match play.assets.as_ref().map(|a| a.skeleton(OBJECT, SKELETON)) {
            Some(Ok(s)) => {
                let mut sk = SkelAnimeStd::init_flex(s.limbs.len(), None);
                if let Some(a) = anim {
                    sk.change(a, 1.5, anim_frame_start, end_frame, ANIMMODE_ONCE, 0.0);
                }
                b.skel = Some(sk);
                b.skeleton = Some(s);
            }
            Some(Err(e)) => log::error!("En_Box: {e:#}"),
            None => {}
        }
        if b.is_small() {
            b.actor.scale = Vec3::splat(0.005);
            b.actor.set_focus(20.0);
        } else {
            b.actor.scale = Vec3::splat(0.01);
            b.actor.set_focus(40.0);
        }
        play.col.dyna.set_source(bg, source(&b.actor));
        Box::new(b)
    }

    /// The hidden start of the appearing types: immobile, 50 below home, invisible, updating
    /// out of view.
    fn hide(&mut self) {
        self.movement_flags |= ENBOX_MOVE_IMMOBILE;
        self.actor.world_pos.y = self.actor.home_pos.y - 50.0;
        self.alpha = 0;
        self.actor.flags |= ACTOR_FLAG_4;
    }

    /// `EnBox_ClipToGround`: onto the floor just under it (its own collision aside).
    fn clip_to_ground(&mut self, play: &PlayState) {
        let mut check = self.actor.world_pos;
        check.y += 1.0;
        let (y, _) = play.col.entity_raycast_down_actor(check, self.bg);
        if y != eng_collision::bgcheck::BGCHECK_Y_MIN {
            self.actor.world_pos.y = y;
        }
    }

    /// `EnBox_SpawnDust`: 20 dust clouds at random places around it (`EnBox_RandomDustKinematic`;
    /// the effect, `func_8002873C`, isn't ported: only its random numbers are drawn).
    fn spawn_dust(&mut self, play: &mut PlayState) {
        for _ in 0..20 {
            play.rand.zero_one();
            play.rand.zero_one();
        }
    }

    /// `EnBox_Fall`.
    fn fall(&mut self, play: &mut PlayState) {
        self.alpha = 255;
        self.movement_flags &= !ENBOX_MOVE_IMMOBILE;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.movement_flags |= ENBOX_MOVE_UNUSED;
            self.movement_flags ^= ENBOX_MOVE_FALL_ANGLE_SIDE;
            let k = if self.ty == ENBOX_TYPE_SWITCH_FLAG_FALL_BIG { 0.55 } else { 0.65 };
            self.actor.velocity.y = -self.actor.velocity.y * k;
            if self.actor.velocity.y < 5.5 {
                self.actor.shape_rot.z = 0;
                self.actor.world_pos.y = self.actor.floor_height;
                self.action = Action::WaitOpen;
                // OnePointCutscene_EndCutscene: no one-point cutscenes.
            }
            // NA_SE_EV_COFFIN_CAP_BOUND: no sound.
            self.spawn_dust(play);
        }
        let y_diff = self.actor.world_pos.y - self.actor.floor_height;
        self.actor.shape_rot.z = if self.movement_flags & ENBOX_MOVE_FALL_ANGLE_SIDE != 0 { (y_diff * 50.0) as i32 as i16 } else { (-y_diff * 50.0) as i32 as i16 };
    }

    /// The treasure chest shop's chests tell Player which is nearest (`func_8002F5F0`: not
    /// ported, the chest game isn't).
    fn nearest_chest_check(&self) {
        let f = self.actor.params & 0x1F;
        let _ = (ENBOX_TREASURE_FLAG_UNK_MIN..ENBOX_TREASURE_FLAG_UNK_MAX).contains(&f);
    }

    /// `EnBox_FallOnSwitchFlag`.
    fn fall_on_switch_flag(&mut self, play: &mut PlayState) {
        self.nearest_chest_check();
        if self.unk_1a8 >= 0 {
            self.action = Action::Fall;
            // OnePointCutscene_Init(play, 4500, 9999, &this->dyna.actor, CAM_ID_MAIN): no
            // one-point cutscenes.
            log::debug!("En_Box: OnePointCutscene_Init(4500) isn't ported");
            play.col.dyna.set_collision_disabled(self.bg, false);
        } else if self.unk_1a8 >= -11 {
            self.unk_1a8 += 1;
        } else if play.flags.get_switch(self.switch_flag as i32) {
            self.unk_1a8 += 1;
        }
    }

    /// `func_809C9700`: a song chest waits for Link to play its song near it. The ocarina isn't
    /// ported, so it never appears: within 150 it asks Player for the ocarina
    /// (`PLAYER_STATE2_23`), which nothing answers.
    fn wait_song(&mut self, play: &mut PlayState) {
        self.nearest_chest_check();
        let Some(p) = play.player.and_then(|h| play.actors.actor(h)) else { return };
        if self.actor.world_pos.distance_squared(p.world_pos) > 150.0 * 150.0 {
            self.unk_1fb = 0;
        }
    }

    /// `EnBox_AppearOnSwitchFlag`.
    fn appear_on_switch_flag(&mut self, play: &mut PlayState) {
        self.nearest_chest_check();
        if play.flags.get_switch(self.switch_flag as i32) {
            // OnePointCutscene_Attention: not ported.
            self.action = Action::AppearInit;
            self.unk_1a8 = -30;
        }
    }

    /// `EnBox_AppearOnRoomClear`.
    fn appear_on_room_clear(&mut self, play: &mut PlayState) {
        self.nearest_chest_check();
        if play.flags.get_temp_clear(self.actor.room) && !play.player_in_cs_mode() {
            play.flags.set_clear(self.actor.room);
            self.action = Action::AppearInit;
            // OnePointCutscene_Attention; OnePointCutscene_CheckForCategory is false without
            // one-point cutscenes.
            self.unk_1a8 = -30;
        }
    }

    /// `EnBox_AppearInit`: `func_8005B198` (the category the one-point camera attends) is never
    /// the chest's here, so the delay decides (`unk_1A8` is -30).
    fn appear_init(&mut self, play: &mut PlayState) {
        if self.unk_1a8 != 0 {
            self.action = Action::AppearAnimation;
            self.unk_1a8 = 0;
            let h = self.actor.home_pos;
            if let Err(e) = play.actor_spawn(ACTOR_DEMO_KANKYO, h, [0; 3], DEMOKANKYO_SPARKLES) {
                log::debug!("En_Box: Demo_Kankyo: {e:?}");
            }
            // NA_SE_EV_TRE_BOX_APPEAR: no sound.
        }
    }

    /// `EnBox_AppearAnimation`: rise 50 over 40 frames, fade in over 20.
    fn appear_animation(&mut self, play: &mut PlayState) {
        play.col.dyna.set_collision_disabled(self.bg, false);
        if self.unk_1a8 < 0 {
            self.unk_1a8 += 1;
        } else if self.unk_1a8 < 40 {
            self.unk_1a8 += 1;
            self.actor.world_pos.y += 1.25;
        } else if self.unk_1a8 < 60 {
            self.alpha = self.alpha.wrapping_add(12);
            self.unk_1a8 += 1;
            self.actor.world_pos.y = self.actor.home_pos.y;
        } else {
            self.action = Action::WaitOpen;
        }
    }

    /// `EnBox_WaitOpen`: when Player opens it (`unk_1F4`), the opening for Link's age (the short
    /// one after a kick), the light for a big chest's long opening, and the treasure flag.
    /// Otherwise it offers its item to a Link in front of it (within 20 across, 0 to 50 in
    /// front, 10 up or down) and facing it (0x3000).
    fn wait_open(&mut self, play: &mut PlayState) {
        self.alpha = 255;
        self.movement_flags |= ENBOX_MOVE_IMMOBILE;
        if self.unk_1f4 != 0 {
            let age = if play.save.adult { 0 } else { 1 };
            let name = ANIMATIONS[if self.unk_1f4 < 0 { 2 } else { 0 } + age];
            if let (Some(a), Some(sk)) = (Self::animation(play, name), self.skel.as_mut()) {
                let last = a.last_frame();
                sk.change(a, 1.5, 0.0, last, ANIMMODE_ONCE, 0.0);
            }
            self.action = Action::Open;
            if self.unk_1f4 > 0 && !self.is_small() {
                let (p, r) = (self.actor.world_pos, self.actor.shape_rot);
                let mut me = self.actor.clone();
                if let Err(e) = play.actor_spawn_as_child(&mut me, ACTOR_DEMO_TRE_LGT, p, [r.x, r.y, r.z], -1) {
                    log::debug!("En_Box: Demo_Tre_Lgt: {e:?}");
                }
                self.actor.child = me.child;
                play.audio.play_fanfare(oot_game::audio::NA_BGM_OPEN_TRE_BOX | 0x900);
            }
            play.flags.set_treasure(self.treasure_flag());
        } else {
            let Some(ph) = play.player else { return };
            let Some(pa) = play.actors.actor(ph) else { return };
            let (ppos, pyaw) = (pa.world_pos, pa.shape_rot.y);
            let local = self.actor.world_to_actor_coords(ppos);
            if local.z > -50.0 && local.z < 0.0 && local.y.abs() < 10.0 && local.x.abs() < 20.0 && self.actor.player_is_facing(pyaw, 0x3000) {
                let actor = self.actor.clone();
                offer_get_item(play, &actor, -self.get_item_id());
            }
            if play.flags.get_treasure(self.treasure_flag()) {
                self.action = Action::Open;
            }
        }
    }

    /// `EnBox_Open`: the opening plays out (the unlock and open sounds on frames 30 and 90
    /// aren't played), then `unk_1F4` counts the frames open up to ±120.
    fn open(&mut self) {
        self.actor.flags &= !ACTOR_FLAG_7;
        let Some(sk) = self.skel.as_mut() else { return };
        if sk.update() {
            if self.unk_1f4 > 0 {
                if self.unk_1f4 < 120 {
                    self.unk_1f4 += 1;
                } else {
                    eng_math::step_to_f(&mut self.unk_1b0, 0.0, 0.05);
                }
            } else if self.unk_1f4 > -120 {
                self.unk_1f4 -= 1;
            } else {
                eng_math::step_to_f(&mut self.unk_1b0, 0.0, 0.05);
            }
        } else if sk.joint_table.get(3).is_some_and(|j| j[2] > 0) {
            let z = sk.joint_table[3][2] as i32;
            self.unk_1b0 = ((0x7D00 - z) as f32 * 0.00006).clamp(0.0, 1.0);
        }
    }

    /// `EnBox_SpawnIceSmoke`: an ice trap's smoke (not drawn: its random numbers are).
    fn spawn_ice_smoke(&mut self, play: &mut PlayState) {
        self.ice_smoke_timer += 1;
        if play.rand.zero_one() < 0.3 {
            play.rand.zero_one();
            play.rand.zero_one();
        }
    }
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl ActorImpl for EnBox {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBox_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.movement_flags & ENBOX_MOVE_STICK_TO_GROUND != 0 {
            self.movement_flags &= !ENBOX_MOVE_STICK_TO_GROUND;
            self.clip_to_ground(play);
        }
        match self.action {
            Action::FallOnSwitchFlag => self.fall_on_switch_flag(play),
            Action::Fall => self.fall(play),
            Action::WaitSong => self.wait_song(play),
            Action::AppearOnSwitchFlag => self.appear_on_switch_flag(play),
            Action::AppearOnRoomClear => self.appear_on_room_clear(play),
            Action::AppearInit => self.appear_init(play),
            Action::AppearAnimation => self.appear_animation(play),
            Action::WaitOpen => self.wait_open(play),
            Action::Open => self.open(),
        }
        if self.movement_flags & ENBOX_MOVE_IMMOBILE == 0 {
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        }
        self.actor.set_focus(if self.is_small() { 20.0 } else { 40.0 });
        if self.get_item_id() == GI_ICE_TRAP && self.action == Action::Open && self.skel.as_ref().is_some_and(|s| s.cur_frame > 45.0) && self.ice_smoke_timer < 100 {
            self.spawn_ice_smoke(play);
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `EnBox_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if let Some(s) = &self.skel {
            rs.joints = Some(eng_anim::anim::JointTable { rot: s.joint_table.clone(), face: 0 });
        }
        rs.values = vec![self.alpha as f32];
        rs.switches = vec![(self.actor.flags & ACTOR_FLAG_7 != 0) as u32];
        rs
    }

    /// `EnBox_Draw`: opaque at full alpha (types 4 and 6: once the lens flag is gone), else
    /// translucent with the alpha as the env's. The lens-hidden chests aren't drawn (the lens
    /// isn't ported).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) else { return };
        let alpha = rs.values.first().copied().unwrap_or(0.0) as i32 as u8;
        let flag7 = rs.switches.first().copied().unwrap_or(0) != 0;
        let lens_type = self.ty == ENBOX_TYPE_4 || self.ty == ENBOX_TYPE_6;
        let bones = skeleton.pose(joints);
        let m = actor_draw_matrix(rs);
        if (alpha == 255 && !lens_type) || (!flag7 && lens_type) {
            let name = if self.ty == ENBOX_TYPE_DECORATED_BIG { BAKE_OPA_BOSS_KEY } else { BAKE_OPA };
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(name)), transform: m, bones, params: DrawParams::default() });
        } else if alpha != 0 && !lens_type {
            let mut sv = SegmentValues::default();
            sv.env[SEG_ENV as usize] = Some([0, 0, 0, alpha]);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_XLU)), transform: m, bones, params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
        // (Types 4 and 6 with ACTOR_FLAG_7, BAKE_XLU_46's render mode: drawn by the lens only.)
        let _ = BAKE_XLU_46;
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
