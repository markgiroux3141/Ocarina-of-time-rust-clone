@echo off
rem GAME-11 milestone 2, the Door of Time to the Chamber of the Sages and out, for the hand test.
rem Empty: the Temple of Time's entrance (ocarina-of-time): its intro plays; put the Ocarina of
rem Time on a C button from the item page (Enter, the stick, a C button), walk round the altar's
rem front to the stones (stop just in front of it) and play the Song of Time: the door opens.
rem "pull" (door-of-time: the door open): walk through the doorway into room 0 (the reveal, Navi's
rem texts), to the sword and A: the pull, Ganondorf, the Chamber of the Sages (Rauru, the Light
rem Medallion), then the adult's return and Sheik's awakening.
rem "chamber": the Chamber's script on its own (layer 4, light-medallion). "awakening": the adult
rem arriving at the pedestal's step (light-medallion): Sheik's awakening. "temple": the adult free
rem in room 0 where the awakening leaves him (adult-awake): walk out. "outside": the Temple's steps
rem in the ruins (shrine_r). "market": the ruined market with its eight ReDeads (keep 150 away, or
rem walk slowly: they hear running). "field": Hyrule Field by the broken drawbridge (Bg_Spot00_Break).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_TEMPLE_OF_TIME_0 --preset ocarina-of-time"
if /i "%~1"=="pull" (
    set "args=--entrance ENTR_TEMPLE_OF_TIME_0 --preset door-of-time"
    shift
) else if /i "%~1"=="chamber" (
    set "args=--adult --entrance ENTR_CHAMBER_OF_THE_SAGES_0 --cutscene 0xFFF0 --preset light-medallion"
    shift
) else if /i "%~1"=="awakening" (
    set "args=--adult --entrance ENTR_TEMPLE_OF_TIME_4 --preset light-medallion"
    shift
) else if /i "%~1"=="temple" (
    set "args=--adult --entrance ENTR_TEMPLE_OF_TIME_0 --preset adult-awake --room 0 --at 0,40,214,0"
    shift
) else if /i "%~1"=="outside" (
    set "args=--adult --entrance ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1 --preset adult-awake"
    shift
) else if /i "%~1"=="market" (
    set "args=--adult --entrance ENTR_MARKET_DAY_2 --preset adult-awake"
    shift
) else if /i "%~1"=="field" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_7 --preset adult-awake"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
