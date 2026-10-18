@echo off
rem Death Mountain Trail with the Goron's Ruby and bombs (goron-ruby). Empty: by Goron City's door
rem (the trail's spawn 1): up the road to the summit, through the falling rocks above y 1500 and past
rem the three Skullwalltulas (the hand test). "wall": on the summit in front of the Great Fairy's
rem wall (a bomb breaks it, in through the hole). "rocks": on the trail inside the falling rocks'
rem area (x -700 to 100, z -1290 to -3600, above y 1500), facing up it. "fountain": the Great Fairy's fountain with the
rem wall broken (fairy-fountain: Zelda's Lullaby on the Triforce, her cutscene, the magic, out).
rem "revisit": the fountain again with magic (fairy-magic: her second visit and the refill). "owl":
rem the summit by the owl with magic (fairy-magic: his flight to Kakariko).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_1 --preset goron-ruby"
if /i "%~1"=="rocks" (
    set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_1 --preset goron-ruby --at -300,1844,-2500,-32768"
    shift
) else if /i "%~1"=="wall" (
    set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_4 --preset goron-ruby --at -179,3314,-4365,-27671"
    shift
) else if /i "%~1"=="fountain" (
    set "args=--entrance ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0 --preset fairy-fountain"
    shift
) else if /i "%~1"=="revisit" (
    set "args=--entrance ENTR_GREAT_FAIRYS_FOUNTAIN_MAGIC_0 --preset fairy-magic"
    shift
) else if /i "%~1"=="owl" (
    set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_4 --preset fairy-magic"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
