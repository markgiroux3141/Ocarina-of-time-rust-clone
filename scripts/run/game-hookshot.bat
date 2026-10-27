@echo off
rem GAME-13 milestone 1, the hookshot and Dampe's race, for the hand test.
rem Empty: Dampe's grave by day (epona-won), at the top of the landing: walk down to Dampe; his
rem greeting opens. Race him through the rooms behind the doors he opens (his fire drops on the
rem way; a door shuts if you lag). The first race's end: the room's chest, the hookshot. Race
rem again with it: under 60 seconds, a piece of heart.
rem "post": Goron City's bottom with the hookshot (hookshot), on the ledge by the stone post: C-Left
rem raises the hookshot (first person: the stick aims, the red reticle where it would hit); let go
rem of C-Left to shoot. The post's top band takes it (Link is pulled up and catches its edge); its
rem lower part throws it back. Try the walls, the targets, a rupee or a pot.
rem "eyes": the Shadow Temple's first room with the hookshot: the flame-shooting eyes (En_Honotrap).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--adult --entrance ENTR_WINDMILL_AND_DAMPES_GRAVE_0 --preset epona-won"
if /i "%~1"=="post" (
    set "args=--adult --entrance ENTR_GORON_CITY_2 --preset hookshot --at=961,480,-148,16384"
    shift
) else if /i "%~1"=="eyes" (
    set "args=--adult --entrance ENTR_SHADOW_TEMPLE_0 --preset hookshot"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
