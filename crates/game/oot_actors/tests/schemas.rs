//! The actor schemas (docs/adr/0066-the-actor-schema.md) against the code and the game's data:
//!
//! - every ported actor has one, well formed;
//! - its module reads its params only through the schema's fields (`FIELD.get(...params)`), and
//!   reads every field it declares (but the debug, unused and rotation ones);
//! - the flag helpers its module calls (`flags.get_switch`, `save.get_event_chk_inf(EVENTCHKINF_X)`,
//!   ...) are the schema's flags, in the same direction, and the reverse;
//! - every placement of it in the game's scenes (rooms, transition actors, Player's spawns) decodes:
//!   named fields take named values, ranges hold, no bit is set outside the fields, no rotation
//!   component is set that the schema doesn't name (all but the yaw).
//!
//! A line the source checks should pass over (a write the C does, a whole params handed on, a
//! computed flag) carries a `// schema: why` comment, on it or on the line above.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use oot_game::actor_ctx::ActorProfile;
use oot_game::actor_schema::{ActorSchema, Field, Flag, Kind, Source};

fn module_path(p: &ActorProfile) -> PathBuf {
    let name = p.name.to_ascii_lowercase();
    let name = name.strip_prefix("sandbox_").unwrap_or(&name);
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(format!("{name}.rs"))
}

/// A module's code: comments cut (each line keeps whether it or the line above has a `schema:`
/// note), everything from `#[cfg(test)]` on dropped.
struct Code {
    lines: Vec<(usize, String, bool)>,
}

fn strip_comment(line: &str) -> (&str, Option<&str>) {
    let b = line.as_bytes();
    let mut in_str = false;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'\'' if !in_str && i + 2 < b.len() && b[i + 2] == b'\'' => i += 2,
            b'/' if !in_str && b.get(i + 1) == Some(&b'/') => return (&line[..i], Some(&line[i..])),
            _ => {}
        }
        i += 1;
    }
    (line, None)
}

fn read_code(p: &ActorProfile) -> Code {
    let src = std::fs::read_to_string(module_path(p)).unwrap_or_else(|e| panic!("{}: {e}", module_path(p).display()));
    let mut lines = Vec::new();
    let mut prev_note = false;
    for (i, line) in src.lines().enumerate() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            break;
        }
        let (code, comment) = strip_comment(line);
        let note = comment.is_some_and(|c| c.contains("schema:"));
        let code_empty = code.trim().is_empty();
        lines.push((i + 1, code.to_string(), note || prev_note));
        // A note on a line of its own (and the comment lines after it) covers the next code line.
        prev_note = (note || prev_note) && code_empty;
    }
    Code { lines }
}

impl Code {
    fn text(&self) -> String {
        self.lines.iter().map(|(_, l, _)| l.as_str()).collect::<Vec<_>>().join("\n")
    }
}

/// `const IDENT: Field` declarations, and the idents listed in `SCHEMA`'s `fields: &[...]`.
fn field_consts(code: &Code) -> (Vec<String>, Option<Vec<String>>) {
    let text = code.text();
    let decl = regex::Regex::new(r"const\s+([A-Z][A-Z0-9_]*)\s*:\s*Field\b").unwrap();
    let declared = decl.captures_iter(&text).map(|c| c[1].to_string()).collect();
    let schema = regex::Regex::new(r"(?s)const\s+SCHEMA\s*:\s*ActorSchema\s*=\s*ActorSchema\s*\{.*?fields\s*:\s*&\[(.*?)\]").unwrap();
    let listed = schema.captures(&text).map(|c| c[1].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());
    (declared, listed)
}

fn ported() -> impl Iterator<Item = &'static ActorProfile> {
    oot_actors::PROFILES.iter()
}

#[test]
fn every_actor_has_a_schema() {
    let todo: Vec<&str> = ported().filter(|p| p.schema.todo).map(|p| p.name).collect();
    assert!(todo.is_empty(), "{} actors without a schema (BACKLOG #52): {}", todo.len(), todo.join(", "));
}

fn fits(f: &Field, v: i32) -> bool {
    f.value(f.put(0, v)) == v
}

#[test]
fn schemas_are_well_formed() {
    let mut errors = Vec::new();
    for p in ported().filter(|p| !p.schema.todo) {
        let s = p.schema;
        let mut names = BTreeSet::new();
        for f in s.fields {
            let e = |m: String| format!("{}: field `{}`: {m}", p.name, f.name);
            if !names.insert(f.name) {
                errors.push(e("named twice".into()));
            }
            if f.bits == 0 || f.shift as u32 + f.bits as u32 > 16 {
                errors.push(e(format!("bits {}..{} outside 16", f.shift, f.shift + f.bits)));
            }
            if f.meaning.is_empty() {
                errors.push(e("no meaning".into()));
            }
            match f.kind {
                Kind::Named(list) => {
                    for (v, n) in list {
                        if !fits(f, *v) {
                            errors.push(e(format!("value {v} ({n}) doesn't fit")));
                        }
                    }
                }
                Kind::Range(lo, hi) => {
                    if lo > hi || !fits(f, lo) || !fits(f, hi) {
                        errors.push(e(format!("range {lo}..={hi} doesn't fit")));
                    }
                }
                _ => {}
            }
            if let Some(parent) = f.part_of {
                match s.field(parent) {
                    Some(g) if g.source == f.source && g.mask() & f.mask() == f.mask() && g.part_of.is_none() => {}
                    _ => errors.push(e(format!("`part_of` `{parent}`: no such whole field over its bits"))),
                }
            }
            if let Some(w) = f.when {
                match s.field(w.field) {
                    None => errors.push(e(format!("`when` names no field `{}`", w.field))),
                    Some(g) => {
                        for v in w.values {
                            if !fits(g, *v) {
                                errors.push(e(format!("`when` value {v} doesn't fit `{}`", g.name)));
                            }
                        }
                    }
                }
            }
        }
        for u in s.flags {
            if !(u.reads || u.sets || u.clears) {
                errors.push(format!("{}: flag {:?} neither read, set nor cleared", p.name, u.flag));
            }
            let (named, kinds): (&str, &[Kind]) = match u.flag {
                Flag::Switch(n) => (n, &[Kind::SwitchFlag, Kind::TempFlag]),
                Flag::Chest(n) => (n, &[Kind::ChestFlag]),
                Flag::Collectible(n) => (n, &[Kind::CollectibleFlag]),
                _ => continue,
            };
            let number = named.strip_prefix("0x").map_or_else(|| named.parse::<i32>().is_ok(), |h| i32::from_str_radix(h, 16).is_ok());
            match s.field(named) {
                Some(f) if !kinds.contains(&f.kind) => errors.push(format!("{}: flag {:?}'s field is a {:?}", p.name, u.flag, f.kind)),
                Some(_) => {}
                None if number => {}
                None => {
                    // A computed one names its field ("switchFlag + 1").
                    let words: Vec<&str> = named.split(|c: char| !(c.is_alphanumeric() || c == '_')).collect();
                    if !s.fields.iter().any(|f| words.contains(&f.name)) {
                        errors.push(format!("{}: flag {:?} names no field and no number", p.name, u.flag));
                    }
                }
            }
        }
    }
    assert!(errors.is_empty(), "{} problems:\n{}", errors.len(), errors.join("\n"));
}

#[test]
fn code_reads_params_through_the_schema() {
    let through = regex::Regex::new(r"(?:[a-z_][a-z0-9_]*::)*[A-Z][A-Z0-9_]*\.(?:get|value)\([^;{}]*?\.params\s*\)").unwrap();
    let write = regex::Regex::new(r"\.params\s*(?:[-+*/%&|^]|<<|>>)?=[^=]").unwrap();
    let read = regex::Regex::new(r"\.params\b").unwrap();
    // Not an actor's params: a draw command's (`cmd.params.segments`), a method (`self.params()`).
    let other = regex::Regex::new(r"\.params\s*[.(]").unwrap();
    let mut errors = Vec::new();
    for p in ported().filter(|p| !p.schema.todo) {
        let code = read_code(p);
        for (n, line, note) in &code.lines {
            if *note {
                continue;
            }
            let rest = through.replace_all(line, "");
            let rest = write.replace_all(&rest, "");
            let rest = other.replace_all(&rest, "");
            if read.is_match(&rest) {
                errors.push(format!("{}:{n}: {}", module_path(p).file_name().unwrap().to_string_lossy(), line.trim()));
            }
        }
        // The fields are the module's consts, all in the schema, each read.
        let (declared, listed) = field_consts(&code);
        let Some(listed) = listed else {
            if !p.schema.fields.is_empty() {
                errors.push(format!("{}: no `const SCHEMA: ActorSchema = ActorSchema {{ fields: &[...] }}` found", p.name));
            }
            continue;
        };
        let (d, l): (BTreeSet<_>, BTreeSet<_>) = (declared.iter().collect(), listed.iter().collect());
        if d != l || listed.len() != p.schema.fields.len() {
            errors.push(format!("{}: Field consts {declared:?} vs the schema's fields {listed:?}", p.name));
            continue;
        }
        let text = code.text();
        for (ident, f) in listed.iter().zip(p.schema.fields) {
            // A part is read through its whole field.
            if matches!(f.kind, Kind::Debug | Kind::Unused | Kind::Rotation) || f.part_of.is_some() {
                continue;
            }
            if !text.contains(&format!("{ident}.get(")) && !text.contains(&format!("{ident}.value(")) {
                errors.push(format!("{}: field `{}` ({ident}) is never read", p.name, f.name));
            }
        }
    }
    assert!(errors.is_empty(), "{} params reads outside the schema:\n{}", errors.len(), errors.join("\n"));
}

/// The kind of flag a call or a `Flag` is, for matching them.
fn flag_kind(f: &Flag) -> (&'static str, Option<&'static str>) {
    match f {
        Flag::Switch(_) => ("switch", None),
        Flag::Chest(_) => ("treasure", None),
        Flag::Collectible(_) => ("collectible", None),
        Flag::Clear => ("clear", None),
        Flag::TempClear => ("temp_clear", None),
        Flag::EventChkInf(n, _) => ("event_chk_inf", Some(n)),
        Flag::InfTable(n, _) => ("inf_table", Some(n)),
        Flag::ItemGetInf(n, _) => ("item_get_inf", Some(n)),
        Flag::EventInf(n, _) => ("event_inf", Some(n)),
        Flag::Other(_) => ("other", None),
    }
}

#[test]
fn flag_calls_match_the_schema() {
    let scene = regex::Regex::new(r"flags\.(get|set|unset)_(switch|treasure|collectible|clear|temp_clear)\(").unwrap();
    let save = regex::Regex::new(r"\b(get|set|clear)_(event_chk_inf|inf_table|item_get_inf|event_inf)\(\s*([^)]*?)\s*\)").unwrap();
    let raw = regex::Regex::new(r"\.(event_chk_inf|inf_table|item_get_inf|event_inf)\[").unwrap();
    let ident = regex::Regex::new(r"^[A-Z][A-Z0-9_]*$").unwrap();
    let mut errors = Vec::new();
    for p in ported().filter(|p| !p.schema.todo) {
        let code = read_code(p);
        // (kind, name or "", access) found in the code.
        let mut found: BTreeSet<(String, String, &str)> = BTreeSet::new();
        for (n, line, note) in &code.lines {
            for c in scene.captures_iter(line) {
                let access = if &c[1] == "get" { "reads" } else if &c[1] == "set" { "sets" } else { "clears" };
                found.insert((c[2].to_string(), String::new(), access));
            }
            for c in save.captures_iter(line) {
                let access = if &c[1] == "get" { "reads" } else if &c[1] == "set" { "sets" } else { "clears" };
                if ident.is_match(&c[3]) {
                    found.insert((c[2].to_string(), c[3].to_string(), access));
                } else if !note {
                    errors.push(format!("{}:{n}: a computed flag without a `schema:` note: {}", p.name, line.trim()));
                }
            }
            if raw.is_match(line) && !note {
                errors.push(format!("{}:{n}: the flags' array read without a `schema:` note: {}", p.name, line.trim()));
            }
        }
        let has = |kind: &str, name: &str, access: &str| {
            p.schema.flags.iter().any(|u| {
                let (k, nm) = flag_kind(&u.flag);
                k == kind
                    && nm.is_none_or(|nm| nm == name)
                    && match access {
                        "reads" => u.reads,
                        "sets" => u.sets,
                        _ => u.clears,
                    }
            })
        };
        for (kind, name, access) in &found {
            if !has(kind, name, access) {
                errors.push(format!("{}: the code {access} {kind} {name}, the schema doesn't say so", p.name));
            }
        }
        for u in p.schema.flags.iter().filter(|u| u.indirect.is_empty()) {
            let (kind, name) = flag_kind(&u.flag);
            if kind == "other" {
                continue;
            }
            for (on, access) in [(u.reads, "reads"), (u.sets, "sets"), (u.clears, "clears")] {
                if on && !found.iter().any(|(k, n, a)| k == kind && name.is_none_or(|nm| nm == n) && *a == access) {
                    errors.push(format!("{}: the schema says it {access} {:?}, no call does (or mark it `indirect`)", p.name, u.flag));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{} flag mismatches:\n{}", errors.len(), errors.join("\n"));
}

/// One placement of an actor in the game's data.
struct Placement {
    at: String,
    params: i16,
    rot: [i16; 3],
    paths: usize,
}

fn placements(a: &oot_game::play_scene::GameAssets) -> HashMap<String, Vec<Placement>> {
    let mut out: HashMap<String, Vec<Placement>> = HashMap::new();
    let mut rooms_seen = BTreeSet::new();
    for e in &a.scenes.scenes {
        let Ok(sd) = a.pack.scene(&e.file) else { continue };
        for (li, ld) in sd.layers.iter().enumerate() {
            let mut push = |id: i16, params: i16, rot: [i16; 3], at: String| {
                let name = a.actors.get(id).map(|i| i.name.clone()).unwrap_or_default();
                out.entry(name).or_default().push(Placement { at, params, rot, paths: ld.paths.len() });
            };
            for (i, s) in ld.spawns.iter().enumerate() {
                push(s.id, s.params, s.rot, format!("{} layer {li} spawn {i}", e.file));
            }
            for (i, t) in ld.transition_actors.iter().enumerate() {
                push(t.id, t.params, [0, t.rot_y, 0], format!("{} layer {li} transition actor {i}", e.file));
            }
            for key in &ld.rooms {
                if !rooms_seen.insert(key.clone()) {
                    continue;
                }
                let Ok(r) = a.pack.room(key) else { continue };
                for (i, x) in r.actors.iter().enumerate() {
                    push(x.id, x.params, x.rot, format!("{key} (layer {li}) actor {i}"));
                }
            }
        }
    }
    out
}

fn applies(s: &ActorSchema, f: &Field, params: i16, rot: [i16; 3]) -> bool {
    let Some(w) = f.when else { return true };
    let Some(g) = s.field(w.field) else { return false };
    if !applies(s, g, params, rot) {
        return false;
    }
    w.values.contains(&g.value(source_value(g.source, params, rot)))
}

fn source_value(src: Source, params: i16, rot: [i16; 3]) -> i16 {
    match src {
        Source::Params => params,
        Source::RotX => rot[0],
        Source::RotY => rot[1],
        Source::RotZ => rot[2],
    }
}

#[test]
fn placements_decode() {
    let Ok(pack) = oot_game::pack::GamePack::open_default() else {
        eprintln!("skipping: no pack");
        return;
    };
    let a = oot_actors::game_assets(pack).expect("the pack's tables");
    let all = placements(&a);
    let texts: BTreeSet<u16> = a.messages.entries.iter().map(|e| e.text_id).collect();
    let objects = a.scenes.objects.len() as i32;
    let mut errors: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut checked = 0;
    for p in ported().filter(|p| !p.schema.todo) {
        let s = p.schema;
        for pl in all.get(p.name).map(|v| v.as_slice()).unwrap_or_default() {
            checked += 1;
            let mut err = |m: String| errors.entry(p.name).or_default().push(format!("{}: params {:#06x} rot {:?}: {m}", pl.at, pl.params as u16, pl.rot));
            let mut covered = [0u16; 4];
            for f in s.fields.iter().filter(|f| applies(s, f, pl.params, pl.rot)) {
                let src = source_value(f.source, pl.params, pl.rot);
                covered[f.source as usize] |= f.mask();
                let v = f.value(src);
                let ok = match f.kind {
                    Kind::Named(list) => list.iter().any(|(n, _)| *n == v),
                    Kind::Range(lo, hi) => (lo..=hi).contains(&v),
                    Kind::ItemId => (0..oot_game::item::GI_MAX as i32).contains(&v),
                    Kind::TextId => texts.contains(&(v as u16)),
                    Kind::PathIndex => (v as usize) < pl.paths || v == (f.mask() >> f.shift) as i32,
                    Kind::Object => (0..objects).contains(&v),
                    _ => true,
                };
                if !ok {
                    let kind = match f.kind {
                        Kind::Named(_) => "named values".to_string(),
                        k => format!("{k:?}"),
                    };
                    err(format!("`{}` is {v} ({v:#x}), not one of its {kind}", f.name));
                }
            }
            let outside = pl.params as u16 & !covered[Source::Params as usize];
            if outside != 0 {
                err(format!("params bits {outside:#06x} outside the fields"));
            }
            for (src, i) in [(Source::RotX, 0), (Source::RotZ, 2)] {
                let outside = pl.rot[i] as u16 & !covered[src as usize];
                if outside != 0 {
                    err(format!("rot {} {:#06x} set and not in the schema", ["x", "y", "z"][i], outside));
                }
            }
            if s.fields.iter().any(|f| f.source == Source::RotY && f.kind != Kind::Rotation) {
                let outside = pl.rot[1] as u16 & !covered[Source::RotY as usize];
                if outside != 0 {
                    err(format!("rot y {outside:#06x} outside its fields"));
                }
            }
        }
    }
    assert!(checked > 0, "no placements checked");
    let report: Vec<String> = errors
        .iter()
        .map(|(n, e)| format!("{n}: {} placements\n  {}", e.len(), e.iter().take(6).cloned().collect::<Vec<_>>().join("\n  ")))
        .collect();
    assert!(errors.is_empty(), "{} actors' placements don't decode:\n{}", errors.len(), report.join("\n"));
}

/// Modules that find object slots themselves (`object_ctx.get_index`, `Object_GetSlot`) declare
/// the objects they need (`ActorSchema::objects`), unless the call carries a `// schema:` note
/// (its own profile's object, one only another actor's params name). Each need is well formed.
#[test]
fn object_lookups_are_declared() {
    let mut errors = Vec::new();
    for p in ported().filter(|p| !p.schema.todo) {
        let code = read_code(p);
        let lookups: Vec<usize> = code.lines.iter().filter(|(_, l, note)| l.contains("object_ctx.get_index(") && !note).map(|(n, _, _)| *n).collect();
        if !lookups.is_empty() && p.schema.objects.is_empty() {
            errors.push(format!("{}: finds object slots (lines {lookups:?}) but declares no `objects`", p.name));
        }
        for n in p.schema.objects {
            if n.object.is_empty() || !(n.object.starts_with("object_") || n.object.starts_with("gameplay_")) {
                errors.push(format!("{}: object need {:?} isn't an object file's name", p.name, n.object));
            }
            if n.object == p.object {
                errors.push(format!("{}: object need {:?} is its profile's", p.name, n.object));
            }
            if let Some(w) = n.when {
                match p.schema.field(w.field) {
                    None => errors.push(format!("{}: object need {:?}: `when` names no field `{}`", p.name, n.object, w.field)),
                    Some(g) => {
                        for v in w.values {
                            if !fits(g, *v) {
                                errors.push(format!("{}: object need {:?}: `when` value {v} doesn't fit `{}`", p.name, n.object, g.name));
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(errors.is_empty(), "{} problems:\n{}", errors.len(), errors.join("\n"));
}

/// Every room of the game's that places an actor loads the `objects` it needs for its params (in
/// the room's list, or the scene's keep, or `gameplay_keep`). Placements whose profile's object
/// isn't loaded are passed over: `Actor_Spawn` never spawns them (the game has a few: Mido in his
/// house, some cutscene layers' actors). The game's own oversights (an actor placed where an
/// object it needs isn't, which then kills itself or waits) are listed in `MISSING_IN_THE_GAME`.
#[test]
fn needed_objects_are_in_the_rooms_that_place_them() {
    /// (actor, scene file, object): placements the game makes without an object they need.
    const MISSING_IN_THE_GAME: &[(&str, &str, &str)] = &[
        // "Bank danger!": pots asking for object_tsubo where the room doesn't load it, gone at once
        ("Obj_Tsubo", "spot06_scene", "object_tsubo"),
        ("Obj_Tsubo", "spot15_scene", "object_tsubo"),
        // cutscene layers placing the bombable walls without object_bwall: killed at once
        ("Bg_Breakwall", "hakaana_ouke_scene", "object_bwall"),
        ("Bg_Breakwall", "spot18_scene", "object_bwall"),
        // type-1 bushes where the room doesn't load object_kusa: killed at once
        ("En_Kusa", "ddan_scene", "object_kusa"),
        ("En_Kusa", "spot06_scene", "object_kusa"),
        ("En_Kusa", "testroom_scene", "object_kusa"),
        // Kokiri Forest's layers 8 and 9: a boy (0xD300) whose objects the room doesn't load
        ("En_Ko", "spot04_scene", "object_km1"),
        ("En_Ko", "spot04_scene", "object_os_anime"),
        // placed recovery hearts without object_gi_heart: collectable, never drawn
        ("En_Item00", "HAKAdanCH_scene", "object_gi_heart"),
        ("En_Item00", "ddan_scene", "object_gi_heart"),
    ];
    let Ok(pack) = oot_game::pack::GamePack::open_default() else {
        eprintln!("skipping: no pack");
        return;
    };
    let a = oot_actors::game_assets(pack).expect("the pack's tables");
    let names: HashMap<&str, &ActorProfile> = ported().map(|p| (p.name, p)).collect();
    let mut missing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut checked = 0;
    let mut seen = BTreeSet::new();
    for e in &a.scenes.scenes {
        let Ok(sd) = a.pack.scene(&e.file) else { continue };
        for ld in &sd.layers {
            for key in &ld.rooms {
                if !seen.insert((key.clone(), ld.keep_object_id)) {
                    continue;
                }
                let Ok(r) = a.pack.room(key) else { continue };
                let loaded = |o: &str| o == "gameplay_keep" || ld.keep_object.as_deref() == Some(o) || a.scenes.object_id(o).is_some_and(|id| r.objects.contains(&id));
                for x in &r.actors {
                    let name = a.actors.name(x.id);
                    let Some(p) = names.get(name.as_str()) else { continue };
                    if !loaded(p.object) {
                        continue;
                    }
                    checked += 1;
                    for o in p.schema.needed_objects(x.params, x.rot) {
                        if !loaded(o) && !MISSING_IN_THE_GAME.contains(&(p.name, e.file.as_str(), o)) {
                            missing.entry(format!("{} needs {o}", p.name)).or_default().insert(format!("{key} (params {:#06x})", x.params as u16));
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 0, "no placements checked");
    let report: Vec<String> = missing.iter().map(|(k, v)| format!("{k}: {} rooms: {}", v.len(), v.iter().cloned().collect::<Vec<_>>().join(", "))).collect();
    assert!(missing.is_empty(), "{} needs not met by the rooms placing them:\n{}", missing.len(), report.join("\n"));
}

/// Where a module's text ids are: the hex literals that are ids of the game's messages, on a line
/// that shows or sets one (`start_textbox`, `continue_textbox`, `text_id =`, `text_id ==`, Navi's
/// `set_navi_text_id`) or inside a block about them: a function with `text` in its name, a `match`
/// or `if` on a `text_id`, a `const` or table with `TEXT` in its name (`MISSING_CUCCO_TEXT_IDS`).
/// Each with its line and whether it carries a `schema:` note.
fn text_literals(code: &Code, ids: &BTreeSet<u16>) -> Vec<(usize, u16, bool)> {
    let call = regex::Regex::new(r"start_textbox\(|continue_textbox\(|text_id\s*(=|==|!=)|set_navi_text_id\(|text_id:").unwrap();
    let func = regex::Regex::new(r"fn\s+[a-z0-9_]*text[a-z0-9_]*\s*[<(]").unwrap();
    let block = regex::Regex::new(r"text_id\b.*\{\s*$|(const|static)\s+[A-Z0-9_]*TEXT[A-Z0-9_]*\s*:").unwrap();
    let hex = regex::Regex::new(r"\b0x([0-9A-Fa-f]{2,4})\b").unwrap();
    let mut out = vec![];
    // An open text block: the depth it began at, and whether its body has (the bracket that opens
    // it: a function's or a `match`'s `{`, a `const`'s first after its `=`). It closes with that
    // bracket; one whose body never opened closes with its line.
    struct Block {
        base: i32,
        body: bool,
        konst: bool,
    }
    let mut blocks: Vec<Block> = vec![];
    let mut depth = 0i32;
    for (n, line, note) in &code.lines {
        let is_const = !func.is_match(line) && (line.contains("const ") || line.contains("static ")) && block.is_match(line);
        let starts = func.is_match(line) || block.is_match(line);
        if !blocks.is_empty() || starts || call.is_match(line) {
            for c in hex.captures_iter(line) {
                let v = u16::from_str_radix(&c[1], 16).unwrap();
                if ids.contains(&v) {
                    out.push((*n, v, *note));
                }
            }
        }
        if starts {
            blocks.push(Block { base: depth, body: false, konst: is_const });
        }
        let mut after_eq = false;
        for ch in line.chars() {
            match ch {
                '=' => after_eq = true,
                '{' | '[' | '(' => {
                    if let Some(b) = blocks.last_mut()
                        && b.base == depth
                        && !b.body
                        && if b.konst { after_eq && ch != '(' } else { ch == '{' }
                    {
                        b.body = true;
                    }
                    depth += 1;
                }
                '}' | ']' | ')' => {
                    depth -= 1;
                    if blocks.last().is_some_and(|b| b.base == depth && b.body) {
                        blocks.pop();
                    }
                }
                _ => {}
            }
        }
        if starts && blocks.last().is_some_and(|b| b.base == depth && !b.body && b.konst) {
            blocks.pop();
        }
    }
    out
}

/// The lines an actor can say (`ActorSchema::texts`, docs/EDITOR-ACTORS.md "Texts"): every text id
/// its module uses (`text_literals`) is one of its schema's lines (or carries a `// schema:` note:
/// an id compared but never shown, a number that only looks like one); every line's id is a
/// message of the game's and is in the module's source; `when` and `plus` name real fields.
#[test]
fn text_ids_are_the_schemas_lines() {
    let Ok(pack) = oot_game::pack::GamePack::open_default() else {
        eprintln!("skipping: no pack");
        return;
    };
    let table = pack.messages().expect("the messages");
    let ids: BTreeSet<u16> = table.entries.iter().map(|e| e.text_id).filter(|&id| id < 0xFFFC).collect();
    let mut errors = Vec::new();
    let (mut with_lines, mut lines) = (0, 0);
    for p in ported().filter(|p| !p.schema.todo) {
        let s = p.schema;
        let code = read_code(p);
        let listed: BTreeSet<u16> = s.texts.iter().map(|t| t.id).collect();
        for (n, id, note) in text_literals(&code, &ids) {
            if !note && !listed.contains(&id) {
                errors.push(format!("{}:{n}: text {id:#06x} isn't one of its schema's `texts`", p.name));
            }
        }
        let src = std::fs::read_to_string(module_path(p)).unwrap().to_ascii_uppercase();
        for t in s.texts {
            let e = |m: String| format!("{}: text {:#06x}: {m}", p.name, t.id);
            if t.plus.is_none() && !ids.contains(&t.id) {
                errors.push(e("no such message in the game".into()));
            }
            if t.plus.is_none() && !src.contains(&format!("0X{:04X}", t.id)) && !src.contains(&format!("0X{:X}", t.id)) {
                errors.push(e("not in the module's source (code, docs or a comment)".into()));
            }
            if let Some(f) = t.plus
                && s.field(f).is_none()
            {
                errors.push(e(format!("`plus` names no field `{f}`")));
            }
            if let Some(w) = t.when {
                match s.field(w.field) {
                    None => errors.push(e(format!("`when` names no field `{}`", w.field))),
                    Some(g) => {
                        for v in w.values {
                            if !fits(g, *v) {
                                errors.push(e(format!("`when` value {v} doesn't fit `{}`", g.name)));
                            }
                        }
                    }
                }
            }
        }
        if !s.texts.is_empty() {
            with_lines += 1;
            lines += s.texts.len();
        }
    }
    eprintln!("{with_lines} actors with {lines} lines");
    assert!(errors.is_empty(), "{} problems:\n{}", errors.len(), errors.join("\n"));
}
