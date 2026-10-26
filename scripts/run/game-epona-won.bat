@echo off
rem GAME-12 milestone 3, Epona won and out (the phase's exit), for the hand test.
rem Empty: the second race just won (ingo-second-win), on Epona by the ranch's way out: Ingo gives
rem her up and shuts the gate (0x203B, 0x203C). Then get out: a spurred gallop at the shut gate
rem (A for carrots; she needs her top speed for it) or over the ranch's outer fence: the field's
rem cutscene of Epona's leap, and she's yours.
rem "south", "east", "west", "gate": straight into one of the four leaps' cutscenes in the field.
rem "field": Hyrule Field with Epona won (epona-won, Phase 14's start), outside the ranch's gate.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--adult --entrance ENTR_LON_LON_RANCH_7 --preset ingo-second-win"
if /i "%~1"=="south" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_11 --preset ingo-second-win"
    shift
) else if /i "%~1"=="west" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_12 --preset ingo-second-win"
    shift
) else if /i "%~1"=="east" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_13 --preset ingo-second-win"
    shift
) else if /i "%~1"=="gate" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_15 --preset ingo-second-win"
    shift
) else if /i "%~1"=="field" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_6 --preset epona-won"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
