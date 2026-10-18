//! The return entrances (GAME-09 milestone 6) against the C (`z_player.c`'s
//! `sReturnEntranceGroupData` and `sReturnEntranceGroupIndices`, read by
//! `Player_HandleExitsAndVoids` with `play->spawn`): every row against the entrance table by
//! name, and each group's look-up.

use std::sync::Arc;

use oot_actors::player::{RETURN_ENTRANCE_GROUP_DATA, RETURN_ENTRANCE_GROUP_INDICES, return_entrance};
use oot_game::play_scene::GameAssets;

fn assets() -> Option<Arc<GameAssets>> {
    let pack = oot_game::pack::GamePack::open_default().ok()?;
    Some(oot_actors::game_assets(pack).expect("the pack's tables"))
}

/// `sReturnEntranceGroupData`'s rows by name (`z_player.c`).
const ROWS: [&str; 14] = [
    "ENTR_DEATH_MOUNTAIN_TRAIL_4",
    "ENTR_DEATH_MOUNTAIN_CRATER_3",
    "ENTR_HYRULE_CASTLE_2",
    "ENTR_KAKARIKO_VILLAGE_9",
    "ENTR_MARKET_DAY_5",
    "ENTR_KAKARIKO_VILLAGE_3",
    "ENTR_MARKET_DAY_6",
    "ENTR_KAKARIKO_VILLAGE_11",
    "ENTR_BACK_ALLEY_DAY_2",
    "ENTR_KAKARIKO_VILLAGE_10",
    "ENTR_MARKET_DAY_8",
    "ENTR_ZORAS_FOUNTAIN_5",
    "ENTR_HYRULE_CASTLE_2",
    "ENTR_DESERT_COLOSSUS_7",
];

#[test]
fn every_row_is_its_entrance() {
    let Some(a) = assets() else { return };
    for (i, name) in ROWS.iter().enumerate() {
        let e = a.scenes.entrance_index(name).unwrap_or_else(|| panic!("no entrance {name}"));
        assert_eq!(RETURN_ENTRANCE_GROUP_DATA[i] as usize, e as usize, "row {i}, {name}");
    }
}

#[test]
fn each_group_by_spawn() {
    // ENTR_RETURN_GREAT_FAIRYS_FOUNTAIN_SPELLS (0x7FF9) to _MAGIC (0x7FFE): the spells' group
    // starts at 11, the shooting gallery's at 9, ENTR_RETURN_2's at 3, the bazaar's at 5,
    // ENTR_RETURN_4's at 7, the magic fountains' at 0.
    assert_eq!(RETURN_ENTRANCE_GROUP_INDICES, [11, 9, 3, 5, 7, 0]);
    // The magic fairy's fountain (daiyousei_izumi), entered at spawn 0 from Death Mountain
    // Trail's summit: its exit 0x7FFE leads to ENTR_DEATH_MOUNTAIN_TRAIL_4 (0x45B); spawn 1 (from
    // the crater) to ENTR_DEATH_MOUNTAIN_CRATER_3 (0x482), spawn 2 (outside Ganon's Castle) to
    // ENTR_HYRULE_CASTLE_2 (0x340).
    assert_eq!(return_entrance(0x7FFE, 0), Some(0x45B));
    assert_eq!(return_entrance(0x7FFE, 1), Some(0x482));
    assert_eq!(return_entrance(0x7FFE, 2), Some(0x340));
    // The spells' fountains: Farore's (spawn 0), Din's (1), Nayru's (2).
    assert_eq!(return_entrance(0x7FF9, 0), Some(0x394));
    assert_eq!(return_entrance(0x7FF9, 1), Some(0x340));
    assert_eq!(return_entrance(0x7FF9, 2), Some(0x57C));
    // The shooting gallery's (Kakariko's spawn 0, the market's 1) and the bazaar's.
    assert_eq!(return_entrance(0x7FFA, 1), Some(0x1CD));
    assert_eq!(return_entrance(0x7FFC, 0), Some(0x201));
    // A spawn past its group reads on into the next (@bug (game), never placed): spawn 3 of the
    // magic fountains' is ENTR_RETURN_2's first row.
    assert_eq!(return_entrance(0x7FFE, 3), Some(0x44B));
    // Past the table: none.
    assert_eq!(return_entrance(0x7FF9, 3), None);
    assert_eq!(return_entrance(0x7FFF, 0), None);
}
