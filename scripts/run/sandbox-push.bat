@echo off
rem Headless: GAME-05 milestone 4c's scripted run inside the Deku Tree, from a debug start on
rem room 3's upper floor behind its push block: Link walks to the block (Navi's hint there read),
rem holds on to it and pushes it along the floor's channel and off its end into the pit (flag
rem 0x10, the chime), then goes down into the pit beside it and climbs onto it. Writes its trace
rem and screenshots to out\run\: holding on, pushing, the block falling, the pit, on the block.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-inside --script push ^
    --trace out\run\push.json --screenshot out\run\push.png --shots-at 330,450,592,660,729 %*
exit /b %errorlevel%
