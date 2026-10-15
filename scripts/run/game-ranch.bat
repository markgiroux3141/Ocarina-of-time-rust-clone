@echo off
rem Lon Lon Ranch from its gate with the Lost Woods' games won (lost-woods-games). Through the
rem corral's gate (in the near fence, to the right) to Malon by the foal: talk to her three times,
rem then take out the ocarina by her (K): she sings Epona's Song, play it back (I J L I J L). The
rem song's scene, then back at the ranch: the foal comes to you. Walk away, wait for her to go
rem home, play Epona's Song (I J L I J L): she comes again. Then back out of the corral, north to
rem the gate's lane: Talon's house is the door on the right. Talk to Talon at his table, yes:
rem catch his three super cuccos (A by one to lift it, hold it till he speaks) in 30 seconds.
rem "malon": by Malon in the corral. "night": the ranch at 22:00, the Guays.
rem "stable": the stable by day (Ingo working). "stable-night": its horses at 22:00.
rem "ingo": the ranch before Talon's return (castle-egg), Ingo working by the gate.
rem "house": in Talon's house by day, at his room's door.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down (the ocarina),
rem L C-Right (the slingshot), I C-Up.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_LON_LON_RANCH_0 --preset lost-woods-games"
if /i "%~1"=="malon" (
    set "args=--entrance ENTR_LON_LON_RANCH_0 --preset lost-woods-games --at=75,0,-650,0"
    shift
)
if /i "%~1"=="night" (
    set "args=--entrance ENTR_LON_LON_RANCH_0 --preset lost-woods-games --time 22:00"
    shift
)
if /i "%~1"=="stable" (
    set "args=--entrance ENTR_STABLE_0 --preset lost-woods-games"
    shift
)
if /i "%~1"=="stable-night" (
    set "args=--entrance ENTR_STABLE_0 --preset lost-woods-games --time 22:00"
    shift
)
if /i "%~1"=="house" (
    set "args=--entrance ENTR_LON_LON_BUILDINGS_0 --preset lost-woods-games"
    shift
)
if /i "%~1"=="ingo" (
    set "args=--entrance ENTR_LON_LON_RANCH_0 --preset castle-egg --at=760,0,-2040,-12000"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
