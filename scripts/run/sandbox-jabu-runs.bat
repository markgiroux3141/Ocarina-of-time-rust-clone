@echo off
rem GAME-10 milestone 4's runs, headless: traces and screenshots into out\run.
rem jabu-cow: the slingshot onto C-Right, room 0's west cow shot, door 1. jabu-elevator: room 1's
rem elevator from door 1's corridor, down, under the wall, the yellow switch 0x19. jabu-boomerang:
rem room 14's chest. jabu-door-14: room 1's bottom after the chest, through door 14.
rem Milestone 5's: jabu-ruto: room 2, Ruto's first meeting, the cow's boulders bombed, the cow
rem shot, door 4. jabu-web: room 11's stick lit, through door 7, room 7's web burnt. jabu-crystal:
rem door 9, room 5's crystal, door 0, the fall into Barinade's room.
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
    --trace out\run\jabu_door_14.json --screenshot out\run\jabu_door_14.png --shots-at 40,100 || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_0 --child --preset jabu-boomerang --script jabu-ruto ^
    --trace out\run\jabu_ruto.json --screenshot out\run\jabu_ruto.png --shots-at 300,600,960,1066 || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_0 --child --preset jabu-room-7 --script jabu-web ^
    --trace out\run\jabu_web.json --screenshot out\run\jabu_web.png --shots-at 640,720,780 || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_JABU_JABU_0 --child --preset jabu-crystal --script jabu-crystal ^
    --trace out\run\jabu_crystal.json --screenshot out\run\jabu_crystal.png --shots-at 340,400,660 %*
exit /b %errorlevel%
