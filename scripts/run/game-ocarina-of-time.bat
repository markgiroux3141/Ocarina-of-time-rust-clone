@echo off
rem Hyrule Field as a child with the three spiritual stones (GAME-10 milestone 7). Empty: the field
rem short of the drawbridge (zora-sapphire): walk north onto the bridge's way; the screen fades and
rem Zelda's escape plays (Zelda and Impa on horseback out of the castle, Impa throws the Ocarina of
rem Time into the moat, Ganondorf and his cape, the storm), then the field again.
rem "escape": the escape's cutscene alone (zelda-fled, cutscene 0xFFF1).
rem "moat": the moat's south bank after the escape (zelda-fled): walk off into the water, dive for
rem the ocarina (bubbles over it; Navi's C-Up message by it), its get-item text; Zelda's Song of
rem Time cutscene in the field, the Temple of Time (Zelda teaches the Song of Time: play it back
rem with the C buttons and A), and the field again (gHyruleFieldGetOoTCs).
rem "temple": the Temple of Time's Song of Time cutscene alone (zelda-fled, cutscene 0xFFF7).
rem "field": Hyrule Field afterwards (ocarina-of-time: Phase 11's exit, Phase 12's start).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_HYRULE_FIELD_0 --preset zora-sapphire --at 160,0,1865,-32768"
if /i "%~1"=="escape" (
    set "args=--entrance ENTR_HYRULE_FIELD_0 --cutscene 0xFFF1 --preset zelda-fled"
    shift
) else if /i "%~1"=="moat" (
    set "args=--entrance ENTR_HYRULE_FIELD_0 --preset zelda-fled --at 299,0,1160,-32768"
    shift
) else if /i "%~1"=="temple" (
    set "args=--entrance ENTR_TEMPLE_OF_TIME_0 --cutscene 0xFFF7 --preset zelda-fled"
    shift
) else if /i "%~1"=="field" (
    set "args=--entrance ENTR_HYRULE_FIELD_16 --preset ocarina-of-time"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
