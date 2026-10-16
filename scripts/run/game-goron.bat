@echo off
rem Death Mountain Trail and Goron City as a child after the ranch (ranch-epona: Zelda's Lullaby,
rem Saria's Song, no bracelet, no bombs). Empty: the trail's foot, the whole way (its intro, the
rem Tektites, the rolling Goron, Goron City's door at the top). "city": Goron City's top (its
rem intro). "darunia": the city's bottom floor by Darunia's door (the lullaby at the spot before
rem it, then Saria's Song to him). "cliff": the trail by Goron City's door with the bracelet
rem (goron-bracelet): up to the cliff above the cavern, its Bomb Flower thrown down onto the
rem boulder. "cavern": before the cavern, the boulder gone (cavern-open). "bombs": Goron City's top
rem with the bracelet and 20 bombs (cavern-bombs: the urn, Medigoron's walls, the rolling Goron).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down (the ocarina),
rem L C-Right, I C-Up.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_0 --preset ranch-epona"
if /i "%~1"=="city" (
    set "args=--entrance ENTR_GORON_CITY_0 --preset ranch-epona"
    shift
) else if /i "%~1"=="darunia" (
    set "args=--entrance ENTR_GORON_CITY_2 --preset ranch-epona"
    shift
) else if /i "%~1"=="cliff" (
    set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_1 --preset goron-bracelet"
    shift
) else if /i "%~1"=="cavern" (
    set "args=--entrance ENTR_DEATH_MOUNTAIN_TRAIL_3 --preset cavern-open"
    shift
) else if /i "%~1"=="bombs" (
    set "args=--entrance ENTR_GORON_CITY_0 --preset cavern-bombs"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
