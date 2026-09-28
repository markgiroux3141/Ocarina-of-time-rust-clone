@echo off
rem A menu of the scripts in this folder, for double-clicking. It comes back after each one.
setlocal
:menu
cls
echo OoT clone: run scripts (settings in scripts\run\_env.bat)
echo.
echo   Build and test
echo     1  build                Build everything (release)
echo     2  test                 All the tests
echo     3  test-playthrough     GAME-02's playthrough test
echo    11  test-sword-chest     GAME-03's Kokiri Sword chest test
echo     4  golden-check         The render and trace regression
echo     5  import               Re-import the asset pack
echo.
echo   Play (a window; close it to come back)
echo     6  game                 Kokiri Forest
echo     7  game-links-house     From Link's bed, the Deku Tree's mouth open
echo     8  game-deku-tree-open  At the Deku Tree, his mouth open
echo     9  game-deku-tree-dead  At the Deku Tree after Gohma
echo    12  game-sword-chest     A new save at the Kokiri Sword's chest
echo.
echo   Headless
echo    10  sandbox-playthrough  The playthrough's trace and screenshots, into out\run
echo.
echo     0  quit (or an empty line)
echo.
set "pick="
rem An empty line quits (set /p fails on it, and at the end of piped input).
set /p "pick=Choose: " || exit /b 0
set "pick=%pick: =%"
if "%pick%"=="0" exit /b 0
set "script="
if "%pick%"=="1" set "script=build"
if "%pick%"=="2" set "script=test"
if "%pick%"=="3" set "script=test-playthrough"
if "%pick%"=="4" set "script=golden-check"
if "%pick%"=="5" set "script=import"
if "%pick%"=="6" set "script=game"
if "%pick%"=="7" set "script=game-links-house"
if "%pick%"=="8" set "script=game-deku-tree-open"
if "%pick%"=="9" set "script=game-deku-tree-dead"
if "%pick%"=="10" set "script=sandbox-playthrough"
if "%pick%"=="11" set "script=test-sword-chest"
if "%pick%"=="12" set "script=game-sword-chest"
if not defined script goto menu
echo.
call "%~dp0%script%.bat"
echo.
echo %script% finished (exit code %errorlevel%).
pause
goto menu
