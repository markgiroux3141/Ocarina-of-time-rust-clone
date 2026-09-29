@echo off
rem GAME-03 milestone 2's tests: the crawl and the crawlspace's camera, the boulder and Link's
rem knockdown, the wonder items, and the exit test, the scripted run from Link's bed on a new save
rem to the Kokiri Sword's chest.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test crawl --test boulder --test wonder_item %* || exit /b 1
cargo test --release -p oot_actors --test playthrough a_new_save_to_the_kokiri_sword %*
exit /b %errorlevel%
