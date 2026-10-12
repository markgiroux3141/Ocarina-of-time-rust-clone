@echo off
rem Castle Town's entrance by day (ENTR_MARKET_ENTRANCE_DAY_0, the deku-tree-dead preset): north
rem into the market (Malon by the fountain, the cucco girl, the townsfolk, the dancing couple).
rem With "night", 20:00 (the night's entrance and market: the guards, the dog). "temple": the
rem Temple of Time's outside (the gossip stones); "altar": inside, by the altar. "alley": the back
rem alley by day.
rem WASD the stick, Space A, E B (the sword), Q Z (lock on), R the shield.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "clock=10:00"
set "entr=ENTR_MARKET_ENTRANCE_DAY_0"
if /i "%~1"=="night" (
    set "clock=20:00"
    shift
)
if /i "%~1"=="temple" (
    set "entr=ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_0"
    shift
)
if /i "%~1"=="altar" (
    set "entr=ENTR_TEMPLE_OF_TIME_0"
    shift
)
if /i "%~1"=="alley" (
    set "entr=ENTR_BACK_ALLEY_DAY_0"
    shift
)
"%BIN%\oot.exe" --entrance %entr% --preset deku-tree-dead --time %clock% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
