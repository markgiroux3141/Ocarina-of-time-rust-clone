//! The actors the editor places (docs/EDITOR-ACTORS.md): every actor the port has
//! (`oot_actors::PROFILES`) with its schema (ADR 0066), a name and a group for the palette
//! (`NAMES`), and its module's docs; and, read from the game's pack in the background, how the
//! game's own scenes place it (its variants: the params and setting rotations it's placed with).

use oot_game::actor_ctx::ActorProfile;
use oot_game::actor_schema::{ActorSchema, Field, Kind, Source};
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;

/// The palette's groups, in its order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Group {
    Character,
    Enemy,
    Boss,
    Object,
    Item,
    Trigger,
    Door,
    SetPiece,
    Effect,
    /// Not offered: Player (the level's start), the sandbox's stand-ins.
    Hidden,
}

impl Group {
    pub const ALL: [Group; 9] = [Group::Character, Group::Enemy, Group::Boss, Group::Object, Group::Item, Group::Trigger, Group::Door, Group::SetPiece, Group::Effect];

    pub fn title(self) -> &'static str {
        match self {
            Group::Character => "Characters",
            Group::Enemy => "Enemies",
            Group::Boss => "Bosses",
            Group::Object => "Objects",
            Group::Item => "Items",
            Group::Trigger => "Triggers",
            Group::Door => "Doors & warps",
            Group::SetPiece => "Set pieces",
            Group::Effect => "Effects",
            Group::Hidden => "Hidden",
        }
    }

    /// One word, for the inspector's eyebrow.
    pub fn one(self) -> &'static str {
        match self {
            Group::Character => "Character",
            Group::Enemy => "Enemy",
            Group::Boss => "Boss",
            Group::Object => "Object",
            Group::Item => "Item",
            Group::Trigger => "Trigger",
            Group::Door => "Door",
            Group::SetPiece => "Set piece",
            Group::Effect => "Effect",
            Group::Hidden => "Hidden",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Group::Character => "People, animals and fairies Link meets",
            Group::Enemy => "What fights back",
            Group::Boss => "A dungeon's boss: its room is built round it",
            Group::Object => "Pots, rocks, crates, blocks, torches, signs: things Link uses or breaks",
            Group::Item => "Chests, collectibles and rewards",
            Group::Trigger => "Invisible spots and switches that start something",
            Group::Door => "Doors, holes and warps: most come with rooms and exits",
            Group::SetPiece => "One scene's own machinery, drawn from that scene's object",
            Group::Effect => "Lights, sounds, weather and what other actors spawn",
            Group::Hidden => "",
        }
    }
}

/// Each ported actor's name in the palette and its group (the overlay's name first).
pub const NAMES: &[(&str, &str, Group)] = &[
    ("Player", "Link", Group::Hidden),
    ("Sandbox_Dummy_Target", "Training dummy", Group::Hidden),
    ("En_Holl", "Room divider", Group::Door),
    ("Bg_Ydan_Hasi", "Deku Tree puzzle pieces", Group::SetPiece),
    ("Obj_Hana", "Field flower, rock or bush", Group::Object),
    ("En_Ishi", "Rock", Group::Object),
    ("En_Kusa", "Bush", Group::Object),
    ("En_Kanban", "Sign", Group::Object),
    ("En_Ko", "Kokiri child", Group::Character),
    ("En_Door", "Door", Group::Door),
    ("En_Wonder_Talk2", "Walk-in text", Group::Trigger),
    ("En_Item00", "Collectible", Group::Item),
    ("Bg_Treemouth", "Deku Tree's mouth", Group::SetPiece),
    ("En_Box", "Chest", Group::Item),
    ("En_Wonder_Item", "Hidden collectible", Group::Trigger),
    ("En_Goroiwa", "Rolling boulder", Group::Object),
    ("En_Md", "Mido", Group::Character),
    ("En_Ossan", "Shopkeeper", Group::Character),
    ("En_GirlA", "Shop item", Group::Item),
    ("En_Tana", "Shop shelves", Group::Object),
    ("En_Elf", "Fairy", Group::Character),
    ("En_River_Sound", "Sound source", Group::Effect),
    ("Demo_Tre_Lgt", "Chest's light", Group::Effect),
    ("Bg_Spot00_Hanebasi", "Castle drawbridge", Group::SetPiece),
    ("En_Viewer", "Cutscene rider", Group::Character),
    ("Object_Kankyo", "Environment effects", Group::Effect),
    ("En_Dekubaba", "Deku Baba", Group::Enemy),
    ("En_Karebaba", "Withered Deku Baba", Group::Enemy),
    ("En_Firefly", "Keese", Group::Enemy),
    ("En_Nutsball", "Deku Scrub's nut", Group::Effect),
    ("Item_Shield", "Deku Shield", Group::Item),
    ("En_Dekunuts", "Mad Scrub", Group::Enemy),
    ("En_Hintnuts", "Hint Deku Scrub", Group::Enemy),
    ("En_Shopnuts", "Business Scrub", Group::Enemy),
    ("En_Dns", "Business Scrub, selling", Group::Character),
    ("En_St", "Skulltula", Group::Enemy),
    ("En_Goma", "Gohma Larva", Group::Enemy),
    ("En_Sw", "Skullwalltula", Group::Enemy),
    ("En_Si", "Gold Skulltula token", Group::Item),
    ("Door_Shutter", "Sliding door", Group::Door),
    ("Bg_Ydan_Sp", "Deku Tree web", Group::SetPiece),
    ("Obj_Switch", "Switch", Group::Trigger),
    ("Obj_Syokudai", "Torch", Group::Object),
    ("Elf_Msg", "Navi call spot", Group::Trigger),
    ("Elf_Msg2", "Navi hint tag", Group::Trigger),
    ("Obj_Kibako2", "Large crate", Group::Object),
    ("Obj_Lift", "Collapsing platform", Group::Object),
    ("Bg_Ydan_Maruta", "Spiked log", Group::SetPiece),
    ("Obj_Oshihiki", "Push block", Group::Object),
    ("Obj_Makeoshihiki", "Push block puzzle", Group::Trigger),
    ("Bg_Haka", "Gravestone to pull", Group::Object),
    ("Obj_Timeblock", "Song of Time block", Group::Object),
    ("Obj_Bombiwa", "Bombable boulder", Group::Object),
    ("En_Arrow", "Arrow", Group::Effect),
    ("En_M_Fire1", "Deku Nut flash", Group::Effect),
    ("Boss_Goma", "Queen Gohma", Group::Boss),
    ("Item_B_Heart", "Heart Container", Group::Item),
    ("Door_Warp1", "Warp", Group::Door),
    ("Demo_Effect", "Cutscene lights", Group::Effect),
    ("Demo_Kankyo", "Cutscene surroundings", Group::Effect),
    ("Bg_Spot09_Obj", "Gerudo Valley bridges", Group::SetPiece),
    ("Bg_Spot16_Doughnut", "Death Mountain's cloud ring", Group::SetPiece),
    ("Demo_Sa", "Saria, in cutscenes", Group::Character),
    ("Obj_Bean", "Soft soil", Group::Object),
    ("En_Owl", "Kaepora Gaebora", Group::Character),
    ("En_Wood02", "Field tree or bush", Group::Object),
    ("Obj_Mure2", "Ring of bushes or rocks", Group::Object),
    ("En_A_Obj", "Signpost or block", Group::Object),
    ("Door_Ana", "Grotto hole", Group::Door),
    ("En_Encount1", "Enemy spawner", Group::Trigger),
    ("En_Part", "Enemy's piece", Group::Effect),
    ("En_Skb", "Stalchild", Group::Enemy),
    ("En_Peehat", "Peahat", Group::Enemy),
    ("En_Hy", "Townsfolk", Group::Character),
    ("En_Heishi4", "Guard who talks", Group::Character),
    ("En_Dog", "Dog", Group::Character),
    ("En_Ma1", "Malon, child", Group::Character),
    ("En_Mu", "Haggling townsfolk", Group::Character),
    ("En_Tg", "Dancing couple", Group::Character),
    ("En_Niw", "Cucco", Group::Character),
    ("En_Niw_Girl", "Girl chasing a cucco", Group::Character),
    ("En_Gs", "Gossip Stone", Group::Object),
    ("En_Butte", "Butterfly", Group::Effect),
    ("En_Insect", "Bugs", Group::Effect),
    ("En_Boom", "Boomerang (thrown)", Group::Effect),
    ("Arms_Hook", "Hookshot's hook", Group::Effect),
    ("En_Po_Relay", "Dampé's ghost", Group::Character),
    ("En_Honotrap", "Fire eye trap", Group::Enemy),
    ("Obj_Mure", "Fish, bugs or butterflies", Group::Effect),
    ("En_Fish", "Fish", Group::Effect),
    ("En_Wonder_Talk", "Spot to check", Group::Trigger),
    ("Bg_Toki_Hikari", "Temple of Time windows", Group::SetPiece),
    ("Bg_Toki_Swd", "Master Sword in its pedestal", Group::SetPiece),
    ("Door_Toki", "Door of Time collision", Group::SetPiece),
    ("Bg_Spot00_Break", "Broken drawbridge (adult field)", Group::SetPiece),
    ("Bg_Umajump", "Jumpable horse fence", Group::SetPiece),
    ("Bg_Ingate", "Lon Lon Ranch gate", Group::SetPiece),
    ("En_Horse_Game_Check", "Horse game referee", Group::Trigger),
    ("En_Ma3", "Malon, adult (obstacle course)", Group::Character),
    ("En_Po_Field", "Poe (Hyrule Field)", Group::Enemy),
    ("Bg_Mjin", "Warp pad", Group::Door),
    ("En_Heishi3", "Castle sentry", Group::Character),
    ("En_Heishi2", "Gate guard", Group::Character),
    ("Bg_Spot15_Saku", "Castle gate's fence", Group::SetPiece),
    ("Bg_Gate_Shutter", "Death Mountain gate", Group::SetPiece),
    ("En_Light", "Flame", Group::Effect),
    ("Obj_Tsubo", "Pot", Group::Object),
    ("Bg_Spot15_Rrbox", "Milk crate", Group::SetPiece),
    ("En_Ta", "Talon", Group::Character),
    ("Oceff_Wipe", "Zelda's Lullaby effect", Group::Effect),
    ("Oceff_Wipe2", "Epona's Song effect", Group::Effect),
    ("Oceff_Wipe3", "Saria's Song effect", Group::Effect),
    ("Oceff_Wipe4", "Scarecrow's Song effect", Group::Effect),
    ("Oceff_Spot", "Sun's Song effect", Group::Effect),
    ("Oceff_Storm", "Song of Storms effect", Group::Effect),
    ("En_Okarina_Effect", "Storm", Group::Effect),
    ("Shot_Sun", "Sun's Song spot", Group::Trigger),
    ("En_Okarina_Tag", "Song spot", Group::Trigger),
    ("En_Heishi1", "Courtyard patrol", Group::Character),
    ("En_Ex_Ruppy", "Rupee, spawned", Group::Item),
    ("En_Zl4", "Zelda, child", Group::Character),
    ("Demo_Im", "Impa", Group::Character),
    ("En_Cow", "Cow", Group::Character),
    ("En_Fu", "Windmill man", Group::Character),
    ("Bg_Relay_Objects", "Windmill set pieces", Group::SetPiece),
    ("En_Niw_Lady", "Anju", Group::Character),
    ("En_Attack_Niw", "Attacking cucco", Group::Enemy),
    ("En_Daiku_Kakariko", "Carpenter", Group::Character),
    ("En_Toryo", "Mutoh", Group::Character),
    ("En_Hs2", "Carpenter's son", Group::Character),
    ("En_Ani", "Man on the roof", Group::Character),
    ("Bg_Spot01_Fusya", "Windmill sails", Group::SetPiece),
    ("Bg_Spot01_Idohashira", "Well's beam", Group::SetPiece),
    ("Bg_Spot01_Idomizu", "Well's water", Group::SetPiece),
    ("Bg_Spot01_Objects2", "Kakariko set pieces", Group::SetPiece),
    ("En_Ssh", "Cursed Skulltula man", Group::Character),
    ("En_Sth", "Cured Skulltula family", Group::Character),
    ("En_Cs", "Graveyard kid", Group::Character),
    ("Obj_Makekinsuta", "Soft soil for bugs", Group::Object),
    ("Bg_Spot02_Objects", "Graveyard set pieces", Group::SetPiece),
    ("En_Weather_Tag", "Weather tag", Group::Trigger),
    ("Bg_Breakwall", "Bombable wall", Group::Object),
    ("En_Tk", "Dampé", Group::Character),
    ("En_It", "Dig spot", Group::Trigger),
    ("En_Rd", "ReDead or Gibdo", Group::Enemy),
    ("Bg_Spot05_Soko", "Sacred Meadow set pieces", Group::SetPiece),
    ("En_Wf", "Wolfos", Group::Enemy),
    ("En_Sa", "Saria", Group::Character),
    ("En_Skj", "Skull Kid", Group::Enemy),
    ("En_Skjneedle", "Skull Kid's needle", Group::Effect),
    ("En_Dnt_Nomal", "Minigame scrub", Group::Character),
    ("En_Ex_Item", "Minigame prize", Group::Item),
    ("En_Crow", "Guay", Group::Enemy),
    ("En_In", "Ingo", Group::Character),
    ("En_Ma2", "Malon, adult", Group::Character),
    ("En_Horse", "Epona (rideable horse)", Group::Character),
    ("En_Horse_Link_Child", "Young Epona", Group::Character),
    ("En_Horse_Normal", "Horse", Group::Character),
    ("En_Bom", "Bomb", Group::Item),
    ("En_Bombf", "Bomb Flower", Group::Object),
    ("En_Tite", "Tektite", Group::Enemy),
    ("Bg_Spot18_Obj", "Goron statue", Group::SetPiece),
    ("Bg_Spot18_Shutter", "Darunia's door", Group::SetPiece),
    ("En_Du", "Darunia", Group::Character),
    ("Bg_Bombwall", "Cracked rock wall", Group::Object),
    ("Bg_Spot16_Bombstone", "Cavern's boulder", Group::SetPiece),
    ("En_Hata", "Flagpole", Group::Object),
    ("En_Go2", "Goron", Group::Character),
    ("Bg_Spot18_Basket", "Goron City's urn", Group::SetPiece),
    ("Bg_Spot18_Futa", "Urn's lid", Group::SetPiece),
    ("En_Gm", "Medigoron", Group::Character),
    ("Obj_Hamishi", "Bronze boulder", Group::Object),
    ("Obj_Hsblock", "Hookshot target", Group::Object),
    ("En_Poh", "Poe", Group::Enemy),
    ("Bg_Ddan_Jd", "Rising pillar", Group::SetPiece),
    ("Bg_Dodoago", "Dodongo skull", Group::SetPiece),
    ("En_Dodojr", "Baby Dodongo", Group::Enemy),
    ("En_Dodongo", "Dodongo", Group::Enemy),
    ("Obj_Elevator", "Small lift", Group::Object),
    ("En_Zf", "Lizalfos or Dinolfos", Group::Enemy),
    ("Bg_Gnd_Darkmeiro", "Clear blocks", Group::Object),
    ("Bg_Hidan_Curtain", "Fire wall", Group::Object),
    ("En_Am", "Armos", Group::Enemy),
    ("En_Vm", "Beamos", Group::Enemy),
    ("Boss_Dodongo", "King Dodongo", Group::Boss),
    ("Boss_Va", "Barinade", Group::Boss),
    ("En_Bdfire", "King Dodongo's fire", Group::Effect),
    ("Demo_Du", "Darunia, in cutscenes", Group::Character),
    ("Demo_Go", "Gorons, in cutscenes", Group::Character),
    ("En_Encount2", "Falling rocks", Group::Trigger),
    ("En_Fire_Rock", "Burning rock", Group::Enemy),
    ("Bg_Dy_Yoseizo", "Great Fairy", Group::Character),
    ("En_Dy_Extra", "Great Fairy's beam", Group::Effect),
    ("En_M_Thunder", "Spin attack", Group::Effect),
    ("Eff_Dust", "Sparkles", Group::Effect),
    ("Bg_Ddan_Kd", "Falling stairs", Group::SetPiece),
    ("En_G_Switch", "Silver rupee or tracker", Group::Trigger),
    ("En_Diving_Game", "Diving game Zora", Group::Character),
    ("En_Kz", "King Zora", Group::Character),
    ("En_Zo", "Zora", Group::Character),
    ("Obj_Comb", "Beehive", Group::Object),
    ("Bg_Spot07_Taki", "Zora's Domain waterfall", Group::SetPiece),
    ("Item_Etcetera", "Item lying out (Ruto's letter)", Group::Item),
    ("Item_Ocarina", "Ocarina of Time (in the moat)", Group::Item),
    ("Bg_Spot06_Objects", "Lake Hylia set pieces", Group::SetPiece),
    ("En_Mk", "Lakeside professor", Group::Character),
    ("En_Ms", "Bean seller", Group::Character),
    ("En_Jj", "Lord Jabu-Jabu", Group::Character),
    ("Bg_Spot08_Bakudankabe", "Fountain's bombable wall", Group::Object),
    ("Bg_Spot03_Taki", "Zora's River waterfall", Group::SetPiece),
    ("En_Rr", "Like Like", Group::Enemy),
    ("En_Bubble", "Shabom", Group::Enemy),
    ("En_Bili", "Biri", Group::Enemy),
    ("En_Brob", "Flobbery", Group::Enemy),
    ("En_Weiyer", "Stinger (water)", Group::Enemy),
    ("En_Eiyer", "Stinger (ground)", Group::Enemy),
    ("Bg_Bdan_Switch", "Jabu-Jabu's switch", Group::Trigger),
    ("Bg_Bdan_Objects", "Jabu-Jabu's lifts and water", Group::Object),
    ("En_Siofuki", "Water spout", Group::Object),
    ("En_Okuta", "Octorok", Group::Enemy),
    ("En_Ba", "Parasitic tentacle", Group::Enemy),
    ("En_Bigokuta", "Big Octo", Group::Enemy),
    ("En_Bx", "Electrified tentacle barrier", Group::Enemy),
    ("Obj_Kibako", "Small crate", Group::Object),
    ("Obj_Roomtimer", "Room clear (timer)", Group::Trigger),
    ("En_Ru1", "Ruto, child", Group::Character),
    ("En_Ganon_Mant", "Ganondorf's cape", Group::SetPiece),
    ("En_Vali", "Bari", Group::Enemy),
    ("En_Tp", "Tailpasaran", Group::Enemy),
    ("En_Fr", "Frogs (and their song spot)", Group::Character),
    ("En_Kakasi", "Scarecrow (long song)", Group::Character),
    ("En_Kakasi2", "Pierre (scarecrow's song)", Group::Character),
    ("En_Kakasi3", "Bonooru (scarecrow)", Group::Character),
    ("En_Rl", "Rauru", Group::Character),
    ("Demo_6K", "Sages' balls of light (cutscenes)", Group::Effect),
    ("En_Xc", "Sheik", Group::Character),
];

/// An actor the editor can place.
#[derive(Clone)]
pub struct Entry {
    pub name: &'static str,
    pub id: i16,
    pub label: String,
    pub group: Group,
    /// Its profile's object file (`gameplay_keep` for one that needs none of its own).
    pub object: &'static str,
    pub schema: &'static ActorSchema,
    /// Its module docs' first paragraph, and its Params and Flags paragraphs (ADR 0062).
    pub about: String,
    pub params_doc: String,
    pub flags_doc: String,
}

impl Entry {
    /// Whether the y rotation is a setting (so the actor has no facing of its own to turn).
    pub fn rot_y_is_setting(&self) -> bool {
        self.schema.fields.iter().any(|f| f.source == Source::RotY && f.kind != Kind::Rotation)
    }

    /// The objects a placement with these params and rotation loads into the room: its profile's
    /// (none for `gameplay_keep`'s actors) and what its schema says it needs for them, less the
    /// keeps the overworld's scenes always have (`gameplay_keep`, `gameplay_field_keep`).
    pub fn objects_for(&self, params: u16, rot: [i16; 3]) -> Vec<&'static str> {
        let mut out = vec![self.object];
        out.extend(self.schema.needed_objects(params as i16, rot));
        out.retain(|o| !matches!(*o, "" | "gameplay_keep" | "gameplay_field_keep"));
        out.dedup();
        out
    }

    /// The fields the inspector shows as settings (not debug, unused or plain rotations), in the
    /// schema's order.
    pub fn settings(&self) -> impl Iterator<Item = &'static Field> {
        self.schema.fields.iter().filter(|f| !matches!(f.kind, Kind::Debug | Kind::Unused | Kind::Rotation))
    }
}

/// How the game's own scenes place an actor.
#[derive(Default, Clone)]
pub struct Placed {
    /// Room actors, transition actors (doors between rooms) and spawns.
    pub rooms: usize,
    pub transitions: usize,
    pub spawns: usize,
    /// Its different configurations, most placed first.
    pub variants: Vec<Variant>,
}

impl Placed {
    pub fn count(&self) -> usize {
        self.rooms + self.transitions + self.spawns
    }
}

/// One configuration the game places an actor with: its params and the rotation components the
/// actor reads as settings (the others 0), how often, and where first.
#[derive(Clone)]
pub struct Variant {
    pub params: u16,
    pub rot: [i16; 3],
    pub count: usize,
    pub first: String,
}

/// Every placeable actor, and (once read) the game's placements.
pub struct Catalogue {
    pub entries: Vec<Entry>,
    by_name: HashMap<&'static str, usize>,
    pub placed: HashMap<String, Placed>,
    /// The object table's files by `OBJECT_*` id, once read.
    pub objects: Vec<String>,
    /// Why the game's data isn't there (no pack, an old one).
    pub game_error: Option<String>,
    pending: Option<mpsc::Receiver<Result<GameData, String>>>,
}

/// What's read from the pack in the background.
pub struct GameData {
    pub placed: HashMap<String, Placed>,
    pub objects: Vec<String>,
}

impl Catalogue {
    /// The actors from the port's profiles (at once), and the game's placements read from the
    /// pack on a thread (`poll` takes them in).
    pub fn load(root: &Path) -> Catalogue {
        let names: HashMap<&str, (&str, Group)> = NAMES.iter().map(|&(n, l, g)| (n, (l, g))).collect();
        let entries: Vec<Entry> = oot_actors::PROFILES
            .iter()
            .map(|p: &ActorProfile| {
                let (label, group) = names.get(p.name).map_or_else(|| (p.name.replace('_', " "), Group::Object), |&(l, g)| (l.to_string(), g));
                let (about, params_doc, flags_doc) = module_docs(root, p.name);
                Entry { name: p.name, id: p.id, label, group, object: p.object, schema: p.schema, about, params_doc, flags_doc }
            })
            .collect();
        let by_name = entries.iter().enumerate().map(|(i, e)| (e.name, i)).collect();
        let (tx, rx) = mpsc::channel();
        let schemas: Vec<(&'static str, &'static ActorSchema)> = entries.iter().map(|e| (e.name, e.schema)).collect();
        std::thread::spawn(move || {
            let _ = tx.send(read_game(&schemas));
        });
        Catalogue { entries, by_name, placed: HashMap::new(), objects: vec![], game_error: None, pending: Some(rx) }
    }

    /// Takes in the game's placements once they're read. True when they've just arrived.
    pub fn poll(&mut self) -> bool {
        let Some(rx) = &self.pending else { return false };
        match rx.try_recv() {
            Ok(r) => {
                self.pending = None;
                match r {
                    Ok(g) => {
                        self.placed = g.placed;
                        self.objects = g.objects;
                    }
                    Err(e) => self.game_error = Some(e),
                }
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.game_error = Some("reading the game's data stopped".into());
                true
            }
        }
    }

    pub fn reading(&self) -> bool {
        self.pending.is_some()
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.by_name.get(name).map(|&i| &self.entries[i])
    }

    pub fn placed(&self, name: &str) -> Option<&Placed> {
        self.placed.get(name)
    }

    /// Whether the game only places it between rooms (a door): it comes with rooms and exits.
    pub fn transition_only(&self, name: &str) -> bool {
        self.placed(name).is_some_and(|p| p.transitions > 0 && p.rooms == 0)
    }

    /// Whether the game never places it: other actors spawn it (an arrow, a song's effect).
    pub fn spawned_only(&self, name: &str) -> bool {
        !self.placed.is_empty() && self.placed(name).is_none_or(|p| p.count() == 0)
    }
}

/// The first paragraph of an actor's module docs, and its Params and Flags paragraphs.
fn module_docs(root: &Path, name: &str) -> (String, String, String) {
    let dir = root.join("crates/game/oot_actors/src");
    let module = if name == "Player" { "player".to_string() } else { name.to_lowercase() };
    let text = std::fs::read_to_string(dir.join(format!("{module}.rs"))).or_else(|_| std::fs::read_to_string(dir.join(&module).join("mod.rs"))).unwrap_or_default();
    let lines: Vec<&str> = text.lines().take_while(|l| l.starts_with("//!")).map(|l| l.trim_start_matches("//!").trim()).collect();
    let mut paras: Vec<String> = vec![];
    let mut cur = String::new();
    for l in lines {
        if l.is_empty() {
            if !cur.is_empty() {
                paras.push(std::mem::take(&mut cur));
            }
        } else {
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(l);
        }
    }
    if !cur.is_empty() {
        paras.push(cur);
    }
    let about = paras.first().cloned().unwrap_or_default();
    // "Params: ... Flags: ..." may share a paragraph
    let mut params = String::new();
    let mut flags = String::new();
    for p in &paras {
        if let Some(rest) = p.strip_prefix("Params:") {
            match rest.split_once(" Flags:") {
                Some((a, b)) => {
                    params = a.trim().to_string();
                    flags = b.trim().to_string();
                }
                None => params = rest.trim().to_string(),
            }
        } else if let Some(rest) = p.strip_prefix("Flags:") {
            flags = rest.trim().to_string();
        }
    }
    (about, params, flags)
}

/// The area an invisible actor watches, drawn in its place in the views (it has nothing to
/// capture): a cylinder or a box (half its width across), up from where it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Area {
    Cylinder { r: f32, h: f32 },
    Box { half: f32, h: f32 },
}

/// The areas of the actors that draw nothing, from their settings or their code's constants
/// (docs/EDITOR-ACTORS.md, "Invisible actors"). `None` for the rest, and for a range that's off.
pub fn area(name: &str, params: u16, rot: [i16; 3]) -> Option<Area> {
    let (rx, rz) = (rot[0] as f32, rot[2] as f32);
    let cyl = |r: f32, h: f32| Some(Area::Cylinder { r, h });
    match name {
        // the radius (or half width) rot.x * 4, the height rot.z * 4 (0: 40); bit 14 a cuboid
        "Elf_Msg" => {
            let r = if rot[0] == 0 { 40.0 } else { rx * 4.0 };
            let h = if rot[2] == 0 { 40.0 } else { rz * 4.0 };
            if params & 0x4000 != 0 { Some(Area::Box { half: r, h }) } else { cyl(r, h) }
        }
        // past each 10 the next targetMode, the rest (1 to 10) times 40; 0 and below none
        "En_Wonder_Talk2" => (rot[2] > 0).then(|| Area::Cylinder { r: (((rot[2] - 1) % 10) + 1) as f32 * 40.0, h: 80.0 }),
        // the spot's range, 40 a unit
        "En_Okarina_Tag" => (rot[2] > 0).then(|| Area::Cylinder { r: rz * 40.0, h: 80.0 }),
        // its range, 120 plus 40 a unit
        "En_Encount1" => cyl(120.0 + 40.0 * rz, 40.0),
        // WEATHER_TAG_RANGE100: Link's distance across the ground under it
        "En_Weather_Tag" => {
            let range = ((params as i16) >> 8) as f32 * 100.0;
            (range >= 1.0).then_some(Area::Cylinder { r: range, h: 100.0 })
        }
        // within 50 across and 30 up (its proximity mode)
        "En_Wonder_Item" => cyl(50.0, 30.0),
        // sCylinderInit
        "Shot_Sun" => cyl(30.0, 60.0),
        // spots: a small marker of their place
        "En_Wonder_Talk" | "Elf_Msg2" => cyl(30.0, 60.0),
        "En_River_Sound" | "Obj_Makekinsuta" | "Obj_Makeoshihiki" => cyl(20.0, 20.0),
        _ => None,
    }
}

/// The rotation components an actor reads as settings, kept from a placement's (the rest 0).
pub fn setting_rot(schema: &ActorSchema, rot: [i16; 3]) -> [i16; 3] {
    schema.setting_rot(rot)
}

/// A scene's name from its enum: "SCENE_KOKIRI_FOREST" -> "Kokiri Forest".
fn scene_title(enum_name: &str) -> String {
    let s = enum_name.strip_prefix("SCENE_").unwrap_or(enum_name);
    s.split('_')
        .map(|w| {
            let w = w.to_lowercase();
            if matches!(w.as_str(), "of" | "the" | "and") {
                return w;
            }
            let mut c = w.chars();
            c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every placement of every actor in the game's scenes (each room once), as variants.
fn read_game(schemas: &[(&'static str, &'static ActorSchema)]) -> Result<GameData, String> {
    let pack = oot_game::pack::GamePack::open_default().map_err(|e| format!("the game's pack: {e:#}"))?;
    let at = pack.actor_table().map_err(|e| format!("the actor table: {e:#}"))?;
    let st = pack.scene_table().map_err(|e| format!("the scene table: {e:#}"))?;
    let schema: HashMap<&str, &ActorSchema> = schemas.iter().copied().collect();
    let mut placed: HashMap<String, Placed> = HashMap::new();
    let mut variants: HashMap<String, Vec<Variant>> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for e in &st.scenes {
        let Ok(sd) = pack.scene(&e.file) else { continue };
        let title = scene_title(&e.enum_name);
        for ld in &sd.layers {
            let mut add = |id: i16, params: i16, rot: [i16; 3], kind: u8| {
                let name = at.name(id);
                let p = placed.entry(name.clone()).or_default();
                match kind {
                    0 => p.rooms += 1,
                    1 => p.transitions += 1,
                    _ => p.spawns += 1,
                }
                let rot = schema.get(name.as_str()).map_or([0; 3], |s| setting_rot(s, rot));
                let vs = variants.entry(name).or_default();
                match vs.iter_mut().find(|v| v.params == params as u16 && v.rot == rot) {
                    Some(v) => v.count += 1,
                    None => vs.push(Variant { params: params as u16, rot, count: 1, first: title.clone() }),
                }
            };
            for s in &ld.spawns {
                add(s.id, s.params, s.rot, 2);
            }
            for t in &ld.transition_actors {
                add(t.id, t.params, [0, t.rot_y, 0], 1);
            }
            for key in &ld.rooms {
                if !seen.insert(key.clone()) {
                    continue;
                }
                let Ok(r) = pack.room(key) else { continue };
                for x in &r.actors {
                    add(x.id, x.params, x.rot, 0);
                }
            }
        }
    }
    for (name, mut vs) in variants {
        vs.sort_by(|a, b| b.count.cmp(&a.count).then(a.params.cmp(&b.params)));
        if let Some(p) = placed.get_mut(&name) {
            p.variants = vs;
        }
    }
    Ok(GameData { placed, objects: st.objects })
}

/// Whether `f` applies to an actor with these params and rotation (its `when`).
pub fn applies(schema: &ActorSchema, f: &Field, params: u16, rot: [i16; 3]) -> bool {
    let Some(w) = f.when else { return true };
    match schema.field(w.field) {
        Some(g) => w.values.contains(&g.value(source_value(g.source, params, rot))),
        None => true,
    }
}

/// The value a field's source holds.
pub fn source_value(src: Source, params: u16, rot: [i16; 3]) -> i16 {
    match src {
        Source::Params => params as i16,
        Source::RotX => rot[0],
        Source::RotY => rot[1],
        Source::RotZ => rot[2],
    }
}

/// A field's value in an actor's params and rotation.
pub fn value(f: &Field, params: u16, rot: [i16; 3]) -> i32 {
    f.value(source_value(f.source, params, rot))
}

/// Sets a field's value in an actor's params or rotation.
pub fn set_value(f: &Field, params: &mut u16, rot: &mut [i16; 3], v: i32) {
    match f.source {
        Source::Params => *params = f.put(*params as i16, v) as u16,
        Source::RotX => rot[0] = f.put(rot[0], v),
        Source::RotY => rot[1] = f.put(rot[1], v),
        Source::RotZ => rot[2] = f.put(rot[2], v),
    }
}

/// The params bits no field covers (set ones are kept as they are, but nothing reads them).
pub fn loose_bits(schema: &ActorSchema, params: u16) -> u16 {
    let covered = schema.fields.iter().filter(|f| f.source == Source::Params).fold(0u16, |m, f| m | f.mask());
    params & !covered
}

/// A field's name for the panel: the C's, in words ("switchFlag" -> "Switch flag").
pub fn field_label(f: &Field) -> String {
    let n = f.name;
    if matches!(n, "params" | "value" | "unusedParams" | "unusedParam") || n.starts_with("unk_") {
        // the meaning says it better, when it's short
        let m = f.meaning.split([',', ';', '(']).next().unwrap_or("").trim();
        if !m.is_empty() && m.len() <= 24 {
            return upper_first(m);
        }
    }
    let mut out = String::new();
    let mut prev_lower = false;
    for c in n.chars() {
        if c == '_' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
        prev_lower = c.is_lowercase();
    }
    upper_first(out.trim())
}

pub fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// A value of a field, in words: its name in a named list, a flag's number, else the number.
pub fn value_text(f: &Field, v: i32) -> String {
    match f.kind {
        Kind::Named(list) => list.iter().find(|x| x.0 == v).map_or(format!("{v:#x}"), |x| x.1.to_string()),
        Kind::Drop => oot_game::actor_schema::ITEM00_NAMES.iter().find(|x| x.0 == v).map_or("no drop".to_string(), |x| x.1.to_string()),
        Kind::ItemId => oot_game::actor_schema::GI_NAMES.iter().find(|x| x.0 == v).map_or(format!("{v:#x}"), |x| x.1.to_string()),
        Kind::SwitchFlag | Kind::ChestFlag | Kind::CollectibleFlag | Kind::TempFlag | Kind::TextId => format!("{v:#04x}"),
        _ => v.to_string(),
    }
}

/// A placement's settings in a few words: its named and flag fields (for the variants' list).
pub fn describe(e: &Entry, params: u16, rot: [i16; 3]) -> String {
    let mut parts = vec![];
    for f in e.settings().filter(|f| f.part_of.is_none() || !e.schema.fields.iter().any(|g| Some(g.name) == f.part_of && g.part_of.is_none())) {
        if !applies(e.schema, f, params, rot) {
            continue;
        }
        let v = value(f, params, rot);
        let t = match f.kind {
            Kind::Named(_) | Kind::Drop | Kind::ItemId => value_text(f, v),
            Kind::SwitchFlag | Kind::TempFlag => format!("switch {v:#04x}"),
            Kind::ChestFlag => format!("chest {v:#04x}"),
            Kind::CollectibleFlag => format!("collectible {v:#04x}"),
            Kind::TextId => format!("text {v:#06x}"),
            Kind::PathIndex => format!("path {v}"),
            _ => format!("{} {v}", field_label(f).to_lowercase()),
        };
        parts.push(t);
        if parts.len() == 3 {
            break;
        }
    }
    if parts.is_empty() {
        format!("params {params:#06x}")
    } else {
        parts.join(" · ")
    }
}

/// The flag a field holds, as a key for finding the level's other actors with the same one: its
/// kind and number.
pub fn flag_key(f: &Field, v: i32) -> Option<(&'static str, i32)> {
    match f.kind {
        Kind::SwitchFlag | Kind::TempFlag => Some(("switch", v)),
        Kind::ChestFlag => Some(("chest", v)),
        Kind::CollectibleFlag => Some(("collectible", v)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ported_actor_is_named() {
        let missing: Vec<&str> = oot_actors::PROFILES.iter().map(|p| p.name).filter(|n| !NAMES.iter().any(|x| x.0 == *n)).collect();
        assert!(missing.is_empty(), "add these to actors::NAMES: {missing:?}");
    }

    #[test]
    fn field_labels_read_as_words() {
        let f = |name: &'static str, meaning: &'static str| Field::params(name, 0, 4, Kind::Range(0, 15), meaning);
        assert_eq!(field_label(&f("switchFlag", "")), "Switch flag");
        assert_eq!(field_label(&f("type", "")), "Type");
        assert_eq!(field_label(&f("ENKUSA_GET_TYPE", "")), "ENKUSA GET TYPE");
        assert_eq!(field_label(&f("params", "the kind of fairy")), "The kind of fairy");
        assert_eq!(field_label(&f("unk_14C", "a much longer meaning than fits a label")), "Unk 14C");
    }

    #[test]
    fn setting_rotations_are_kept_and_the_rest_dropped() {
        static FIELDS: [Field; 2] = [Field::rot("rotZ", Source::RotZ, 0, 8, Kind::Range(0, 255), ""), Field::rot("tilt", Source::RotX, 0, 16, Kind::Rotation, "")];
        static S: ActorSchema = ActorSchema { fields: &FIELDS, flags: &[], todo: false, objects: &[], texts: &[] };
        assert_eq!(setting_rot(&S, [0x1234, 0x4000, 0x0305]), [0, 0, 0x0005]);
        let (mut p, mut r) = (0u16, [0i16; 3]);
        set_value(&FIELDS[0], &mut p, &mut r, 7);
        assert_eq!((p, r, value(&FIELDS[0], p, r)), (0, [0, 0, 7], 7));
    }

    /// The game's placements, read from the pack (skipped without one): pots are placed in rooms
    /// with variants, doors only between rooms, arrows never.
    #[test]
    fn the_games_placements_are_read() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut c = Catalogue::load(&root);
        let t0 = std::time::Instant::now();
        while c.reading() {
            c.poll();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if let Some(e) = &c.game_error {
            eprintln!("skipping: {e}");
            return;
        }
        eprintln!("read in {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);
        let pot = c.placed("Obj_Tsubo").expect("pots are placed");
        assert!(pot.rooms > 50 && pot.variants.len() > 5, "{} rooms, {} variants", pot.rooms, pot.variants.len());
        assert!(c.transition_only("En_Door") && !c.transition_only("Obj_Tsubo"));
        assert!(c.spawned_only("En_Arrow") && !c.spawned_only("Obj_Tsubo"));
        assert_eq!(c.objects.get(1).map(String::as_str), Some("gameplay_keep"));
        let pot = c.get("Obj_Tsubo").unwrap();
        assert!(pot.about.contains("a pot") && pot.params_doc.contains("dropParams") && !pot.flags_doc.is_empty(), "{:?}", (&pot.about, &pot.params_doc, &pot.flags_doc));
    }

    /// The invisible actors' areas, from their settings: Navi's spots by their rotation (4 a unit,
    /// 0 is 40; bit 14 a cuboid), the song and talk spots' ranges, the weather's in hundreds.
    #[test]
    fn invisible_actors_areas_come_from_their_settings() {
        assert_eq!(area("Elf_Msg", 0, [0; 3]), Some(Area::Cylinder { r: 40.0, h: 40.0 }));
        assert_eq!(area("Elf_Msg", 0x4000, [20, 0, 10]), Some(Area::Box { half: 80.0, h: 40.0 }));
        assert_eq!(area("En_Okarina_Tag", 0, [0; 3]), None);
        assert_eq!(area("En_Okarina_Tag", 0, [0, 0, 3]), Some(Area::Cylinder { r: 120.0, h: 80.0 }));
        assert_eq!(area("En_Wonder_Talk2", 0, [0, 0, 13]), Some(Area::Cylinder { r: 120.0, h: 80.0 }));
        assert_eq!(area("En_Weather_Tag", 0x0607, [0; 3]), Some(Area::Cylinder { r: 600.0, h: 100.0 }));
        assert_eq!(area("Obj_Tsubo", 0, [0; 3]), None);
    }

    #[test]
    fn scenes_are_titled() {
        assert_eq!(scene_title("SCENE_KOKIRI_FOREST"), "Kokiri Forest");
        assert_eq!(scene_title("SCENE_TEMPLE_OF_TIME"), "Temple of Time");
    }
}
