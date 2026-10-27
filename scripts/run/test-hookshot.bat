@echo off
rem GAME-13 milestone 1's tests: the runs (Dampe's race's start, the hookshot to Goron City's post),
rem the hookshot (Arms_Hook, Player's flight), En_Po_Relay, En_Honotrap and the schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test hookshot_run --test hookshot --test en_po_relay --test en_honotrap --test obj_hsblock --test schemas %*
exit /b %errorlevel%
