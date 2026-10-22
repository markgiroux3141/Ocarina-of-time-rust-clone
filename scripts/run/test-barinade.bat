@echo off
rem GAME-10 milestone 6's tests: Barinade (Boss_Va: his parts, intro, phases, death, effects and the
rem cleared room), Ruto's warp (Door_Warp1's WARP_BLUE_RUTO, one-point 9705), the Zora's Sapphire's
rem cutscene played through, Ruto (En_Ru1), the two runs (Route::Barinade, Route::Sapphire) and the
rem actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test boss_va --test ruto_warp --test barinade_runs --test en_ru1 --test blue_warp --test schemas %*
exit /b %errorlevel%
