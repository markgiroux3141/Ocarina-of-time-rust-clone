@echo off
rem The guards' courtyard by day from the castle's drain (castle-courtyard: Talon gone home, the Cucco
rem on C-Left, the Fairy Ocarina on C-Down). The patrols look only on their own screen; their look is
rem a line at head height that walls and hedges stop; the south wall's strip is out of their rounds.
rem "night": the courtyard at 22:00 (two guards by the drain).
rem "zelda": Zelda's courtyard, at its entrance (Zelda at the window up the steps).
rem "impa": Zelda's courtyard with Zelda's Letter (zeldas-letter): Impa by the entrance; walk past her.
rem "field": Hyrule Field's layer 12, Impa's goodbye (CS_INDEX_8), with the letter and the lullaby.
rem WASD the stick (Shift to walk), Space A, E B, Q Z, R R, J C-Left, K C-Down (the ocarina).
rem Zelda's Lullaby: J I L J I L (C-Left C-Up C-Right, twice).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "entr=ENTR_CASTLE_COURTYARD_GUARDS_DAY_0"
set "preset=castle-courtyard"
set "clock=10:00"
set "extra="
if /i "%~1"=="night" (
    set "clock=22:00"
    shift
)
if /i "%~1"=="zelda" (
    set "entr=ENTR_CASTLE_COURTYARD_ZELDA_0"
    shift
)
if /i "%~1"=="impa" (
    set "entr=ENTR_CASTLE_COURTYARD_ZELDA_0"
    set "preset=zeldas-letter"
    shift
)
if /i "%~1"=="field" (
    set "entr=ENTR_HYRULE_FIELD_0"
    set "preset=zeldas-letter"
    set "extra=--cutscene 0xFFF8"
    shift
)
"%BIN%\oot.exe" --entrance %entr% --preset %preset% --time %clock% %extra% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
