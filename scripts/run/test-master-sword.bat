@echo off
rem GAME-11 milestone 1's tests: the age switch (Inventory_SwapAgeEquipment, linkAgeOnLoad),
rem Bg_Toki_Swd, Door_Toki, Player's time travel (the adult's pedestal, start mode 1, cutscene
rem modes 18 and 30), the adult's draw details, the two runs (Route::MasterSwordPull,
rem Route::MasterSwordBack), and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test master_sword --test demo_kankyo --test schemas %*
exit /b %errorlevel%
