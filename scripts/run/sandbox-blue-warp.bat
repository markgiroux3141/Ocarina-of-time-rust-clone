@echo off
rem Headless: GAME-05 milestone 6b's scripted run from a debug start in front of the blue warp in
rem Queen Gohma's cleared room: into it, the float, the white fade out with the Kokiri Emerald,
rem Kokiri Forest by blue warp and the Deku Tree's emerald cutscene, part 1, to its terminator.
rem Writes its trace and screenshots to out\run\: the warp grown, Link in its rays, the white fade,
rem the arrival, the Deku Tree's text.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_BOSS_0 --child --preset deku-tree-gohma-cleared --script blue-warp ^
    --trace out\run\blue-warp.json --screenshot out\run\blue-warp.png --shots-at 90,200,260,400,700 %*
exit /b %errorlevel%
