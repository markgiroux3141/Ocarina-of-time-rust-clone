@echo off
rem Barinade's room as a child after GAME-10 milestone 5 (jabu-barinade). Empty: Link falls into
rem the room: Barinade's intro (the door shut, the Bari called down, the title card), then the fight
rem (the hand test; put the boomerang on a C button from the item page first, or use "fight").
rem "fight": the room re-entered with his battle begun (barinade-fight: the short intro, the Bari
rem called at once; the boomerang on C-Down): the supports, the zappers, the Bari, the stunned body.
rem "beaten": the room after his defeat (barinade-beaten: Ruto rises through her blue warp, the heart
rem container beside her; take the heart, step into her warp, her words, the warp out to the
rem fountain's cutscene). "fountain": the Zora's Sapphire's cutscene at Zora's Fountain
rem (zora-sapphire, cutscene 0xFFF0). "sapphire": Zora's Fountain with the Sapphire (zora-sapphire:
rem the three stones, Phase 11's exit part 1, milestone 7's start).
rem WASD the stick (Shift walks), Space A, E B, Q Z, R R, J C-Left, K C-Down, L C-Right, I C-Up,
rem Enter Start.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
set "args=--entrance ENTR_JABU_JABU_BOSS_0 --preset jabu-barinade"
if /i "%~1"=="fight" (
    set "args=--entrance ENTR_JABU_JABU_BOSS_0 --preset barinade-fight"
    shift
) else if /i "%~1"=="beaten" (
    set "args=--entrance ENTR_JABU_JABU_BOSS_0 --preset barinade-beaten"
    shift
) else if /i "%~1"=="fountain" (
    set "args=--entrance ENTR_ZORAS_FOUNTAIN_0 --cutscene 0xFFF0 --preset zora-sapphire"
    shift
) else if /i "%~1"=="sapphire" (
    set "args=--entrance ENTR_ZORAS_FOUNTAIN_0 --preset zora-sapphire"
    shift
)
"%BIN%\oot.exe" %args% %1 %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
