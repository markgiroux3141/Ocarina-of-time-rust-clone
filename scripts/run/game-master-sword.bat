@echo off
rem GAME-11 milestone 1's time machine, for the hand test. Empty: the Temple of Time's room 0, a
rem child on the pedestal's step with the Door of Time open and the Master Sword already obtained
rem once (master-sword): face the sword, A (the pull's cutscene; the temple dims), then the white
rem fill and Link arrives an adult on the pedestal with the Master Sword in hand, the Hylian
rem Shield, bombs on C-Down and the Ocarina of Time on C-Right. "back": that adult on the pedestal
rem with the Prelude of Light's flag (master-sword-adult): face the sword, A (it goes back in the
rem pedestal), then Link arrives a child with the Kokiri Sword on B and his own C buttons.
rem "door": the Temple's entrance with the Door of Time shut (ocarina-c: the Ocarina of Time on
rem C-Down; its intro plays first): the closed door should now stop Link (Door_Toki); the Song of
rem Time at the altar opens it.
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_TEMPLE_OF_TIME_0 --preset master-sword --room 0 --at 0,70,46,-3641"
if /i "%~1"=="back" (
    set "args=--adult --entrance ENTR_TEMPLE_OF_TIME_2 --preset master-sword-adult"
    shift
) else if /i "%~1"=="door" (
    set "args=--entrance ENTR_TEMPLE_OF_TIME_0 --preset ocarina-c"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
