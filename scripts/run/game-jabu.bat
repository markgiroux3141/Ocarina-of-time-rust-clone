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
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
