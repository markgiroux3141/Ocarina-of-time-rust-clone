@echo off
rem The game's characters as .glb files for Blender or the game editor (ootx export-glb,
rem docs\adr\0068-glb-export.md): out\glb\characters\<Name>.glb, one per character by its
rem English name ("Saria.glb", "Link (adult).glb"), standing in a T-pose. Then opens the folder.
rem Arguments go to ootx export-glb, e.g.
rem   export-glb.bat --anims              their animations too (20 fps)
rem   export-glb.bat --filter Kokiri      only the names that contain Kokiri
rem   export-glb.bat --all                everything the pack has, into out\glb\all
rem Derived from your ROM: it stays in the ignored out folder.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" ootx || exit /b 1
"%BIN%\ootx.exe" export-glb --out "%REPO%\out\glb" %* || exit /b 1
if exist "%REPO%\out\glb\characters" (start "" "%REPO%\out\glb\characters") else (start "" "%REPO%\out\glb")
exit /b 0
