//! The interface (`z_parameter.c`, the heart meter of `z_lifemeter.c`, `Interface_Init` in
//! `z_construct.c`): the HUD's hearts, rupee counter and buttons, the A button's do-action
//! label and its flip, and the interface's alpha types.
//!
//! - **The frame:** `Interface_Update` runs in `Play_Update` after `Message_Update` (`update`):
//!   the buttons' status (`func_80083108`), the alpha type's fade (`Interface_ChangeAlpha`, from
//!   the camera's interface flags and the message box), the health and rupee accumulators, the
//!   beating heart, and the A button's flip to a new do-action. `Interface_Draw` (in
//!   `Play_DrawOverlayElements`, before the message box) reads that state (`draw_hud_1`,
//!   `draw_hud_2`, either side of the Z-target reticle).
//! - **Drawing** (docs/adr/0017-interface-sprites.md): each texture rectangle is a `Sprite`.
//!   The beating heart is a quad under a matrix in the overlay's space. The A button and its
//!   label are quads turning about X in their own 45x45 viewport with a 60° perspective
//!   (`func_8008A8B8`): that projection, placed on the viewport's part of the screen, is part
//!   of their transform.
//! - **The B and C items:** each button's item icon (`icon_item_static`, every item up to the
//!   fishing rod baked) and the C items' ammo counts (`Interface_DrawAmmoCount`), from the
//!   save's buttons. `Interface_LoadItemIcon1/2` load nothing: the icon drawn is the button's
//!   item as it draws.
//! - **Not ported:** the magic meter (no magic), the minimap, the timers, the C-Up Navi prompt,
//!   the B button's label (only the ocarina loads one), the B button's ammo count (only while
//!   riding or in a minigame), double defence's hearts, the sounds, and the pause menu. `func_80083108`'s
//!   riding, minigame, fishing, water and horse-race cases, and its item-type restrictions, are
//!   left out: with nothing on the C buttons they don't change a button's status.
//!
//! The HUD is drawn once `Play_Init` has run `Interface_Init` (`initialised`); the sandbox's
//! scene views keep the spikes' picture without it.

use glam::{Mat4, Vec3, Vec4};

use crate::gbi::{Dl, G_IM_FMT_I, G_IM_FMT_IA, G_IM_FMT_RGBA, G_IM_SIZ_4B, G_IM_SIZ_8B, G_IM_SIZ_32B, G_TX_WRAP, ac, cc_ab, cc_c, cc_d, setup_dl};
use crate::save::SaveContext;
use crate::sprite::{Load, Quad, Sprite, SpriteBake, TexSrc};

// DoAction (z64.h).
pub const DO_ACTION_ATTACK: u16 = 0x00;
pub const DO_ACTION_CHECK: u16 = 0x01;
pub const DO_ACTION_ENTER: u16 = 0x02;
pub const DO_ACTION_RETURN: u16 = 0x03;
pub const DO_ACTION_OPEN: u16 = 0x04;
pub const DO_ACTION_JUMP: u16 = 0x05;
pub const DO_ACTION_DECIDE: u16 = 0x06;
pub const DO_ACTION_DIVE: u16 = 0x07;
pub const DO_ACTION_FASTER: u16 = 0x08;
pub const DO_ACTION_THROW: u16 = 0x09;
pub const DO_ACTION_NONE: u16 = 0x0A;
pub const DO_ACTION_CLIMB: u16 = 0x0B;
pub const DO_ACTION_DROP: u16 = 0x0C;
pub const DO_ACTION_DOWN: u16 = 0x0D;
pub const DO_ACTION_SAVE: u16 = 0x0E;
pub const DO_ACTION_SPEAK: u16 = 0x0F;
pub const DO_ACTION_NEXT: u16 = 0x10;
pub const DO_ACTION_GRAB: u16 = 0x11;
pub const DO_ACTION_STOP: u16 = 0x12;
pub const DO_ACTION_PUTAWAY: u16 = 0x13;
/// `DO_ACTION_1` .. `DO_ACTION_8`: the dive depth's digits.
pub const DO_ACTION_1: u16 = 0x15;
pub const DO_ACTION_MAX: u16 = 0x1D;

/// `DO_ACTION_TEX_SIZE`: a 48x16 IA4 label.
const DO_ACTION_TEX_SIZE: u32 = 48 * 16 / 2;

/// `BTN_ENABLED`, `BTN_DISABLED`.
pub const BTN_ENABLED: u8 = 0x00;
pub const BTN_DISABLED: u8 = 0xFF;

// A_BUTTON_X .. C_RIGHT_BUTTON_Y (z64interface.h), and the registers placed at them
// (R_ITEM_BTN_X, R_ITEM_ICON_X, R_A_BTN_X, ... in z_construct.c).
const A_BUTTON_X: i16 = 186;
const A_BUTTON_Y: i16 = 9;
/// `R_ITEM_BTN_X(i)`, `R_ITEM_BTN_Y(i)`: B, C-left, C-down, C-right.
const ITEM_BTN_X: [i16; 4] = [160, 227, 249, 271];
const ITEM_BTN_Y: [i16; 4] = [17, 18, 34, 18];
/// `R_ITEM_BTN_WIDTH(i)`, `R_ITEM_ICON_WIDTH(i)` (the icons are at `R_ITEM_ICON_X(i)`,
/// `R_ITEM_ICON_Y(i)`: the buttons' places).
const ITEM_BTN_WIDTH: [i16; 4] = [29, 27, 27, 27];
const ITEM_ICON_WIDTH: [i16; 4] = [30, 24, 24, 24];
/// `R_ITEM_AMMO_X(i)`, `R_ITEM_AMMO_Y(i)`: the ammo count's first digit.
const ITEM_AMMO_X: [i16; 4] = [160 + 2, 227 + 1, 249 + 1, 271 + 1];
const ITEM_AMMO_Y: [i16; 4] = [17 + 18, 18 + 17, 34 + 17, 18 + 17];
/// `R_B_BTN_COLOR`, `R_C_BTN_COLOR`, `R_A_BTN_COLOR`.
const B_BTN_COLOR: [u8; 3] = [255, 30, 30];
const C_BTN_COLOR: [u8; 3] = [255, 160, 0];
const A_BTN_COLOR: [u8; 3] = [0, 200, 50];
/// `XREG(18)`, `WREG(46)` (English): how far in front of its eye the A button and its label are,
/// in tenths; `XREG(21)`, `XREG(28)`: the label quad's width and height; `WREG(5)`: the flip's
/// speed divisor.
const XREG_18: f32 = -380.0;
const WREG_46: f32 = -380.0;
const XREG_21: i32 = 48;
const XREG_28: i32 = 16;
const WREG_5: f32 = 3.0;

/// `HEARTS_PRIM_*`, `HEARTS_ENV_*` (z64interface.h).
const HEARTS_PRIM: [i16; 3] = [255, 70, 50];
const HEARTS_ENV: [i16; 3] = [50, 40, 60];

/// `sRestrictionFlags`' entries: what the scene doesn't allow (`Interface_SetSceneRestrictions`).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InterfaceTables {
    /// `{ sceneId, flags1, flags2, flags3 }`, up to the `0xFF` end.
    pub restrictions: Vec<[u8; 4]>,
}

/// `interfaceCtx->restrictions`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Restrictions {
    pub h_gauge: u8,
    pub b_button: u8,
    pub a_button: u8,
    pub bottles: u8,
    pub trade_items: u8,
    pub hookshot: u8,
    pub ocarina: u8,
    pub warp_songs: u8,
    pub suns_song: u8,
    pub farores: u8,
    pub dins_nayrus: u8,
    pub all: u8,
}

/// `InterfaceContext`.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceContext {
    /// `Interface_Init` ran (in `Play_Init`): the HUD is drawn and updated.
    pub initialised: bool,
    /// `unk_1EC`: the A button's flip (1, 2: to the new label; 3, 4 when paused).
    pub unk_1ec: u16,
    /// `unk_1EE`: the do-action the A button shows; `unk_1F0`: the one it's turning to.
    pub unk_1ee: u16,
    pub unk_1f0: u16,
    /// `unk_1F4`: the flip's angle, in 1/10000 radians.
    pub unk_1f4: f32,
    /// `unk_1FA`: the B button shows a label (`Interface_LoadActionLabelB`); `unk_1FC` which.
    pub unk_1fa: bool,
    pub unk_1fc: u16,
    /// `doActionSegment`: the three labels loaded (0 the A button's, 1 the one it turns to or
    /// the B button's, 2 the pause menu's START), as do-actions; `None` where
    /// `Interface_LoadActionLabel` cleared it.
    pub do_action_segment: [Option<u16>; 3],
    /// `aAlpha`, `bAlpha`, `cLeftAlpha`, `cDownAlpha`, `cRightAlpha`, `healthAlpha`,
    /// `startAlpha`, `magicAlpha`, `minimapAlpha`.
    pub a_alpha: i16,
    pub b_alpha: i16,
    pub c_left_alpha: i16,
    pub c_down_alpha: i16,
    pub c_right_alpha: i16,
    pub health_alpha: i16,
    pub start_alpha: i16,
    pub magic_alpha: i16,
    pub minimap_alpha: i16,
    /// `unk_226`, `unk_228` (`Health_InitMeter`).
    pub unk_226: i16,
    pub unk_228: i16,
    /// `beatingHeartOscillator` and its direction; `heartColorOscillator` and its direction.
    pub beating_heart_oscillator: i16,
    pub beating_heart_oscillator_direction: i16,
    pub heart_color_oscillator: i16,
    pub heart_color_oscillator_direction: i16,
    /// `heartsPrimR..B[2]`, `heartsEnvR..B[2]`, `beatingHeartPrim`, `beatingHeartEnv`.
    pub hearts_prim: [[i16; 3]; 2],
    pub hearts_env: [[i16; 3]; 2],
    pub beating_heart_prim: [u8; 3],
    pub beating_heart_env: [u8; 3],
    pub restrictions: Restrictions,
    /// `naviCalling` (`Interface_SetNaviCall`), and `z_parameter.c`'s `sCUpInvisible` and
    /// `sCUpTimer` it resets: the C-Up button's "Navi" prompt (not drawn).
    pub navi_calling: bool,
    pub c_up_invisible: i16,
    pub c_up_timer: i16,
}

impl Default for InterfaceContext {
    fn default() -> InterfaceContext {
        InterfaceContext {
            initialised: false,
            unk_1ec: 0,
            unk_1ee: 0,
            unk_1f0: 0,
            unk_1f4: 0.0,
            unk_1fa: false,
            unk_1fc: 0,
            // Interface_Init's DMA: the first two labels (Attack, Check) and Return.
            do_action_segment: [Some(DO_ACTION_ATTACK), Some(DO_ACTION_CHECK), Some(DO_ACTION_RETURN)],
            a_alpha: 0,
            b_alpha: 0,
            c_left_alpha: 0,
            c_down_alpha: 0,
            c_right_alpha: 0,
            health_alpha: 0,
            start_alpha: 0,
            magic_alpha: 0,
            minimap_alpha: 0,
            unk_226: 0,
            // XREG(95).
            unk_228: 200,
            beating_heart_oscillator: 0,
            beating_heart_oscillator_direction: 0,
            heart_color_oscillator: 0,
            heart_color_oscillator_direction: 0,
            hearts_prim: [HEARTS_PRIM; 2],
            hearts_env: [HEARTS_ENV; 2],
            beating_heart_prim: [0; 3],
            beating_heart_env: [0; 3],
            restrictions: Restrictions::default(),
            navi_calling: false,
            c_up_invisible: 0,
            c_up_timer: 0,
        }
    }
}

/// `Interface_ChangeAlpha`: the interface fades to `alpha_type`'s buttons (`unk_13E8` steps it
/// in `Interface_Update`).
pub fn change_alpha(save: &mut SaveContext, alpha_type: u16) {
    if alpha_type != save.unk_13ea {
        save.unk_13ea = alpha_type;
        save.unk_13e8 = alpha_type;
        save.unk_13ec = 1;
    }
}

/// What `Interface_Update` reads of the rest of play.
pub struct IfaceFrame {
    /// `play->sceneId`.
    pub scene_id: u16,
    /// `msgCtx->msgMode == MSGMODE_NONE`.
    pub msg_none: bool,
    /// Player's `stateFlags1 & PLAYER_STATE1_21` (climbing) and `stateFlags2 & PLAYER_STATE2_18`.
    pub climbing: bool,
    pub state2_18: bool,
    /// `transitionTrigger == TRANS_TRIGGER_OFF && transitionMode == TRANS_MODE_OFF`.
    pub no_transition: bool,
    /// `roomCtx.curRoom.behaviorType1 == ROOM_BEHAVIOR_TYPE1_1`.
    pub dungeon_room: bool,
}

/// The overworld scenes of the minimap's switch: `SCENE_SPOT00` (0x51) to `SCENE_SPOT13`,
/// `SCENE_SPOT15` to `SCENE_SPOT18`, `SCENE_SPOT20` and `SCENE_GANON_TOU` (0x64), which are
/// 0x51..=0x64 (`scene_table.h`).
fn is_overworld(scene_id: u16) -> bool {
    (0x51..=0x64).contains(&scene_id)
}

impl InterfaceContext {
    /// `Interface_Init` (and `Health_InitMeter`, `Interface_SetSceneRestrictions`).
    pub fn init(save: &mut SaveContext, tables: &InterfaceTables, scene_id: u16) -> InterfaceContext {
        save.unk_13e8 = 0;
        save.unk_13ea = 0;
        let mut c = InterfaceContext { initialised: true, ..Default::default() };
        // Health_InitMeter.
        c.unk_228 = 0x140;
        c.unk_226 = save.health;
        c.set_scene_restrictions(tables, scene_id);
        c
    }

    /// `Interface_SetSceneRestrictions`.
    fn set_scene_restrictions(&mut self, tables: &InterfaceTables, scene_id: u16) {
        self.restrictions = Restrictions::default();
        for &[id, f1, f2, f3] in &tables.restrictions {
            if id == 0xFF {
                break;
            }
            if id as u16 == scene_id & 0xFF {
                self.restrictions = Restrictions {
                    h_gauge: (f1 & 0xC0) >> 6,
                    b_button: (f1 & 0x30) >> 4,
                    a_button: (f1 & 0x0C) >> 2,
                    bottles: f1 & 0x03,
                    trade_items: (f2 & 0xC0) >> 6,
                    hookshot: (f2 & 0x30) >> 4,
                    ocarina: (f2 & 0x0C) >> 2,
                    warp_songs: f2 & 0x03,
                    suns_song: (f3 & 0xC0) >> 6,
                    farores: (f3 & 0x30) >> 4,
                    dins_nayrus: (f3 & 0x0C) >> 2,
                    all: f3 & 0x03,
                };
                return;
            }
        }
    }

    /// `Interface_LoadActionLabel` (English): the label's texture into `doActionSegment`, or
    /// cleared for `DO_ACTION_NONE`.
    pub fn load_action_label(&mut self, mut action: u16, load_offset: usize) {
        if action >= DO_ACTION_MAX {
            action = DO_ACTION_NONE;
        }
        self.do_action_segment[load_offset] = (action != DO_ACTION_NONE).then_some(action);
    }

    /// `Interface_SetNaviCall`: 0x1D or 0x1E (Navi's hello or call, sounds not played) start the
    /// C-Up prompt outside cutscenes; 0x1F stops it.
    pub fn set_navi_call(&mut self, navi_call_state: u16, cs_idle: bool) {
        if (navi_call_state == 0x1D || navi_call_state == 0x1E) && !self.navi_calling && cs_idle {
            self.navi_calling = true;
            self.c_up_invisible = 0;
            self.c_up_timer = 10;
        } else if navi_call_state == 0x1F && self.navi_calling {
            self.navi_calling = false;
        }
    }

    /// `Interface_SetDoAction`: the A button flips to `action` (the pause menu isn't ported).
    pub fn set_do_action(&mut self, action: u16) {
        if self.unk_1f0 != action {
            self.unk_1f0 = action;
            self.unk_1ec = 1;
            self.unk_1f4 = 0.0;
            self.load_action_label(action, 1);
        }
    }

    /// `Interface_LoadActionLabelB`.
    pub fn load_action_label_b(&mut self, action: u16) {
        self.unk_1fc = action;
        self.do_action_segment[1] = Some(action);
        self.unk_1fa = true;
    }

    /// `func_80082644`: the buttons fade to `alpha`, a disabled one only to 70.
    fn func_80082644(&mut self, save: &SaveContext, alpha: i16) {
        let s = save.button_status;
        for (a, st) in [(&mut self.b_alpha, s[0]), (&mut self.c_left_alpha, s[1]), (&mut self.c_down_alpha, s[2]), (&mut self.c_right_alpha, s[3]), (&mut self.a_alpha, s[4])] {
            if st == BTN_DISABLED {
                if *a != 70 {
                    *a = 70;
                }
            } else if *a != 255 {
                *a = alpha;
            }
        }
    }

    /// `func_8008277C`.
    fn func_8008277c(&mut self, save: &SaveContext, max_alpha: i16, alpha: i16) {
        if save.unk_13e7 != 0 {
            self.func_80082644(save, alpha);
            return;
        }
        for a in [&mut self.b_alpha, &mut self.a_alpha, &mut self.c_left_alpha, &mut self.c_down_alpha, &mut self.c_right_alpha] {
            clamp_down(a, max_alpha);
        }
    }

    /// `func_80082850`: alpha type `unk_13E8`'s fade, `max_alpha` falling as `alpha` rises.
    fn func_80082850(&mut self, save: &SaveContext, max_alpha: i16, scene_id: u16, dungeon_room: bool) {
        let alpha = 255 - max_alpha;
        let rise = |a: &mut i16| {
            if *a != 255 {
                *a = alpha;
            }
        };
        match save.unk_13e8 {
            1 | 2 | 8 => {
                if save.unk_13e8 == 8 {
                    rise(&mut self.b_alpha);
                } else {
                    clamp_down(&mut self.b_alpha, max_alpha);
                }
                for a in [&mut self.a_alpha, &mut self.c_left_alpha, &mut self.c_down_alpha, &mut self.c_right_alpha, &mut self.health_alpha, &mut self.magic_alpha, &mut self.minimap_alpha] {
                    clamp_down(a, max_alpha);
                }
            }
            3 => {
                clamp_down(&mut self.a_alpha, max_alpha);
                self.func_8008277c(save, max_alpha, alpha);
                clamp_down(&mut self.magic_alpha, max_alpha);
                clamp_down(&mut self.minimap_alpha, max_alpha);
                rise(&mut self.health_alpha);
            }
            4 => {
                for a in [&mut self.b_alpha, &mut self.a_alpha, &mut self.c_left_alpha, &mut self.c_down_alpha, &mut self.c_right_alpha, &mut self.health_alpha, &mut self.magic_alpha, &mut self.minimap_alpha] {
                    clamp_down(a, max_alpha);
                }
                rise(&mut self.a_alpha);
            }
            5 => {
                self.func_8008277c(save, max_alpha, alpha);
                clamp_down(&mut self.minimap_alpha, max_alpha);
                rise(&mut self.a_alpha);
                rise(&mut self.health_alpha);
                rise(&mut self.magic_alpha);
            }
            6 => {
                self.func_8008277c(save, max_alpha, alpha);
                rise(&mut self.a_alpha);
                rise(&mut self.health_alpha);
                rise(&mut self.magic_alpha);
                if is_overworld(scene_id) {
                    self.minimap_alpha = if self.minimap_alpha < 170 { alpha } else { 170 };
                } else {
                    rise(&mut self.minimap_alpha);
                }
            }
            7 => {
                clamp_down(&mut self.minimap_alpha, max_alpha);
                self.func_80082644(save, alpha);
                rise(&mut self.health_alpha);
                rise(&mut self.magic_alpha);
            }
            9 => {
                for a in [&mut self.b_alpha, &mut self.a_alpha, &mut self.c_left_alpha, &mut self.c_down_alpha, &mut self.c_right_alpha, &mut self.minimap_alpha] {
                    clamp_down(a, max_alpha);
                }
                rise(&mut self.health_alpha);
                rise(&mut self.magic_alpha);
            }
            10 => {
                for a in [&mut self.a_alpha, &mut self.c_left_alpha, &mut self.c_down_alpha, &mut self.c_right_alpha, &mut self.health_alpha, &mut self.magic_alpha, &mut self.minimap_alpha] {
                    clamp_down(a, max_alpha);
                }
                rise(&mut self.b_alpha);
            }
            11 => {
                for a in [&mut self.b_alpha, &mut self.a_alpha, &mut self.c_left_alpha, &mut self.c_down_alpha, &mut self.c_right_alpha, &mut self.minimap_alpha, &mut self.magic_alpha] {
                    clamp_down(a, max_alpha);
                }
                rise(&mut self.health_alpha);
            }
            12 => {
                rise(&mut self.a_alpha);
                rise(&mut self.b_alpha);
                rise(&mut self.minimap_alpha);
                for a in [&mut self.c_left_alpha, &mut self.c_down_alpha, &mut self.c_right_alpha, &mut self.magic_alpha, &mut self.health_alpha] {
                    clamp_down(a, max_alpha);
                }
            }
            13 => {
                self.func_8008277c(save, max_alpha, alpha);
                clamp_down(&mut self.minimap_alpha, max_alpha);
                clamp_down(&mut self.a_alpha, max_alpha);
                rise(&mut self.health_alpha);
                rise(&mut self.magic_alpha);
            }
            _ => {}
        }
        if dungeon_room && self.minimap_alpha >= 255 {
            self.minimap_alpha = 255;
        }
    }

    /// `func_80083108`'s cases that apply here: while climbing (or `PLAYER_STATE2_18`) the B and
    /// C buttons are disabled; otherwise the scene's restrictions: B's, then each C item's by
    /// its kind (bottles, trade items, the hookshots, the ocarinas, Farore's Wind, Din's Fire
    /// and Nayru's Love), then `restrictions.all` for the rest. A status change fades the
    /// interface back in (alpha type 50).
    ///
    /// B with nothing, or a bow, slingshot or bombchu a minigame put there, gets back what
    /// `buttonStatus[0]` kept, unless B has had no sword since the file began
    /// (`infTable[INFTABLE_1DX_INDEX]`). Riding, the minigames, the fishing pond, the Chamber
    /// of Sages and `func_8008F2F8`'s water and heat hazards aren't ported.
    fn func_80083108(&mut self, save: &mut SaveContext, f: &IfaceFrame) {
        use crate::item::*;
        use crate::save::INFTABLE_1DX_INDEX;
        // cutsceneIndex < 0xFFF0; no riding, shooting gallery, fishing pond or Chamber of Sages.
        save.unk_13e7 = 0;
        if !f.msg_none {
            return;
        }
        let mut sp28 = false;
        // (func_8008F2F8: no water or heat hazard is detected here.)
        if f.climbing || f.state2_18 {
            if save.button_status[0] != BTN_DISABLED {
                save.button_status[..4].fill(BTN_DISABLED);
                save.unk_13ea = 0;
                change_alpha(save, 50);
            }
            return;
        }
        let b = save.equips.button_items[0];
        let b_is_ammo = b == ITEM_SLINGSHOT || b == ITEM_BOW || b == ITEM_BOMBCHU || b == ITEM_NONE;
        if self.restrictions.b_button == 0 {
            if b_is_ammo {
                if b != ITEM_NONE || save.inf_table[INFTABLE_1DX_INDEX] == 0 {
                    save.equips.button_items[0] = save.button_status[0];
                    sp28 = true;
                    // (Interface_LoadItemIcon1: the icons are baked.)
                }
            } else if save.button_status[0] == BTN_DISABLED {
                sp28 = true;
                // (buttonStatus[0] & 0xFF) is BTN_DISABLED here, so the C's inner test always
                // takes its first branch.
                save.button_status[0] = BTN_ENABLED;
            }
        } else if self.restrictions.b_button == 1 {
            if b_is_ammo {
                if b != ITEM_NONE || save.inf_table[INFTABLE_1DX_INDEX] == 0 {
                    save.equips.button_items[0] = save.button_status[0];
                    sp28 = true;
                }
            } else {
                if save.button_status[0] == BTN_ENABLED {
                    sp28 = true;
                }
                save.button_status[0] = BTN_DISABLED;
            }
        }
        // Each C item by its kind: disabled with its restriction, enabled without.
        let bottle = |i: u8| (ITEM_BOTTLE..=ITEM_POE).contains(&i);
        let trade = |i: u8| (ITEM_WEIRD_EGG..=ITEM_CLAIM_CHECK).contains(&i);
        let r = self.restrictions;
        let kinds: [(u8, &dyn Fn(u8) -> bool); 6] = [
            (r.bottles, &bottle),
            (r.trade_items, &trade),
            (r.hookshot, &|i| i == ITEM_HOOKSHOT || i == ITEM_LONGSHOT),
            (r.ocarina, &|i| i == ITEM_OCARINA_FAIRY || i == ITEM_OCARINA_TIME),
            (r.farores, &|i| i == ITEM_FARORES_WIND),
            (r.dins_nayrus, &|i| i == ITEM_DINS_FIRE || i == ITEM_NAYRUS_LOVE),
        ];
        let set = |save: &mut SaveContext, i: usize, disabled: bool, sp28: &mut bool| {
            let (from, to) = if disabled { (BTN_ENABLED, BTN_DISABLED) } else { (BTN_DISABLED, BTN_ENABLED) };
            if save.button_status[i] == from {
                *sp28 = true;
            }
            save.button_status[i] = to;
        };
        for (restricted, is_kind) in kinds {
            for i in 1..4 {
                if is_kind(save.equips.button_items[i]) {
                    set(save, i, restricted != 0, &mut sp28);
                }
            }
        }
        /// `SCENE_TAKARAYA` (the treasure chest shop, where the Lens of Truth stays usable).
        const SCENE_TAKARAYA: u16 = 0x10;
        for i in 1..4 {
            let it = save.equips.button_items[i];
            let ocarina = it == ITEM_OCARINA_FAIRY || it == ITEM_OCARINA_TIME;
            if r.all != 0 {
                if !ocarina && !bottle(it) && !trade(it) {
                    let lens_in_shop = f.scene_id == SCENE_TAKARAYA && it == ITEM_LENS;
                    set(save, i, !lens_in_shop, &mut sp28);
                }
            } else if it != ITEM_DINS_FIRE && it != ITEM_HOOKSHOT && it != ITEM_LONGSHOT && it != ITEM_FARORES_WIND && it != ITEM_NAYRUS_LOVE && !ocarina && !bottle(it) && !trade(it) {
                set(save, i, false, &mut sp28);
            }
        }
        if sp28 {
            save.unk_13ea = 0;
            if f.no_transition {
                change_alpha(save, 50);
            }
        }
    }

    /// `Health_UpdateBeatingHeart`: the oscillator 0..10 and back (the low-health alarm isn't
    /// played).
    fn health_update_beating_heart(&mut self) {
        if self.beating_heart_oscillator_direction != 0 {
            self.beating_heart_oscillator -= 1;
            if self.beating_heart_oscillator <= 0 {
                self.beating_heart_oscillator = 0;
                self.beating_heart_oscillator_direction = 0;
            }
        } else {
            self.beating_heart_oscillator += 1;
            if self.beating_heart_oscillator >= 10 {
                self.beating_heart_oscillator = 10;
                self.beating_heart_oscillator_direction = 1;
            }
        }
    }

    /// `Health_UpdateMeter` for the normal hearts (the burning and drowning colours are
    /// unused, the normal factors are 0, and double defence isn't kept).
    fn health_update_meter(&mut self) {
        if self.heart_color_oscillator_direction != 0 {
            self.heart_color_oscillator -= 1;
            if self.heart_color_oscillator <= 0 {
                self.heart_color_oscillator = 0;
                self.heart_color_oscillator_direction = 0;
            }
        } else {
            self.heart_color_oscillator += 1;
            if self.heart_color_oscillator >= 10 {
                self.heart_color_oscillator = 10;
                self.heart_color_oscillator_direction = 1;
            }
        }
        self.hearts_prim = [HEARTS_PRIM; 2];
        self.hearts_env = [HEARTS_ENV; 2];
        // sHeartsPrimFactors[0] and sHeartsEnvFactors[0] are 0: the beating heart keeps the
        // normal colours.
        self.beating_heart_prim = HEARTS_PRIM.map(|c| c as u8);
        self.beating_heart_env = HEARTS_ENV.map(|c| c as u8);
    }

    /// `Interface_Update`.
    pub fn update(&mut self, save: &mut SaveContext, f: &IfaceFrame) {
        if !self.initialised {
            return;
        }
        // (No pause menu, minigame or cutscene layer; no game over.)
        self.func_80083108(save, f);
        match save.unk_13e8 {
            1..=13 => {
                let alpha = (255 - ((save.unk_13ec as i16) << 5)).max(0);
                self.func_80082850(save, alpha, f.scene_id, f.dungeon_room);
                save.unk_13ec += 1;
                if alpha == 0 {
                    save.unk_13e8 = 0;
                }
            }
            50 => {
                let alpha = (255 - ((save.unk_13ec as i16) << 5)).max(0);
                let alpha1 = (255 - alpha).min(255);
                self.func_80082644(save, alpha1);
                if self.health_alpha != 255 {
                    self.health_alpha = alpha1;
                }
                if self.magic_alpha != 255 {
                    self.magic_alpha = alpha1;
                }
                if is_overworld(f.scene_id) {
                    self.minimap_alpha = if self.minimap_alpha < 170 { alpha1 } else { 170 };
                } else if self.minimap_alpha != 255 {
                    self.minimap_alpha = alpha1;
                }
                save.unk_13ec += 1;
                if alpha1 == 255 {
                    save.unk_13e8 = 0;
                }
            }
            52 => {
                save.unk_13e8 = 1;
                self.func_80082850(save, 0, f.scene_id, f.dungeon_room);
                save.unk_13e8 = 0;
            }
            _ => {}
        }
        // (Map_Update: no minimap.)
        if save.health_accumulator != 0 {
            save.health_accumulator -= 4;
            save.health += 4;
            if save.health >= save.health_capacity {
                save.health = save.health_capacity;
                save.health_accumulator = 0;
            }
        }
        self.health_update_beating_heart();
        self.health_update_meter();
        if save.rupee_accumulator != 0 {
            if save.rupee_accumulator > 0 {
                if save.rupees < save.wallet_capacity() {
                    save.rupee_accumulator -= 1;
                    save.rupees += 1;
                } else {
                    save.rupees = save.wallet_capacity();
                    save.rupee_accumulator = 0;
                }
            } else if save.rupees != 0 {
                if save.rupee_accumulator <= -50 {
                    save.rupee_accumulator += 10;
                    save.rupees = (save.rupees - 10).max(0);
                } else {
                    save.rupee_accumulator += 1;
                    save.rupees -= 1;
                }
            } else {
                save.rupee_accumulator = 0;
            }
        }
        // The A button's flip: to 90°, then from -90° back to 0 with the new label.
        match self.unk_1ec {
            1 | 3 => {
                self.unk_1f4 += 31400.0 / WREG_5;
                if self.unk_1f4 >= 15700.0 {
                    self.unk_1f4 = -15700.0;
                    self.unk_1ec = 2;
                }
            }
            2 | 4 => {
                self.unk_1f4 += 31400.0 / WREG_5;
                if self.unk_1f4 >= 0.0 {
                    self.unk_1f4 = 0.0;
                    self.unk_1ec = 0;
                    self.unk_1ee = self.unk_1f0;
                    let mut action = self.unk_1ee;
                    if action == DO_ACTION_MAX || action == DO_ACTION_MAX + 1 {
                        action = DO_ACTION_NONE;
                    }
                    self.load_action_label(action, 0);
                }
            }
            _ => {}
        }
        // (Magic, timers, the minigame score and the Sun's Song aren't ported.)
    }

    /// `Interface_Draw` up to the Z-target reticle (`func_8002C124` is drawn between the two
    /// halves): the heart meter, the rupee icon and counter.
    pub fn draw_hud_1(&self, save: &SaveContext, out: &mut Vec<Sprite>) {
        self.health_draw_meter(save, out);
        // Rupee icon: Gfx_TextureIA8(gRupeeCounterIconTex, 16, 16, 26, 206, 16, 16).
        out.push(Sprite::rect(RUPEE_ICON, 26.0, 206.0, 42.0, 222.0, Some([200, 255, 100, self.magic_alpha as u8]), Some([0, 80, 0, 255])));
        // (The small key counter only shows in dungeons.)
        let rgb = if save.rupees == save.wallet_capacity() {
            [120, 255, 0]
        } else if save.rupees != 0 {
            [255, 255, 255]
        } else {
            [100, 100, 100]
        };
        let prim = Some([rgb[0], rgb[1], rgb[2], self.magic_alpha as u8]);
        let mut digits = [0i16, 0, save.rupees];
        if digits[2] > 9999 || digits[2] < 0 {
            digits[2] &= 0xDDD;
        }
        while digits[2] >= 100 {
            digits[0] += 1;
            digits[2] -= 100;
        }
        while digits[2] >= 10 {
            digits[1] += 1;
            digits[2] -= 10;
        }
        /// `rupeeDigitsFirst`, `rupeeDigitsCount` by wallet.
        const FIRST: [usize; 3] = [1, 0, 0];
        const COUNT: [usize; 3] = [2, 3, 3];
        let w = (save.cur_upg_value(crate::item::UPG_WALLET) as usize).min(2);
        for k in 0..COUNT[w] {
            let x = 42.0 + 8.0 * k as f32;
            out.push(Sprite::rect(digit_sprite(digits[FIRST[w] + k] as usize), x, 206.0, x + 8.0, 222.0, prim, None));
        }
        // (Magic_DrawMeter: no magic; Minimap_Draw: not ported.)
    }

    /// `Interface_Draw` after the reticle: the item buttons, the B item, the A button and its
    /// label.
    pub fn draw_hud_2(&self, save: &SaveContext, out: &mut Vec<Sprite>) {
        // Interface_DrawItemButtons.
        let btn = |i: usize, name: &str, rgb: [u8; 3], alpha: i16, out: &mut Vec<Sprite>| {
            let (x, y, w) = (ITEM_BTN_X[i] as f32, ITEM_BTN_Y[i] as f32, ITEM_BTN_WIDTH[i] as f32);
            out.push(Sprite::rect(name, x, y, x + w, y + w, Some([rgb[0], rgb[1], rgb[2], alpha as u8]), None));
        };
        btn(0, BUTTON, B_BTN_COLOR, self.b_alpha, out);
        let c_alphas = [self.c_left_alpha, self.c_down_alpha, self.c_right_alpha];
        for i in 1..4 {
            btn(i, BUTTON, C_BTN_COLOR, c_alphas[i - 1], out);
        }
        // (The START button only with the pause menu; the C-Up prompt with Navi calling.)
        // Empty C button arrows.
        for i in 1..4 {
            if save.equips.button_items[i] > 0xF0 {
                btn(i, EMPTY_C[i - 1], C_BTN_COLOR, c_alphas[i - 1], out);
            }
        }
        // The B item (the B label only follows Interface_LoadActionLabelB; B's ammo count only
        // while riding or in a minigame).
        if !self.unk_1fa && save.equips.button_items[0] != crate::save::ITEM_NONE {
            item_icon(0, save.equips.button_items[0], self.b_alpha, out);
        }
        // The C items, each with its ammo count.
        for i in 1..4 {
            let item = save.equips.button_items[i];
            if item < 0xF0 {
                item_icon(i, item, c_alphas[i - 1], out);
                draw_ammo_count(save, i, c_alphas[i - 1], out);
            }
        }
        // The A button, and its label, in the A button's viewport (func_8008A8B8).
        let view = a_button_view(A_BUTTON_Y as f32, A_BUTTON_Y as f32 + 45.0, A_BUTTON_X as f32, A_BUTTON_X as f32 + 45.0);
        let turn = Mat4::from_rotation_x(self.unk_1f4 / 10000.0);
        let a = self.a_alpha as u8;
        let button = view * Mat4::from_translation(Vec3::new(0.0, 0.0, XREG_18 / 10.0)) * turn;
        out.push(Sprite { name: A_BUTTON.into(), transform: button, prim: Some([A_BTN_COLOR[0], A_BTN_COLOR[1], A_BTN_COLOR[2], a]), env: None });
        let slot = if self.unk_1ec < 2 || self.unk_1ec == 3 { 0 } else { 1 };
        if let Some(action) = self.do_action_segment[slot] {
            let label = view * Mat4::from_translation(Vec3::new(0.0, 0.0, WREG_46 / 10.0)) * turn;
            out.push(Sprite { name: do_action_sprite(action), transform: label, prim: Some([255, 255, 255, a]), env: None });
        }
    }

    /// `Health_DrawMeter` (without double defence): each heart's quarter from `health`, the
    /// last one beating.
    fn health_draw_meter(&self, save: &SaveContext, out: &mut Vec<Sprite>) {
        /// `sHeartTextures` by `health % 16`: full, then quarter, half, three-quarter.
        const HEART_BY_FRACTION: [usize; 16] = [4, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3];
        let cur_heart_fraction = (save.health % 0x10) as usize;
        let total_heart_count = save.health_capacity / 0x10;
        let mut full_heart_count = save.health / 0x10;
        if save.health % 0x10 == 0 {
            full_heart_count -= 1;
        }
        let pulse = self.beating_heart_oscillator as f32 * 0.1;
        let alpha = self.health_alpha as u8;
        let (mut offset_x, mut offset_y) = (0.0f32, 0.0f32);
        for heart_index in 0..total_heart_count {
            let (prim, env) = if heart_index == full_heart_count {
                (self.beating_heart_prim, self.beating_heart_env)
            } else {
                let p = self.hearts_prim[0];
                let e = self.hearts_env[0];
                ([p[0] as u8, p[1] as u8, p[2] as u8], [e[0] as u8, e[1] as u8, e[2] as u8])
            };
            let tex = if heart_index < full_heart_count {
                HEART_FULL
            } else if heart_index == full_heart_count {
                HEART_BY_FRACTION[cur_heart_fraction]
            } else {
                HEART_EMPTY
            };
            let prim = Some([prim[0], prim[1], prim[2], alpha]);
            let env = Some([env[0], env[1], env[2], 255]);
            if heart_index != full_heart_count {
                // 16 texels scaled by 0.68, on quarter pixels.
                let (cx, cy) = (30.0 + offset_x, 26.0 + offset_y);
                let half = 8.0 * 0.68;
                let q = |v: f32| ((v * 4.0) as i32) as f32 / 4.0;
                out.push(Sprite::rect(heart_sprite(tex), q(cx - half), q(cy - half), q(cx + half), q(cy + half), prim, env));
            } else {
                // Matrix_SetTranslateScaleMtx2 in the overlay's space, beatingHeartVtx ±8.
                let s = 1.0 - 0.32 * pulse;
                let m = Mat4::from_translation(Vec3::new(-130.0 + offset_x, 94.5 - offset_y, 0.0)) * Mat4::from_scale(Vec3::splat(s));
                out.push(Sprite { name: beating_heart_sprite(tex), transform: m, prim, env });
            }
            offset_x += 10.0;
            if heart_index == 9 {
                offset_y += 10.0;
                offset_x = 0.0;
            }
        }
    }
}

/// `Interface_DrawItemIconTexture`: button `i`'s item icon, `G_CC_MODULATERGBA_PRIM` with white
/// at the button's alpha. Items past the fishing rod have no icon in `icon_item_static`.
fn item_icon(i: usize, item: u8, alpha: i16, out: &mut Vec<Sprite>) {
    if item > LAST_ICON_ITEM {
        log::debug!("the HUD has no icon for item {item:#04x}");
        return;
    }
    let (x, y, w) = (ITEM_BTN_X[i] as f32, ITEM_BTN_Y[i] as f32, ITEM_ICON_WIDTH[i] as f32);
    out.push(Sprite::rect(item_icon_sprite(item), x, y, x + w, y + w, Some([255, 255, 255, alpha as u8]), None));
}

/// `Interface_DrawAmmoCount`: the ammo of a C button's stick, nut, bomb, bow (or magic arrow),
/// slingshot, bombchu or bean, in green when full and grey at 0 (white otherwise), the tens only
/// when there are any. (B's minigame counts aren't ported.)
fn draw_ammo_count(save: &SaveContext, button: usize, alpha: i16, out: &mut Vec<Sprite>) {
    use crate::item::*;
    let mut i = save.equips.button_items[button];
    if !(i == ITEM_STICK || i == ITEM_NUT || i == ITEM_BOMB || i == ITEM_BOW || (ITEM_BOW_ARROW_FIRE..=ITEM_BOW_ARROW_LIGHT).contains(&i) || i == ITEM_SLINGSHOT || i == ITEM_BOMBCHU || i == ITEM_BEAN) {
        return;
    }
    if (ITEM_BOW_ARROW_FIRE..=ITEM_BOW_ARROW_LIGHT).contains(&i) {
        i = ITEM_BOW;
    }
    let mut ammo = save.ammo(i) as i16;
    let a = alpha as u8;
    let mut prim = [255, 255, 255, a];
    let full = |upg: usize| ammo == save.cur_capacity(upg) as i16;
    if (i == ITEM_BOW && full(UPG_QUIVER))
        || (i == ITEM_BOMB && full(UPG_BOMB_BAG))
        || (i == ITEM_SLINGSHOT && full(UPG_BULLET_BAG))
        || (i == ITEM_STICK && full(UPG_STICKS))
        || (i == ITEM_NUT && full(UPG_NUTS))
        || (i == ITEM_BOMBCHU && ammo == 50)
        || (i == ITEM_BEAN && ammo == 15)
    {
        prim = [120, 255, 0, a];
    }
    if ammo == 0 {
        prim = [100, 100, 100, a];
    }
    let mut tens = 0;
    while ammo >= 10 {
        ammo -= 10;
        tens += 1;
    }
    let (x, y) = (ITEM_AMMO_X[button] as f32, ITEM_AMMO_Y[button] as f32);
    // Gfx_TextureIA8(gAmmoDigit0Tex + 64 * n, 8, 8, x, y, 8, 8, 1 << 10, 1 << 10); env (0, 0,
    // 0, 255) from Interface_DrawItemButtons.
    let env = Some([0, 0, 0, 255]);
    if tens != 0 {
        out.push(Sprite::rect(ammo_digit_sprite(tens as usize), x, y, x + 8.0, y + 8.0, Some(prim), env));
    }
    out.push(Sprite::rect(ammo_digit_sprite(ammo.clamp(0, 9) as usize), x + 6.0, y, x + 14.0, y + 8.0, Some(prim), env));
}

/// `x` down to `max` unless it's 0 (`if ((x != 0) && (x > max)) x = max`).
fn clamp_down(x: &mut i16, max: i16) {
    if *x != 0 && *x > max {
        *x = max;
    }
}

/// `func_8008A8B8(play, topY, bottomY, leftX, rightX)`: a viewport over that part of the screen,
/// its eye at 0 looking down -z, `View_SetPerspective(60, 10, 60)`, as a transform into the
/// overlay's space (the projection's result placed on the viewport; the GPU divides by w).
fn a_button_view(top: f32, bottom: f32, left: f32, right: f32) -> Mat4 {
    let aspect = (right - left) / (bottom - top);
    let persp = eng_math::gu_perspective(60.0, aspect, 10.0, 60.0);
    let (hw, hh) = ((right - left) / 2.0, (bottom - top) / 2.0);
    let (cx, cy) = ((left + right) / 2.0 - 160.0, 120.0 - (top + bottom) / 2.0);
    // Clip x, y onto the viewport's rectangle; z to 0 (the overlay has no depth).
    let place = Mat4::from_cols(Vec4::new(hw, 0.0, 0.0, 0.0), Vec4::new(0.0, hh, 0.0, 0.0), Vec4::ZERO, Vec4::new(cx, cy, 0.0, 1.0));
    place * persp
}

// The HUD's sprites.
const RUPEE_ICON: &str = "hud/rupee_icon";
const BUTTON: &str = "hud/button";
const A_BUTTON: &str = "hud/a_button";
const EMPTY_C: [&str; 3] = ["hud/empty_c_left", "hud/empty_c_down", "hud/empty_c_right"];
const HEART_EMPTY: usize = 0;
const HEART_FULL: usize = 4;
/// `gHeartEmptyTex`, `gHeartQuarterTex`, `gHeartHalfTex`, `gHeartThreeQuarterTex`,
/// `gHeartFullTex` (parameter_static).
const HEART_TEX: [&str; 5] = ["gHeartEmptyTex", "gHeartQuarterTex", "gHeartHalfTex", "gHeartThreeQuarterTex", "gHeartFullTex"];
/// The last item with a 32x32 icon at `item * 0x1000` in `icon_item_static` (`gItemIcons`):
/// `ITEM_FISHING_POLE`.
pub const LAST_ICON_ITEM: u8 = 0x59;

fn heart_sprite(i: usize) -> String {
    format!("hud/heart{i}")
}
fn beating_heart_sprite(i: usize) -> String {
    format!("hud/heart_beat{i}")
}
fn digit_sprite(d: usize) -> String {
    format!("hud/digit{d}")
}
fn item_icon_sprite(item: u8) -> String {
    format!("hud/item{item:02X}")
}
fn ammo_digit_sprite(d: usize) -> String {
    format!("hud/ammo_digit{d}")
}
pub fn do_action_sprite(action: u16) -> String {
    format!("hud/do_action{action:02X}")
}

/// `SETUPDL_42`: `gsSPTexture(0xFFFF, 0xFFFF, 0, G_TX_RENDERTILE, G_ON)`,
/// `gsDPSetCombineMode(G_CC_MODULATEIDECALA, G_CC_MODULATEIDECALA)`,
/// `gsDPSetOtherMode(G_AD_NOTPATTERN | G_CD_MAGICSQ | G_CK_NONE | G_TC_FILT | G_TF_BILERP |
/// G_TT_NONE | G_TL_TILE | G_TD_CLAMP | G_TP_PERSP | G_CYC_1CYCLE | G_PM_NPRIMITIVE,
/// G_AC_NONE | G_ZS_PIXEL | G_RM_XLU_SURF | G_RM_XLU_SURF2)`,
/// `gsSPLoadGeometryMode(G_SHADE | G_CULL_BACK | G_SHADING_SMOOTH)`.
fn setup_dl_42() -> Dl {
    let mut d = Dl::default();
    d.pipe_sync();
    d.0.push((0xD700_0002, 0xFFFF_FFFF));
    // G_CC_MODULATEIDECALA: TEXEL0, 0, SHADE, 0, 0, 0, 0, TEXEL0 (SHADE is 4 in every slot).
    let cc = [cc_ab::TEXEL0, cc_ab::ZERO, cc_ab::SHADE, cc_d::ZERO, ac::ZERO, ac::ZERO, ac::ZERO, ac::TEXEL0];
    d.combine_lerp(cc, cc);
    d.0.push((0xEF08_2C10, 0x0050_4240));
    d.0.push((0xD900_0000, 0x0020_0404));
    d
}

/// `gSPClearGeometryMode(G_CULL_BOTH)`, `gSPSetGeometryMode(G_CULL_BACK)` (F3DEX2 `G_GEOMETRYMODE`).
fn clear_cull(d: &mut Dl) {
    d.0.push((0xD9FF_F9FF, 0));
}
fn set_cull_back(d: &mut Dl) {
    d.0.push((0xD9FF_FFFF, 0x0000_0400));
}

/// `gDPSetCombineLERP(PRIMITIVE, ENVIRONMENT, TEXEL0, ENVIRONMENT, TEXEL0, 0, PRIMITIVE, 0, x2)`.
const PRIM_ENV_BY_TEXEL: [u32; 8] = [cc_ab::PRIMITIVE, cc_ab::ENVIRONMENT, cc_c::TEXEL0, cc_d::ENVIRONMENT, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];
/// `gDPSetCombineLERP(0, 0, 0, PRIMITIVE, TEXEL0, 0, PRIMITIVE, 0, x2)`.
const PRIM_BY_TEXEL_ALPHA: [u32; 8] = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::PRIMITIVE, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];

/// The HUD's sprites, each with the setup `Interface_Draw` and `Health_DrawMeter` draw it
/// under.
pub fn bakes() -> Vec<SpriteBake> {
    let mut v = Vec::new();
    let param = |symbol: &str| TexSrc::Symbol { file: "parameter_static".into(), symbol: symbol.into() };
    let ia8 = |w: u32, h: u32| Load::new(G_IM_FMT_IA, G_IM_SIZ_8B, w, h, G_TX_WRAP);
    // Health_DrawMeter: Gfx_SetupDL_39Overlay + the prim/env combiner for the hearts; the
    // beating one after Gfx_SetupDL_42Overlay, beatingHeartVtx.
    for (i, t) in HEART_TEX.iter().enumerate() {
        let mut setup = setup_dl::setup_dl_39();
        setup.combine_lerp(PRIM_ENV_BY_TEXEL, PRIM_ENV_BY_TEXEL);
        v.push(SpriteBake { name: heart_sprite(i), tex: param(t), load: ia8(16, 16), setup, prim: true, env: true, quad: Quad::Rect { s: 16, t: 16 } });
        let mut setup = setup_dl_42();
        setup.combine_lerp(PRIM_ENV_BY_TEXEL, PRIM_ENV_BY_TEXEL);
        let vtx = [([-8, 8, 0], [0, 0]), ([8, 8, 0], [512, 0]), ([-8, -8, 0], [0, 512]), ([8, -8, 0], [512, 512])];
        v.push(SpriteBake { name: beating_heart_sprite(i), tex: param(t), load: ia8(16, 16), setup, prim: true, env: true, quad: Quad::Vtx(vtx) });
    }
    // The rupee icon after Gfx_SetupDL_39Overlay (G_CC_MODULATEIA_PRIM).
    v.push(SpriteBake { name: RUPEE_ICON.into(), tex: param("gRupeeCounterIconTex"), load: ia8(16, 16), setup: setup_dl::setup_dl_39(), prim: true, env: true, quad: Quad::Rect { s: 16, t: 16 } });
    // The counter digits: Gfx_TextureI8 after the prim-alpha combiner.
    for d in 0..10 {
        let mut setup = setup_dl::setup_dl_39();
        setup.combine_lerp(PRIM_BY_TEXEL_ALPHA, PRIM_BY_TEXEL_ALPHA);
        v.push(SpriteBake {
            name: digit_sprite(d),
            tex: param(&format!("gCounterDigit{d}Tex")),
            load: Load::new(G_IM_FMT_I, G_IM_SIZ_8B, 8, 16, G_TX_WRAP),
            setup,
            prim: true,
            env: false,
            quad: Quad::Rect { s: 8, t: 16 },
        });
    }
    // Interface_DrawItemButtons: G_CC_MODULATEIA_PRIM after Gfx_SetupDL_39Overlay.
    let button = |name: &str, symbol: &str| SpriteBake { name: name.into(), tex: param(symbol), load: ia8(32, 32), setup: setup_dl::setup_dl_39(), prim: true, env: false, quad: Quad::Rect { s: 32, t: 32 } };
    v.push(button(BUTTON, "gButtonBackgroundTex"));
    for (name, sym) in EMPTY_C.iter().zip(["gEmptyCLeftArrowTex", "gEmptyCDownArrowTex", "gEmptyCRightArrowTex"]) {
        v.push(button(name, sym));
    }
    // Interface_DrawItemIconTexture: icon_item_static's 32x32 RGBA32 icon (G_CC_MODULATERGBA_PRIM,
    // the same combiner as MODULATEIA_PRIM), for every item up to the fishing rod.
    for item in 0..=LAST_ICON_ITEM {
        v.push(SpriteBake {
            name: item_icon_sprite(item),
            tex: TexSrc::File { file: "icon_item_static".into(), offset: item as u32 * 0x1000 },
            load: Load::new(G_IM_FMT_RGBA, G_IM_SIZ_32B, 32, 32, G_TX_WRAP),
            setup: setup_dl::setup_dl_39(),
            prim: true,
            env: false,
            quad: Quad::Rect { s: 32, t: 32 },
        });
    }
    // Interface_DrawAmmoCount: the digits after the prim-and-env combiner (PRIMITIVE,
    // ENVIRONMENT, TEXEL0, ENVIRONMENT, TEXEL0, 0, PRIMITIVE, 0), gAmmoDigit0Tex + 64 * n.
    for d in 0..10 {
        let mut setup = setup_dl::setup_dl_39();
        setup.combine_lerp(PRIM_ENV_BY_TEXEL, PRIM_ENV_BY_TEXEL);
        v.push(SpriteBake {
            name: ammo_digit_sprite(d),
            tex: param(&format!("gAmmoDigit{d}Tex")),
            load: ia8(8, 8),
            setup,
            prim: true,
            env: true,
            quad: Quad::Rect { s: 8, t: 8 },
        });
    }
    // Interface_DrawActionButton after Gfx_SetupDL_42Overlay, G_CULL_BOTH cleared and
    // G_CC_MODULATEIA_PRIM: actionVtx[0..4], 28 across.
    let mut setup = setup_dl_42();
    clear_cull(&mut setup);
    setup.combine_lerp(setup_dl::MODULATEIA_PRIM, setup_dl::MODULATEIA_PRIM);
    let vtx = [([-14, 14, 0], [0, 0]), ([14, 14, 0], [1024, 0]), ([-14, -14, 0], [0, 1024]), ([14, -14, 0], [1024, 1024])];
    v.push(SpriteBake { name: A_BUTTON.into(), tex: param("gButtonBackgroundTex"), load: ia8(32, 32), setup, prim: true, env: false, quad: Quad::Vtx(vtx) });
    // Interface_DrawActionLabel: G_CULL_BACK set, the prim/env combiner, env (0, 0, 0, 0);
    // actionVtx[4..8] (XREG(21) by XREG(28)); do_action_static's English label.
    let (hx, hy) = (XREG_21 / 2, XREG_28 / 2);
    let vtx = [([-hx, hy, 0], [0, 0]), ([-hx + XREG_21, hy, 0], [1536, 0]), ([-hx, hy - XREG_28, 0], [0, 512]), ([-hx + XREG_21, hy - XREG_28, 0], [1536, 512])];
    for action in 0..DO_ACTION_MAX {
        if action == DO_ACTION_NONE {
            continue;
        }
        let mut setup = setup_dl_42();
        clear_cull(&mut setup);
        set_cull_back(&mut setup);
        setup.combine_lerp(PRIM_ENV_BY_TEXEL, PRIM_ENV_BY_TEXEL);
        setup.env_color([0, 0, 0, 0]);
        v.push(SpriteBake {
            name: do_action_sprite(action),
            tex: TexSrc::File { file: "do_action_static".into(), offset: action as u32 * DO_ACTION_TEX_SIZE },
            load: Load::new(G_IM_FMT_IA, G_IM_SIZ_4B, 48, 16, G_TX_WRAP),
            setup,
            prim: true,
            env: false,
            quad: Quad::Vtx(vtx),
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_save() -> SaveContext {
        SaveContext::new(0, false, 0)
    }

    fn frame() -> IfaceFrame {
        // SCENE_SPOT04.
        IfaceFrame { scene_id: 0x55, msg_none: true, climbing: false, state2_18: false, no_transition: true, dungeon_room: false }
    }

    #[test]
    fn the_hud_fades_in_over_eight_frames_after_alpha_type_50() {
        let mut s = new_save();
        let mut c = InterfaceContext::init(&mut s, &InterfaceTables::default(), 0x55);
        change_alpha(&mut s, 50);
        let mut seen = Vec::new();
        for _ in 0..9 {
            c.update(&mut s, &frame());
            seen.push(c.b_alpha);
        }
        // alpha1 = 255 - (255 - (unk_13EC << 5)): 32 a frame, 255 from the eighth.
        assert_eq!(seen, [32, 64, 96, 128, 160, 192, 224, 255, 255]);
        assert_eq!((c.health_alpha, s.unk_13e8), (255, 0));
        // An overworld scene: the minimap stops at 170.
        assert_eq!(c.minimap_alpha, 170);
    }

    #[test]
    fn the_a_button_flips_in_four_frames() {
        let mut s = new_save();
        let mut c = InterfaceContext::init(&mut s, &InterfaceTables::default(), 0x55);
        c.set_do_action(DO_ACTION_CHECK);
        assert_eq!((c.unk_1ec, c.do_action_segment[1]), (1, Some(DO_ACTION_CHECK)));
        let mut angles = Vec::new();
        for _ in 0..4 {
            c.update(&mut s, &frame());
            angles.push((c.unk_1ec, c.unk_1f4 as i32));
        }
        // 31400 / WREG(5) (3) a frame: 10466 → past 15700, so -15700 → -5233 → 0.
        assert_eq!(angles, [(1, 10466), (2, -15700), (2, -5233), (0, 0)]);
        assert_eq!((c.unk_1ee, c.do_action_segment[0]), (DO_ACTION_CHECK, Some(DO_ACTION_CHECK)));
    }

    #[test]
    fn the_heart_meter_and_the_rupee_counter() {
        let mut s = new_save();
        let mut c = InterfaceContext::init(&mut s, &InterfaceTables::default(), 0x55);
        c.health_alpha = 255;
        // 2.5 hearts of 3: two full, the third half (sHeartTextures[8]) and beating.
        s.health = 0x28;
        let mut out = Vec::new();
        c.health_draw_meter(&s, &mut out);
        let names: Vec<&str> = out.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["hud/heart4", "hud/heart4", "hud/heart_beat2"]);
        // The first heart: centre (30, 26), 5.44 either side, on quarter pixels.
        assert_eq!(out[0].transform, crate::sprite::rect_transform(24.5, 20.5, 35.25, 31.25));
        // Rupees: the child's wallet shows two digits.
        s.rupees = 7;
        let mut out = Vec::new();
        c.draw_hud_1(&s, &mut out);
        let digits: Vec<&str> = out.iter().filter(|x| x.name.starts_with("hud/digit")).map(|x| x.name.as_str()).collect();
        assert_eq!(digits, ["hud/digit0", "hud/digit7"]);
    }

    #[test]
    fn a_house_disables_the_b_button() {
        // sRestrictionFlags' SCENE_LINK_HOME (0x34): flags1 0x10, bButton 1.
        let tables = InterfaceTables { restrictions: vec![[0x34, 0x10, 0x10, 0x15], [0xFF, 0, 0, 0]] };
        // The map select's file: the Kokiri Sword on B.
        let mut s = SaveContext::debug(0, false, 0);
        let mut c = InterfaceContext::init(&mut s, &tables, 0x34);
        assert_eq!(c.restrictions.b_button, 1);
        let f = IfaceFrame { scene_id: 0x34, ..frame() };
        for _ in 0..10 {
            c.update(&mut s, &f);
        }
        // func_80083108 disables B (the sword isn't ammo) and fades back in: B at 70.
        assert_eq!((s.button_status[0], c.b_alpha, c.a_alpha), (BTN_DISABLED, 70, 255));
        // A new file has nothing on B and infTable[INFTABLE_1DX_INDEX] set: B is left as it is.
        let mut s = new_save();
        let mut c = InterfaceContext::init(&mut s, &tables, 0x34);
        for _ in 0..10 {
            c.update(&mut s, &f);
        }
        assert_eq!((s.button_status[0], s.equips.button_items[0]), (BTN_ENABLED, crate::item::ITEM_NONE));
    }
}
