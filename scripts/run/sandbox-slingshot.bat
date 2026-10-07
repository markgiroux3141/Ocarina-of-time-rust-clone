@echo off
rem Headless: GAME-05 milestone 5a's scripted run inside the Deku Tree, from a debug start in
rem room 1 250 in front of its eye switch: Link takes the Fairy Slingshot out (C-Right), aims in
rem first person at the eye and shoots a seed into it; the eye closes, the door to room 2 unbars
rem with its camera, and he goes through it into room 2. Writes its trace and screenshots to
rem out\run\: drawn in first person, the seed's hit, the door unbarring, room 2.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-slingshot --script slingshot ^
    --trace out\run\slingshot.json --screenshot out\run\slingshot.png --shots-at 12,26,31,60,281 %*
exit /b %errorlevel%
