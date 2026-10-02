@echo off
rem A shortcut to the crawlspace on the way to the Kokiri Sword (Kokiri Forest, ENTR_KOKIRI_FOREST_0):
rem Link 59 in front of its mouth, facing it. W up to it, Space (A) when it says Enter, W through
rem it: climbing out at the far end plays its one-point cutscene (9601: the camera's spline up and
rem out over Link, then back to the normal camera without a jump). Backing out the way you came
rem (S) plays the other (9602).
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_KOKIRI_FOREST_0 --at=-785,120,1000,0 %*
exit /b %errorlevel%
