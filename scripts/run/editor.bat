@echo off
rem The game editor (crates\tools\overworld_editor): builds it and the game (release) into the
rem scripts' target, then opens it with the scripts' pack, so the actors palette reads how the
rem game places its actors and Play runs the game built beside it (docs\EDITOR-ACTORS.md).
rem Arguments go to the editor: a level file, --select actor:N.
setlocal
call "%~dp0_env.bat"
cargo build --release -p oot -p overworld_editor || exit /b 1
start "" "%BIN%\overworld_editor.exe" %*
exit /b 0
