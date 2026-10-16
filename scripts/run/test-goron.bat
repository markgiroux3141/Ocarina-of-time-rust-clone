@echo off
rem GAME-09 milestone 2's tests: the Gorons (En_Go2), Darunia (En_Du) with his door
rem (Bg_Spot18_Shutter) and statue (Bg_Spot18_Obj), the Tektites (En_Tite), the trail's boulder
rem (Bg_Spot16_Bombstone), walls (Bg_Bombwall) and flag (En_Hata), Goron City's urn
rem (Bg_Spot18_Basket, Bg_Spot18_Futa), Medigoron (En_Gm), the bronze boulders (Obj_Hamishi), the
rem hookshot post (Obj_Hsblock), the two runs (Route::Darunia, Route::Boulder), and the actor
rem schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test en_go2 --test en_du --test en_tite --test bg_spot16_bombstone --test bg_bombwall --test en_hata --test bg_spot18_basket --test en_gm --test obj_hamishi --test obj_hsblock --test goron_city --test schemas %*
exit /b %errorlevel%
