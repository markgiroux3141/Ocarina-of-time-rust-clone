@echo off
rem Headless: GAME-05 milestone 6c's scripted run from room 9's debug start (the room cleared, its
rem door to room 11 open; deku-tree-slingshot): through the door, onto room 11's floor and into
rem Queen Gohma's room, standing in its corridor. Writes its trace and screenshots to out\run\:
rem Link through the door into room 11, and the end.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_DEKU_TREE_0 --child --preset deku-tree-slingshot --script boss-room ^
    --trace out\run\boss-room.json --screenshot out\run\boss-room.png --shots-at 265 %*
exit /b %errorlevel%
