@echo off
rem Headless: GAME-05 milestone 5b-2's map run inside the Deku Tree with the compass: Start, R to
rem the map page (1F: Link's head, room 0 pulsing, chest 3's mark), the stick right onto the
rem floors and up to 2F (its maps, chest 1's mark), Start closes the menu. Writes its trace and
rem screenshots to out\run\: the map page on 1F, then on 2F.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-compass --script dungeon-map ^
    --trace out\run\dungeon_map.json --screenshot out\run\dungeon_map.png --shots-at 70,72 %*
exit /b %errorlevel%
