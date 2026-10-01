//! `Bg_Treemouth` (`ovl_Bg_Treemouth/z_bg_treemouth.c`): the Great Deku Tree's mouth in Kokiri
//! Forest's room 1. It's a DynaPoly actor (`gDekuTreeMouthCol` from `object_spot04_objects`)
//! that slides from its closed place, blocking the way in, down and forward to its open one,
//! along `unk_168` (0 closed, 1 open): `BgTreemouth_Update` puts it at
//! `(4029, 136, -1255) + unk_168 * (-160, -399, 92)` every frame.
//!
//! What opens it is the story (`func_808BC8B8`):
//! - Link comes near and faces the tree the first time: `EVENTCHKINF_0C` is set and the talk's
//!   cutscene starts (`D_808BCE20`);
//! - with `EVENTCHKINF_0C`, Z-targeting the tree asks again (`D_808BD2A0`);
//! - in either cutscene, answering yes sets `EVENTCHKINF_05` and cues the mouth open
//!   (`func_808BC9EC`, then `D_808BD520`); no replays `D_808BD790`;
//! - with `EVENTCHKINF_05` the mouth is held open.
//!
//! The scripts are the overlay's four `CutsceneData` arrays (`z_bg_treemouth_cutscene_data.c`),
//! read from the pack (docs/adr/0022-cutscenes.md) and played by `oot_game::cutscene`. The first
//! talk (`D_808BCE20`) walks Link in, says 0x107D, asks 0x1015 and ends (`CS_MISC` 12 at frame
//! 180, `CS_STATE_UNSKIPPABLE_INIT`); `func_808BC9EC` then goes straight on with the answer's
//! script: yes (`D_808BD520`) says 0x1017 and cues the mouth open (cue 3 from frame 20), no
//! (`D_808BD790`) says 0x1018.
//!
//! Also not ported: the scene layer 6 cutscene's falling bark (`EffectSsHahen_SpawnBurst`; its
//! `Rand_ZeroOne` calls are made), and the cull zone.
//!
//! The draw is `gDekuTreeMouthDL` with an env alpha of 500 or, with `EVENTCHKINF_07` (the tree
//! dead), 2150, times 0.1. The list's combiner blends its two textures by that alpha
//! (`(TEXEL1 - TEXEL0) * ENV_ALPHA + TEXEL0`), as the scene draw config's segment 0x0B does
//! for the tree. It's a bake with the env colour on dynamic segment 0x0B
//! (docs/adr/0018-bg-treemouth.md).

use std::sync::Arc;

use anyhow::Result;
use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_gfx::{DrawCmd, MeshKey, SegmentValues};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_0, ACTOR_FLAG_4, ACTOR_FLAG_5, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile};
use oot_game::cutscene::{CS_STATE_IDLE, CS_STATE_SKIPPABLE_EXEC, CS_STATE_UNSKIPPABLE_INIT};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::{EVENTCHKINF_0C, EVENTCHKINF_05, EVENTCHKINF_07};

/// `ACTOR_BG_TREEMOUTH` (`actor_table.h`: 0x003E).
pub const ACTOR_BG_TREEMOUTH: i16 = 0x003E;

pub const OBJECT: &str = "object_spot04_objects";
pub const COLLISION: &str = "gDekuTreeMouthCol";
pub const DISPLAY_LIST: &str = "gDekuTreeMouthDL";

/// `Bg_Treemouth_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_TREEMOUTH, name: "Bg_Treemouth", category: ACTORCAT_BG, flags: ACTOR_FLAG_4 | ACTOR_FLAG_5, object: OBJECT };

/// `DPM_UNK`: the mouth carries nothing standing on it.
const DPM_UNK: u32 = 0;

/// `sInitChain`: `ICHAIN_U8(targetMode, 5)`, `ICHAIN_VEC3F(scale, 1)` (the cull zone isn't
/// ported).
const TARGET_MODE: u8 = 5;

/// `BgTreemouth_Update`: the mouth's place at `unk_168` 0, and the way it moves to 1.
const CLOSED_POS: Vec3 = Vec3::new(4029.0, 136.0, -1255.0);
const OPEN_OFFSET: Vec3 = Vec3::new(-160.0, -399.0, 92.0);

/// `func_808BC8B8`: how near (`Actor_IsFacingAndNearPlayer`'s range) and how squarely Link
/// must face the tree (0x4E20 the first time, 0x7530 with `EVENTCHKINF_0C`).
const NEAR_RANGE: f32 = 1658.0;
const FACING_FIRST: i16 = 0x4E20;
const FACING_AGAIN: i16 = 0x7530;

/// `func_808BC9EC`: Player's place when the cutscene starts with him within 350.
const CS_START_RANGE: f32 = 350.0;
const CS_START_PLAYER_POS: Vec3 = Vec3::new(3827.0, -161.0, -1142.0);

/// `BgTreemouth_Draw`'s env alpha, `u16 alpha = 500` or 2150 with the tree dead.
const ALPHA_ALIVE: u16 = 500;
const ALPHA_DEAD: u16 = 2150;

/// The segment the bake takes the env colour from (docs/adr/0018-bg-treemouth.md).
const SEG_ENV: u8 = 0x0B;
const BAKE: &str = "Bg_Treemouth/mouth";

/// `gDekuTreeMouthCol`.
pub fn load_collision(play: &PlayState) -> Result<Arc<CollisionHeader>> {
    let assets = play.assets.as_ref().ok_or_else(|| anyhow::anyhow!("no asset pack"))?;
    Ok(Arc::new(assets.pack.collision(&keys::collision(OBJECT, COLLISION))?))
}

/// `BgTreemouth_Draw`'s mesh: `Gfx_SetupDL_25Opa`, the env colour, `gDekuTreeMouthDL`.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false })],
        prelude: vec![SEG_ENV],
        body: BakeBody::DLists(vec![(OBJECT.into(), DISPLAY_LIST.into())]),
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_808BC8B8`: waiting for Link, or held open with `EVENTCHKINF_05`.
    Wait,
    /// `func_808BC9EC`: a cutscene was asked for; waiting for it to start.
    WaitCsStart,
    /// `func_808BCAF0`: in the yes cutscene, waiting for the mouth's cue.
    WaitCsCue,
    /// `func_808BC65C`: in a cutscene layer, waiting for the mouth's cue.
    WaitLayerCsCue,
    /// `func_808BC80C`: opening a little (cue 2), as the tree talks.
    TalkOpen,
    /// `func_808BC864`: closing again, then back to `func_808BC65C`.
    TalkClose,
    /// `func_808BC6F8`: opening all the way (cue 3).
    Open,
    /// `BgTreemouth_DoNothing`.
    DoNothing,
}

impl Action {
    /// The decomp's name for the action function.
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Wait => "func_808BC8B8",
            Action::WaitCsStart => "func_808BC9EC",
            Action::WaitCsCue => "func_808BCAF0",
            Action::WaitLayerCsCue => "func_808BC65C",
            Action::TalkOpen => "func_808BC80C",
            Action::TalkClose => "func_808BC864",
            Action::Open => "func_808BC6F8",
            Action::DoNothing => "BgTreemouth_DoNothing",
        }
    }
}

pub struct BgTreemouth {
    /// `dyna.actor`.
    pub actor: Actor,
    /// `dyna.bgId`.
    pub bg: u16,
    /// `unk_168`: how far open, 0 to 1.
    pub unk_168: f32,
    pub action: Action,
}

/// `IS_CUTSCENE_LAYER` (`z64save.h`: `sceneLayer >= SCENE_LAYER_CUTSCENE_FIRST`, 4).
fn is_cutscene_layer(play: &PlayState) -> bool {
    play.save.scene_layer >= 4
}

/// `Actor_IsFacingAndNearPlayer` (`z_actor.c`).
pub fn actor_is_facing_and_near_player(actor: &Actor, range: f32, max_angle: i16) -> bool {
    let yaw_diff = actor.yaw_towards_player.wrapping_sub(actor.shape_rot.y);
    if (yaw_diff as i32).abs() < max_angle as i32 {
        let d = (actor.xz_dist_to_player * actor.xz_dist_to_player + actor.y_dist_to_player * actor.y_dist_to_player).sqrt();
        if d < range {
            return true;
        }
    }
    false
}

impl BgTreemouth {
    /// `BgTreemouth_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // Actor_ProcessInitChain(sInitChain).
        actor.target_mode = TARGET_MODE;
        actor.scale = Vec3::ONE;
        // DynaPolyActor_Init(DPM_UNK), CollisionHeader_GetVirtual(&gDekuTreeMouthCol),
        // DynaPoly_SetBgActor.
        let bg = match load_collision(play) {
            Ok(h) => play.col.dyna.set_bg_actor(h, source(&actor), DPM_UNK),
            Err(e) => {
                log::error!("Bg_Treemouth: {e:#}");
                BG_ACTOR_MAX
            }
        };
        // ActorShape_Init(&shape, 0.0f, NULL, 0.0f), Actor_SetFocus(50).
        actor.shape_y_offset = 0.0;
        actor.set_focus(50.0);
        let mut m = BgTreemouth { actor, bg, unk_168: 0.0, action: Action::Wait };
        let adult = play.save.adult;
        if !is_cutscene_layer(play) && !adult {
            m.action = Action::Wait;
        } else if adult || play.save.scene_layer == 7 {
            m.unk_168 = 0.0;
            m.action = Action::DoNothing;
        } else {
            m.unk_168 = 1.0;
            m.action = Action::Open;
        }
        m.actor.text_id = 0x905;
        Box::new(m)
    }

    /// `func_808BC65C` and `func_808BCAF0`: the cutscene's cue for the mouth (`npcActions[0]`),
    /// 2 to talk and 3 to open.
    fn wait_cs_cue(&mut self, play: &mut PlayState) {
        if play.cs_ctx.state != CS_STATE_IDLE
            && let Some(cue) = play.cs_ctx.npc_actions[0]
        {
            if cue.action == 2 {
                self.action = Action::TalkOpen;
            } else if cue.action == 3 {
                play.audio.func_80078884(oot_game::audio::sfx::NA_SE_EV_WOODDOOR_OPEN);
                self.action = Action::Open;
            }
        }
    }

    /// `func_808BC6F8`.
    fn open(&mut self, play: &mut PlayState) {
        if self.unk_168 < 1.0 {
            self.unk_168 += 0.01;
        } else {
            self.unk_168 = 1.0;
        }
        if play.save.scene_layer == 6 && play.cs_ctx.frames >= 0x2BD && play.gameplay_frames.is_multiple_of(8) {
            // The bark falling as the tree dies: sp34 = (Rand_ZeroOne() * 1158 + 3407, 970,
            // Rand_ZeroOne() * 2026 - 2163), EffectSsHahen_SpawnBurst(...): effects aren't ported.
            let _x = play.rand.zero_one() * 1158.0 + 3407.0;
            let _z = play.rand.zero_one() * 2026.0 + -2163.0;
        }
    }

    /// `func_808BC80C`.
    fn talk_open(&mut self) {
        self.unk_168 += 0.05;
        if self.unk_168 >= 0.8 {
            self.action = Action::TalkClose;
        }
    }

    /// `func_808BC864`.
    fn talk_close(&mut self) {
        self.unk_168 -= 0.03;
        if self.unk_168 <= 0.0 {
            self.action = Action::WaitLayerCsCue;
        }
    }

    /// `func_808BC8B8`.
    fn wait(&mut self, play: &mut PlayState) {
        let adult = play.save.adult;
        if !play.save.get_event_chk_inf(EVENTCHKINF_05) || adult {
            if !adult {
                if play.save.get_event_chk_inf(EVENTCHKINF_0C) {
                    if actor_is_facing_and_near_player(&self.actor, NEAR_RANGE, FACING_AGAIN) {
                        self.actor.flags |= ACTOR_FLAG_0;
                        if self.actor.is_targeted {
                            self.actor.flags &= !ACTOR_FLAG_0;
                            play.cs_ctx.segment = play.cutscene_script("D_808BD2A0");
                            play.save.cutscene_trigger = 1;
                            self.action = Action::WaitCsStart;
                        }
                    }
                } else if actor_is_facing_and_near_player(&self.actor, NEAR_RANGE, FACING_FIRST) {
                    play.save.set_event_chk_inf(EVENTCHKINF_0C);
                    play.cs_ctx.segment = play.cutscene_script("D_808BCE20");
                    play.save.cutscene_trigger = 1;
                    self.action = Action::WaitCsStart;
                }
            }
        } else {
            self.unk_168 = 1.0;
        }
    }

    /// `func_808BC9EC`: when the talk's cutscene starts (`CS_STATE_UNSKIPPABLE_INIT`), Player
    /// is moved to the tree if he's near, and the answer to the tree's question picks the next
    /// script: yes (`choiceIndex` 0) sets `EVENTCHKINF_05` and plays the opening
    /// (`D_808BD520`), no replays `D_808BD790` and waits again.
    fn wait_cs_start(&mut self, play: &mut PlayState) {
        if play.cs_ctx.state != CS_STATE_UNSKIPPABLE_INIT {
            return;
        }
        if actor_is_facing_and_near_player(&self.actor, CS_START_RANGE, FACING_AGAIN)
            && let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h))
        {
            p.world_pos = CS_START_PLAYER_POS;
        }
        let cs = &mut play.cs_ctx;
        cs.frames = 0;
        cs.unk_18 = 0xFFFF;
        cs.unk_1a = 0;
        cs.unk_1b = 0;
        cs.state = CS_STATE_SKIPPABLE_EXEC;
        play.demo.d_8015fcc0 = 0xFFFF;
        play.demo.d_8015fcc2 = 0xFFFF;
        play.demo.d_8015fcc4 = 0xFFFF;
        if play.msg_ctx.choice_index == 0 {
            play.cs_ctx.segment = play.cutscene_script("D_808BD520");
            play.save.set_event_chk_inf(EVENTCHKINF_05);
            self.action = Action::WaitCsCue;
        } else {
            play.cs_ctx.segment = play.cutscene_script("D_808BD790");
            play.cs_ctx.frames = 0;
            self.action = Action::Wait;
        }
    }

    /// The transform `DynaPoly_UpdateContext` reads.
    fn source(&self) -> BgActorSource {
        source(&self.actor)
    }

    /// `BgTreemouth_Draw`'s alpha before the `* 0.1f`.
    pub fn draw_alpha(play: &PlayState) -> u16 {
        let mut alpha = ALPHA_ALIVE;
        if (!is_cutscene_layer(play) || play.save.adult) && play.save.get_event_chk_inf(EVENTCHKINF_07) {
            alpha = ALPHA_DEAD;
        }
        if play.save.scene_layer == 6 {
            let unk_74 = play.scene.as_ref().map(|s| s.draw.room_unk_74[0]).unwrap_or(0);
            alpha = (unk_74 as i32 + 0x1F4) as u16;
        }
        alpha
    }
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl ActorImpl for BgTreemouth {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `BgTreemouth_Update`: the action, then the place from `unk_168`, which
    /// `DynaPoly_UpdateContext` reads after the BG category.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Wait => self.wait(play),
            Action::WaitCsStart => self.wait_cs_start(play),
            Action::WaitCsCue | Action::WaitLayerCsCue => self.wait_cs_cue(play),
            Action::TalkOpen => self.talk_open(),
            Action::TalkClose => self.talk_close(),
            Action::Open => self.open(play),
            Action::DoNothing => {}
        }
        let t = self.unk_168;
        self.actor.world_pos = Vec3::new(t * OPEN_OFFSET.x + CLOSED_POS.x, t * OPEN_OFFSET.y + CLOSED_POS.y, t * OPEN_OFFSET.z + CLOSED_POS.z);
        play.col.dyna.set_source(self.bg, self.source());
    }
    /// `BgTreemouth_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }
    /// `BgTreemouth_Draw`: `gDPSetEnvColor(128, 128, 128, alpha * 0.1f)`, then
    /// `gDekuTreeMouthDL` at `Actor_Draw`'s matrix.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let a = (Self::draw_alpha(play) as f32 * 0.1) as u32 as u8;
        let mut sv = SegmentValues::default();
        sv.env[SEG_ENV as usize] = Some([128, 128, 128, a]);
        let key = MeshKey::named(keys::bake(BAKE));
        out.opa.push(DrawCmd { mesh: key, transform: oot_game::play::actor_draw_matrix(rs), bones: Vec::new(), params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } });
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
