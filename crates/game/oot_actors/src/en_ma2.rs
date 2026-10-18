//! `En_Ma2` (`ovl_En_Ma2/z_en_ma2.c`): adult Malon, in the stable and the ranch.
//!
//! Where she is (`EnMa2_GetSpawnIndex`, by Link's age, the scene, the time, `shape.rot.z` and
//! Epona): never for a child (`MALON_ADULT_SPAWN_NONE`: killed); for an adult before Epona, in the
//! stable by day (rot.z 5, `MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA`: standing with her basket) and
//! in the ranch by night (rot.z 6, `_NIGHT_RANCH_NO_EPONA`: singing, waiting for Epona's Song on
//! the ocarina); after Epona, in the ranch by day (rot.z 7) or by night (rot.z 8)
//! (`MALON_ADULT_SPAWN_ALWAYS`); anything else is killed.
//!
//! **Ported:** `EnMa2_GetSpawnIndex` whole, and `EnMa2_Init`'s kill for `MALON_ADULT_SPAWN_NONE`.
//! **Not ported** (adult Malon, Phase 13): for the other spawn indexes her init spawns a
//! placeholder. Her texts (`EnMa2_GetTextId`, `EnMa2_UpdateTalkState`), `EnMa2_UpdateTracking`,
//! `EnMa2_IsSinging`, `EnMa2_UpdateEyes`, `EnMa2_ChangeAnim`, `EnMa2_UpdateSinging`, her actions
//! (`EnMa2_WaitToEndTalk`, `EnMa2_WaitForOcarina`, `EnMa2_WaitForEponasSong`,
//! `EnMa2_ForceTalkAfterSong`), `EnMa2_Update` and her draw aren't.
//!
//! Params: `params` unused (placed -1, read by nothing). `shape.rot.z`: 5 the stable's Malon by day
//! before Epona; 6 the ranch's by night before Epona; 7 and 8 the ranch's by day and by night after.
//! Flags: reads EVENTCHKINF_EPONA_OBTAINED (where she is, her texts). The adult's part (not
//! ported) reads INFTABLE_8D (`MALON_ADULT_SPAWN_ALWAYS` sings until it's set) and sets it for text
//! 0x2053, which she never shows (a Gossip Stone's text: unreachable); reads and sets
//! INFTABLE_PLAYED_SONG_FOR_ADULT_MALON (Epona's Song played to her) and
//! INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG (text 0x2051 closed).

use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, Source};
use oot_game::{event_chk_inf, inf_table};
use oot_game::play::PlayState;

/// `ACTOR_EN_MA2` (`actor_table.h`: 0x00D9).
pub const ACTOR_EN_MA2: i16 = 0x00D9;
pub const OBJECT: &str = "object_ma2";

/// `En_Ma2_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_MA2,
    name: "En_Ma2",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`: placed -1; nothing reads it.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed -1; read by nothing");
/// `this->actor.shape.rot.z` (`EnMa2_GetSpawnIndex`): which Malon.
pub const ROT_Z: Field = Field::rot(
    "shape.rot.z",
    Source::RotZ,
    0,
    16,
    Kind::Named(&[
        (5, "the stable by day, before Epona"),
        (6, "the ranch by night, before Epona (Epona's Song)"),
        (7, "the ranch by day, after Epona"),
        (8, "the ranch by night, after Epona"),
    ]),
    "which adult Malon, by the scene, the time and Epona",
);

// `EVENTCHKINF_*`, `INFTABLE_*` (`save.h`).
pub const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
pub const INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG: u16 = 0x8C;
pub const INFTABLE_8D: u16 = 0x8D;
pub const INFTABLE_PLAYED_SONG_FOR_ADULT_MALON: u16 = 0x8E;

/// En_Ma2's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS, ROT_Z],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("where she is; her texts"),
        FlagUse::new(inf_table!(INFTABLE_8D)).reads().sets().note("MALON_ADULT_SPAWN_ALWAYS sings until set; set by text 0x2053, which she never shows").indirect("not ported (the adult's)"),
        FlagUse::new(inf_table!(INFTABLE_PLAYED_SONG_FOR_ADULT_MALON)).reads().sets().note("Epona's Song played to her").indirect("not ported (the adult's)"),
        FlagUse::new(inf_table!(INFTABLE_TALKED_TO_ADULT_MALON_AFTER_SONG)).reads().sets().note("text 0x2051 closed").indirect("not ported (the adult's)"),
    ],
    todo: false,
    objects: &[],
};

// `SCENE_*` (`scene_table.h`).
const SCENE_STABLE: u16 = 0x36;
const SCENE_LON_LON_RANCH: u16 = 0x63;

// `EnMa2SpawnIndex`.
pub const MALON_ADULT_SPAWN_NONE: u16 = 0;
pub const MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA: u16 = 1;
pub const MALON_ADULT_SPAWN_NIGHT_RANCH_NO_EPONA: u16 = 2;
pub const MALON_ADULT_SPAWN_ALWAYS: u16 = 3;

/// Adult Malon killed by her init (`MALON_ADULT_SPAWN_NONE`).
pub struct EnMa2 {
    pub actor: Actor,
}

impl EnMa2 {
    /// `EnMa2_Init`: killed for `MALON_ADULT_SPAWN_NONE`; else, a placeholder.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnMa2 { actor };
        match Self::get_spawn_index(&this.actor, play) {
            MALON_ADULT_SPAWN_NONE => {
                this.actor.kill();
                Box::new(this)
            }
            i => {
                log::info!("En_Ma2: adult Malon (Phase 13) isn't ported: a placeholder (spawn index {i})");
                oot_game::spawn::Placeholder::init(this.actor, play)
            }
        }
    }

    /// `EnMa2_GetSpawnIndex`.
    pub fn get_spawn_index(actor: &Actor, play: &PlayState) -> u16 {
        let s = &play.save;
        if !s.adult {
            return MALON_ADULT_SPAWN_NONE;
        }
        let epona = s.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
        let rot_z = ROT_Z.get(actor.shape_rot.z);
        if !epona && play.scene_id == SCENE_STABLE && s.is_day() && rot_z == 5 {
            return MALON_ADULT_SPAWN_DAY_STABLES_NO_EPONA;
        }
        if !epona && play.scene_id == SCENE_LON_LON_RANCH && !s.is_day() && rot_z == 6 {
            return MALON_ADULT_SPAWN_NIGHT_RANCH_NO_EPONA;
        }
        if !epona || play.scene_id != SCENE_LON_LON_RANCH {
            return MALON_ADULT_SPAWN_NONE;
        }
        if rot_z == 7 && s.is_day() {
            return MALON_ADULT_SPAWN_ALWAYS;
        }
        if rot_z == 8 && !s.is_day() {
            return MALON_ADULT_SPAWN_ALWAYS;
        }
        MALON_ADULT_SPAWN_NONE
    }
}

impl ActorImpl for EnMa2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, _play: &mut PlayState) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
