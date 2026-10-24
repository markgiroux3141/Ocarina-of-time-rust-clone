@echo off
rem GAME-11 milestone 3, Sheik's Prelude, the sword put back and the warp, for the hand test.
rem Empty: the Temple of Time's entrance, an adult with the Forest Medallion's flag (prelude, a
rem debug start: the Forest Temple is Phase 14's): walk round the altar and through the doorway:
rem Sheik's Prelude of Light (her harp; play the song back: Up C, Right C, Up C, Right C, Left C,
rem Up C: I, L, I, L, J, I), her Deku Nut's flash.
rem "back": the adult in the pedestal's room with the Prelude learned (prelude-learned): A at the
rem pedestal puts the Master Sword back; Link arrives a child. "again": that child arriving on the
rem pedestal (child-again): A pulls the sword again; Link arrives an adult. "warp": Hyrule Field
rem by the broken drawbridge (prelude-learned): the ocarina (L), the Prelude, "yes": the warp (Link
rem gone in the white-out) to the Temple's light pad.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--adult --entrance ENTR_TEMPLE_OF_TIME_0 --preset prelude"
if /i "%~1"=="back" (
    set "args=--adult --entrance ENTR_TEMPLE_OF_TIME_0 --preset prelude-learned --room 0 --at 0,40,214,-32768"
    shift
) else if /i "%~1"=="again" (
    set "args=--entrance ENTR_TEMPLE_OF_TIME_2 --preset child-again"
    shift
) else if /i "%~1"=="warp" (
    set "args=--adult --entrance ENTR_HYRULE_FIELD_7 --preset prelude-learned"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
