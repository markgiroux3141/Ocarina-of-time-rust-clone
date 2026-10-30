@echo off
rem GAME-13 milestone 2, the Lost Woods, the meadow and the Minuet, for the hand test.
rem Empty: the Lost Woods as an adult from Kokiri Forest (hookshot): the way to Mido (right, north,
rem right), who blocks it; the ocarina (L, C-Right) and Saria's Song; past him (north, left, north)
rem to the meadow. Its maze past the Moblins' patrols (the spears see you down their corridors and
rem charge) and the club Moblin on the ramp (its shockwave); at the top Sheik drops in: the Minuet of
rem Forest. Then the hookshot (J, C-Left) into the tree's branch over the ledge, and the temple.
rem "meadow": the meadow's entrance (hookshot): the maze and the Moblins, Sheik at the top.
rem "club": the meadow's ramp below the club Moblin (hookshot).
rem "top": the meadow's top under the temple's ledge with the Minuet (minuet): the hookshot into
rem the branch, down onto the ledge, into the temple.
rem "warp": Hyrule Field with the Minuet (minuet): play it (C-Right), warp to the meadow.
rem "rupees": Kokiri Forest's spawn 11 (hookshot): the ring of rupees hanging over the bean's spot
rem north-west of it (Obj_Mure3); the hookshot drags them in.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--adult --entrance ENTR_LOST_WOODS_0 --preset hookshot"
if /i "%~1"=="meadow" (
    set "args=--adult --entrance ENTR_SACRED_FOREST_MEADOW_0 --preset hookshot"
    shift
) else if /i "%~1"=="club" (
    set "args=--adult --entrance ENTR_SACRED_FOREST_MEADOW_0 --preset hookshot --at=0,160,-1000,32768"
    shift
) else if /i "%~1"=="top" (
    set "args=--adult --entrance ENTR_SACRED_FOREST_MEADOW_0 --preset minuet --at=-30,480,-2850,32768"
    shift
) else if /i "%~1"=="warp" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_7 --preset minuet"
    shift
) else if /i "%~1"=="rupees" (
    set "args=--adult --entrance ENTR_KOKIRI_FOREST_11 --preset hookshot"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
