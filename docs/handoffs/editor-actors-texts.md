# Handoff: the editor's actors' texts (what an actor says)

Paste the prompt below into a fresh session (from the repo root). Written 2026-10-19, after the
actors' models (ADR 0078, docs/EDITOR-ACTORS.md "Built (2026-10-19)").

---

## Prompt

We're continuing the game editor's actors tab (stage E3) with its texts. Read these first:
`docs/EDITOR-ACTORS.md` (the whole of it, especially "Decisions", 3, and "Texts: what an actor
says"), `docs/adr/0077-the-actors-tab.md`, `docs/adr/0078-actor-models-captured-from-the-game.md`,
`docs/adr/0066-the-actor-schema.md` (with its amendment), `docs/EDITOR-PLAY-IN-THE-GAME.md` (a level
is a scene of its own), `crates/tools/overworld_editor/README.md`, and `docs/ROADMAP.md`'s "The game
editor track" and "How a phase starts and how a milestone is done".

**The goal:** every line an actor can say is listed in its inspector, shown as the game's text box
would show it, and editable; an edit is stored with the level and is what the game shows when the
level is played (the real scenes keep their own text). This is the "low-hanging customisation"
the user chose (Decisions, 3): no change to any actor's behaviour, only the text behind the ids its
code already uses.

**The four parts** (adjust after a short survey, and tell the user what changed):
1. *The lines in the schema*: a `texts` list in `oot_game::actor_schema::ActorSchema`, as `objects`
   is (`ObjectNeed`): each entry the text id, `when` (the field values or the condition in words:
   "none of the cuccos in the pen", "after the Kokiri Sword"), and a note. Fill it for every ported
   actor (helpers by group, as ADR 0066's pass did, each id checked against the decomp's C), and
   extend `oot_actors/tests/schemas.rs`: every text id literal a module passes to the message calls
   (`start_textbox`, `continue_textbox`, `text_id = ...`, `Actor_OfferTalk`'s, the NPC helpers') is in
   its schema's `texts`, or carries a `// schema:` note (ids computed at run time, tables). Actors
   whose params hold a text id (signs, Gossip Stones, walk-in texts) already have a `TextId` field:
   the inspector uses that.
2. *The encoder*: the game decodes messages (`crates/game/oot_game/src/message.rs`, the control
   codes: colours, pauses, choices, icons, the player's name, box breaks); the tools have a port of
   the decomp's `msgdis.py` (`crates/tools/oot_extract/src/text.rs`). Add the other way: a readable
   form (the decomp's own `message_data_static` macros, or a simpler one the user approves) back to
   the game's bytes, with a round-trip test over the whole English message table
   (`GamePack::messages()`: decode then encode gives the same bytes for every id). Put it where both
   the editor and the importer can use it (a module of `oot_game`, no ROM access).
3. *The level's overrides*: the document stores the level's edited texts (`overworld::doc`, by id,
   with room for new ids above the game's last, for custom actors later); the export writes them into
   `level.json`; `oot_import::level::write_scene` writes them into the level's records (layered over
   the pack, as its rooms are); the game's message lookup (`MessageContext::find_message`, which
   reads `MessageTable`) takes the level's text for that id first while the level's scene is the one
   being played. The real Kakariko keeps its own.
4. *The inspector's Dialogue section* (`crates/tools/overworld_editor/src/app/actor_ui.rs`): the
   actor's lines in the order and under the conditions its schema lists, each drawn as the game's
   text box draws it (the box type, colours, choices, icons; the game's own sprites and font, as the
   message box renders them, if that's simplest and faithful), with an edit field in the readable
   form, a "back to the game's" button, and a warning where a line is too long for its box.

**Where things are:** the editor's actors (`actors.rs`, `app/actor_ui.rs`, `captures.rs`); the
document's actors (`crates/tools/overworld/src/doc.rs`) and the export (`overworld::export`); the
level's records (`oot_import::level::write_scene`, `add_scene`; the game's `oot::enter_level`); the
message box (`oot_game::message`: `MessageTable`, `find_message`, `start_textbox`, the draw); the
schemas (`oot_game::actor_schema`, each actor's `pub const SCHEMA`). Run anything that reads the pack
with `OOT_DATA_DIR=<repo>\out\data26` (the scripts' data folder); the editor with
`scripts\run\editor.bat` (menu 147).

**How to work** (the user's standing preferences, from the project's memory):
- Follow the original: the text box drawn as the game draws it, the encoder the decoder's inverse;
  don't engineer what the game doesn't have where a simple constraint does.
- Never script long routes or fights for testing; give a hand-test list (keys and places) at the end
  instead. No side-by-side checklists against an emulator.
- The real game's look wins over a reading of the code.
- Milestone checks: `cargo test --release --workspace` (with `OOT_DATA_DIR` as above) and
  `cargo test -p layering` pass, the release build has no warnings; docs: EDITOR-ACTORS.md "Built"
  and "Next", an ADR for real decisions (next free: ADR 0079), BACKLOG rows for what's left (next
  free: #94), ROADMAP's E3 row; a script in `scripts\run` if it helps (next free menu: 149).
- Commit and push only when the user asks.

**Hand back:** what was built, how many actors have their lines listed (and which ids couldn't be
tied to an actor), the round-trip test's result, screenshots of the Dialogue section, the checks run,
and a hand-test list (an edited line seen in the game when the level is played).
