@echo off
rem The coverage page (docs\COVERAGE.md): every scene's actors from the pack (ootx census), against
rem what oot_actors ports, by the roadmap's phases (scripts\coverage.py holds the scenes' phases).
rem Run it at the end of a milestone; commit the page with the milestone.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" ootx || exit /b 1
"%BIN%\ootx.exe" census || exit /b 1
python scripts\coverage.py
exit /b %errorlevel%
