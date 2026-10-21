//! `En_Jj` (`ovl_En_Jj/z_en_jj.c`): Lord Jabu-Jabu, in Zora's Fountain (spot08 room 0, facing -x:
//! -1 at (-1445, 20, -48) in a child's layers 0 and 1; at (-949, 69, -48) in layer 4, the
//! Sapphire's cutscene; two in the unused layers 5 and 6; none for an adult). Three types by
//! params (`EnJjType`):
//!
//! - **`JABUJABU_MAIN`** (-1): his head and drawn body, 0.087 big, a DynaPoly actor of
//!   `gJabuJabuHeadCol`, with `gJabuJabuSkel` playing `gJabuJabuAnim` looped (`NA_SE_EV_JABJAB_GROAN`
//!   on its frame 41). He spawns his body's collision as his child (a `JABUJABU_COLLISION`, 10
//!   along -x), and an OC cylinder (170 by 150, immovable) that he puts at (-1245, 20, -48) and
//!   sets for pushes while he waits for the fish.
//!   - **The fish** (`EnJj_WaitForFish`): Link within 300 across of the feeding spot (-1589, 53,
//!     -43) while `play->isPlayerDroppingFish` (`Player_IsDroppingFish`: the bottle's release with
//!     a fish, until its frame 76 empties the bottle): 100 frames (`EnJj_BeginCutscene`), then
//!     **`gJabuInhalingCs`** (`play->csCtx.script`, `cutsceneTrigger` 1), the body's collision off,
//!     the active camera told it's done (`Camera_SetFinishedFlag`), `EVENTCHKINF_OPENED_JABU_JABU`
//!     and the chime (`NA_SE_SY_CORRECT_CHIME`); he then waits on `EnJj_RemoveDust`. The script
//!     (1,629 frames; it ends at 345, `CS_DEST_JABU_JABU`, to `ENTR_JABU_JABU_0`) gives the fish
//!     (the `En_Fish` let out) channel 1 and him channel 2.
//!   - **His cues** (`EnJj_CutsceneUpdate`, while a cutscene runs with a cue on channel 2, instead
//!     of his action and the groan): cue 1 after cue 3 the eyes back to blinking at random; cue 2
//!     the breath (`unk_30A` bit 0: `NA_SE_EV_JABJAB_BREATHE` every frame and the mouth opening
//!     102 a frame to -5202) and, once, `Eff_Dust` type 0 at (-1100, 105, -27) as his child (bit
//!     3); cue 3 one blink and another (`extraBlinkCounter` 1, bit 1). The script's channel 2 is
//!     two lists, `CS_CMD_ACTOR_CUE_2_4`'s and `_2_5`'s (the later wins where both run: cue 1 to
//!     frame 90, cue 2 to 330, cue 3 to 380, then the first list's cue 1).
//!   - **Later visits** (`EVENTCHKINF_OPENED_JABU_JABU` set at init): `EnJj_WaitToOpenMouth` until
//!     Link is within 300 across of him, then `EnJj_OpenMouth`: 102 a frame to -5202, the body's
//!     collision off once past -2600 (the way into his mouth).
//!   - **His eyes** (`EnJj_Blink`): open, half, closed, a frame each, then open for 20 to 39
//!     frames (`Rand_S16Offset(20, 20)`; `extraBlinkCounter` more blinks first, `extraBlinkTotal`
//!     always 0).
//!   - **His mouth**: `mouthOpenAngle` is the head's z (`jointTable[10]`, `gJabuJabuHeadLimb`)
//!     after each `SkelAnime_Update`.
//! - **`JABUJABU_COLLISION`** (0): his body's collision, `gJabuJabuBodyCol` (its ceilings off:
//!   `DynaPoly_DisableCeilingCollision`), at his scale; no update (`EnJj_UpdateStaticCollision`),
//!   no draw.
//! - **`JABUJABU_UNUSED_COLLISION`** (1): `gJabuJabuUnusedCol` (shaped like a screen), the same;
//!   nothing spawns or places it.
//!
//! Drawn (`EnJj_Draw`, the main type's only) after `Gfx_SetupDL_37Opa` with the eye on segment 8
//! (`eyeTextures`), bobbing (`cosf(curFrame × π / 41) × 10 − 10` up, in his scale's units) and 10
//! times bigger (`Matrix_Scale(10, 10, 10)`): a bake per eye. No shadow (`ActorShape_Init` with
//! none).
//!
//! `EnJj_RemoveDust` (kill `dust` once, bit 2) never runs in play: the cutscene starts on the
//! update he asks for it and reaches its frame 1 there (`Cutscene_SetupScripted` and
//! `CutsceneHandler_StartScript` each run the script once), so from his next update channel 2 has
//! a cue until the script's terminator leaves the fountain; the dust goes with the scene.
//!
//! The whole overlay is ported, with its script (`z_en_jj_cutscene_data.inc.c`, read from the
//! overlay by the importer).
//!
//! Params: `type` the whole params, signed (`EnJjType`: -1 `JABUJABU_MAIN`, 0
//! `JABUJABU_COLLISION`, 1 `JABUJABU_UNUSED_COLLISION`); the fountain places -1, which spawns 0.
//! Flags: reads `EVENTCHKINF_OPENED_JABU_JABU` (0x3A; set at init: his mouth opens for Link) and
//! sets it as the inhaling cutscene starts.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::event_chk_inf;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{Anim, SkelAnimeStd};

use crate::eff_dust::{ACTOR_EFF_DUST, EFF_DUST_TYPE_0};

/// `ACTOR_EN_JJ` (`actor_table.h`: 0x005A).
pub const ACTOR_EN_JJ: i16 = 0x005A;
/// `OBJECT_JJ` (`object_table.h`: 0x0040).
pub const OBJECT: &str = "object_jj";

/// `En_Jj_Profile`: `ACTORCAT_ITEMACTION`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_JJ`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_JJ,
    name: "En_Jj",
    category: ACTORCAT_ITEMACTION,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `EnJjType` (`z_en_jj.h`).
pub const JABUJABU_MAIN: i16 = -1;
pub const JABUJABU_COLLISION: i16 = 0;
pub const JABUJABU_UNUSED_COLLISION: i16 = 1;

/// The whole params: the type (`switch (this->dyna.actor.params)`).
pub const TYPE: Field = Field::whole(
    "type",
    Kind::Named(&[
        (-1, "JABUJABU_MAIN: Lord Jabu-Jabu, his head and drawn body"),
        (0, "JABUJABU_COLLISION: his body's collision (he spawns it)"),
        (1, "JABUJABU_UNUSED_COLLISION: an unused collision shaped like a screen"),
    ]),
    "what it is",
);

/// `EVENTCHKINF_OPENED_JABU_JABU` (`save.h`: 0x3A).
pub const EVENTCHKINF_OPENED_JABU_JABU: u16 = 0x3A;

/// En_Jj's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_OPENED_JABU_JABU)).reads().sets().note("the main type: set as he inhales Link (the fish's cutscene); set at init, his mouth opens for Link")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `NA_SE_EV_JABJAB_BREATHE` (`environmentbank_table.h`: 0x286D).
pub const NA_SE_EV_JABJAB_BREATHE: u16 = 0x286D;
/// `NA_SE_EV_JABJAB_GROAN` (`environmentbank_table.h`: 0x28B6).
pub const NA_SE_EV_JABJAB_GROAN: u16 = 0x28B6;

/// The inhaling cutscene (`z_en_jj_cutscene_data.inc.c`).
pub const JABU_INHALING_CS: &str = "gJabuInhalingCs";

/// `EnJj_WaitForFish`'s `feedingSpot`.
pub const FEEDING_SPOT: Vec3 = Vec3::new(-1589.0, 53.0, -43.0);

/// `EnJjEyeState`.
pub const JABUJABU_EYE_OPEN: u8 = 0;
pub const JABUJABU_EYE_HALF: u8 = 1;
pub const JABUJABU_EYE_CLOSED: u8 = 2;
pub const JABUJABU_EYE_MAX: u8 = 3;

/// `EnJj_Draw`'s `eyeTextures`.
pub const EYE_TEXTURES: [&str; 3] = ["gJabuJabuEyeOpenTex", "gJabuJabuEyeHalfTex", "gJabuJabuEyeClosedTex"];
const SEG_EYES: u8 = 0x08;
const SEG_SETUP: u8 = 0x0C;

/// The head's joint (`jointTable[10]`: `gJabuJabuHeadLimb`, the skeleton's limb 9).
pub const HEAD_JOINT: usize = 10;

/// `unk_30A`'s bits: 1 breathing (cue 2), 2 a cue 3's blink set, 4 the dust looked for, 8 the dust
/// spawned.
pub const UNK_30A_BREATHING: u16 = 1;
pub const UNK_30A_BLINKED: u16 = 2;
pub const UNK_30A_DUST_REMOVED: u16 = 4;
pub const UNK_30A_DUST_SPAWNED: u16 = 8;

/// `sCylinderInit`: 170 by 150, hit by Player's `0x00000004` (never set for hits), pushing (OC)
/// every type.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0004, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 170, height: 150, y_shift: 0, pos: [0, 0, 0] },
};

/// The skeleton's bake with eye `eye` on segment 8, after `Gfx_SetupDL_37Opa`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Jj/eye{eye}")
}

/// The meshes `EnJj_Draw` draws: `gJabuJabuSkel` with each eye.
pub fn bakes() -> Vec<MeshBake> {
    EYE_TEXTURES
        .iter()
        .enumerate()
        .map(|(eye, tex)| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![(SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }), (SEG_SETUP, BakeSegment::Commands(crate::en_owl::setup_dl_37()))],
            prelude: vec![SEG_SETUP],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gJabuJabuSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc` (and `EnJj_UpdateStaticCollision`, the collision types' update).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnJj_UpdateStaticCollision`: the collision types; nothing.
    StaticCollision,
    WaitToOpenMouth,
    OpenMouth,
    WaitForFish,
    BeginCutscene,
    RemoveDust,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::StaticCollision => "EnJj_UpdateStaticCollision",
            Action::WaitToOpenMouth => "EnJj_WaitToOpenMouth",
            Action::OpenMouth => "EnJj_OpenMouth",
            Action::WaitForFish => "EnJj_WaitForFish",
            Action::BeginCutscene => "EnJj_BeginCutscene",
            Action::RemoveDust => "EnJj_RemoveDust",
        }
    }
}

pub struct EnJj {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    /// The main type's skeleton (`None` for the collision types, which have none).
    pub skel: Option<SkelAnimeStd>,
    skeleton: Option<Arc<Skeleton>>,
    pub collider: ColliderCylinder,
    pub action: Action,
    /// `bodyCollisionActor`: the `JABUJABU_COLLISION` he spawned.
    pub body_collision_actor: Option<ActorHandle>,
    /// `dust`: the `Eff_Dust` cue 2 spawned.
    pub dust: Option<ActorHandle>,
    pub mouth_open_angle: i16,
    pub unk_30a: u16,
    pub cutscene_countdown_timer: i16,
    pub eye_index: u8,
    pub blink_timer: u8,
    pub extra_blink_counter: u8,
    pub extra_blink_total: u8,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `Math_Vec3f_DistXZ`.
fn vec3f_dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

impl EnJj {
    /// The type (`this->dyna.actor.params`).
    pub fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    fn load_collision(play: &PlayState, symbol: &str) -> Option<Arc<CollisionHeader>> {
        crate::obj_kibako2::load_collision(play, OBJECT, symbol)
    }

    /// `DynaPolyActor_Init(&this->dyna, 0)`, `CollisionHeader_GetVirtual(symbol)`,
    /// `DynaPoly_SetBgActor`.
    fn set_bg_actor(&mut self, play: &mut PlayState, symbol: &str) {
        if let Some(h) = Self::load_collision(play, symbol) {
            self.bg = play.col.dyna.set_bg_actor(h, source(&self.actor), 0);
        }
    }

    /// `EnJj_Init`: 0.087 big and the culling volume (`sInitChain`), no shadow; then by type. The
    /// main: the skeleton looped, the eyes open, waiting for the fish (or, with
    /// `EVENTCHKINF_OPENED_JABU_JABU`, to open his mouth); his body's collision spawned as his
    /// child 10 along -x; `gJabuJabuHeadCol`; the cylinder, immovable. The collision types:
    /// their collision (the body's without ceilings), no update, no draw.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnJj {
            actor,
            bg: BG_ACTOR_MAX,
            skel: None,
            skeleton: None,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            action: Action::StaticCollision,
            body_collision_actor: None,
            dust: None,
            mouth_open_angle: 0,
            unk_30a: 0,
            cutscene_countdown_timer: 0,
            eye_index: 0,
            blink_timer: 0,
            extra_blink_counter: 0,
            extra_blink_total: 0,
        };
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 87), cullingVolumeDistance 4000,
        // cullingVolumeScale 3300, cullingVolumeDownward 1100.
        this.actor.scale = Vec3::splat(0.087);
        this.actor.culling_volume_distance = 4000.0;
        this.actor.culling_volume_scale = 3300.0;
        this.actor.culling_volume_downward = 1100.0;
        // ActorShape_Init(&shape, 0.0f, NULL, 0.0f).
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::None, 0.0);
        match this.ty() {
            JABUJABU_MAIN => {
                let loaded = play.assets.clone().ok_or_else(|| anyhow::anyhow!("no assets")).and_then(|a| -> anyhow::Result<(Arc<Skeleton>, Anim)> {
                    Ok((a.skeleton(OBJECT, "gJabuJabuSkel")?, a.animation(OBJECT, "gJabuJabuAnim")?))
                });
                match loaded {
                    Ok((skeleton, anim)) => {
                        // SkelAnime_InitFlex(.., &gJabuJabuSkel, &gJabuJabuAnim, .., 22), then
                        // Animation_PlayLoop(&gJabuJabuAnim).
                        this.skel = Some(SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim)));
                        this.skeleton = Some(skeleton);
                    }
                    Err(e) => log::error!("En_Jj: {e:#}"),
                }
                this.action = if play.save.get_event_chk_inf(EVENTCHKINF_OPENED_JABU_JABU) { Action::WaitToOpenMouth } else { Action::WaitForFish };
                let p = this.actor.world_pos;
                let rot_y = this.actor.world_rot.y;
                match play.actor_spawn_as_child(&mut this.actor, ACTOR_EN_JJ, Vec3::new(p.x - 10.0, p.y, p.z), [0, rot_y, 0], JABUJABU_COLLISION) {
                    Ok(h) => this.body_collision_actor = Some(h),
                    Err(e) => log::warn!("En_Jj: the body's collision not spawned: {e:?}"),
                }
                this.set_bg_actor(play, "gJabuJabuHeadCol");
                // Collider_InitCylinder, Collider_SetCylinder: sCylinderInit, done above.
                this.actor.col_chk_info.mass = MASS_IMMOVABLE;
            }
            JABUJABU_COLLISION => {
                this.set_bg_actor(play, "gJabuJabuBodyCol");
                play.col.dyna.disable_ceiling_collision(this.bg);
                // this->dyna.actor.update = EnJj_UpdateStaticCollision; draw = NULL;
                // Actor_SetScale(0.087f): the scale it has.
            }
            JABUJABU_UNUSED_COLLISION => {
                this.set_bg_actor(play, "gJabuJabuUnusedCol");
            }
            _ => {}
        }
        Box::new(this)
    }

    /// The bg id of his body's collision (`bodyCollisionActor->bgId`).
    fn body_bg(&self, play: &PlayState) -> Option<u16> {
        self.body_collision_actor.and_then(|h| play.actors.get(h)).and_then(|a| a.dyna_bg_id())
    }

    /// `DynaPoly_DisableCollision(.., bodyCollisionActor->bgId)`.
    fn disable_body_collision(&self, play: &mut PlayState) {
        if let Some(bg) = self.body_bg(play) {
            play.col.dyna.set_collision_disabled(bg, true);
        }
    }

    /// `EnJj_Blink`: at the timer's end the eye on a step (open, half, closed); back open, another
    /// blink for each `extraBlinkCounter`, else 20 to 39 frames open.
    fn blink(&mut self, play: &mut PlayState) {
        if self.blink_timer > 0 {
            self.blink_timer -= 1;
        } else {
            self.eye_index += 1;
            if self.eye_index >= JABUJABU_EYE_MAX {
                self.eye_index = JABUJABU_EYE_OPEN;
                if self.extra_blink_counter > 0 {
                    self.extra_blink_counter -= 1;
                } else {
                    self.blink_timer = play.rand.s16_offset(20, 20) as u8;
                    self.extra_blink_counter = self.extra_blink_total;
                }
            }
        }
    }

    /// `EnJj_OpenMouth`: 102 a frame while at -5200 or more; the body's collision off once past
    /// -2600.
    fn open_mouth(&mut self, play: &mut PlayState) {
        if self.mouth_open_angle >= -5200 {
            self.mouth_open_angle -= 102;
            if self.mouth_open_angle < -2600 {
                self.disable_body_collision(play);
            }
        }
    }

    /// `EnJj_WaitToOpenMouth`: Link within 300 across.
    fn wait_to_open_mouth(&mut self) {
        if self.actor.xz_dist_to_player < 300.0 {
            self.action = Action::OpenMouth;
        }
    }

    /// `Player_IsDroppingFish` through `play->isPlayerDroppingFish`.
    fn is_player_dropping_fish(play: &PlayState) -> bool {
        play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).is_some_and(|p| p.is_dropping_fish())
    }

    /// `EnJj_WaitForFish`: Link within 300 across of the feeding spot letting a fish out: 100
    /// frames to the cutscene. The cylinder at (-1245, 20, -48), set for pushes.
    fn wait_for_fish(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos).unwrap_or_default();
        if vec3f_dist_xz(FEEDING_SPOT, player_pos) < 300.0 && Self::is_player_dropping_fish(play) {
            self.cutscene_countdown_timer = 100;
            self.action = Action::BeginCutscene;
        }
        self.collider.dim.pos = [-1245, 20, -48];
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `EnJj_BeginCutscene`: the countdown; at its end `gJabuInhalingCs`, the body's collision
    /// off, the camera done, `EVENTCHKINF_OPENED_JABU_JABU`, the chime.
    fn begin_cutscene(&mut self, play: &mut PlayState) {
        if self.cutscene_countdown_timer > 0 {
            self.cutscene_countdown_timer -= 1;
        } else {
            self.action = Action::RemoveDust;
            play.cs_ctx.segment = play.cutscene_script(JABU_INHALING_CS);
            play.save.cutscene_trigger = 1;
            self.disable_body_collision(play);
            let cam = play.active_cam_id;
            play.camera_set_finished_flag(cam);
            play.save.set_event_chk_inf(EVENTCHKINF_OPENED_JABU_JABU);
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
        }
    }

    /// `EnJj_CutsceneUpdate`: his cue on channel 2 (see the module's docs), then the breath.
    fn cutscene_update(&mut self, play: &mut PlayState, cue: CsCmdActorCue) {
        match cue.action {
            1 => {
                if self.unk_30a & UNK_30A_BLINKED != 0 {
                    self.eye_index = 0;
                    self.blink_timer = play.rand.s16_offset(20, 20) as u8;
                    self.extra_blink_counter = 0;
                    self.extra_blink_total = 0;
                    self.unk_30a ^= UNK_30A_BLINKED;
                }
            }
            2 => {
                self.unk_30a |= UNK_30A_BREATHING;
                if self.unk_30a & UNK_30A_DUST_SPAWNED == 0 {
                    // Actor_SpawnAsChild(.., ACTOR_EFF_DUST, -1100, 105, -27, 0, 0, 0, EFF_DUST_TYPE_0):
                    // NULL when it fails.
                    self.dust = play.actor_spawn_as_child(&mut self.actor, ACTOR_EFF_DUST, Vec3::new(-1100.0, 105.0, -27.0), [0; 3], EFF_DUST_TYPE_0).ok();
                    self.unk_30a |= UNK_30A_DUST_SPAWNED;
                }
            }
            3 => {
                if self.unk_30a & UNK_30A_BLINKED == 0 {
                    self.eye_index = 0;
                    self.blink_timer = 0;
                    self.extra_blink_counter = 1;
                    self.extra_blink_total = 0;
                    self.unk_30a |= UNK_30A_BLINKED;
                }
            }
            _ => {}
        }
        if self.unk_30a & UNK_30A_BREATHING != 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_JABJAB_BREATHE - SFX_FLAG);
            if self.mouth_open_angle >= -5200 {
                self.mouth_open_angle -= 102;
            }
        }
    }

    /// `EnJj_RemoveDust`: once (bit 2), the dust killed if there is one, and `child` cleared.
    fn remove_dust(&mut self, play: &mut PlayState) {
        if self.unk_30a & UNK_30A_DUST_REMOVED == 0 {
            self.unk_30a |= UNK_30A_DUST_REMOVED;
            if let Some(dust) = self.dust {
                if let Some(d) = play.actors.actor_mut(dust) {
                    d.kill();
                }
                self.actor.child = None;
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::StaticCollision => {}
            Action::WaitToOpenMouth => self.wait_to_open_mouth(),
            Action::OpenMouth => self.open_mouth(play),
            Action::WaitForFish => self.wait_for_fish(play),
            Action::BeginCutscene => self.begin_cutscene(play),
            Action::RemoveDust => self.remove_dust(play),
        }
    }

    /// `EnJj_Draw`'s bob: `cosf(curFrame × (M_PI / 41)) × 10 − 10`.
    pub fn bob(cur_frame: f32) -> f32 {
        ((cur_frame * (std::f32::consts::PI / 41.0)).cos() * 10.0) - 10.0
    }
}

impl ActorImpl for EnJj {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnJj_Update` (the collision types: `EnJj_UpdateStaticCollision`, nothing): in a cutscene
    /// with a cue on channel 2, his cue; else his action and, on the animation's frame 41, the
    /// groan. Then the blink, the animation, the scale, and the head's z the mouth's angle.
    fn update(&mut self, play: &mut PlayState) {
        if self.action == Action::StaticCollision {
            return;
        }
        match play.cs_ctx.npc_actions[2] {
            Some(cue) if play.cs_ctx.state != CS_STATE_IDLE => self.cutscene_update(play, cue),
            _ => {
                self.run_action(play);
                if self.skel.as_ref().is_some_and(|s| s.cur_frame == 41.0) {
                    audio_play_actor_sfx2(play, NA_SE_EV_JABJAB_GROAN);
                }
            }
        }
        self.blink(play);
        if let Some(s) = &mut self.skel {
            s.update();
        }
        self.actor.scale = Vec3::splat(0.087);
        // Head.
        if let Some(j) = self.skel.as_mut().and_then(|s| s.joint_table.get_mut(HEAD_JOINT)) {
            j[2] = self.mouth_open_angle;
        }
    }

    /// `EnJj_Destroy`: the collision (and the main type's cylinder, nothing to free).
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        if let Some(s) = &self.skel {
            r.joints = Some(eng_anim::anim::JointTable { rot: s.joint_table.clone(), face: 0 });
            r.values = vec![Self::bob(s.cur_frame)];
        }
        r.switches = vec![self.eye_index.min(JABUJABU_EYE_CLOSED) as u32];
        r
    }

    /// `EnJj_Draw` (the main type's; the collision types have none): `Gfx_SetupDL_37Opa`, the eye
    /// on segment 8, the bob, ×10, the skeleton (`SkelAnime_DrawFlexOpa`, no overrides).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints), [bob], [eye]) = (&self.skeleton, &rs.joints, rs.values.as_slice(), rs.switches.as_slice()) else { return };
        let bones = skeleton.pose(joints);
        let m = actor_draw_matrix(rs) * Mat4::from_translation(Vec3::new(0.0, *bob, 0.0)) * Mat4::from_scale(Vec3::splat(10.0));
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(*eye as usize))), transform: m, bones, params: Default::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0 && self.ty() == JABUJABU_MAIN).then_some(ColliderMut::Cylinder(&mut self.collider))
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
    fn the_bob_is_level_at_the_loop_s_start_and_20_down_at_frame_41() {
        assert_eq!(EnJj::bob(0.0), 0.0);
        assert!((EnJj::bob(41.0) + 20.0).abs() < 1e-4);
    }
}
