//! The message box (`z_message.c`, with the font loads of `z_kanfont.c`): the text of signs
//! and NPCs, typed one character a frame into the box, A to go on or close.
//!
//! - **The text** comes from the pack's `table/messages` (`MessageTable`): the ROM's English
//!   `sNesMessageEntryTable` and `nes_message_data_static`, read by the importer. German,
//!   French and the credits (`sStaffMessageEntryTable`) aren't imported.
//! - **The frame:** `Message_Update` runs in `Play_Update` after the actors (`update`), and
//!   `Message_Draw` in `Play_Draw` after the interface. The draw changes state too
//!   (`Message_DrawText` advances the typing and ends the box, `Message_DrawTextboxIcon` flashes
//!   the arrow), so like the reticle it runs once per game frame (`draw_update`), leaving the
//!   sprites it drew; `draw` only submits them.
//! - **Drawing** (docs/adr/0017-interface-sprites.md): each `gSPTextureRectangle` is a
//!   `Sprite` of a baked quad: the textbox per type, one per font glyph, and the three icons.
//!
//! Ported: the plain textbox modes (start, grow, decode, type, await input, box breaks, next
//! text ids, choices, fading, persistent and event ends, closing), the colours, the player's
//! name. Not ported: the ocarina modes, item icons and textbox backgrounds (decoded, not
//! drawn), the credits, the ocarina's sounds, and the debug message viewer (`BREG(0)` 0). `YREG(31)` is the
//! shop's (`yreg_31`).

#![allow(non_snake_case)] // sMessageStartFrameCount and the unk_ fields keep the decomp's names

use eng_gfx::DrawLists;
use eng_input::pad::{BTN_A, BTN_B, BTN_CUP, Input};

use crate::actor_ctx::ActorHandle;
use crate::gbi::{G_IM_FMT_I, G_IM_FMT_IA, G_IM_SIZ_4B, G_TX_CLAMP, G_TX_MIRROR, G_TX_NOMIRROR, ac, cc_ab, cc_c, cc_d, setup_dl};
use crate::interface::{DO_ACTION_NEXT, DO_ACTION_RETURN, InterfaceContext, change_alpha};
use crate::item::QUEST_HEART_PIECE_COUNT;
use crate::save::SaveContext;
use crate::sprite::{Load, Quad, Sprite, SpriteBake, TexSrc};

// MSGMODE_* (z64.h).
pub const MSGMODE_NONE: u8 = 0x00;
pub const MSGMODE_TEXT_START: u8 = 0x01;
pub const MSGMODE_TEXT_BOX_GROWING: u8 = 0x02;
pub const MSGMODE_TEXT_STARTING: u8 = 0x03;
pub const MSGMODE_TEXT_NEXT_MSG: u8 = 0x04;
pub const MSGMODE_TEXT_CONTINUING: u8 = 0x05;
pub const MSGMODE_TEXT_DISPLAYING: u8 = 0x06;
pub const MSGMODE_TEXT_AWAIT_INPUT: u8 = 0x07;
pub const MSGMODE_TEXT_DELAYED_BREAK: u8 = 0x08;
pub const MSGMODE_OCARINA_STARTING: u8 = 0x09;
pub const MSGMODE_OCARINA_PLAYING: u8 = 0x0C;
pub const MSGMODE_SONG_PLAYED_ACT: u8 = 0x17;
pub const MSGMODE_SONG_DEMONSTRATION_DONE: u8 = 0x1A;
pub const MSGMODE_OCARINA_AWAIT_INPUT: u8 = 0x1F;
pub const MSGMODE_SCARECROW_LONG_RECORDING_START: u8 = 0x21;
pub const MSGMODE_TEXT_AWAIT_NEXT: u8 = 0x34;
pub const MSGMODE_TEXT_DONE: u8 = 0x35;
pub const MSGMODE_TEXT_CLOSING: u8 = 0x36;
pub const MSGMODE_PAUSED: u8 = 0x37;

// TEXT_STATE_* (z64.h): what Message_GetState reports.
pub const TEXT_STATE_NONE: u8 = 0;
pub const TEXT_STATE_DONE_HAS_NEXT: u8 = 1;
pub const TEXT_STATE_CLOSING: u8 = 2;
pub const TEXT_STATE_DONE_FADING: u8 = 3;
pub const TEXT_STATE_CHOICE: u8 = 4;
pub const TEXT_STATE_EVENT: u8 = 5;
pub const TEXT_STATE_DONE: u8 = 6;
pub const TEXT_STATE_SONG_DEMO_DONE: u8 = 7;
pub const TEXT_STATE_8: u8 = 8;
pub const TEXT_STATE_9: u8 = 9;
pub const TEXT_STATE_AWAITING_NEXT: u8 = 10;

// TEXTBOX_TYPE_* (message_data_static.h), TEXTBOX_POS_*.
pub const TEXTBOX_TYPE_BLACK: u8 = 0;
pub const TEXTBOX_TYPE_WOODEN: u8 = 1;
pub const TEXTBOX_TYPE_BLUE: u8 = 2;
pub const TEXTBOX_TYPE_OCARINA: u8 = 3;
pub const TEXTBOX_TYPE_NONE_BOTTOM: u8 = 4;
pub const TEXTBOX_TYPE_NONE_NO_SHADOW: u8 = 5;
pub const TEXTBOX_POS_TOP: u8 = 1;
pub const TEXTBOX_POS_MIDDLE: u8 = 2;

// TEXTBOX_ENDTYPE_* (z64.h).
pub const TEXTBOX_ENDTYPE_DEFAULT: u8 = 0x00;
pub const TEXTBOX_ENDTYPE_2_CHOICE: u8 = 0x10;
pub const TEXTBOX_ENDTYPE_3_CHOICE: u8 = 0x20;
pub const TEXTBOX_ENDTYPE_HAS_NEXT: u8 = 0x30;
pub const TEXTBOX_ENDTYPE_PERSISTENT: u8 = 0x40;
pub const TEXTBOX_ENDTYPE_EVENT: u8 = 0x50;
pub const TEXTBOX_ENDTYPE_FADING: u8 = 0x60;

// TextBoxIcon (z64.h).
pub const TEXTBOX_ICON_TRIANGLE: u8 = 0;
pub const TEXTBOX_ICON_SQUARE: u8 = 1;
pub const TEXTBOX_ICON_ARROW: u8 = 2;

// The control characters (message_data_fmt.h, CTRL_*).
pub const MESSAGE_NEWLINE: u8 = 0x01;
pub const MESSAGE_END: u8 = 0x02;
pub const MESSAGE_BOX_BREAK: u8 = 0x04;
pub const MESSAGE_COLOR: u8 = 0x05;
pub const MESSAGE_SHIFT: u8 = 0x06;
pub const MESSAGE_TEXTID: u8 = 0x07;
pub const MESSAGE_QUICKTEXT_ENABLE: u8 = 0x08;
pub const MESSAGE_QUICKTEXT_DISABLE: u8 = 0x09;
pub const MESSAGE_PERSISTENT: u8 = 0x0A;
pub const MESSAGE_EVENT: u8 = 0x0B;
pub const MESSAGE_BOX_BREAK_DELAYED: u8 = 0x0C;
pub const MESSAGE_AWAIT_BUTTON_PRESS: u8 = 0x0D;
pub const MESSAGE_FADE: u8 = 0x0E;
pub const MESSAGE_NAME: u8 = 0x0F;
pub const MESSAGE_OCARINA: u8 = 0x10;
pub const MESSAGE_FADE2: u8 = 0x11;
pub const MESSAGE_SFX: u8 = 0x12;
pub const MESSAGE_ITEM_ICON: u8 = 0x13;
pub const MESSAGE_TEXT_SPEED: u8 = 0x14;
pub const MESSAGE_BACKGROUND: u8 = 0x15;
pub const MESSAGE_MARATHON_TIME: u8 = 0x16;
pub const MESSAGE_RACE_TIME: u8 = 0x17;
pub const MESSAGE_POINTS: u8 = 0x18;
pub const MESSAGE_TOKENS: u8 = 0x19;
pub const MESSAGE_UNSKIPPABLE: u8 = 0x1A;
pub const MESSAGE_TWO_CHOICE: u8 = 0x1B;
pub const MESSAGE_THREE_CHOICE: u8 = 0x1C;
pub const MESSAGE_FISH_INFO: u8 = 0x1D;
pub const MESSAGE_HIGHSCORE: u8 = 0x1E;
pub const MESSAGE_TIME: u8 = 0x1F;

// MSGCOL_* (message_data_fmt.h).
pub const TEXT_COLOR_DEFAULT: u8 = 0;
pub const TEXT_COLOR_RED: u8 = 1;
pub const TEXT_COLOR_ADJUSTABLE: u8 = 2;
pub const TEXT_COLOR_BLUE: u8 = 3;
pub const TEXT_COLOR_LIGHTBLUE: u8 = 4;
pub const TEXT_COLOR_PURPLE: u8 = 5;
pub const TEXT_COLOR_YELLOW: u8 = 6;
pub const TEXT_COLOR_BLACK: u8 = 7;

/// `FONT_CHAR_TEX_SIZE`: a glyph is a 16x16 I4 texture. `MESSAGE_STATIC_TEX_SIZE`.
pub const FONT_CHAR_TEX_SIZE: u32 = 16 * 16 / 2;
pub const MESSAGE_STATIC_TEX_SIZE: u32 = 0x1000;
/// How many glyphs `nes_font_static` has (its XML's textures, `' '` to `0xAB`).
pub const FONT_GLYPHS: u8 = 140;

/// `MESSAGE_SPACE_WIDTH`.
const MESSAGE_SPACE_WIDTH: i16 = 6;

/// `sFontWidths`: each glyph's advance at scale 100.
#[rustfmt::skip]
const S_FONT_WIDTHS: [f32; 144] = [
    8.0, 8.0, 6.0, 9.0, 9.0, 14.0, 12.0, 3.0, 7.0, 7.0, 7.0, 9.0, 4.0, 6.0, 4.0, 9.0,
    10.0, 5.0, 9.0, 9.0, 10.0, 9.0, 9.0, 9.0, 9.0, 9.0, 6.0, 6.0, 9.0, 11.0, 9.0, 11.0,
    13.0, 12.0, 9.0, 11.0, 11.0, 8.0, 8.0, 12.0, 10.0, 4.0, 8.0, 10.0, 8.0, 13.0, 11.0, 13.0,
    9.0, 13.0, 10.0, 10.0, 9.0, 10.0, 11.0, 15.0, 11.0, 10.0, 10.0, 7.0, 10.0, 7.0, 10.0, 9.0,
    5.0, 8.0, 9.0, 8.0, 9.0, 9.0, 6.0, 9.0, 8.0, 4.0, 6.0, 8.0, 4.0, 12.0, 9.0, 9.0,
    9.0, 9.0, 7.0, 8.0, 7.0, 8.0, 9.0, 12.0, 8.0, 9.0, 8.0, 7.0, 5.0, 7.0, 10.0, 10.0,
    12.0, 6.0, 12.0, 12.0, 11.0, 8.0, 8.0, 8.0, 6.0, 6.0, 13.0, 13.0, 10.0, 10.0, 10.0, 9.0,
    8.0, 8.0, 8.0, 8.0, 8.0, 9.0, 9.0, 9.0, 9.0, 6.0, 9.0, 9.0, 9.0, 9.0, 9.0, 14.0,
    14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0, 14.0,
];

/// One `MessageTableEntry` of `sNesMessageEntryTable`: the text id, the box's type and
/// position (`typePos`), and its segment-7 address in `nes_message_data_static`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MessageTableEntry {
    pub text_id: u16,
    pub type_pos: u8,
    pub segment: u32,
}

/// The English messages (`table/messages`): `sNesMessageEntryTable` up to and including its
/// `0xFFFF` terminator, and the whole of `nes_message_data_static`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MessageTable {
    pub entries: Vec<MessageTableEntry>,
    pub data: Vec<u8>,
}

impl MessageTable {
    /// The entry for `text_id`.
    pub fn entry(&self, text_id: u16) -> Option<&MessageTableEntry> {
        self.entries.iter().take_while(|e| e.text_id != 0xFFFF).find(|e| e.text_id == text_id)
    }

    /// A message's raw bytes as `Message_FindMessagePAL` measures them (to the next entry).
    pub fn raw(&self, text_id: u16) -> Option<&[u8]> {
        let seg = self.entries.first()?.segment;
        let i = self.entries.iter().position(|e| e.text_id == text_id && e.text_id != 0xFFFF)?;
        let (found, next) = (self.entries[i].segment, self.entries.get(i + 1)?.segment);
        self.data.get((found - seg) as usize..(next - seg) as usize)
    }
}

/// The parts of `Font` the message box uses. The texture buffers hold which glyph is where
/// (the texels themselves are in the baked sprites).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Font {
    pub msg_offset: u32,
    pub msg_length: u32,
    /// `msgBuf`: this message's bytes.
    pub msg_buf: Vec<u8>,
    /// `charTexBuf`: the glyphs `Font_LoadChar` loaded (`character - ' '`), one per
    /// `FONT_CHAR_TEX_SIZE`.
    pub char_tex_buf: Vec<u8>,
    /// `charTexBuf[0]` as `Message_FindMessagePAL` sets it: the message's `typePos`.
    pub type_pos: u8,
    /// `iconBuf`: the `TEXTBOX_ICON_*` `Font_LoadMessageBoxIcon` loaded.
    pub icon: u8,
}

impl Font {
    /// `Font_LoadChar`.
    fn load_char(&mut self, character: u8, code_point_index: usize) {
        let slot = code_point_index / FONT_CHAR_TEX_SIZE as usize;
        if self.char_tex_buf.len() <= slot {
            self.char_tex_buf.resize(slot + 1, 0);
        }
        self.char_tex_buf[slot] = character;
    }

    /// `Font_LoadMessageBoxIcon`.
    fn load_message_box_icon(&mut self, icon: u8) {
        self.icon = icon;
    }

    fn byte(&self, pos: usize) -> u8 {
        self.msg_buf.get(pos).copied().unwrap_or(0)
    }
}

/// The debug registers the message box reads and writes (`regs.h`), with their start values
/// from `Regs_InitDataImpl` and `Regs_InitDataImpl` (`z_construct.c`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextRegs {
    /// `R_TEXT_INIT_XPOS` (`XREG(54)`), `R_TEXT_INIT_YPOS`, `R_TEXT_LINE_SPACING`,
    /// `R_TEXT_CHAR_SCALE`.
    pub text_init_xpos: i16,
    pub text_init_ypos: i16,
    pub text_line_spacing: i16,
    pub text_char_scale: i16,
    /// `R_TEXT_DROP_SHADOW_OFFSET`, `R_TEXTBOX_BG_YPOS`.
    pub text_drop_shadow_offset: i16,
    pub textbox_bg_ypos: i16,
    /// `R_TEXTBOX_END_XPOS`, `R_TEXTBOX_END_YPOS`: where the arrow or square goes.
    pub textbox_end_xpos: i16,
    pub textbox_end_ypos: i16,
    /// `R_TEXT_CHOICE_XPOS`, `R_TEXT_CHOICE_YPOS(0..3)`.
    pub text_choice_xpos: i16,
    pub text_choice_ypos: [i16; 3],
    /// `R_TEXTBOX_X_TARGET` .. `R_TEXTBOX_TEXHEIGHT_TARGET` (`XREG(72..77)`).
    pub textbox_x_target: i16,
    pub textbox_y_target: i16,
    pub textbox_width_target: i16,
    pub textbox_height_target: i16,
    pub textbox_texwidth_target: i16,
    pub textbox_texheight_target: i16,
    /// `XREG(92)`, `XREG(93)`, `XREG(94)`: the screen height below which a variable box goes
    /// to the bottom (fixed cameras, the market, elsewhere).
    pub xreg_92: i16,
    pub xreg_93: i16,
    pub xreg_94: i16,
    /// `R_TEXTBOX_X`, `R_TEXTBOX_Y` (`VREG(0)`, `VREG(1)`), `R_TEXTBOX_WIDTH`, `_HEIGHT`
    /// (`YREG(22)`, `YREG(23)`), `R_TEXTBOX_TEXWIDTH`, `_TEXHEIGHT` (`YREG(16)`, `YREG(17)`).
    pub textbox_x: i16,
    pub textbox_y: i16,
    pub textbox_width: i16,
    pub textbox_height: i16,
    pub textbox_texwidth: i16,
    pub textbox_texheight: i16,
    /// `R_TEXTBOX_ICON_XPOS`, `_YPOS`, `_SIZE` (`YREG(71)`, `(72)`, `(75)`).
    pub textbox_icon_xpos: i16,
    pub textbox_icon_ypos: i16,
    pub textbox_icon_size: i16,
    /// `R_TEXT_ADJUST_COLOR_1_*`, `_2_*` (`VREG(33..38)`).
    pub text_adjust_color_1: [i16; 3],
    pub text_adjust_color_2: [i16; 3],
}

impl Default for TextRegs {
    fn default() -> TextRegs {
        TextRegs {
            text_init_xpos: 65,
            text_init_ypos: 60,
            text_line_spacing: 16,
            text_char_scale: 80,
            text_drop_shadow_offset: 1,
            textbox_bg_ypos: 3,
            textbox_end_xpos: 158,
            textbox_end_ypos: 102,
            text_choice_xpos: 48,
            text_choice_ypos: [54, 70, 86],
            textbox_x_target: 54,
            textbox_y_target: 48,
            textbox_width_target: 128,
            textbox_height_target: 64,
            textbox_texwidth_target: 2048,
            textbox_texheight_target: 512,
            xreg_92: 100,
            xreg_93: 100,
            xreg_94: 160,
            textbox_x: 0,
            textbox_y: 0,
            // Regs_InitDataImpl.
            textbox_width: 50,
            textbox_height: 0,
            textbox_texwidth: 0,
            textbox_texheight: 0,
            textbox_icon_xpos: 0,
            textbox_icon_ypos: 0,
            textbox_icon_size: 0,
            text_adjust_color_1: [70, 255, 80],
            text_adjust_color_2: [70, 255, 80],
        }
    }
}

/// What the message box reads of the rest of play each frame.
pub struct MsgFrame<'a> {
    pub table: &'a MessageTable,
    pub input: &'a Input,
    /// The game's side of the audio: the message box's sounds.
    pub audio: &'a mut crate::audio::GameAudio,
    pub save: &'a mut SaveContext,
    pub iface: &'a mut InterfaceContext,
    /// `R_SCENE_CAM_TYPE`, `play->sceneId`.
    pub scene_cam_type: u8,
    pub scene_id: u16,
    /// `Actor_GetScreenPos`'s y of Player and of `talkActor` (when there is one).
    pub player_screen_y: i16,
    pub talk_actor_screen_y: Option<i16>,
    /// `play->csCtx.state == 0`, `play->activeCamId == CAM_ID_MAIN`.
    pub cs_idle: bool,
    pub active_cam_main: bool,
}

/// `MessageContext`, with the file's statics.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageContext {
    pub font: Font,
    pub regs: TextRegs,
    pub text_id: u16,
    pub choice_text_id: u16,
    /// `textBoxProperties`, `textBoxType`, `textBoxPos`.
    pub text_box_properties: u8,
    pub text_box_type: u8,
    pub text_box_pos: u8,
    /// `msgLength`: non-zero while a message is open.
    pub msg_length: i32,
    pub msg_mode: u8,
    /// `msgBufDecoded`: the current box's characters.
    pub msg_buf_decoded: Vec<u8>,
    pub msg_buf_pos: u16,
    pub unk_e3d0: u16,
    /// `textDrawPos`: characters up to here are drawn.
    pub text_draw_pos: u16,
    pub decoded_text_len: u16,
    pub text_unskippable: u16,
    pub text_pos_x: i16,
    pub text_pos_y: i16,
    pub text_color: [i16; 4],
    pub textbox_end_type: u8,
    pub choice_index: u8,
    pub choice_num: u8,
    pub state_timer: u8,
    pub text_delay_timer: u16,
    pub text_delay: u16,
    pub ocarina_mode: u16,
    pub ocarina_action: u16,
    pub textbox_background_idx: u16,
    pub textbox_background_fore_color_idx: u8,
    pub textbox_background_back_color_idx: u8,
    pub textbox_background_y_offset_idx: u8,
    /// `YREG(31)`: set by `En_Ossan` while Link shops (`EnOssan_SetStateStartShopping`), when
    /// the shopkeeper drives the box: no do-action changes, no B to skip, no A at a box break
    /// or at the end. `Message_Init` (`z_construct.c`) zeroes it.
    pub yreg_31: i16,
    pub textbox_background_unk_arg: u8,
    /// `textboxColorRed` .. `Blue`, `AlphaTarget`, `AlphaCurrent`.
    pub textbox_color: [i16; 3],
    pub textbox_color_alpha_target: i16,
    pub textbox_color_alpha_current: i16,
    pub talk_actor: Option<ActorHandle>,
    pub last_ocarina_button_index: u8,
    s: Statics,
    /// What `Message_Draw` drew this game frame (`draw_update`).
    pub sprites: Vec<Sprite>,
}

/// `z_message.c`'s file statics.
#[derive(Debug, Clone, PartialEq)]
struct Statics {
    /// `sTextFade`, `sMessageStartFrameCount`, `sTextboxSkipped`, `sNextTextId`, `sMessageHasSetSfx`.
    text_fade: bool,
    message_start_frame_count: u8,
    textbox_skipped: bool,
    next_text_id: u16,
    message_has_set_sfx: bool,
    /// `sLastPlayedSong` (0xFF).
    last_played_song: i16,
    /// `Message_HandleChoiceSelection`'s `sAnalogStickHeld`.
    analog_stick_held: bool,
    /// `Message_Update`'s `D_80153D74`.
    d_80153d74: u8,
    /// `Message_DrawTextboxIcon`'s `sIconPrimR`.., `sIconFlashTimer`, `sIconFlashColorIdx`,
    /// `sIconEnvR`..
    icon_prim: [i16; 3],
    icon_flash_timer: i16,
    icon_flash_color_idx: usize,
    icon_env: [i16; 3],
}

impl Default for Statics {
    fn default() -> Statics {
        Statics {
            text_fade: false,
            message_start_frame_count: 0,
            textbox_skipped: false,
            next_text_id: 0,
            message_has_set_sfx: false,
            last_played_song: 0xFF,
            analog_stick_held: false,
            d_80153d74: 0,
            icon_prim: [0, 200, 80],
            icon_flash_timer: 12,
            icon_flash_color_idx: 0,
            icon_env: [0, 0, 0],
        }
    }
}

impl Default for MessageContext {
    fn default() -> MessageContext {
        MessageContext::new()
    }
}

/// `Message_ShouldAdvanceSilent`: A, B or C-Up pressed.
pub fn should_advance_silent(input: &Input) -> bool {
    input.press.held(BTN_A) || input.press.held(BTN_B) || input.press.held(BTN_CUP)
}

/// `Message_ShouldAdvance`: the same, with `NA_SE_SY_MESSAGE_PASS` when it is.
pub fn should_advance(input: &Input, audio: &mut crate::audio::GameAudio) -> bool {
    let pressed = should_advance_silent(input);
    if pressed {
        audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_MESSAGE_PASS);
    }
    pressed
}

/// `Audio_PlaySfxGeneral(0, &gSfxDefaultPos, ...)`: the message box's silent sound (id 0: the
/// request is queued, and `Audio_ProcessSfxRequest` drops it).
fn sfx_none(audio: &mut crate::audio::GameAudio) {
    audio.play_sfx_centered(0);
}

/// The textboxes' sprite names.
pub fn box_sprite(text_box_type: u8) -> String {
    format!("message/box{text_box_type}")
}
pub fn glyph_sprite(glyph: u8) -> String {
    format!("message/char{:02X}", glyph as u32 + 0x20)
}
pub fn icon_sprite(icon: u8) -> String {
    format!("message/icon{icon}")
}
pub fn item_icon_sprite(item: u8) -> String {
    format!("message/item{item:02X}")
}

/// Where `Message_LoadItemIcon` loads an item's icon from: `(file, offset, size)`. Items below
/// `ITEM_MEDALLION_FOREST` are `icon_item_static`'s 32x32 icons (`item * 0x1000`, up to the
/// fishing rod); the medallions, stones and dungeon items `icon_item_24_static`'s 24x24 ones
/// (`(item - ITEM_MEDALLION_FOREST) * 0x900`, up to the large magic jar).
pub fn item_icon_texture(item: u8) -> Option<(&'static str, u32, u32)> {
    use crate::item::{ITEM_MAGIC_JAR_BIG, ITEM_MEDALLION_FOREST};
    if item <= crate::interface::LAST_ICON_ITEM {
        Some(("icon_item_static", item as u32 * 0x1000, 32))
    } else if (ITEM_MEDALLION_FOREST..=ITEM_MAGIC_JAR_BIG).contains(&item) {
        Some(("icon_item_24_static", (item - ITEM_MEDALLION_FOREST) as u32 * 0x900, 24))
    } else {
        None
    }
}

impl MessageContext {
    /// `Message_Init`: nothing open (`Font_LoadOrderedFont`, the name entry's font, isn't
    /// needed).
    pub fn new() -> MessageContext {
        MessageContext {
            font: Font::default(),
            regs: TextRegs::default(),
            text_id: 0,
            choice_text_id: 0,
            text_box_properties: 0,
            text_box_type: 0,
            text_box_pos: 0,
            msg_length: 0,
            msg_mode: MSGMODE_NONE,
            msg_buf_decoded: vec![0; 200],
            msg_buf_pos: 0,
            unk_e3d0: 0,
            text_draw_pos: 0,
            decoded_text_len: 0,
            text_unskippable: 0,
            text_pos_x: 0,
            text_pos_y: 0,
            text_color: [255, 255, 255, 255],
            textbox_end_type: 0,
            choice_index: 0,
            choice_num: 0,
            state_timer: 0,
            text_delay_timer: 0,
            text_delay: 0,
            ocarina_mode: 0,
            ocarina_action: 0,
            textbox_background_idx: 0,
            textbox_background_fore_color_idx: 0,
            textbox_background_back_color_idx: 0,
            textbox_background_y_offset_idx: 0,
            yreg_31: 0,
            textbox_background_unk_arg: 0,
            textbox_color: [0; 3],
            textbox_color_alpha_target: 0,
            textbox_color_alpha_current: 0,
            talk_actor: None,
            last_ocarina_button_index: 0,
            s: Statics::default(),
            sprites: Vec::new(),
        }
    }

    fn decoded(&self, i: usize) -> u8 {
        self.msg_buf_decoded.get(i).copied().unwrap_or(0)
    }

    fn set_decoded(&mut self, i: usize, v: u8) {
        if self.msg_buf_decoded.len() <= i {
            self.msg_buf_decoded.resize(i + 1, 0);
        }
        self.msg_buf_decoded[i] = v;
    }

    /// `Message_GetState`.
    pub fn get_state(&self) -> u8 {
        if self.msg_length == 0 {
            TEXT_STATE_NONE
        } else if self.msg_mode == MSGMODE_TEXT_DONE {
            match self.textbox_end_type {
                TEXTBOX_ENDTYPE_HAS_NEXT => TEXT_STATE_DONE_HAS_NEXT,
                TEXTBOX_ENDTYPE_2_CHOICE | TEXTBOX_ENDTYPE_3_CHOICE => TEXT_STATE_CHOICE,
                TEXTBOX_ENDTYPE_EVENT | TEXTBOX_ENDTYPE_PERSISTENT => TEXT_STATE_EVENT,
                TEXTBOX_ENDTYPE_FADING => TEXT_STATE_DONE_FADING,
                _ => TEXT_STATE_DONE,
            }
        } else if self.msg_mode == MSGMODE_TEXT_AWAIT_NEXT {
            TEXT_STATE_AWAITING_NEXT
        } else if self.msg_mode == MSGMODE_SONG_DEMONSTRATION_DONE {
            TEXT_STATE_SONG_DEMO_DONE
        } else if self.ocarina_mode == 3 {
            TEXT_STATE_8
        } else if self.msg_mode == MSGMODE_OCARINA_AWAIT_INPUT {
            TEXT_STATE_9
        } else if self.msg_mode == MSGMODE_TEXT_CLOSING && self.state_timer == 1 {
            TEXT_STATE_CLOSING
        } else {
            TEXT_STATE_DONE_FADING
        }
    }

    /// `Message_CloseTextbox`.
    pub fn close_textbox(&mut self, audio: &mut crate::audio::GameAudio) {
        if self.msg_length != 0 {
            self.state_timer = 2;
            self.msg_mode = MSGMODE_TEXT_CLOSING;
            self.textbox_end_type = TEXTBOX_ENDTYPE_DEFAULT;
            sfx_none(audio);
        }
    }

    /// `Message_HandleChoiceSelection`.
    fn handle_choice_selection(&mut self, input: &Input, audio: &mut crate::audio::GameAudio, num_choices: u8) {
        use crate::audio::sfx::NA_SE_SY_CURSOR;
        let stick_y = input.rel.stick_y as i32;
        if stick_y >= 30 && !self.s.analog_stick_held {
            self.s.analog_stick_held = true;
            self.choice_index = self.choice_index.wrapping_sub(1);
            if self.choice_index > 128 {
                self.choice_index = 0;
            } else {
                audio.play_sfx_centered(NA_SE_SY_CURSOR);
            }
        } else if stick_y <= -30 && !self.s.analog_stick_held {
            self.s.analog_stick_held = true;
            self.choice_index += 1;
            if self.choice_index > num_choices {
                self.choice_index = num_choices;
            } else {
                audio.play_sfx_centered(NA_SE_SY_CURSOR);
            }
        } else if stick_y.abs() < 30 {
            self.s.analog_stick_held = false;
        }
        self.text_pos_x = self.regs.text_choice_xpos;
        self.text_pos_y = if num_choices == 1 { self.regs.text_choice_ypos[self.choice_index as usize + 1] } else { self.regs.text_choice_ypos[self.choice_index as usize] };
    }

    /// `Message_DrawTextChar`: the shadow one pixel down and right, then the glyph.
    fn draw_text_char(&mut self, glyph: u8, out: &mut Vec<Sprite>) {
        let (x, y) = (self.text_pos_x as f32, self.text_pos_y as f32);
        let size = ((self.regs.text_char_scale as f32 / 100.0) * 16.0) as i32 as f32;
        let name = glyph_sprite(glyph);
        let alpha = self.text_color[3] as u8;
        if self.text_box_type != TEXTBOX_TYPE_NONE_NO_SHADOW {
            let o = self.regs.text_drop_shadow_offset as f32;
            out.push(Sprite::rect(name.clone(), x + o, y + o, x + o + size, y + o + size, Some([0, 0, 0, alpha]), None));
        }
        let c = self.text_color;
        out.push(Sprite::rect(name, x, y, x + size, y + size, Some([c[0] as u8, c[1] as u8, c[2] as u8, alpha]), None));
    }

    /// `Message_GrowTextbox`: eight frames from a wide flat box to the full one, fading in.
    fn grow_textbox(&mut self) {
        const WIDTH: [f32; 8] = [1.2, 1.5, 1.8, 2.0, 2.1, 2.2, 2.1, 2.0];
        const HEIGHT: [f32; 8] = [0.6, 0.75, 0.9, 1.0, 1.05, 1.1, 1.05, 1.0];
        let r = &mut self.regs;
        let t = self.state_timer as usize;
        let width = r.textbox_width_target as f32 * (WIDTH[t] + WIDTH[t]);
        let height = r.textbox_height_target as f32 * HEIGHT[t];
        let tex_width = r.textbox_texwidth_target as f32 / (WIDTH[t] + WIDTH[t]);
        let tex_height = r.textbox_texheight_target as f32 / HEIGHT[t];
        r.textbox_y = r.textbox_y_target + (r.textbox_y_target - (r.textbox_y_target as f32 * HEIGHT[t] + 0.5) as i16) / 2;
        self.textbox_color_alpha_current += self.textbox_color_alpha_target / 8;
        self.state_timer += 1;
        let r = &mut self.regs;
        if self.state_timer == 8 {
            r.textbox_x = r.textbox_x_target;
            r.textbox_y = r.textbox_y_target;
            self.msg_mode = MSGMODE_TEXT_STARTING;
            self.textbox_color_alpha_current = self.textbox_color_alpha_target;
        }
        r.textbox_width = (width + 0.5) as i16 / 2;
        r.textbox_height = (height + 0.5) as i16;
        r.textbox_texwidth = (tex_width + 0.5) as i16;
        r.textbox_texheight = (tex_height + 0.5) as i16;
        r.textbox_x = (r.textbox_x_target + r.textbox_width_target) - (r.textbox_width / 2);
    }

    /// `Message_FindMessagePAL` for `LANGUAGE_ENG`: the message's offset and length (to the next
    /// entry), and its `typePos`. A missing id gives the table's first message.
    fn find_message(&mut self, table: &MessageTable, text_id: u16) {
        let Some(first) = table.entries.first() else { return };
        let seg = first.segment;
        let mut i = 0;
        while i < table.entries.len() && table.entries[i].text_id != 0xFFFF {
            if table.entries[i].text_id == text_id {
                break;
            }
            i += 1;
        }
        if i >= table.entries.len() || table.entries[i].text_id == 0xFFFF {
            log::debug!("message {text_id:#06x} not found");
            i = 0;
        }
        let found = table.entries[i].segment;
        self.font.type_pos = table.entries[i].type_pos;
        let next = table.entries.get(i + 1).map(|e| e.segment).unwrap_or(found);
        self.font.msg_offset = found.wrapping_sub(seg);
        self.font.msg_length = next.wrapping_sub(found);
    }

    /// `Message_SetTextColor`.
    fn set_text_color(&mut self, color: u8) {
        let wooden = self.text_box_type == TEXTBOX_TYPE_WOODEN;
        let rgb = match color {
            TEXT_COLOR_RED => if wooden { [255, 120, 0] } else { [255, 60, 60] },
            TEXT_COLOR_ADJUSTABLE => if wooden { self.regs.text_adjust_color_1 } else { self.regs.text_adjust_color_2 },
            TEXT_COLOR_BLUE => if wooden { [80, 110, 255] } else { [80, 90, 255] },
            TEXT_COLOR_LIGHTBLUE => {
                if wooden {
                    [90, 180, 255]
                } else if self.text_box_type == TEXTBOX_TYPE_NONE_NO_SHADOW {
                    [80, 150, 180]
                } else {
                    [100, 180, 255]
                }
            }
            TEXT_COLOR_PURPLE => if wooden { [210, 100, 255] } else { [255, 150, 180] },
            TEXT_COLOR_YELLOW => if wooden { [255, 255, 30] } else { [225, 255, 50] },
            TEXT_COLOR_BLACK => [0, 0, 0],
            _ => if self.text_box_type == TEXTBOX_TYPE_NONE_NO_SHADOW { [0, 0, 0] } else { [255, 255, 255] },
        };
        self.text_color[..3].copy_from_slice(&rgb);
    }

    /// `Message_DrawTextboxIcon`: the arrow, square or choice arrow, its colours flashing
    /// between two pairs over 12 frames.
    fn draw_textbox_icon(&mut self, x: i16, y: i16, out: &mut Vec<Sprite>) {
        const PRIM: [[i16; 3]; 2] = [[0, 200, 80], [50, 255, 130]];
        const ENV: [[i16; 3]; 2] = [[0, 0, 0], [0, 255, 130]];
        let s = &mut self.s;
        let idx = s.icon_flash_color_idx;
        for k in 0..3 {
            let step = (s.icon_prim[k] - PRIM[idx][k]).abs() / s.icon_flash_timer;
            if s.icon_prim[k] >= PRIM[idx][k] {
                s.icon_prim[k] -= step;
            } else {
                s.icon_prim[k] += step;
            }
        }
        for k in 0..3 {
            let step = (s.icon_env[k] - ENV[idx][k]).abs() / s.icon_flash_timer;
            if s.icon_env[k] >= ENV[idx][k] {
                s.icon_env[k] -= step;
            } else {
                s.icon_env[k] += step;
            }
        }
        s.icon_flash_timer -= 1;
        if s.icon_flash_timer == 0 {
            s.icon_prim = PRIM[idx];
            s.icon_env = ENV[idx];
            s.icon_flash_timer = 12;
            s.icon_flash_color_idx ^= 1;
        }
        let (p, e) = (self.s.icon_prim, self.s.icon_env);
        let size = (16.0 * (self.regs.text_char_scale as f32 / 100.0)) as i32 as f32;
        let (x, y) = (x as f32, y as f32);
        out.push(Sprite::rect(icon_sprite(self.font.icon), x, y, x + size, y + size, Some([p[0] as u8, p[1] as u8, p[2] as u8, 255]), Some([e[0] as u8, e[1] as u8, e[2] as u8, 255])));
        self.state_timer = self.state_timer.wrapping_add(1);
    }

    /// `Message_DrawItemIcon`: the item's icon (`Message_LoadItemIcon`'s: 32x32 from
    /// `icon_item_static` below `ITEM_MEDALLION_FOREST`, else 24x24 from `icon_item_24_static`)
    /// at the icon registers, `G_CC_MODULATEIA_PRIM` with white at the text's alpha; the text
    /// moves 32 past it, with the silent sound while it types. An item without an icon in
    /// either file (the songs read past `icon_item_static`'s icons) draws nothing.
    fn draw_item_icon(&mut self, i: u16, audio: &mut crate::audio::GameAudio, out: &mut Vec<Sprite>) -> u16 {
        if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
            sfx_none(audio);
        }
        let item = self.decoded(i as usize + 1);
        if item_icon_texture(item).is_some() {
            let r = &self.regs;
            let (x, y, s) = ((self.text_pos_x + r.textbox_icon_xpos) as f32, r.textbox_icon_ypos as f32, r.textbox_icon_size as f32);
            out.push(Sprite::rect(item_icon_sprite(item), x, y, x + s, y + s, Some([255, 255, 255, self.text_color[3] as u8]), None));
        }
        self.text_pos_x += 32;
        i + 1
    }

    /// `Message_DrawText`: the current box's characters up to `textDrawPos`, with the control
    /// characters that end or pause it taking effect when the typing reaches them.
    fn draw_text(&mut self, f: &mut MsgFrame, out: &mut Vec<Sprite>) {
        self.text_pos_x = self.regs.text_init_xpos;
        self.text_pos_y = self.regs.text_init_ypos;
        let c = if self.text_box_type == TEXTBOX_TYPE_NONE_NO_SHADOW { 0 } else { 255 };
        self.text_color[0] = c;
        self.text_color[1] = c;
        self.text_color[2] = c;
        self.unk_e3d0 = 0;
        let mut char_tex_idx: usize = 0;
        let mut i: u16 = 0;
        while i < self.text_draw_pos {
            let character = self.decoded(i as usize);
            match character {
                MESSAGE_NEWLINE => {
                    self.text_pos_x = self.regs.text_init_xpos;
                    if self.choice_num == 1 || self.choice_num == 3 {
                        self.text_pos_x += 32;
                    }
                    if self.choice_num == 2 && self.text_pos_y != self.regs.text_init_ypos {
                        self.text_pos_x += 32;
                    }
                    self.text_pos_y += self.regs.text_line_spacing;
                }
                MESSAGE_COLOR => {
                    i += 1;
                    self.set_text_color(self.decoded(i as usize) & 0xF);
                }
                b' ' => self.text_pos_x += MESSAGE_SPACE_WIDTH,
                MESSAGE_BOX_BREAK => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        if !self.s.textbox_skipped {
                            sfx_none(f.audio);
                            self.msg_mode = MSGMODE_TEXT_AWAIT_NEXT;
                            self.font.load_message_box_icon(TEXTBOX_ICON_TRIANGLE);
                        } else {
                            self.msg_mode = MSGMODE_TEXT_NEXT_MSG;
                            self.text_unskippable = 0;
                            self.msg_buf_pos += 1;
                        }
                    }
                    return;
                }
                MESSAGE_SHIFT => {
                    i += 1;
                    self.text_pos_x += self.decoded(i as usize) as i16;
                }
                MESSAGE_TEXTID => {
                    self.textbox_end_type = TEXTBOX_ENDTYPE_HAS_NEXT;
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        sfx_none(f.audio);
                        self.msg_mode = MSGMODE_TEXT_DONE;
                        self.font.load_message_box_icon(TEXTBOX_ICON_TRIANGLE);
                    }
                    return;
                }
                MESSAGE_QUICKTEXT_ENABLE | MESSAGE_QUICKTEXT_DISABLE => {
                    if character == MESSAGE_QUICKTEXT_ENABLE
                        && i + 1 == self.text_draw_pos
                        && (self.msg_mode == MSGMODE_TEXT_DISPLAYING || (self.msg_mode >= MSGMODE_OCARINA_STARTING && self.msg_mode < MSGMODE_SCARECROW_LONG_RECORDING_START))
                    {
                        // Everything up to the next pause or end at once.
                        let mut j = i;
                        loop {
                            let look = self.decoded(j as usize);
                            if look == MESSAGE_SHIFT {
                                j += 2;
                            } else if !matches!(look, MESSAGE_QUICKTEXT_DISABLE | MESSAGE_PERSISTENT | MESSAGE_EVENT | MESSAGE_BOX_BREAK_DELAYED | MESSAGE_AWAIT_BUTTON_PRESS | MESSAGE_BOX_BREAK | MESSAGE_END) {
                                j += 1;
                            } else {
                                break;
                            }
                        }
                        i = j - 1;
                        self.text_draw_pos = i + 1;
                    }
                }
                MESSAGE_AWAIT_BUTTON_PRESS => {
                    if i + 1 == self.text_draw_pos {
                        if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                            self.msg_mode = MSGMODE_TEXT_AWAIT_INPUT;
                            self.font.load_message_box_icon(TEXTBOX_ICON_TRIANGLE);
                        }
                        return;
                    }
                }
                MESSAGE_BOX_BREAK_DELAYED => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        i += 1;
                        self.state_timer = self.decoded(i as usize);
                        self.msg_mode = MSGMODE_TEXT_DELAYED_BREAK;
                    }
                    return;
                }
                MESSAGE_FADE2 => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        self.msg_mode = MSGMODE_TEXT_DONE;
                        self.textbox_end_type = TEXTBOX_ENDTYPE_FADING;
                        // stateTimer is a u8: `hi << 8` is lost, only the low byte stays.
                        i += 2;
                        self.state_timer = self.decoded(i as usize);
                    }
                    return;
                }
                MESSAGE_SFX => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING && !self.s.message_has_set_sfx {
                        self.s.message_has_set_sfx = true;
                        log::debug!("サウンド（ＳＥ）");
                        let sfx_hi = (self.decoded(i as usize + 1) as u16) << 8;
                        f.audio.play_sfx_centered(sfx_hi | self.decoded(i as usize + 2) as u16);
                    }
                    i += 2;
                }
                MESSAGE_ITEM_ICON => i = self.draw_item_icon(i, f.audio, out),
                MESSAGE_BACKGROUND => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        sfx_none(f.audio);
                    }
                    // The background images aren't drawn; the text still moves past them.
                    self.text_pos_x += 32;
                }
                MESSAGE_TEXT_SPEED => {
                    i += 1;
                    self.text_delay = self.decoded(i as usize) as u16;
                }
                MESSAGE_UNSKIPPABLE => self.text_unskippable = 1,
                MESSAGE_TWO_CHOICE | MESSAGE_THREE_CHOICE => {
                    self.textbox_end_type = if character == MESSAGE_TWO_CHOICE { TEXTBOX_ENDTYPE_2_CHOICE } else { TEXTBOX_ENDTYPE_3_CHOICE };
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        self.choice_text_id = self.text_id;
                        self.state_timer = 4;
                        self.choice_index = 0;
                        self.font.load_message_box_icon(TEXTBOX_ICON_ARROW);
                    }
                }
                MESSAGE_END => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        self.msg_mode = MSGMODE_TEXT_DONE;
                        if self.textbox_end_type == TEXTBOX_ENDTYPE_DEFAULT {
                            f.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_MESSAGE_END);
                            self.font.load_message_box_icon(TEXTBOX_ICON_SQUARE);
                            if f.cs_idle {
                                f.iface.set_do_action(DO_ACTION_RETURN);
                            }
                        }
                    }
                    return;
                }
                MESSAGE_OCARINA => {
                    if i + 1 == self.text_draw_pos {
                        // Message_HandleOcarina: the ocarina modes aren't ported.
                        return;
                    }
                }
                MESSAGE_FADE => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        self.msg_mode = MSGMODE_TEXT_DONE;
                        self.textbox_end_type = TEXTBOX_ENDTYPE_FADING;
                        i += 1;
                        self.state_timer = self.decoded(i as usize);
                        self.font.load_message_box_icon(TEXTBOX_ICON_SQUARE);
                        if f.cs_idle {
                            f.iface.set_do_action(DO_ACTION_RETURN);
                        }
                    }
                    return;
                }
                MESSAGE_PERSISTENT => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        sfx_none(f.audio);
                        self.msg_mode = MSGMODE_TEXT_DONE;
                        self.textbox_end_type = TEXTBOX_ENDTYPE_PERSISTENT;
                    }
                    return;
                }
                MESSAGE_EVENT => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING {
                        self.msg_mode = MSGMODE_TEXT_DONE;
                        self.textbox_end_type = TEXTBOX_ENDTYPE_EVENT;
                        self.font.load_message_box_icon(TEXTBOX_ICON_TRIANGLE);
                        f.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_MESSAGE_END);
                    }
                    return;
                }
                _ => {
                    if self.msg_mode == MSGMODE_TEXT_DISPLAYING && i + 1 == self.text_draw_pos && self.text_delay_timer == self.text_delay {
                        sfx_none(f.audio);
                    }
                    let glyph = self.font.char_tex_buf.get(char_tex_idx).copied().unwrap_or(0);
                    self.draw_text_char(glyph, out);
                    char_tex_idx += 1;
                    let w = S_FONT_WIDTHS.get(character.wrapping_sub(b' ') as usize).copied().unwrap_or(0.0);
                    self.text_pos_x = self.text_pos_x.wrapping_add((w * (self.regs.text_char_scale as f32 / 100.0)) as i32 as i16);
                }
            }
            i += 1;
        }
        if self.text_delay_timer == 0 {
            self.text_draw_pos = i + 1;
            self.text_delay_timer = self.text_delay;
        } else {
            self.text_delay_timer -= 1;
        }
    }

    /// `Message_LoadItemIcon` (the icon's texture isn't loaded).
    fn load_item_icon(&mut self, item_id: u8, y: i16) {
        /// `ITEM_MEDALLION_FOREST`.
        const ITEM_MEDALLION_FOREST: u8 = 0x66;
        let r = &mut self.regs;
        if item_id < ITEM_MEDALLION_FOREST {
            r.textbox_icon_xpos = r.text_init_xpos - 74;
            r.textbox_icon_ypos = y + 6;
            r.textbox_icon_size = 32;
        } else {
            r.textbox_icon_xpos = r.text_init_xpos - 72;
            r.textbox_icon_ypos = y + 10;
            r.textbox_icon_size = 24;
        }
        self.msg_buf_pos += 1;
        self.choice_num = 1;
    }

    /// `Message_Decode`: the next box of the message into `msgBufDecoded`, loading each
    /// character's glyph, and where its text starts (by its number of lines).
    fn decode(&mut self, save: &SaveContext) {
        self.text_delay_timer = 0;
        self.text_unskippable = 0;
        self.text_delay = 0;
        self.s.text_fade = false;
        let mut char_tex_idx: usize = 0;
        let mut decoded_buf_pos: i16 = 0;
        let mut num_lines = 0;
        let fs = FONT_CHAR_TEX_SIZE as usize;
        loop {
            let pos = self.msg_buf_pos as usize;
            let cur_char = self.font.byte(pos);
            self.set_decoded(decoded_buf_pos as usize, cur_char);
            if matches!(cur_char, MESSAGE_BOX_BREAK | MESSAGE_TEXTID | MESSAGE_BOX_BREAK_DELAYED | MESSAGE_EVENT | MESSAGE_END) {
                self.msg_mode = MSGMODE_TEXT_DISPLAYING;
                self.text_draw_pos = 1;
                let ty = self.regs.textbox_y;
                self.regs.text_init_ypos = ty + 8;
                if self.text_box_type != TEXTBOX_TYPE_NONE_BOTTOM {
                    match num_lines {
                        0 => self.regs.text_init_ypos = (ty + 26) as u16 as i16,
                        1 => self.regs.text_init_ypos = (ty + 20) as u16 as i16,
                        2 => self.regs.text_init_ypos = (ty + 16) as u16 as i16,
                        _ => {}
                    }
                }
                if cur_char == MESSAGE_TEXTID {
                    decoded_buf_pos += 1;
                    let hi = self.font.byte(pos + 1);
                    self.set_decoded(decoded_buf_pos as usize, hi);
                    decoded_buf_pos += 1;
                    let lo = self.font.byte(pos + 2);
                    self.set_decoded(decoded_buf_pos as usize, lo);
                    self.s.next_text_id = ((hi as u16) << 8) | lo as u16;
                }
                if cur_char == MESSAGE_BOX_BREAK_DELAYED {
                    decoded_buf_pos += 1;
                    self.set_decoded(decoded_buf_pos as usize, self.font.byte(pos + 1));
                    self.msg_buf_pos += 2;
                }
                self.decoded_text_len = decoded_buf_pos as u16;
                if self.s.textbox_skipped {
                    self.text_draw_pos = self.decoded_text_len;
                }
                break;
            } else if cur_char == MESSAGE_NAME {
                // The file's name, trailing spaces (0x3E) dropped, in the font's codes.
                let name = &save.player_name;
                let mut len = name.len();
                while len > 0 && name[len - 1] == 0x3E {
                    len -= 1;
                }
                for &n in &name[..len] {
                    let c = match n {
                        0x3E => b' ',
                        0x40 => b'.',
                        0x3F => b'-',
                        n if n < 0xA => n + b'0',
                        n if n < 0x24 => n + b'7',
                        n if n < 0x3E => n + b'=',
                        n => n,
                    };
                    if c != b' ' {
                        self.font.load_char(c - b' ', char_tex_idx);
                        char_tex_idx += fs;
                    }
                    self.set_decoded(decoded_buf_pos as usize, c);
                    decoded_buf_pos += 1;
                }
                decoded_buf_pos -= 1;
            } else if matches!(cur_char, MESSAGE_MARATHON_TIME | MESSAGE_RACE_TIME | MESSAGE_POINTS | MESSAGE_TOKENS | MESSAGE_FISH_INFO | MESSAGE_TIME) {
                // The timers, scores and the clock: only the clock is kept by the save here.
                for d in &self.number_digits(cur_char, save) {
                    if *d != b' ' {
                        self.font.load_char(d - b' ', char_tex_idx);
                        char_tex_idx += fs;
                    }
                    self.set_decoded(decoded_buf_pos as usize, *d);
                    decoded_buf_pos += 1;
                }
                decoded_buf_pos -= 1;
            } else if cur_char == MESSAGE_HIGHSCORE {
                // The save keeps no high scores: every one reads as a score of 0 (HS_UNK_05
                // would write nothing).
                self.msg_buf_pos += 1;
                self.font.load_char(b'0' - b' ', char_tex_idx);
                char_tex_idx += fs;
                self.set_decoded(decoded_buf_pos as usize, b'0');
            } else if cur_char == MESSAGE_ITEM_ICON {
                decoded_buf_pos += 1;
                let item = self.font.byte(pos + 1);
                self.set_decoded(decoded_buf_pos as usize, item);
                let y = self.regs.textbox_y + 10;
                self.load_item_icon(item, y);
            } else if cur_char == MESSAGE_BACKGROUND {
                self.textbox_background_idx = self.font.byte(pos + 1) as u16 * 2;
                self.textbox_background_fore_color_idx = (self.font.byte(pos + 2) & 0xF0) >> 4;
                self.textbox_background_back_color_idx = self.font.byte(pos + 2) & 0xF;
                self.textbox_background_y_offset_idx = (self.font.byte(pos + 3) & 0xF0) >> 4;
                self.textbox_background_unk_arg = self.font.byte(pos + 3) & 0xF;
                self.msg_buf_pos += 3;
                self.regs.textbox_bg_ypos = self.regs.textbox_y + 8;
                num_lines = 2;
                self.regs.text_init_xpos = 50;
            } else if cur_char == MESSAGE_COLOR {
                decoded_buf_pos += 1;
                self.msg_buf_pos += 1;
                self.set_decoded(decoded_buf_pos as usize, self.font.byte(self.msg_buf_pos as usize));
            } else if cur_char == MESSAGE_NEWLINE {
                num_lines += 1;
            } else if !matches!(cur_char, MESSAGE_QUICKTEXT_ENABLE | MESSAGE_QUICKTEXT_DISABLE | MESSAGE_AWAIT_BUTTON_PRESS | MESSAGE_OCARINA | MESSAGE_PERSISTENT | MESSAGE_UNSKIPPABLE) {
                if cur_char == MESSAGE_FADE {
                    self.s.text_fade = true;
                    decoded_buf_pos += 1;
                    self.msg_buf_pos += 1;
                    self.set_decoded(decoded_buf_pos as usize, self.font.byte(self.msg_buf_pos as usize));
                } else if cur_char == MESSAGE_FADE2 {
                    self.s.text_fade = true;
                    for _ in 0..2 {
                        decoded_buf_pos += 1;
                        self.msg_buf_pos += 1;
                        self.set_decoded(decoded_buf_pos as usize, self.font.byte(self.msg_buf_pos as usize));
                    }
                } else if cur_char == MESSAGE_SHIFT || cur_char == MESSAGE_TEXT_SPEED {
                    decoded_buf_pos += 1;
                    self.msg_buf_pos += 1;
                    self.set_decoded(decoded_buf_pos as usize, self.font.byte(self.msg_buf_pos as usize));
                } else if cur_char == MESSAGE_SFX {
                    for _ in 0..2 {
                        decoded_buf_pos += 1;
                        self.msg_buf_pos += 1;
                        self.set_decoded(decoded_buf_pos as usize, self.font.byte(self.msg_buf_pos as usize));
                    }
                } else if cur_char == MESSAGE_TWO_CHOICE {
                    self.choice_num = 2;
                } else if cur_char == MESSAGE_THREE_CHOICE {
                    self.choice_num = 3;
                } else if cur_char != b' ' {
                    self.font.load_char(cur_char.wrapping_sub(b' '), char_tex_idx);
                    char_tex_idx += fs;
                }
            }
            decoded_buf_pos += 1;
            self.msg_buf_pos += 1;
        }
    }

    /// The characters `Message_Decode` writes for a number: `MESSAGE_TIME` (the clock as
    /// "hh:mm"); the timers, scores and tokens the save doesn't keep read as 0.
    fn number_digits(&self, c: u8, save: &SaveContext) -> Vec<u8> {
        let d = |v: i16| b'0' + v as u8;
        match c {
            MESSAGE_TIME => {
                let time_in_seconds = save.day_time as f32 * (24.0 * 60.0 / 65536.0);
                let mut digits = [0i16, (time_in_seconds / 60.0) as i16, 0, (time_in_seconds as i16) % 60];
                while digits[1] >= 10 {
                    digits[0] += 1;
                    digits[1] -= 10;
                }
                while digits[3] >= 10 {
                    digits[2] += 1;
                    digits[3] -= 10;
                }
                vec![d(digits[0]), d(digits[1]), b':', d(digits[2]), d(digits[3])]
            }
            MESSAGE_MARATHON_TIME | MESSAGE_RACE_TIME => vec![b'0', b'0', b'"', b'0', b'0', b'"'],
            _ => vec![b'0'],
        }
    }

    /// `Message_OpenText` for the English table.
    fn open_text(&mut self, f: &mut MsgFrame, mut text_id: u16) {
        if self.msg_mode == MSGMODE_NONE {
            f.save.prev_hud_visibility_mode = f.save.hud_visibility_mode;
        }
        if f.scene_cam_type == crate::scene::SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT {
            change_alpha(f.save, 5);
        }
        self.s.message_has_set_sfx = false;
        self.s.message_start_frame_count = 0;
        self.s.textbox_skipped = false;
        // Text ids 0x500..0x5FF are the credits (sTextIsCredits): not imported.
        self.regs.text_char_scale = 75;
        self.regs.text_line_spacing = 12;
        self.regs.text_init_xpos = 65;
        if text_id == 0xC2 || text_id == 0xFA {
            // The piece of heart texts follow each other: one per piece already held.
            text_id += ((f.save.inventory.quest_items & 0xF000_0000) >> QUEST_HEART_PIECE_COUNT) as u16;
        }
        // (The Biggoron's Sword and gold Skulltula variants of 0xC and 0xB4 need equipment
        // and flags this save doesn't keep.)
        if matches!(text_id, 0x4077 | 0x407A | 0x2061 | 0x5035 | 0x40AC) {
            change_alpha(f.save, 1);
        }
        self.text_id = text_id;
        self.find_message(f.table, text_id);
        self.msg_length = self.font.msg_length as i32;
        let start = self.font.msg_offset as usize;
        self.font.msg_buf = f.table.data.get(start..start + self.font.msg_length as usize).map(|b| b.to_vec()).unwrap_or_default();
        self.text_box_properties = self.font.type_pos;
        self.text_box_type = self.text_box_properties >> 4;
        self.text_box_pos = self.text_box_properties & 0xF;
        let ty = self.text_box_type;
        if ty < TEXTBOX_TYPE_NONE_BOTTOM {
            // (The box's image is `message_static` + messageStaticIndices[type] * 0x1000, the
            // box's sprite.)
            self.textbox_color = match ty {
                TEXTBOX_TYPE_BLACK => [0, 0, 0],
                TEXTBOX_TYPE_WOODEN => [70, 50, 30],
                TEXTBOX_TYPE_BLUE => [0, 10, 50],
                _ => [255, 0, 0],
            };
            self.textbox_color_alpha_target = match ty {
                TEXTBOX_TYPE_WOODEN => 230,
                TEXTBOX_TYPE_OCARINA => 180,
                _ => 170,
            };
            self.textbox_color_alpha_current = 0;
        }
        self.choice_num = 0;
        self.text_unskippable = 0;
        self.textbox_end_type = 0;
        self.msg_buf_pos = 0;
        self.unk_e3d0 = 0;
        self.text_draw_pos = 0;
    }

    /// `Message_StartTextbox`: opens `text_id`, spoken by `actor`.
    pub fn start_textbox(&mut self, f: &mut MsgFrame, text_id: u16, actor: Option<ActorHandle>) {
        self.ocarina_action = 0xFFFF;
        self.open_text(f, text_id);
        self.talk_actor = actor;
        self.msg_mode = MSGMODE_TEXT_START;
        self.state_timer = 0;
        self.text_delay_timer = 0;
        self.ocarina_mode = 0;
    }

    /// `Message_ContinueTextbox`: the box stays, with `text_id`'s text.
    pub fn continue_textbox(&mut self, f: &mut MsgFrame, text_id: u16) {
        self.msg_length = 0;
        self.open_text(f, text_id);
        self.msg_mode = MSGMODE_TEXT_CONTINUING;
        self.textbox_color_alpha_current = self.textbox_color_alpha_target;
        self.state_timer = 3;
        self.textbox_end_type = 0;
        self.msg_buf_pos = 0;
        self.unk_e3d0 = 0;
        self.text_draw_pos = 0;
        self.text_delay_timer = 0;
        self.text_color[3] = 255;
        if self.yreg_31 == 0 && !f.iface.unk_1fa {
            f.iface.set_do_action(DO_ACTION_NEXT);
        }
        self.textbox_color_alpha_current = self.textbox_color_alpha_target;
    }

    /// `Message_DrawTextBox`: the box's image, mirrored across its width, in its colour.
    fn draw_text_box(&self, out: &mut Vec<Sprite>) {
        let r = &self.regs;
        let (x, y) = (r.textbox_x as f32, r.textbox_y as f32);
        let c = self.textbox_color;
        let prim = [c[0] as u8, c[1] as u8, c[2] as u8, self.textbox_color_alpha_current as u8];
        out.push(Sprite::rect(box_sprite(self.text_box_type), x, y, x + r.textbox_width as f32, y + r.textbox_height as f32, Some(prim), None));
        // (The ocarina box's treble clef isn't drawn.)
    }

    /// `Message_DrawMain` for the text modes (the ocarina's staff and notes aren't ported).
    fn draw_main(&mut self, f: &mut MsgFrame, out: &mut Vec<Sprite>) {
        if self.msg_length == 0 {
            return;
        }
        /// `OCARINA_ACTION_FROGS`.
        const OCARINA_ACTION_FROGS: u16 = 0x2D;
        if self.ocarina_action != OCARINA_ACTION_FROGS
            && self.msg_mode != MSGMODE_SONG_PLAYED_ACT
            && self.msg_mode >= MSGMODE_TEXT_BOX_GROWING
            && self.msg_mode < MSGMODE_TEXT_CLOSING
            && self.text_box_type < TEXTBOX_TYPE_NONE_BOTTOM
        {
            self.draw_text_box(out);
        }
        match self.msg_mode {
            MSGMODE_TEXT_START | MSGMODE_TEXT_BOX_GROWING | MSGMODE_TEXT_STARTING | MSGMODE_TEXT_NEXT_MSG => {}
            MSGMODE_TEXT_CONTINUING => {
                if self.state_timer == 1 {
                    self.draw_text(f, out);
                }
            }
            MSGMODE_TEXT_DISPLAYING | MSGMODE_TEXT_DELAYED_BREAK => self.draw_text(f, out),
            MSGMODE_TEXT_AWAIT_INPUT | MSGMODE_TEXT_AWAIT_NEXT => {
                self.draw_text(f, out);
                let (x, y) = (self.regs.textbox_end_xpos, self.regs.textbox_end_ypos);
                self.draw_textbox_icon(x, y, out);
            }
            MSGMODE_TEXT_DONE => {
                self.draw_text(f, out);
                match self.textbox_end_type {
                    TEXTBOX_ENDTYPE_2_CHOICE => {
                        self.handle_choice_selection(f.input, f.audio, 1);
                        let (x, y) = (self.text_pos_x, self.text_pos_y);
                        self.draw_textbox_icon(x, y, out);
                    }
                    TEXTBOX_ENDTYPE_3_CHOICE => {
                        self.handle_choice_selection(f.input, f.audio, 2);
                        let (x, y) = (self.text_pos_x, self.text_pos_y);
                        self.draw_textbox_icon(x, y, out);
                    }
                    TEXTBOX_ENDTYPE_PERSISTENT => {
                        if (0x6D..0x73).contains(&self.text_id) {
                            self.state_timer += 1;
                            if self.state_timer >= 31 {
                                self.state_timer = 2;
                                self.msg_mode = MSGMODE_TEXT_CLOSING;
                            }
                        }
                    }
                    TEXTBOX_ENDTYPE_FADING => {}
                    _ => {
                        let (x, y) = (self.regs.textbox_end_xpos, self.regs.textbox_end_ypos);
                        self.draw_textbox_icon(x, y, out);
                    }
                }
            }
            MSGMODE_TEXT_CLOSING | MSGMODE_PAUSED => {}
            m if (MSGMODE_OCARINA_STARTING..MSGMODE_TEXT_AWAIT_NEXT).contains(&m) => {
                // The ocarina and song modes aren't ported.
            }
            _ => self.msg_mode = MSGMODE_TEXT_DISPLAYING,
        }
    }

    /// `Message_Draw`'s state changes, once per game frame: what `Message_DrawMain` draws is
    /// left in `sprites`.
    pub fn draw_update(&mut self, f: &mut MsgFrame) {
        let mut out = Vec::new();
        self.draw_main(f, &mut out);
        self.sprites = out;
    }

    /// `Message_Draw`'s draws into `OVERLAY_DISP`.
    pub fn draw(&self, out: &mut DrawLists) {
        out.overlay_2d.extend(self.sprites.iter().map(|s| s.draw_cmd()));
    }

    /// `Message_Update`.
    pub fn update(&mut self, f: &mut MsgFrame) {
        const X_POSITIONS: [i16; 6] = [34; 6];
        const LOWER_Y: [i16; 6] = [142, 142, 142, 142, 174, 142];
        const UPPER_Y: [i16; 6] = [38, 38, 38, 38, 174, 38];
        const MID_Y: [i16; 6] = [90, 90, 90, 90, 174, 90];
        const END_ICON_Y_OFFSET: [i16; 6] = [59, 59, 59, 59, 34, 59];
        use crate::scene::{SCENE_CAM_TYPE_DEFAULT, SCENE_CAM_TYPE_FIXED_MARKET};
        /// `SCENE_CASTLE_COURTYARD_GUARDS_DAY`, `SCENE_MARKET_DAY`, `_NIGHT`, `_RUINS`.
        const SCENE_CASTLE_COURTYARD_GUARDS_DAY: u16 = 0x45;
        const SCENE_MARKET_DAY: u16 = 0x20;
        const SCENE_MARKET_NIGHT: u16 = 0x21;
        const SCENE_MARKET_RUINS: u16 = 0x22;

        if self.msg_length == 0 {
            return;
        }
        match self.msg_mode {
            MSGMODE_TEXT_START => {
                self.s.message_start_frame_count = self.s.message_start_frame_count.wrapping_add(1);
                let var = if f.scene_cam_type == SCENE_CAM_TYPE_FIXED_MARKET {
                    self.s.message_start_frame_count >= 4
                } else if f.scene_cam_type != SCENE_CAM_TYPE_DEFAULT || f.scene_id == SCENE_CASTLE_COURTYARD_GUARDS_DAY {
                    true
                } else {
                    self.s.message_start_frame_count >= 4 || self.talk_actor.is_none()
                };
                if !var {
                    return;
                }
                // @bug (game): averageY is left unset without a talk actor; only a variable
                // position reads it. 0 here.
                let mut average_y: i16 = 0;
                match f.talk_actor_screen_y.filter(|_| self.talk_actor.is_some()) {
                    Some(actor_y) => {
                        // s16s promoted to int, the sum truncated back to the s16 averageY.
                        let (player_y, actor_y) = (f.player_screen_y as i32, actor_y as i32);
                        average_y = if player_y >= actor_y { (player_y - actor_y) / 2 + actor_y } else { (actor_y - player_y) / 2 + player_y } as i16;
                    }
                    None => {
                        self.regs.textbox_x = self.regs.textbox_x_target;
                        self.regs.textbox_y = self.regs.textbox_y_target;
                    }
                }
                let v = (self.text_box_type as usize).min(5);
                let r = &mut self.regs;
                if self.text_box_pos == 0 {
                    let limit = if f.scene_cam_type != SCENE_CAM_TYPE_DEFAULT || f.scene_id == SCENE_CASTLE_COURTYARD_GUARDS_DAY {
                        r.xreg_92
                    } else if matches!(f.scene_id, SCENE_MARKET_DAY | SCENE_MARKET_NIGHT | SCENE_MARKET_RUINS) {
                        r.xreg_93
                    } else {
                        r.xreg_94
                    };
                    r.textbox_y_target = if average_y < limit { LOWER_Y[v] } else { UPPER_Y[v] };
                } else if self.text_box_pos == TEXTBOX_POS_TOP {
                    r.textbox_y_target = UPPER_Y[v];
                } else if self.text_box_pos == TEXTBOX_POS_MIDDLE {
                    r.textbox_y_target = MID_Y[v];
                } else {
                    r.textbox_y_target = LOWER_Y[v];
                }
                r.textbox_x_target = X_POSITIONS[v];
                r.textbox_end_ypos = END_ICON_Y_OFFSET[v] + r.textbox_y_target;
                r.text_choice_ypos = [r.textbox_y_target + 20, r.textbox_y_target + 32, r.textbox_y_target + 44];
                if self.text_box_type == TEXTBOX_TYPE_NONE_BOTTOM || self.text_box_type == TEXTBOX_TYPE_NONE_NO_SHADOW {
                    self.msg_mode = MSGMODE_TEXT_STARTING;
                    r.textbox_x = r.textbox_x_target;
                    r.textbox_y = r.textbox_y_target;
                    r.textbox_width = 256;
                    r.textbox_height = 64;
                    r.textbox_texwidth = 512;
                    r.textbox_texheight = 512;
                } else {
                    self.grow_textbox();
                    // TODO (the decomp's): this may be NA_SE_PL_WALK_GROUND - SFX_FLAG, or not.
                    f.audio.play_sfx_if_not_in_cutscene(0);
                    self.state_timer = 0;
                    self.msg_mode = MSGMODE_TEXT_BOX_GROWING;
                }
            }
            MSGMODE_TEXT_BOX_GROWING => self.grow_textbox(),
            MSGMODE_TEXT_STARTING => {
                self.msg_mode = MSGMODE_TEXT_NEXT_MSG;
                if self.yreg_31 == 0 {
                    f.iface.set_do_action(DO_ACTION_NEXT);
                }
            }
            MSGMODE_TEXT_NEXT_MSG => {
                self.decode(f.save);
                if self.s.text_fade {
                    change_alpha(f.save, 1);
                }
                if self.s.d_80153d74 != 0 {
                    self.text_draw_pos = self.decoded_text_len;
                    self.s.d_80153d74 = 0;
                }
            }
            MSGMODE_TEXT_CONTINUING => {
                self.state_timer = self.state_timer.wrapping_sub(1);
                if self.state_timer == 0 {
                    self.decode(f.save);
                }
            }
            MSGMODE_TEXT_DISPLAYING => {
                if self.text_box_type != TEXTBOX_TYPE_NONE_BOTTOM && self.yreg_31 == 0 && f.input.press.held(BTN_B) && self.text_unskippable == 0 {
                    self.s.textbox_skipped = true;
                    self.text_draw_pos = self.decoded_text_len;
                }
            }
            MSGMODE_TEXT_AWAIT_INPUT => {
                if self.yreg_31 == 0 && should_advance(f.input, f.audio) {
                    self.msg_mode = MSGMODE_TEXT_DISPLAYING;
                    self.text_draw_pos += 1;
                }
            }
            MSGMODE_TEXT_DELAYED_BREAK => {
                self.state_timer = self.state_timer.wrapping_sub(1);
                if self.state_timer == 0 {
                    self.msg_mode = MSGMODE_TEXT_NEXT_MSG;
                }
            }
            MSGMODE_TEXT_AWAIT_NEXT => {
                if should_advance(f.input, f.audio) {
                    self.msg_mode = MSGMODE_TEXT_NEXT_MSG;
                    self.text_unskippable = 0;
                    self.msg_buf_pos += 1;
                }
            }
            MSGMODE_TEXT_DONE => {
                if self.textbox_end_type == TEXTBOX_ENDTYPE_FADING {
                    self.state_timer = self.state_timer.wrapping_sub(1);
                    if self.state_timer == 0 {
                        self.close_textbox(f.audio);
                    }
                } else if self.textbox_end_type != TEXTBOX_ENDTYPE_PERSISTENT && self.textbox_end_type != TEXTBOX_ENDTYPE_EVENT && self.yreg_31 == 0 {
                    if self.textbox_end_type == TEXTBOX_ENDTYPE_2_CHOICE && self.ocarina_mode == 1 {
                        if should_advance(f.input, f.audio) {
                            self.ocarina_mode = if self.choice_index == 0 { 2 } else { 4 };
                            self.close_textbox(f.audio);
                        }
                    } else if should_advance_silent(f.input) {
                        if self.textbox_end_type == TEXTBOX_ENDTYPE_HAS_NEXT {
                            f.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_MESSAGE_PASS);
                            let next = self.s.next_text_id;
                            self.continue_textbox(f, next);
                        } else {
                            f.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_DECIDE);
                            self.close_textbox(f.audio);
                        }
                    }
                }
            }
            MSGMODE_TEXT_CLOSING => {
                self.state_timer = self.state_timer.wrapping_sub(1);
                if self.state_timer != 0 {
                    return;
                }
                if (0xC2..0xC7).contains(&self.text_id) || (0xFA..0xFE).contains(&self.text_id) {
                    // Refill 20 hearts.
                    f.save.health_accumulator = 0x140;
                }
                if matches!(self.text_id, 0x301F | 0xA | 0xC | 0xCF | 0x21C | 9 | 0x4078 | 0x2015 | 0x3040) {
                    f.save.prev_hud_visibility_mode = 0x32;
                }
                // Outside a script, with the main camera active: the interface comes back.
                let song_choice = (0x88D..0x893).contains(&self.text_id) && self.choice_index == 0;
                if f.cs_idle && !matches!(self.text_id, 0x2061 | 0x2025 | 0x208C | 0x3055) && !song_choice && f.save.cutscene_index < 0xFFF0 && f.active_cam_main {
                    if matches!(f.save.prev_hud_visibility_mode, 0 | 1 | 2) {
                        f.save.prev_hud_visibility_mode = 0x32;
                    }
                    f.save.hud_visibility_mode = 0;
                    let t = f.save.prev_hud_visibility_mode;
                    change_alpha(f.save, t);
                }
                self.msg_length = 0;
                self.msg_mode = MSGMODE_NONE;
                f.iface.unk_1fa = false;
                f.iface.unk_1fc = 0;
                self.text_id = 0;
                self.state_timer = 0;
                if self.textbox_end_type == TEXTBOX_ENDTYPE_PERSISTENT {
                    self.textbox_end_type = TEXTBOX_ENDTYPE_DEFAULT;
                    self.ocarina_mode = 2;
                } else {
                    self.textbox_end_type = TEXTBOX_ENDTYPE_DEFAULT;
                }
                if f.save.inventory.quest_items & 0xF000_0000 == 4 << QUEST_HEART_PIECE_COUNT {
                    f.save.inventory.quest_items ^= 4 << QUEST_HEART_PIECE_COUNT;
                    f.save.health_capacity += 0x10;
                    f.save.health += 0x10;
                }
                // (The Saria's Song and free-play ocarina endings aren't ported.)
                self.s.last_played_song = 0xFF;
            }
            MSGMODE_PAUSED => {}
            _ => self.last_ocarina_button_index = 0xFF,
        }
    }
}

/// The message box's sprites: each textbox type's image as `Message_DrawTextBox` loads it, the
/// font's glyphs as `Message_DrawTextChar` does, and the three icons as
/// `Message_DrawTextboxIcon` does.
pub fn bakes() -> Vec<SpriteBake> {
    let mut out = Vec::new();
    // Message_DrawMain: Gfx_SetupDL_39Ptr, then Message_DrawTextBox's prim colour (and env
    // colour for the wooden and ocarina boxes, which G_CC_MODULATEIA_PRIM ignores).
    // messageStaticIndices: the box types' images in message_static.
    const MESSAGE_STATIC_INDICES: [u32; 4] = [0, 1, 3, 2];
    for ty in 0..4u8 {
        let mut setup = setup_dl::setup_dl_39();
        let (fmt, cmt) = if ty == TEXTBOX_TYPE_BLACK || ty == TEXTBOX_TYPE_BLUE {
            (G_IM_FMT_I, G_TX_NOMIRROR)
        } else {
            setup.env_color(if ty == TEXTBOX_TYPE_OCARINA { [0, 0, 0, 255] } else { [50, 20, 0, 255] });
            (G_IM_FMT_IA, G_TX_MIRROR)
        };
        out.push(SpriteBake {
            name: box_sprite(ty),
            tex: TexSrc::File { file: "message_static".into(), offset: MESSAGE_STATIC_INDICES[ty as usize] * MESSAGE_STATIC_TEX_SIZE },
            // gDPLoadTextureBlock_4b(textboxSegment, fmt, 128, 64, 0, G_TX_MIRROR, cmt, 7, 0, ...).
            load: Load { fmt, siz: G_IM_SIZ_4B, width: 128, height: 64, cms: G_TX_MIRROR, cmt, masks: 7, maskt: 0 },
            setup,
            prim: true,
            env: false,
            // The full box covers the image twice across (mirrored), once down.
            quad: Quad::Rect { s: 256, t: 64 },
        });
    }
    // Message_DrawMain: Gfx_SetupDL_39Ptr, gDPSetAlphaCompare(G_AC_NONE),
    // gDPSetCombineLERP(0, 0, 0, PRIMITIVE, TEXEL0, 0, PRIMITIVE, 0, x2).
    let text_setup = || {
        let mut d = setup_dl::setup_dl_39();
        d.alpha_compare_none();
        let cc = [cc_ab::ZERO, cc_ab::ZERO, cc_c::ZERO, cc_d::PRIMITIVE, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];
        d.combine_lerp(cc, cc);
        d
    };
    let glyph_load = Load::new(G_IM_FMT_I, G_IM_SIZ_4B, 16, 16, G_TX_NOMIRROR | G_TX_CLAMP);
    for g in 0..FONT_GLYPHS {
        out.push(SpriteBake {
            name: glyph_sprite(g),
            tex: TexSrc::File { file: "nes_font_static".into(), offset: g as u32 * FONT_CHAR_TEX_SIZE },
            load: glyph_load,
            setup: text_setup(),
            prim: true,
            env: false,
            quad: Quad::Rect { s: 16, t: 16 },
        });
    }
    // Message_DrawTextboxIcon: gDPSetCombineLERP(PRIMITIVE, ENVIRONMENT, TEXEL0, ENVIRONMENT,
    // TEXEL0, 0, PRIMITIVE, 0, x2) after the text's setup; the icons follow the four box
    // images in message_static (Font_LoadMessageBoxIcon).
    // Message_DrawItemIcon: gDPSetCombineMode(G_CC_MODULATEIA_PRIM) after the text's setup,
    // gDPLoadTextureBlock(RGBA32, 32x32 or 24x24, G_TX_NOMIRROR | G_TX_WRAP).
    for item in 0..=0xFFu8 {
        let Some((file, offset, size)) = item_icon_texture(item) else { continue };
        let mut setup = text_setup();
        setup.combine_lerp(setup_dl::MODULATEIA_PRIM, setup_dl::MODULATEIA_PRIM);
        out.push(SpriteBake {
            name: item_icon_sprite(item),
            tex: TexSrc::File { file: file.into(), offset },
            load: Load::new(crate::gbi::G_IM_FMT_RGBA, crate::gbi::G_IM_SIZ_32B, size, size, crate::gbi::G_TX_WRAP),
            setup,
            prim: true,
            env: false,
            quad: Quad::Rect { s: size, t: size },
        });
    }
    for icon in [TEXTBOX_ICON_TRIANGLE, TEXTBOX_ICON_SQUARE, TEXTBOX_ICON_ARROW] {
        let mut setup = text_setup();
        let cc = [cc_ab::PRIMITIVE, cc_ab::ENVIRONMENT, cc_c::TEXEL0, cc_d::ENVIRONMENT, ac::TEXEL0, ac::ZERO, ac::PRIMITIVE, ac::ZERO];
        setup.combine_lerp(cc, cc);
        out.push(SpriteBake {
            name: icon_sprite(icon),
            tex: TexSrc::File { file: "message_static".into(), offset: 4 * MESSAGE_STATIC_TEX_SIZE + icon as u32 * FONT_CHAR_TEX_SIZE },
            load: glyph_load,
            setup,
            prim: true,
            env: true,
            quad: Quad::Rect { s: 16, t: 16 },
        });
    }
    out
}
