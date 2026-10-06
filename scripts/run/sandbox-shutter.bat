@echo off
rem Headless: GAME-05 milestone 4a's scripted run inside the Deku Tree, from a debug start on room
rem 0's top floor: Link steps on the floor switch (the web over room 10's door burns, the golden
rem torches light, the attention cameras), opens room 10's sliding door and walks through, and
rem the door slams and bars behind him. Writes its trace and screenshots to out\run\: the
rem switch, the web burning, Navi's hint by the door, the door opening, and the door barred behind
rem Link.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-inside --script shutter ^
    --trace out\run\shutter.json --screenshot out\run\shutter.png --shots-at 50,65,330,535,560 %*
exit /b %errorlevel%
