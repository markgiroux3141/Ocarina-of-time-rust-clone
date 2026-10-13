@echo off
rem GAME-07 milestone 5's tests: Player's lift, carry and throw (a cucco lifted, carried and thrown, its
rem glide slowing Link's fall; a small rock lifted and broken where it lands; a bush only tugged at
rem without strength, pulled up with the bracelet), Anju and her pen's count, the attack cuccos, the
rem carpenters, their boss and the son, the man on the roof, the windmill, the well, the gallery, the
rem cow, the windmill man and the well's draining, the House of Skulltula's family, the exit's run
rem (Route::Kakariko), and Kakariko's, the grottos' and the guard house's draw configs.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test kakariko --test anju --test carpenters --test kakariko_village --test windmill --test skulltula_house %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_game --test scene %*
if errorlevel 1 exit /b %errorlevel%
cargo test --release -p oot_import --test pack ported_draw_configs %*
exit /b %errorlevel%
