@echo off
rem Queen Gohma's room after her defeat (the deku-tree-gohma-cleared preset: her room cleared, its
rem heart container taken, four hearts), a debug start 150 in front of the blue warp she leaves:
rem wait for it to grow in, walk into it: Link floats up in its light, the white fade, and he
rem arrives by blue warp before the Deku Tree, who talks (Space through his texts). His cutscene
rem then goes on to Ganondorf's tale, whose scenes' actors are placeholders (BACKLOG #23).
rem WASD the stick, Space A.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_BOSS_0 --preset deku-tree-gohma-cleared --at=0,-640,150,32768 %*
exit /b %errorlevel%
