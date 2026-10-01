@echo off
rem GAME-04's exit test (Phase 5): scripted runs' sound effect requests against the calls in the
rem C, frame by frame, with the audio offline. The Mido and shop run: the Kokiri Sword's chest
rem (the lid on its animation's frames 30 and 90, the light's flash, the chest's and the item's
rem fanfares), Mido's four chests (their rupees' sound), every rupee taken and counted, the
rem wonder items' drops, Navi going into Link's hat. Doors (a Kakariko house's exit, a room
rem door): open and close on their frames, and the entrance's sound on the far side. The Deku
rem Tree run: each bush cut, through a sound source at the bush.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test sfx_route %*
exit /b %errorlevel%
