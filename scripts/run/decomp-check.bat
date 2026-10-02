@echo off
rem The decomp upgrade's pack check (ADR 0031): imports a loose pack from oot.toml's decomp and
rem compares it, record by record, with a loose pack of the build before the upgrade, the old
rem names renamed through docs\name-map.
rem   decomp-check.bat [<baseline loose pack>]   (default out\loose-2f4c25d)
rem The baseline comes from commit 0ed116d with oot.toml's decomp at 2f4c25d:
rem   git worktree add ..\oot-clone-base 0ed116d, build it, then in it
rem   ootx import --loose <this repo>\out\loose-2f4c25d
rem Prints the records that are the same, the same once renamed, and the rest, which
rem docs\GAME-05-deku-tree.md (milestone 1) explains. Add --dump DIR to see the differing ones.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" ootx || exit /b 1
set "BASE=%~1"
if "%BASE%"=="" set "BASE=out\loose-2f4c25d"
if not exist "%BASE%\meta" goto no_base
if exist out\loose-new rmdir /s /q out\loose-new
"%BIN%\ootx.exe" import --loose out\loose-new || exit /b 1
python scripts\name_map.py compare-pack "%BASE%" out\loose-new %2 %3
exit /b %errorlevel%

:no_base
echo %BASE%: no baseline loose pack (see the comment at the top of this file).
exit /b 1
