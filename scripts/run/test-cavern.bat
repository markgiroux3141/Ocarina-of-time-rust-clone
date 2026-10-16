@echo off
rem GAME-09 milestone 3's tests: the skull (Bg_Dodoago: the eyes, the jaw, the bomb catcher), the
rem pillars (Bg_Ddan_Jd), the Baby Dodongos (En_Dodojr), the Lizalfos (En_Zf), the Poes (En_Poh),
rem the Armos (En_Am), the elevator (Obj_Elevator), the fire wall (Bg_Hidan_Curtain), the
rem dark-maze blocks (Bg_Gnd_Darkmeiro), one-points 3060, 3065 and 3380, the sword's and the
rem Skulltula's trails (EffectBlure), the four runs (Route::Pillar, BombBag, Skull,
rem Gravestone), and the actor schemas.
setlocal
call "%~dp0_env.bat"
cargo test --release -p oot_actors --test bg_dodoago --test bg_ddan_jd --test en_dodojr --test en_zf --test en_poh --test en_am --test obj_elevator --test bg_hidan_curtain --test bg_gnd_darkmeiro --test onepoint --test sword --test skulltula_st --test cavern --test schemas %*
exit /b %errorlevel%
