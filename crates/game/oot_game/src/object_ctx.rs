//! `ObjectContext` (`z_scene.c`): which object files are loaded, in which bank. An actor spawns
//! only if its object is in a bank (`Actor_Spawn`), initialises once the object is loaded
//! (`Actor_Init`, `Actor_UpdateAll`), and is killed when a room change drops its object
//! (`Actor_KillAllWithMissingObject`).
//!
//! Every object is in the asset pack, so nothing is actually loaded here. What's kept is the
//! bookkeeping and the timing: `Object_LoadPersistent` is a blocking DMA (loaded at once), while the
//! objects a room's object list swaps in (`func_800982FC`) load asynchronously. The first
//! `Object_UpdateEntries` after the swap starts their DMA and the next one sees it done, so they
//! are loaded one frame later.
//!
//! What the game writes into a loaded object's RAM (Queen Gohma's textures, erased as she dies:
//! `BossGoma_ClearPixels`) is kept per bank (`ObjectContext::written`), the file's bytes copied
//! from the pack where the writes start, and lost when the bank's object changes, as a new DMA
//! overwrites the RAM.

use std::collections::BTreeMap;

/// `OBJECT_EXCHANGE_BANK_MAX` (`object.h`).
pub const OBJECT_EXCHANGE_BANK_MAX: usize = 19;
/// `OBJECT_INVALID`, `OBJECT_GAMEPLAY_KEEP` (`object_table.h`).
pub const OBJECT_INVALID: i16 = 0;
pub const OBJECT_GAMEPLAY_KEEP: i16 = 1;
/// `gLinkObjectIds`: `OBJECT_LINK_BOY`, `OBJECT_LINK_CHILD`.
pub const OBJECT_LINK_BOY: i16 = 0x14;
pub const OBJECT_LINK_CHILD: i16 = 0x15;

/// `ObjectEntry`: the object id, negative while its DMA is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectEntry {
    pub id: i16,
    /// `dmaRequest.vromAddr != 0`: the DMA was started.
    pub dma_started: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectContext {
    pub status: [ObjectEntry; OBJECT_EXCHANGE_BANK_MAX],
    pub num: usize,
    /// `numPersistentEntries`: banks before the room's (the keeps and Link's object).
    pub num_persistent_entries: usize,
    pub main_keep_index: usize,
    pub sub_keep_index: usize,
    /// The object RAM the game has written, by bank and the region's offset in the file: the
    /// region's bytes as they are now.
    pub written: BTreeMap<(usize, u32), Vec<u8>>,
}

impl Default for ObjectContext {
    fn default() -> ObjectContext {
        ObjectContext {
            status: [ObjectEntry { id: OBJECT_INVALID, dma_started: false }; OBJECT_EXCHANGE_BANK_MAX],
            num: 0,
            num_persistent_entries: 0,
            main_keep_index: 0,
            sub_keep_index: 0,
            written: BTreeMap::new(),
        }
    }
}

impl ObjectContext {
    /// `Object_InitContext`: empty banks, then `gameplay_keep` (the main keep).
    pub fn init_bank() -> ObjectContext {
        let mut o = ObjectContext::default();
        o.main_keep_index = o.spawn(OBJECT_GAMEPLAY_KEEP);
        o
    }

    /// `Object_LoadPersistent`: loads `id` into the next bank (a blocking DMA). Returns the bank.
    pub fn spawn(&mut self, id: i16) -> usize {
        assert!(self.num < OBJECT_EXCHANGE_BANK_MAX, "this->num < OBJECT_EXCHANGE_BANK_MAX");
        self.status[self.num] = ObjectEntry { id, dma_started: false };
        self.forget_written(self.num);
        self.num += 1;
        self.num_persistent_entries = self.num;
        self.num - 1
    }

    /// `Object_UpdateEntries`: starts pending DMAs, and finishes those started last time.
    pub fn update_bank(&mut self) {
        for s in &mut self.status[..self.num] {
            if s.id < 0 {
                if !s.dma_started {
                    s.dma_started = true;
                } else {
                    s.id = -s.id;
                }
            }
        }
    }

    /// `Object_GetSlot`: the bank holding `id` (loaded or pending).
    pub fn get_index(&self, id: i16) -> Option<usize> {
        self.status[..self.num].iter().position(|s| s.id.unsigned_abs() == id.unsigned_abs())
    }

    /// `Object_IsLoaded`.
    pub fn is_loaded(&self, bank: usize) -> bool {
        self.status.get(bank).is_some_and(|s| s.id > 0)
    }

    /// `func_800982FC`: starts swapping `id` into `bank`.
    fn exchange(&mut self, bank: usize, id: i16) {
        self.status[bank] = ObjectEntry { id: -id, dma_started: false };
        self.forget_written(bank);
    }

    /// A new object in `bank`: what was written into the old one's RAM is gone.
    fn forget_written(&mut self, bank: usize) {
        self.written.retain(|&(b, _), _| b != bank);
    }

    /// The written region of `bank`'s object at `offset`, if the game has written it.
    pub fn written(&self, bank: usize, offset: u32) -> Option<&[u8]> {
        self.written.get(&(bank, offset)).map(|v| v.as_slice())
    }

    /// The region of `bank`'s object at `offset` to write into: `file` (the object's bytes there,
    /// from the pack) the first time.
    pub fn written_mut(&mut self, bank: usize, offset: u32, file: impl FnOnce() -> Vec<u8>) -> &mut Vec<u8> {
        self.written.entry((bank, offset)).or_insert_with(file)
    }

    /// `Scene_CommandObjectList`: keeps the room banks that already hold the list's objects in
    /// order, drops the rest (returning true: `Actor_KillAllWithMissingObject` must kill actors that lost their
    /// object), and swaps in the remaining objects of the list.
    pub fn command_object_list(&mut self, list: &[i16]) -> bool {
        let mut dropped = false;
        let mut i = self.num_persistent_entries;
        let mut k = 0;
        while i < self.num {
            if list.get(k) != Some(&self.status[i].id) {
                for s in &mut self.status[i..self.num] {
                    *s = ObjectEntry { id: OBJECT_INVALID, dma_started: false };
                }
                self.written.retain(|&(b, _), _| b < i);
                self.num = i;
                dropped = true;
                continue;
            }
            i += 1;
            k += 1;
        }
        assert!(list.len() <= OBJECT_EXCHANGE_BANK_MAX, "scene_info->object_bank.num <= OBJECT_EXCHANGE_BANK_MAX");
        while k < list.len() {
            self.exchange(i, list[k]);
            i += 1;
            k += 1;
        }
        self.num = i;
        dropped
    }

    /// The loaded or pending object ids, bank order.
    pub fn ids(&self) -> Vec<i16> {
        self.status[..self.num].iter().map(|s| s.id).collect()
    }
}

/// An RGBA16 texture's texels (`G_IM_FMT_RGBA`, `G_IM_SIZ_16b`, big-endian, as an object holds
/// them) as a draw's own image (`eng_gfx::DrawImage`), decoded as the importer decodes textures.
pub fn rgba16_image(texels: &[u8], width: u32, height: u32) -> eng_gfx::DrawImage {
    let n = (width * height) as usize;
    let mut rgba = Vec::with_capacity(n * 4);
    for i in 0..n {
        let v = texels.get(i * 2..i * 2 + 2).map_or(0, |b| u16::from_be_bytes([b[0], b[1]]));
        rgba.extend_from_slice(&eng_gbi::texture::rgba16(v));
    }
    eng_gfx::DrawImage { width, height, rgba: rgba.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_objects_load_a_frame_after_the_swap() {
        let mut o = ObjectContext::init_bank();
        let keep = o.spawn(2);
        assert_eq!((o.main_keep_index, keep, o.num_persistent_entries), (0, 1, 2));
        assert!(!o.command_object_list(&[10, 11]));
        let b = o.get_index(11).unwrap();
        assert!(!o.is_loaded(b));
        o.update_bank(); // DMA started
        assert!(!o.is_loaded(b));
        o.update_bank(); // done
        assert!(o.is_loaded(b));
        // A room with 10 then 12: 10 stays, 11 goes (actors lose it), 12 comes in.
        assert!(o.command_object_list(&[10, 12]));
        assert_eq!(o.ids(), vec![1, 2, 10, -12]);
        assert!(o.get_index(11).is_none());
    }

    #[test]
    fn written_object_ram_is_lost_with_the_object() {
        let mut o = ObjectContext::init_bank();
        o.spawn(2);
        o.command_object_list(&[10, 11]);
        o.update_bank();
        o.update_bank();
        let b = o.get_index(11).unwrap();
        o.written_mut(b, 0x100, || vec![1, 2, 3])[1] = 0;
        assert_eq!(o.written(b, 0x100), Some(&[1, 0, 3][..]));
        // The same objects again: kept (the bank keeps its object, so its RAM).
        o.command_object_list(&[10, 11]);
        assert_eq!(o.written(b, 0x100), Some(&[1, 0, 3][..]));
        // 11 dropped: gone with it.
        o.command_object_list(&[10, 12]);
        assert_eq!(o.written(b, 0x100), None);
    }
}
