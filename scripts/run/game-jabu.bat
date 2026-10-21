@echo off
rem GAME-10 milestone 4's places in Jabu-Jabu's Belly (Master Quest) as a child, for the hand tests
rem a script can't do well (the fights). Empty or "cow": room 0's entrance (the Shaboms and the
rem Octorok; put the slingshot on a C button and shoot the west cow's target: door 1 opens).
rem "elevator": room 1 at the end of door 1's corridor (run off its end onto the elevator as it
rem comes down past you; at the bottom, swim west, dive under the wall, climb out onto the yellow
rem switch 0x19: door 2 opens). "flobbery": room 1's west walkway by the Flobberies (the Biris
rem float over the shaft). "pit": room 1's bottom facing the pit (the Stinger in the water, the tall
rem switch on the bottom). "spout": the same place facing the spout across door 14 (it lifts you
rem and pushes you back). "platform": room 1's top walkway by the falling platform (step on it: it
rem drops 400, the camera under it). "room2": room 2 (the Biris, the Flobbery, the floor drifting
rem into its holes). "room14": room 14's floor (the two Stingers, the Like Like, the Lizalfos; the
rem yellow switch raises the water for 100 frames). "boomerang": room 14 cleared, in front of the
rem boomerang's chest. "door14": room 1's bottom after the chest (the spout down): through door 14.
rem Milestone 5's: "ruto": room 2 inside door 2 (walk towards Ruto: her first meeting, her drop).
rem "carry": room 3 by Ruto after it (talk, lift her with A, carry, throw, drop her in the water,
rem set her on the blue switch). "stick": room 11 cleared (the chest's Deku Stick, the lit torch;
rem the web in room 7). "barriers": room 7's gray barriers. "gray", "red", "green": inside rooms
rem 9, 8 and 10 facing their tentacle (the boomerang on a C button from the item page: four hits).
rem "crates": room 7's crates, the blue switch's boulders blown (a crate on it opens door 6).
rem "crystal": room 1 in door 9's corridor, the tentacles dead. "room5": room 5 inside door 9
rem (the crystal over the pool). "door0": room 5 after the crystal, in front of door 0 (room 15's
rem floor drops you into Barinade's room).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-jabu"
if /i "%~1"=="cow" (
    shift
) else if /i "%~1"=="elevator" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-1 --room 1 --at 0,-320,-1500,-32768"
    shift
) else if /i "%~1"=="flobbery" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-1 --room 1 --at -200,-325,-1600,-32768"
    shift
) else if /i "%~1"=="pit" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-1 --room 1 --at 160,-1233,-1703,-16384"
    shift
) else if /i "%~1"=="spout" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-1 --room 1 --at 160,-1233,-1703,16384"
    shift
) else if /i "%~1"=="platform" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-1 --room 1 --at 0,80,-1700,16384"
    shift
) else if /i "%~1"=="room2" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-14 --room 2 --at 0,-340,-2560,-32768"
    shift
) else if /i "%~1"=="room14" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-14 --room 14 --at 1000,-1293,-2450,0"
    shift
) else if /i "%~1"=="boomerang" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-14 --clear 14 --room 14 --at 780,-1073,-2376,-16384"
    shift
) else if /i "%~1"=="door14" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-boomerang --room 1 --at 160,-1233,-1703,16384"
    shift
) else if /i "%~1"=="ruto" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-boomerang --clear 2 --room 2 --at 0,-340,-2560,-32768"
    shift
) else if /i "%~1"=="carry" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-7 --room 3 --at 140,-1113,-3270,16384"
    shift
) else if /i "%~1"=="stick" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-room-7 --clear 11 --room 11 --switch 0x01 --at 660,-340,-5260,-32768"
    shift
) else if /i "%~1"=="barriers" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-web --room 7 --at -640,-340,-4800,-16384"
    shift
) else if /i "%~1"=="gray" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-web --room 9 --at 1120,-340,-4683,16384"
    shift
) else if /i "%~1"=="red" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-web --room 8 --at 0,-340,-5370,-32768"
    shift
) else if /i "%~1"=="green" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-web --room 10 --at -1120,-340,-4683,-16384"
    shift
) else if /i "%~1"=="crates" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-crystal --room 7 --at -560,-340,-4660,-16384"
    shift
) else if /i "%~1"=="crystal" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-crystal --clear 1 --room 1 --at 420,-320,-1703,16384"
    shift
) else if /i "%~1"=="room5" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-crystal --room 5 --at 700,-320,-1703,16384"
    shift
) else if /i "%~1"=="door0" (
    set "args=--entrance ENTR_JABU_JABU_0 --preset jabu-barinade --clear 5 --room 5 --at 1360,-320,-2040,-32768"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
