@echo off
rem Headless: GAME-06 milestone 1a's scripted run: Kokiri Forest's cutscene layer 6 from its start
rem (--cutscene 0xFFF2 on the deku-tree-dead preset), A through the Deku Tree's texts, to the
rem terminator's ENTR_KOKIRI_FOREST_11. Writes its trace and screenshots to out\run\: the
rem emerald's green light out of the tree, the emerald over Link, the white out, the tree's death.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_KOKIRI_FOREST_0 --child --preset deku-tree-dead --cutscene 0xFFF2 --script emerald ^
    --trace out\run\emerald.json --screenshot out\run\emerald.png --shots-at 885,900,1100,1800 %*
exit /b %errorlevel%
