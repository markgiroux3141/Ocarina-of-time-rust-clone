//! Navi's C-Up texts and Saria's (`z_elf_message.c`, `z64elf_message.h`).
//!
//! A script is a list of 4-byte `ElfMessage` commands: byte 0 packs the command type (bits 5..7,
//! `ELF_MSG_TYPE_*`), the condition's type (bits 1..4, `ELF_MSG_CONDITION_*`) and what the
//! condition must be (bit 0); byte 1 the condition's data (a flag, an item, or an "other"
//! condition in the top nibble and its value in the bottom one); byte 2 the text (plus 0x100);
//! byte 3 an item. `ElfMessage_GetTextFromMsgs` walks it to the first command whose condition
//! holds.
//!
//! The pack holds the ROM's bytes (`ElfMessageTables`): `sNaviMsgFiles`' two files
//! (`elf_message_field`, `elf_message_ydan`: a scene header's `SCENE_CMD_ID_SPECIAL_FILES` picks one
//! for `play->cUpElfMsgs`) and `code`'s `sChildSariaMsgs` and `sAdultSariaMsgs`. The importer
//! builds each from its C and checks it against the ROM.

use crate::play::{PlayState, Rand};
use crate::save::SaveContext;

/// The pack's `table/elf_messages`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ElfMessageTables {
    /// `sNaviMsgFiles` (`z_scene.c`) in order: `elf_message_field`, `elf_message_ydan`.
    pub files: Vec<Vec<u8>>,
    /// `sChildSariaMsgs`, `sAdultSariaMsgs` (`z_elf_message.c`).
    pub child_saria: Vec<u8>,
    pub adult_saria: Vec<u8>,
}

// `z64elf_message.h`.
pub const ELF_MSG_TYPE_CHECK: u8 = 0;
pub const ELF_MSG_TYPE_UNK_1: u8 = 1;
pub const ELF_MSG_TYPE_UNK_2: u8 = 2;
pub const ELF_MSG_TYPE_SKIP: u8 = 3;
pub const ELF_MSG_TYPE_END: u8 = 7;
pub const ELF_MSG_CONDITION_FLAG: u8 = 0;
pub const ELF_MSG_CONDITION_DUNGEON_ITEM: u8 = 1;
pub const ELF_MSG_CONDITION_ITEM: u8 = 2;
pub const ELF_MSG_CONDITION_OTHER: u8 = 3;
pub const ELF_MSG_CONDITION_STRENGTH_UPG: u8 = 0;
pub const ELF_MSG_CONDITION_BOOTS: u8 = 1;
pub const ELF_MSG_CONDITION_SONG: u8 = 2;
pub const ELF_MSG_CONDITION_MEDALLION: u8 = 3;
pub const ELF_MSG_CONDITION_MAGIC: u8 = 4;

/// `ACTOR_EN_SA` (Saria), whom `ElfMessage_GetSariaText` looks for within 800.
const ACTOR_EN_SA: i16 = 0x0146;

/// One `ElfMessage`.
fn msg(msgs: &[u8], i: usize) -> [u8; 4] {
    msgs.get(i * 4..i * 4 + 4).map(|b| [b[0], b[1], b[2], b[3]]).unwrap_or([ELF_MSG_TYPE_END << 5, 0, 0x5F, 0])
}

/// `ElfMessage_CheckCondition`.
pub fn check_condition(m: [u8; 4], save: &SaveContext) -> bool {
    use crate::item::*;
    let want = (m[0] & 1) == 1;
    match (m[0] & 0x1E) >> 1 {
        ELF_MSG_CONDITION_FLAG => {
            let flag = 1u16 << (m[1] & 0x0F);
            want == ((flag & save.event_chk_inf[((m[1] & 0xF0) >> 4) as usize]) != 0)
        }
        ELF_MSG_CONDITION_DUNGEON_ITEM => {
            // CHECK_DUNGEON_ITEM(item, mapIndex): dungeonItems[mapIndex] & gBitFlags[item].
            let bit = m[1].wrapping_sub(ITEM_KEY_BOSS) as u32;
            let items = save.inventory.dungeon_items.get(save.map_index as usize).copied().unwrap_or(0) as u32;
            want == (items & 1u32.checked_shl(bit).unwrap_or(0) != 0)
        }
        ELF_MSG_CONDITION_ITEM => want == (m[3] == save.inv_content(m[1])),
        ELF_MSG_CONDITION_OTHER => match (m[1] & 0xF0) >> 4 {
            ELF_MSG_CONDITION_STRENGTH_UPG => want == ((m[1] & 0x0F) as u32 == save.cur_upg_value(UPG_STRENGTH)),
            ELF_MSG_CONDITION_BOOTS => want == save.check_owned_equip(EQUIP_TYPE_BOOTS, (m[3].wrapping_sub(ITEM_BOOTS_KOKIRI)) as u16 + EQUIP_INV_BOOTS_KOKIRI),
            ELF_MSG_CONDITION_SONG => want == save.check_quest_item(m[3].wrapping_sub(ITEM_SONG_MINUET) as u32 + QUEST_SONG_MINUET),
            ELF_MSG_CONDITION_MEDALLION => want == save.check_quest_item(m[3].wrapping_sub(ITEM_MEDALLION_FOREST) as u32 + QUEST_MEDALLION_FOREST),
            ELF_MSG_CONDITION_MAGIC => want == save.is_magic_acquired,
            _ => unplanned(m),
        },
        _ => unplanned(m),
    }
}

/// "Unplanned conditions" (`LOG_STRING`, then `ASSERT(0)`).
fn unplanned(m: [u8; 4]) -> bool {
    log::error!("ElfMessage {m:02X?}: unplanned condition (ASSERT)");
    false
}

/// `func_8006BE88`: every `ELF_MSG_TYPE_UNK_1` command's condition and the next one's.
fn func_8006be88(msgs: &[u8], i: &mut usize, save: &SaveContext) -> bool {
    let mut temp = true;
    while (msg(msgs, *i)[0] & 0xE0) == ELF_MSG_TYPE_UNK_1 << 5 {
        if !check_condition(msg(msgs, *i), save) {
            temp = false;
        }
        *i += 1;
    }
    if temp { check_condition(msg(msgs, *i), save) } else { false }
}

/// `func_8006BF1C`: one of a run of `ELF_MSG_TYPE_UNK_2` commands whose conditions hold, at
/// random.
fn func_8006bf1c(msgs: &[u8], i: &mut usize, save: &SaveContext, rand: &mut Rand) -> bool {
    let mut sp44 = [false; 10];
    let (mut temp1, mut temp2) = (0i32, 0usize);
    let mut at = *i;
    loop {
        sp44[temp2.min(9)] = check_condition(msg(msgs, at), save);
        temp1 += sp44[temp2.min(9)] as i32;
        temp2 += 1;
        at += 1;
        if (msg(msgs, at)[0] & 0xE0) != ELF_MSG_TYPE_UNK_2 << 5 {
            break;
        }
    }
    if temp1 == 0 {
        return false;
    }
    let mut temp3 = rand.zero_float(temp1 as f32) as i32;
    for k in 0..temp2 {
        if sp44[k.min(9)] {
            if temp3 > 0 {
                temp3 -= 1;
            } else {
                return true;
            }
        }
        *i += 1;
    }
    false
}

/// `ElfMessage_GetTextFromMsgs`.
pub fn get_text_from_msgs(msgs: &[u8], save: &SaveContext, rand: &mut Rand) -> u16 {
    let mut i = 0usize;
    for _ in 0..0x1000 {
        let m = msg(msgs, i);
        match (m[0] & 0xE0) >> 5 {
            ELF_MSG_TYPE_CHECK => {
                if check_condition(m, save) {
                    return m[2] as u16 | 0x100;
                }
            }
            ELF_MSG_TYPE_UNK_1 => {
                if func_8006be88(msgs, &mut i, save) {
                    return msg(msgs, i)[2] as u16 | 0x100;
                }
            }
            ELF_MSG_TYPE_UNK_2 => {
                if func_8006bf1c(msgs, &mut i, save, rand) {
                    return msg(msgs, i)[2] as u16 | 0x100;
                }
            }
            ELF_MSG_TYPE_SKIP => {
                if check_condition(m, save) {
                    i += m[2] as usize;
                    i -= 1;
                }
            }
            ELF_MSG_TYPE_END => return m[2] as u16 | 0x100,
            _ => {
                log::error!("ElfMessage {m:02X?}: unplanned command (ASSERT)");
            }
        }
        i += 1;
    }
    0
}

impl PlayState {
    /// `ElfMessage_GetCUpText`: Navi's C-Up text from the scene's `cUpElfMsgs`, 0 without them.
    pub fn elf_message_get_c_up_text(&mut self) -> u16 {
        let Some(assets) = self.assets.clone() else { return 0 };
        let Some(msgs) = self.c_up_elf_msgs.and_then(|i| assets.elf_messages.files.get(i)) else { return 0 };
        get_text_from_msgs(msgs, &self.save, &mut self.rand)
    }

    /// `ElfMessage_GetSariaText`: a child with Saria within 800 gets 0x160 ("talk to her face to
    /// face"); otherwise the age's script.
    pub fn elf_message_get_saria_text(&mut self) -> u16 {
        let Some(assets) = self.assets.clone() else { return 0 };
        let msgs = if !self.save.adult {
            let near = self.player.and_then(|p| self.actors.actor(p)).map(|p| p.world_pos).is_some_and(|pos| {
                self.actors.category(crate::actor_ctx::ACTORCAT_NPC).iter().any(|&h| self.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_SA && a.world_pos.distance(pos) < 800.0))
            });
            if near {
                return 0x0160;
            }
            &assets.elf_messages.child_saria
        } else {
            &assets.elf_messages.adult_saria
        };
        get_text_from_msgs(msgs, &self.save, &mut self.rand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ELF_MSG_FLAG(CHECK, textId, false, flag)`, `ELF_MSG_END(textId)`.
    fn flag(text: u8, f: u16) -> [u8; 4] {
        [(ELF_MSG_TYPE_CHECK << 5) | (ELF_MSG_CONDITION_FLAG << 1), f as u8, text, 0]
    }
    fn end(text: u8) -> [u8; 4] {
        [ELF_MSG_TYPE_END << 5, 0, text, 0]
    }

    #[test]
    fn the_first_unset_flag_s_text() {
        // gOverworldNaviMsgs' first two: EVENTCHKINF_05 unset says 0x140, then EVENTCHKINF_09.
        let msgs: Vec<u8> = [flag(0x40, 0x05), flag(0x41, 0x09), end(0x5F)].concat();
        let mut s = SaveContext::new(0, false, 0);
        let mut r = Rand::default();
        assert_eq!(get_text_from_msgs(&msgs, &s, &mut r), 0x140);
        s.set_event_chk_inf(0x05);
        assert_eq!(get_text_from_msgs(&msgs, &s, &mut r), 0x141);
        s.set_event_chk_inf(0x09);
        assert_eq!(get_text_from_msgs(&msgs, &s, &mut r), 0x15F);
    }
}
