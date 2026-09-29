# Backlog

Bugs and gaps found by playing, not yet scheduled into a milestone. Each entry says what was
seen, the likely cause, and where it belongs. Move an entry into a milestone's scope when it's
picked up, and strike it here when it's fixed.

| # | Found | What | Likely cause | Where it belongs |
|---|---|---|---|---|
| 1 | 2026-09-28, GAME-03 m3, by hand | In the Kokiri shop, flat-coloured, untextured shapes (yellow, and a green-topped pot) at the window's left and right edges | The shop is a prerendered room. Its picture covers the 4:3 middle only; at 16:9 the room's own geometry shows beyond it, where the game never drew anything. The untextured look needs checking (the materials of those lists) | A small fix: mask or clamp the view to 4:3 in prerendered rooms, or investigate the lists (ADR 0014) |
| 2 | 2026-09-28, GAME-03 m3, by hand | Going down the ladder from Link's house, the camera is too close to Link | Player asks for `CAM_MODE_CLIMB`; that mode's camera function isn't ported, so it runs `Camera_Normal1` on the NORMAL data (ADR 0013's fallback) | The camera modes still on the fallback (`Jump*`, `Battle1`, ...): a camera milestone, or Phase 6 with `Battle1` |
| 3 | 2026-09-28, GAME-03 m3, by hand | Climbing out of the crawlspace, the camera pitches up, then jitters for about half a second | The crawl's one-point cutscenes on the way out (9601, 9602) aren't ported, so the next floor's bg camera takes over abruptly from `Camera_Subj4` (GAME-03 m2's known gap) | GAME-03 m4 (cutscenes), with `OnePointCutscene_Init` |
| 4 | 2026-09-28, GAME-03 m3 | Known omissions, confirmed: no guarding with the shield (R), no lifting or throwing, no pause menu (Start is the equipping stand-in) | Not ported yet: Player's guard and lift actions; `z_kaleido_scope` | Guarding and lifting: Phase 6 (the Deku Tree's enemies and pots). The pause menu: Phase 6's minimal pause menu (ROADMAP) |
