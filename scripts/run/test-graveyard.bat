@echo off
rem GAME-07 milestone 6's tests: Player's grab, the ReDeads (their scream, the grab, the Sun's Song's
rem stun), Dampe and his dig, the graveyard kid, the soft soil's Gold Skulltula, the royal tomb's
rem gravestone and the opening's lightning, light and spark, the blast (EffectSsBomb2), the weather
rem tag's thunderstorm, the shield grave's wall, the exit's run (Route::Graveyard), and the graves'
rem and the tomb's draw configs against the C interpreter.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test redead --test dampe --test graveyard_kid --test royal_tomb --test graves --test graveyard %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_import --test pack ported_draw_configs %*
exit /b %errorlevel%
