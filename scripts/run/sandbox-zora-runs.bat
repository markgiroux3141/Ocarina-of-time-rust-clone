@echo off
rem GAME-10 milestones 2 and 3's runs, headless: traces and screenshots into out\run.
rem zoras-river: Zelda's Lullaby at Zora's River's falls, the jump into the Domain and its intro.
rem king-zora: Ruto's letter shown to King Zora, his mweep aside. jabu-fish: a fish let out by
rem Lord Jabu-Jabu, his inhaling cutscene, inside.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_ZORAS_RIVER_2 --child --preset fairy-magic --script zoras-river ^
    --trace out\run\zoras_river.json --screenshot out\run\zoras_river.png --shots-at 80,320,406,500 || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_ZORAS_DOMAIN_1 --child --preset zora-letter --script king-zora ^
    --trace out\run\king_zora.json --screenshot out\run\king_zora.png --shots-at 600,800,1000,1250 || exit /b 1
"%BIN%\oot_sandbox.exe" --entrance ENTR_ZORAS_FOUNTAIN_2 --child --preset jabu-fish --script jabu-fish ^
    --trace out\run\jabu_fish.json --screenshot out\run\jabu_fish.png --shots-at 230,400,500,700 %*
exit /b %errorlevel%
