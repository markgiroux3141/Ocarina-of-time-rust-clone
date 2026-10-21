@echo off
rem GAME-10 milestone 4's runs, headless: traces and screenshots into out\run.
rem jabu-cow: the slingshot onto C-Right, room 0's west cow shot, door 1. jabu-elevator: room 1's
rem elevator from door 1's corridor, down, under the wall, the yellow switch 0x19. jabu-boomerang:
rem room 14's chest. jabu-door-14: room 1's bottom after the chest, through door 14.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_0 --child --preset jabu-jabu --script jabu-cow ^
    --trace out\run\jabu_cow.json --screenshot out\run\jabu_cow.png --shots-at 100,175,400,560 || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_0 --child --preset jabu-room-1 --script jabu-elevator ^
    --trace out\run\jabu_elevator.json --screenshot out\run\jabu_elevator.png --shots-at 170,300,500,600 || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_0 --child --preset jabu-room-14 --script jabu-boomerang ^
    --trace out\run\jabu_boomerang.json --screenshot out\run\jabu_boomerang.png --shots-at 60,200 || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_0 --child --preset jabu-boomerang --script jabu-door-14 ^
    --trace out\run\jabu_door_14.json --screenshot out\run\jabu_door_14.png --shots-at 40,100 %*
exit /b %errorlevel%
