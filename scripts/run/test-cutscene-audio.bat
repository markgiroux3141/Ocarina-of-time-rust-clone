@echo off
rem GAME-04b milestone 3's tests: cutscene audio. The Deku Tree's talk plays its scripts' music
rem commands on their frames (the fade, his theme, the stop, the next), Link groans, sighs and
rem slips off his bed on the wake-up's frames, and an attention cutscene on Link puts him in mode
rem 69 through Player's cutscene mode tables.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test cutscene -- music groans %* || exit /b 1
cargo test --release -p oot_actors --test onepoint an_attention_cutscene_on_link_himself %*
exit /b %errorlevel%
