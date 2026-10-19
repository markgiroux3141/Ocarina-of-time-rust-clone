@echo off
rem The game editor's captures of the actors (docs\EDITOR-ACTORS.md, "Models in the views"):
rem every placeable actor in params 0 and the ways the game places it, spawned headless and drawn
rem by its own code (oot capture-actors), into out\editor\actors, with each one drawn to a picture
rem and all of them on a contact sheet (out\editor\actors\png\sheet.png, opened at the end). The
rem editor makes the captures by itself; this is for a look at them all. Arguments go to the
rem capture: --all (make every one again), --only En_Ko,Obj_Tsubo.
setlocal
call "%~dp0_env.bat"
cargo build --release -p oot || exit /b 1
"%BIN%\oot.exe" capture-actors --pngs %* || exit /b 1
start "" "%REPO%\out\editor\actors\png\sheet.png"
exit /b 0
