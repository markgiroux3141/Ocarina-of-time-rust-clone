@echo off
rem Headless: GAME-05 milestone 4b's scripted run inside the Deku Tree, from a debug start by
rem room 0's middle-floor golden torch (ten Deku Sticks on C-Left, the torches lit): Link takes a
rem stick out, lights it at the torch (Navi's hint read there), runs round the floor and jumps its
rem gap, burns the web over room 1's door with it (the stick burning out as it does), and goes
rem through the door into room 1. Writes its trace and screenshots to out\run\: the stick out,
rem lit, the jump, the web burning, and the door.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-sticks --script stick ^
    --trace out\run\stick.json --screenshot out\run\stick.png --shots-at 30,55,245,340,620 %*
exit /b %errorlevel%
