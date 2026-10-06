//! `En_Si` (`ovl_En_Si/z_en_si.c`): the Gold Skulltula Token a killed Gold Skulltula (`En_Sw`)
//! leaves, spinning where it died. Params: the Gold Skulltula's (bits 8..12 the flags' index,
//! 0..7 its flag).
//!
//! - **Spinning** (`func_80AFB768`): it grows from 0.025 to 0.25 and turns by 0x400 a frame.
//!   Touched by Link (`OC2_HIT_PLAYER`, outside a cutscene), it's collected: `Item_Give`
//!   (`ITEM_SKULL_TOKEN`: one more token, the quest item), Link frozen 10 frames, text 0xB4
//!   ("You got a Gold Skulltula Token! You've collected N tokens"), the small item fanfare.
//! - **Hookshotted** (`func_80AFB89C`): spinning while the hookshot pulls it; let go, collected.
//! - **Collected** (`func_80AFB950`): undrawn, Link kept frozen while the text shows; as it
//!   closes, its flag is set (`SET_GS_FLAGS`, so its Gold Skulltula no longer spawns) and it's
//!   gone.
//!
//! The whole overlay is ported. Its draw is `GetItem_Draw(GID_SKULL_TOKEN_2)`
//! (`GetItem_DrawSkullToken`, baked with the get-item models); `func_8002ED80` and
//! `func_8002EBCC` only set the look-at for the texgen shine, which the renderer takes from the
//! view. The hookshot isn't ported, so `ACTOR_FLAG_HOOKSHOT_ATTACHED` is never set.

use eng_collision::math3d::Cylinder16;
use eng_math::smooth_step_to_f;
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorImpl, ActorProfile};
use oot_game::audio::NA_BGM_SMALL_ITEM_GET;
use oot_game::collision_check::*;
use oot_game::item::{ITEM_SKULL_TOKEN, item_give};
use oot_game::message::TEXT_STATE_CLOSING;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

pub const ACTOR_EN_SI: i16 = 0x019C;
const OBJECT: &str = "object_st";

/// `ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR`, `ACTOR_FLAG_HOOKSHOT_ATTACHED` (`actor.h`).
const ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR: u32 = 1 << 9;
const ACTOR_FLAG_HOOKSHOT_ATTACHED: u32 = 1 << 13;

/// `En_Si_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_SI, name: "En_Si", category: ACTORCAT_ITEMACTION, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOOKSHOT_PULLS_ACTOR, object: OBJECT };

/// `GID_SKULL_TOKEN_2` (`item.h`): the token in `object_st` (`gSkulltulaTokenDL`,
/// `gSkulltulaTokenFlameDL`).
pub const GID_SKULL_TOKEN_2: i16 = 0x74;

/// The token's text (`Message_StartTextbox(play, 0xB4, NULL)`).
pub const TEXT_ID_TOKEN: u16 = 0xB4;

/// `sCylinderInit`: touched by Link and hit by his attacks (`0x00000090`), hookable; 20 by 18,
/// 2 up.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit {
        col_type: COL_MATERIAL_NONE,
        at_flags: AT_NONE,
        ac_flags: AC_ON | AC_TYPE_PLAYER,
        oc_flags1: OC1_ON | OC1_NO_PUSH | OC1_TYPE_ALL,
        oc_flags2: OC2_TYPE_1,
        shape: COLSHAPE_CYLINDER,
    },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0090, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 18, y_shift: 2, pos: [0, 0, 0] },
};

/// `D_80AFBADC`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80AFB768`: spinning, waiting for Link.
    Spin,
    /// `func_80AFB89C`: pulled by the hookshot.
    Hookshotted,
    /// `func_80AFB950`: collected, its text showing.
    Collected,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Spin => "func_80AFB768",
            Action::Hookshotted => "func_80AFB89C",
            Action::Collected => "func_80AFB950",
        }
    }
}

pub struct EnSi {
    pub actor: Actor,
    pub action: Action,
    pub collider: ColliderCylinder,
    pub unk_19c: u8,
}

impl EnSi {
    /// `EnSi_Init`.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        actor.scale = Vec3::splat(0.025);
        actor.shape_y_offset = 42.0;
        Box::new(EnSi { actor, action: Action::Spin, collider, unk_19c: 0 })
    }

    /// `func_80AFB748`: a hit does nothing.
    fn func_80afb748(&mut self) -> bool {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
        }
        false
    }

    /// `Item_Give(play, ITEM_SKULL_TOKEN)`, `player->actor.freezeTimer = 10`,
    /// `Message_StartTextbox(play, 0xB4, NULL)`, `Audio_PlayFanfare(NA_BGM_SMALL_ITEM_GET)`:
    /// collected (`func_80AFB768`'s and `func_80AFB89C`'s).
    fn collect(&mut self, play: &mut PlayState) {
        item_give(&mut play.save, Some(&mut play.audio), ITEM_SKULL_TOKEN);
        freeze_player(play);
        play.start_textbox(TEXT_ID_TOKEN, None);
        play.audio.play_fanfare(NA_BGM_SMALL_ITEM_GET);
        self.action = Action::Collected;
    }

    /// `Math_SmoothStepToF(&scale.x, 0.25f, 0.4f, 1.0f, 0.0f)`, `Actor_SetScale`, and a turn of
    /// 0x400.
    fn grow_and_spin(&mut self) {
        smooth_step_to_f(&mut self.actor.scale.x, 0.25, 0.4, 1.0, 0.0);
        self.actor.scale = Vec3::splat(self.actor.scale.x);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x400);
    }

    /// `func_80AFB768`: hookshotted, pulled; else growing and spinning, and (outside a
    /// cutscene) collected on touching Link, or its collider registered.
    fn func_80afb768(&mut self, play: &mut PlayState) {
        if self.actor.flags & ACTOR_FLAG_HOOKSHOT_ATTACHED == ACTOR_FLAG_HOOKSHOT_ATTACHED {
            self.action = Action::Hookshotted;
        } else {
            self.grow_and_spin();
            if !play.player_in_cs_mode() {
                self.func_80afb748();
                if self.collider.base.oc_flags2 & OC2_HIT_PLAYER != 0 {
                    self.collider.base.oc_flags2 &= !OC2_HIT_PLAYER;
                    self.collect(play);
                } else {
                    self.collider.update(&self.actor);
                    play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
                    play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
                }
            }
        }
    }

    /// `func_80AFB89C`: growing and spinning; let go by the hookshot, collected.
    fn func_80afb89c(&mut self, play: &mut PlayState) {
        self.grow_and_spin();
        if self.actor.flags & ACTOR_FLAG_HOOKSHOT_ATTACHED != ACTOR_FLAG_HOOKSHOT_ATTACHED {
            self.collect(play);
        }
    }

    /// `func_80AFB950`: Link kept frozen until the text closes; then its flag set
    /// (`SET_GS_FLAGS(PARAMS_GET_S(params, 8, 5), PARAMS_GET_S(params, 0, 8))`) and it's gone.
    fn func_80afb950(&mut self, play: &mut PlayState) {
        if play.message_state() != TEXT_STATE_CLOSING {
            freeze_player(play);
        } else {
            let p = self.actor.params as i32;
            play.save.set_gs_flags((p >> 8) & 0x1F, p & 0xFF);
            self.actor.kill();
        }
    }
}

/// `player->actor.freezeTimer = 10`.
fn freeze_player(play: &mut PlayState) {
    if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
        p.freeze_timer = 10;
    }
}

impl ActorImpl for EnSi {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnSi_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        match self.action {
            Action::Spin => self.func_80afb768(play),
            Action::Hookshotted => self.func_80afb89c(play),
            Action::Collected => self.func_80afb950(play),
        }
        self.actor.set_focus(16.0);
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![(self.action == Action::Collected) as u32];
        rs
    }

    /// `EnSi_Draw`: until collected, `GetItem_Draw(play, GID_SKULL_TOKEN_2)` at its matrix
    /// (`func_8002ED80`'s and `func_8002EBCC`'s look-at: the renderer's texgen reads the view).
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.first().copied().unwrap_or(0) != 0 {
            return;
        }
        if let Some(a) = &play.assets {
            oot_game::draw::get_item_draw(&a.items, GID_SKULL_TOKEN_2, actor_draw_matrix(rs), play.gameplay_frames, view, out);
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
