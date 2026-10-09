@echo off
rem Queen Gohma's room (ENTR_DEKU_TREE_BOSS_0) with the Kokiri Sword and Deku Shield, the Fairy
rem Slingshot on C-Right (L), Deku nuts on C-Down (K) and Deku Sticks on C-Left (J): a debug
rem start (the deku-tree-gohma preset), file 2 of an SRAM in memory.
rem   game-gohma.bat         her whole intro: walk in to the room's entrance; once Link is free,
rem                          look up at her on the ceiling (L aims in first person, or I) until
rem                          she notices
rem   game-gohma.bat again   her battle already begun (EVENTCHKINF_BEGAN_GOHMA_BATTLE, as after a
rem                          game over in her room): straight to her eye roll and her drop, no
rem                          title card
rem The fight: when her eye turns red (she rears up), a seed into it stuns her (Q locks on, L
rem aims at her, let go to shoot); then E (B) for the sword and Q-locked Space (A) jump slashes.
rem On the ceiling, shoot her eye while it's red (before she lays eggs) to knock her down.
rem WASD the stick, Q is Z, E is B, Space A, R the shield, Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "PRESET=deku-tree-gohma"
if /i "%~1"=="again" set "PRESET=deku-tree-gohma-again"
"%BIN%\oot.exe" --entrance ENTR_DEKU_TREE_BOSS_0 --preset %PRESET%
exit /b %errorlevel%
