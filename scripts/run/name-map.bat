@echo off
rem Regenerates docs\name-map (ADR 0031): builds both decomps in Docker, then pairs their names.
rem   name-map.bat <old decomp> <new decomp> <baserom.z64> [<builds folder>]
rem e.g. name-map.bat "D:\OOT Modding\OTT decomp\z64oot" "D:\OOT Modding\oot-main" "D:\OOT Modding\Debug Roms\baserom.z64"
rem The checkouts are only read: each is cloned into a Docker volume (oot-old, oot-new) and
rem built there with its own Dockerfile (scripts\decomp\build_*.sh). The two ELFs go to the
rem builds folder (default %TEMP%\oot-decomp-builds), outside the repo. The first run takes a
rem while (two images, two full builds); later ones reuse the volumes' builds.
rem Then: git diff docs\name-map, and check unpaired.tsv's new rows.
setlocal
call "%~dp0_env.bat"
if "%~3"=="" goto usage
set "OLD_DECOMP=%~f1"
set "NEW_DECOMP=%~f2"
set "ROM=%~f3"
set "ROM_DIR=%~dp3"
rem Without its trailing backslash, which Docker would read as part of the path.
set "ROM_DIR=%ROM_DIR:~0,-1%"
set "BUILDS=%~f4"
if "%~4"=="" set "BUILDS=%TEMP%\oot-decomp-builds"
if not exist "%OLD_DECOMP%\Dockerfile" goto no_old
if not exist "%NEW_DECOMP%\Dockerfile" goto no_new
if /i not "%~nx3"=="baserom.z64" goto rom_name
if not exist "%BUILDS%" mkdir "%BUILDS%"

echo Building the images...
docker build -q -t oot-old - < "%OLD_DECOMP%\Dockerfile" || goto failed
docker build -q -t oot-new - < "%NEW_DECOMP%\Dockerfile" || goto failed
docker volume create oot-old > nul || goto failed
docker volume create oot-new > nul || goto failed

echo Building 2f4c25d (log: %BUILDS%\old.log)...
docker run --rm -v oot-old:/oot -v "%OLD_DECOMP%:/src:ro" -v "%ROM_DIR%:/roms:ro" -v "%REPO%\scripts\decomp:/scripts:ro" -v "%BUILDS%:/out" oot-old bash /scripts/build_old.sh > "%BUILDS%\old.log" 2>&1 || goto failed
echo Building 52a510f (log: %BUILDS%\new.log)...
docker run --rm -v oot-new:/oot -v "%NEW_DECOMP%:/src:ro" -v "%ROM_DIR%:/roms:ro" -v "%REPO%\scripts\decomp:/scripts:ro" -v "%BUILDS%:/out" oot-new bash /scripts/build_new.sh > "%BUILDS%\new.log" 2>&1 || goto failed

echo Pairing the names...
python scripts\name_map.py build --old-elf "%BUILDS%\old.elf" --new-elf "%BUILDS%\new.elf" --old-decomp "%OLD_DECOMP%" --new-decomp "%NEW_DECOMP%" --new-label 52a510f
exit /b %errorlevel%

:usage
echo usage: name-map.bat ^<old decomp^> ^<new decomp^> ^<baserom.z64^> [^<builds folder^>]
exit /b 1
:no_old
echo %OLD_DECOMP%: no Dockerfile (is it the decomp's checkout?)
exit /b 1
:no_new
echo %NEW_DECOMP%: no Dockerfile (is it the decomp's checkout?)
exit /b 1
:rom_name
echo The ROM must be named baserom.z64 (the build scripts copy /roms/baserom.z64).
exit /b 1
:failed
echo Failed: see the logs in %BUILDS%.
exit /b 1
