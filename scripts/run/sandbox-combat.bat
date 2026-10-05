@echo off
rem Headless: GAME-05 milestone 3a's scripted run inside the Deku Tree: a withered Deku Baba
rem slashed and its stick picked up, a Keese's dive blocked with the shield, the Keese slashed
rem and its drop picked up, with the battle camera on. Writes its trace and screenshots to
rem out\run\: the Deku Baba upright, dying, the block, and the Keese dying.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-inside --script combat ^
    --trace out\run\combat.json --screenshot out\run\combat.png --shots-at 95,110,689,1858 %*
exit /b %errorlevel%
