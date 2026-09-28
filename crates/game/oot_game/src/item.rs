//! Items: giving them to Link (`Item_Give`, `Health_ChangeBy`, `Rupees_ChangeBy` in
//! `z_parameter.c`) and the tables random drops pick from (`sItemDropIds`, `sDropQuantities`
//! in `z_en_item00.c`, read by the importer into `table/item_drops`).
//!
//! `Item_Give` is ported for what drops and collectibles give here: rupees, recovery hearts,
//! heart containers and pieces. The inventory (bombs, arrows, seeds, sticks, nuts, bottles,
//! equipment) isn't kept, so any other item is logged and not given.

use crate::save::SaveContext;

// `ItemID` (z64item.h).
pub const ITEM_BOW: u8 = 0x03;
pub const ITEM_HEART_CONTAINER: u8 = 0x72;
pub const ITEM_HEART_PIECE: u8 = 0x73;
pub const ITEM_KEY_SMALL: u8 = 0x77;
pub const ITEM_HEART_PIECE_2: u8 = 0x7A;
pub const ITEM_RECOVERY_HEART: u8 = 0x83;
pub const ITEM_RUPEE_GREEN: u8 = 0x84;
pub const ITEM_RUPEE_BLUE: u8 = 0x85;
pub const ITEM_RUPEE_RED: u8 = 0x86;
pub const ITEM_RUPEE_PURPLE: u8 = 0x87;
pub const ITEM_RUPEE_GOLD: u8 = 0x88;
pub const ITEM_INVALID_8: u8 = 0x89;
pub const ITEM_BOMBS_5: u8 = 0x8E;
pub const ITEM_ARROWS_SMALL: u8 = 0x92;
pub const ITEM_ARROWS_MEDIUM: u8 = 0x93;
pub const ITEM_ARROWS_LARGE: u8 = 0x94;
pub const ITEM_NONE: u8 = 0xFF;

/// `QUEST_HEART_PIECE_COUNT` (z64item.h): the pieces are counted in `questItems`' top four
/// bits.
pub const QUEST_HEART_PIECE_COUNT: u32 = 0x1C;

/// `sItemDropIds` and `sDropQuantities` (16 per drop table), as `Item_DropCollectibleRandom`
/// indexes them.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemDropTables {
    pub ids: Vec<u8>,
    pub quantities: Vec<u8>,
}

/// `Health_ChangeBy`: false once Link is out of health (double defence isn't kept, so damage
/// isn't halved; the recovery sound isn't played).
pub fn health_change_by(save: &mut SaveContext, amount: i16) -> bool {
    save.health += amount;
    if save.health > save.health_capacity {
        save.health = save.health_capacity;
    }
    if save.health <= 0 {
        save.health = 0;
        return false;
    }
    true
}

/// `Rupees_ChangeBy`: counted in by `Interface_Update`.
pub fn rupees_change_by(save: &mut SaveContext, rupee_change: i16) {
    save.rupee_accumulator += rupee_change;
}

/// `Item_Give` for the items the save keeps; returns what the C returns (`ITEM_NONE` when
/// the item was taken).
pub fn item_give(save: &mut SaveContext, item: u8) -> u8 {
    /// `sRupeeRefillCounts`.
    const RUPEE_REFILL_COUNTS: [i16; 6] = [1, 5, 20, 50, 200, 10];
    if item == ITEM_HEART_PIECE_2 || item == ITEM_HEART_PIECE {
        save.quest_items = save.quest_items.wrapping_add(1 << QUEST_HEART_PIECE_COUNT);
        ITEM_NONE
    } else if item == ITEM_HEART_CONTAINER {
        save.health_capacity += 0x10;
        save.health += 0x10;
        ITEM_NONE
    } else if item == ITEM_RECOVERY_HEART {
        health_change_by(save, 0x10);
        item
    } else if (ITEM_RUPEE_GREEN..=ITEM_INVALID_8).contains(&item) {
        rupees_change_by(save, RUPEE_REFILL_COUNTS[(item - ITEM_RUPEE_GREEN) as usize]);
        ITEM_NONE
    } else {
        log::debug!("Item_Give({item:#04x}): the inventory isn't kept");
        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rupees_and_hearts() {
        let mut s = SaveContext::new(0, false, 0);
        // A red rupee is 20 (sRupeeRefillCounts[2]), counted in later.
        assert_eq!(item_give(&mut s, ITEM_RUPEE_RED), ITEM_NONE);
        assert_eq!((s.rupees, s.rupee_accumulator), (0, 20));
        // A recovery heart at full health changes nothing but is still "given" (returns it).
        assert_eq!(item_give(&mut s, ITEM_RECOVERY_HEART), ITEM_RECOVERY_HEART);
        assert_eq!(s.health, 0x30);
        s.health = 0x18;
        item_give(&mut s, ITEM_RECOVERY_HEART);
        assert_eq!(s.health, 0x28);
        // Four pieces count in questItems' top bits.
        item_give(&mut s, ITEM_HEART_PIECE);
        assert_eq!(s.quest_items >> QUEST_HEART_PIECE_COUNT, 1);
    }
}
