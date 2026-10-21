@echo off
rem Water and bottles (GAME-10 milestone 1) as a child with magic and bombs.
rem Empty: Zora's Domain's shallows by the fish with the milk on C-Left (zora-bottle): C-Left
rem drinks it (twice: half, then the bottle's empty); C-Left takes the empty bottle out, C-Left
rem again swings it (catch a fish: go slowly at one); C-Left with the fish lets it out. "lake":
rem Lake Hylia over Ruto's letter's spot with the silver scale (lake-dive): A held at the surface
rem dives to 240 (without the scale, 120). "river": Zora's River's mouth (the current carries a
rem swimmer). "tunnel": in from Zora's Domain's tunnel (spawn 7, under water: Link swims in).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_ZORAS_DOMAIN_0 --preset zora-bottle --at 560,-20,-420,16384"
if /i "%~1"=="lake" (
    set "args=--entrance ENTR_LAKE_HYLIA_0 --preset lake-dive --at -930,-1313,4297,0"
    shift
) else if /i "%~1"=="river" (
    set "args=--entrance ENTR_ZORAS_RIVER_0 --preset fairy-magic"
    shift
) else if /i "%~1"=="tunnel" (
    set "args=--entrance ENTR_LAKE_HYLIA_7 --preset lake-dive"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
