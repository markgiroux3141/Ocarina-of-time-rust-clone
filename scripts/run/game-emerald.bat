@echo off
rem Kokiri Forest after the blue warp (the deku-tree-dead preset), started in a cutscene layer as
rem this debug ROM's map select can: with no argument layer 6 (0xFFF2), the Kokiri Emerald's last
rem part: the Deku Tree's texts, the emerald's green light out of the tree, the emerald held up and
rem floating over Link, the tree's death, then the forest at ENTR_KOKIRI_FOREST_11. With "farore",
rem layer 4 (0xFFF0): Farore's light over the forest at night, her light shower, her texts.
rem Space through the texts. WASD the stick, Space A.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "cs=0xFFF2"
if /i "%~1"=="farore" (
    set "cs=0xFFF0"
    shift
)
"%BIN%\oot.exe" --entrance ENTR_KOKIRI_FOREST_0 --preset deku-tree-dead --cutscene %cs% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
