@echo off
rem A child in a Kakariko house, in front of its door (the door test's spot): Space (A) opens it.
rem Listen for the door (NA_SE_OC_DOOR_OPEN as it swings, at the door), then Kakariko Village
rem (ENTR_KAKARIKO_VILLAGE_6), where Link walks in with the door's sound again (Player_Init's
rem entranceSound). Kakariko's actors are mostly placeholders.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_KAKARIKO_CENTER_GUEST_HOUSE_0 --at=100,0,180,0 %*
exit /b %errorlevel%
