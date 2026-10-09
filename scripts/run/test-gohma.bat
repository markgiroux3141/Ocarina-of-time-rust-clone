@echo off
rem GAME-05 milestone 6a's tests: Queen Gohma (Boss_Goma) against the C: her init, her intro (the
rem zoom, the slab, the second try), her eye's hits (seeds, nuts, swords, on the ceiling), her eggs,
rem her death (the pieces, her textures erased, the heart and the warp), Item_B_Heart, the jump
rem slash, her bakes; the exit's run (Route::Gohma); then 3b's larvae, the slab's and the title
rem card's tests and the object RAM's.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test gohma --test gohma_run --test gohma_larvae --test doors %* || exit /b 1
cargo test --release -p oot_game --lib title_card object_ctx
exit /b %errorlevel%
