@echo off
rem GAME-12 milestone 4's tests: the four runs (Malon's course offered and started, its record,
rem Malon by night, the field's Big Poe), En_Ma3, En_Ma2, En_Po_Field and the schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test ranch_after_runs --test en_ma3 --test en_ma2 --test en_po_field --test schemas %*
exit /b %errorlevel%
