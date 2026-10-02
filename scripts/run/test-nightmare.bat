@echo off
rem GAME-04b milestone 6's tests: the opening's nightmare. The drawbridge lowering on the
rem script's flag and its chains following, the torches' lights, Zelda and Ganondorf on their
rem cues, the horses' skin skeletons, the rain's drops; and the horses' skins in the pack
rem against the ROM.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test nightmare %*
if errorlevel 1 goto end
cargo test --release -p oot_import --test pack the_horses_skins_are_the_roms %*
:end
exit /b %errorlevel%
