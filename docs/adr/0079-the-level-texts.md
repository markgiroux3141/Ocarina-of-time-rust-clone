# 0079: What an actor says: its lines in the schema, a readable form of the game's messages, and a level's texts over the game's

- **Status:** accepted, built (2026-10-19)
- **Date:** 2026-10-19
- **Builds on:** [ADR 0066](0066-the-actor-schema.md) (the actor schema), [ADR 0077](0077-the-actors-tab.md)
  (the actors tab), [ADR 0078](0078-actor-models-captured-from-the-game.md) (the game's renderer in the
  editor), docs/EDITOR-PLAY-IN-THE-GAME.md (a level is a scene of its own), docs/EDITOR-ACTORS.md
  ("Decisions", 3).

## Context

The user's "low-hanging customisation" (EDITOR-ACTORS.md, Decisions 3): what needs no change to an
actor's behaviour. After its settings and its objects, its texts: every line an actor can say listed
in its inspector, shown as the game's text box shows it, and editable, the edit stored with the level
and played there, the game's own scenes keeping theirs.

The game had a decoder (the message box, `oot_game::message`, a port of `z_message.c`) and the tools a
port of the decomp's `msgdis.py` (`oot_extract::text`, lossy: it drops the end code and the padding).
Nothing listed which text ids an actor's code uses: they're literals in its text functions, its
`textId` assignments, tables (`MISSING_CUCCO_TEXT_IDS`) and named consts (`TEXT_POE_SOUL`).

## Decision

1. **The lines are the schema's** (`ActorSchema::texts`, a list of `TextLine`, as `objects` is of
   `ObjectNeed`): the id, `when` (field values), `cond` (the condition in words: "Link a child, no cuccos
   in the pen"), a note, in the order the code uses them. A text the params hold (a sign's, a Gossip
   Stone's, Navi's spots', a door's) is one line `TextLine::field(0x300, "params")`: the base the C ors
   or adds the field into (the schemas named those fields as ranges, not `Kind::TextId`, so the line
   carries the base). The ids are the ones a custom level plays; lines only some scenes show say so in
   `cond` ("only in Kakariko"), and the inspector says they don't show in a custom level. Texts a
   message goes on to by itself (`{TEXTID}`) aren't listed: the editor follows them.
2. **The schema test checks them** (`text_ids_are_the_schemas_lines`): every hex literal that is a
   message id of the game's, on a line that shows or sets one (`start_textbox`, `continue_textbox`,
   `text_id =`, `==`, Navi's `set_navi_text_id`) or in a block about them (a function with `text` in
   its name, a `match` or `if` on a `text_id`, a `const` or table named `*TEXT*`), is one of the
   schema's lines or carries a `// schema:` note; every line's id is a message of the game's and in
   the module's source; `when` and `plus` name fields. Filled for every ported actor by six helpers by
   group, each id checked against the decomp's C: 54 actors, 701 lines.
3. **The readable form** (`oot_game::message_text`, no ROM access, for the editor and the importer):
   the decomp's charmap (`charmap.nes.txt`: `[A]`, `[C-Up]`, `▼`, the accents) with the control codes as
   the brace tags our `msgdis.py` port prints (`{COLOR:RED}`, `{TEXTID:0x1036}`, `{NAME}`,
   `{BOX_BREAK}`), a newline a newline, any other byte `{0xNN}`. Not the decomp's C macros: the editor
   edits one text in one field, and the tags read as the tools already show messages. `encode` is the
   decoder's inverse: the end code and zero padding to four bytes as the table lays them out. Every one
   of the 2115 English messages round-trips byte for byte, none needing an escaped byte.
4. **The measure for "too long"** (`measure`, `layout_warnings`) advances as `Message_DrawText` does
   (the font's widths at scale 75, spaces, `{SHIFT}`, item icons, the choice and icon indents, trailing
   spaces drawing nothing): a box shows 4 lines, and a line may reach the box's right edge (x 290 from
   the text's 65): the game's own widest line is 223 of the 225, and no game message but 0xFFFC (the
   name entry's character list, never a box) trips it. A test holds the measure to the game's data.
5. **A level's texts are a scene's** (the user's recommended scope): the document keeps them by id
   (`overworld::doc::Text`: the readable text, the box type and position), with room for new ids from
   0x8000 (`FIRST_NEW_TEXT_ID`; the game's last is below) for custom actors later. The export writes
   them into `level.json` as they are; `oot_import::level::write_scene` encodes them into the level's
   record `texts/<scene file>` (`SceneTexts`), layered over the pack as its rooms are (one that won't
   encode is left out with a warning, and the game's plays). `Play_Init` plays a scene with
   `GameAssets::messages_for(scene file)`: the game's table, with the scene's texts over it
   (`MessageTable::layered`: the messages laid out again in order, so `Message_FindMessagePAL`'s length
   to the next entry stays each one's own; new ids before 0xFFFC). The game's scenes have no such
   record, so the real Kakariko keeps its text whatever a level changes. A project-wide table (the mod
   pack, E9) can come with sharing.
6. **The text box in the inspector is the game's**: `message_text::preview` runs the real
   `MessageContext` headless on the line (`Message_StartTextbox` with no actor, a frame at a time, A at
   each box's wait and `{TEXTID}`), and takes each typed-out box's sprites; the editor draws them with
   the game's renderer from the pack's bakes (the box's image, the font, the icons) over the 320x240
   screen at twice its size, cropped to the box (`text_box.rs`). The box type, the colours as the box
   type has them, the player's name ("Link"), the icons and the choices are the game's.
7. **The Dialogue section** (`app/dialogue_ui.rs`): the lines for the actor's params, each with its
   id, condition and note, its boxes and the texts it goes on to, and per text an editor: the box type
   and position, the text unwrapped in the readable form, why it won't encode, a "too long" warning per
   line, and "Back to the game's". An edit equal to the game's is no edit.

## Consequences

- Every actor ported from now on lists its lines; the schema test fails on a text id it doesn't (or a
  `// schema:` note says why). "How a milestone is done" (ROADMAP.md) gains it through the test.
- The helpers found no params or ids where the port differs from the C, but parts the port leaves out
  show in the lists: the mask reactions `MaskReaction_GetTextId` gives (0 in the port) for Malon, Ingo,
  Anju and Talon's helpers, Navi's enemy texts (`naviEnemyId + 0x600`, skipped in Player though actors
  now set it), adult Ingo's and Malon's, the Happy Mask and Goron shops' (BACKLOG #94 to #96).
- Lines shown by cutscene scripts (the Deku Tree's, the song teachings') aren't an actor's: they come
  with the cutscene editor (E7).
- The preview shows a variable box at the bottom, as with no actor near the top of the screen, and the
  name as "Link"; the measure counts the name as four characters.
