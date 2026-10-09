@echo off
rem GAME-05 milestone 5b-2's tests: the pause menu's dungeon map page (z_kaleido_map.c,
rem z_lmap_mark.c, KaleidoScope_LoadDungeonMap, _UpdateDungeonMap, _OverridePalIndexCI4): the
rem menu's INIT loading Link's floor with the current room on palette index 14, the room's colour
rem pulsing and the room maps drawn through the palette, the floors' and items' columns, the
rem title, items, Link's head, skull and Gold Skulltula icon, the compass's chest marks; the game
rem over's screens (KaleidoScope_DrawGameOver, the prompt page) and its states' fields; every quad
rem baked; the pause map's tables against the ROM; the exits' runs (Route::DungeonMap,
rem Route::GameOver); then 5b-1's pause tests again.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test dungeon_map --test game_over_screens --test pause --test pause_run %* || exit /b 1
cargo test --release -p oot_game --lib kaleido || exit /b 1
cargo test --release -p oot_import --test pack map_tables
exit /b %errorlevel%
