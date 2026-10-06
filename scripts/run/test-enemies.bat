@echo off
rem GAME-05 milestone 3b's tests: the Deku Scrubs (the Mad Scrub's states, its nut bounced back,
rem stunned and set alight; the hint scrubs' order puzzle and talk; the Business Scrub and the
rem salesman's sale), the Skulltula (on its thread, its front and back, its bounces and death,
rem touching Link), the Skullwalltula and the Gold Skulltula with its token, Gohma's eggs and
rem larvae, and the exit's run (Route::Scrub).
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test scrubs --test skulltula_st --test skulltulas --test gohma_larvae --test scrub_run %*
exit /b %errorlevel%
