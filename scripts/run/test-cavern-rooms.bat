@echo off
rem GAME-09 milestone 5's tests: the falling stairs (Bg_Ddan_Kd, one-point 3050), the silver rupees
rem and the gallery's rupees and archery pots (En_G_Switch), the Dodongo (En_Dodongo,
rem Effect_Ss_D_Fire), the Beamos (En_Vm, Effect_Ss_Dead_Dd, Effect_Ss_Dead_Ds), the four runs
rem (Route::FallingStairs, SilverRupees, ShieldChest, TimeBlocks), the six rooms' placements, and the
rem actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test cavern_rooms --test bg_ddan_kd --test en_g_switch --test en_dodongo --test en_vm --test schemas %*
exit /b %errorlevel%
