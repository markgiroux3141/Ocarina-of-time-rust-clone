@echo off
rem GAME-05 milestone 2's tests: damage and health. Each of Link's hit responses with its timers
rem (the stagger, the knockdown, frozen, electrified, burning, the swimming hit), the red flash,
rem death through the game over menu's stand-in to the respawn, a bottled fairy's revival, the
rem Deku Baba's damage tables, Health_ChangeBy, the damage table's lookup, the colour filter's
rem fog, and the exit's runs (the dummy hits Link; a Deku Baba bites him, then its stem is cut).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test damage %* || exit /b 1
cargo test --release -p oot_game --test damage %* || exit /b 1
cargo test --release -p oot_game --test collision_check %*
exit /b %errorlevel%
