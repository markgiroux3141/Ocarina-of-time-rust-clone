//! `En_River_Sound` (`ovl_En_River_Sound/z_en_river_sound.c`): an invisible actor that makes a
//! place sound. `params` bits 0..7 are the kind (`RiverSoundType`), bits 8..15 a path for the
//! rivers. A river's sound follows Link along its path (the nearest point of the path's line,
//! `EnRiverSound_GetSfxPos`), at a frequency by the current under it (the floor's conveyor
//! speed). The others sound where they stand (the waterfalls, the lava, the torches, Kokiri
//! Forest's small waterfall), in the middle of the screen (the sandstorm, the Chamber of Sages,
//! the rumbling), or play music nearby (Saria's Song in the Lost Woods and in Goron City, the
//! Great Fairy's). Some kinds act once at init and go: the nature ambience, Ganon's Tower's
//! music levels.
//!
//! The whole overlay is ported. The sounds are its draw's (`EnRiverSound_Draw`), made where
//! `Actor_DrawAll` makes them (`ActorImpl::draw_sfx`), and not on the first frame.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_4, ACTOR_FLAG_5, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorContext, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::bgm::SariaPos;
use oot_game::audio::sfx::*;
use oot_game::audio::{NA_BGM_GREAT_FAIRY, NA_BGM_SARIA_THEME, NATURE_ID_KOKIRI_REGION};
use oot_game::item::{QUEST_SONG_LULLABY, QUEST_SONG_SARIA};
use oot_game::play::PlayState;
use oot_game::surface::SurfaceType;

/// `ACTOR_EN_RIVER_SOUND` (`actor_table.h`: 0x003B).
pub const ACTOR_EN_RIVER_SOUND: i16 = 0x003B;

/// `En_River_Sound_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_RIVER_SOUND, name: "En_River_Sound", category: ACTORCAT_BG, flags: ACTOR_FLAG_4 | ACTOR_FLAG_5, object: "gameplay_keep" };

/// `RiverSoundType` (`z_en_river_sound.h`).
pub const RS_RIVER_DEFAULT_LOW_FREQ: i16 = 0x00;
pub const RS_RIVER_DEFAULT_MEDIUM_FREQ: i16 = 0x04;
pub const RS_RIVER_DEFAULT_HIGH_FREQ: i16 = 0x05;
pub const RS_LOWER_MAIN_BGM_VOLUME: i16 = 0x0B;
pub const RS_LOST_WOODS_SARIAS_SONG: i16 = 0x0C;
pub const RS_GORON_CITY_SARIAS_SONG: i16 = 0x0D;
pub const RS_SANDSTORM: i16 = 0x0E;
pub const RS_CHAMBER_OF_SAGES_1: i16 = 0x10;
pub const RS_CHAMBER_OF_SAGES_2: i16 = 0x11;
pub const RS_RUMBLING: i16 = 0x12;
pub const RS_GREAT_FAIRY: i16 = 0x13;
pub const RS_NATURE_AMBIENCE: i16 = 0xF7;
pub const RS_GANON_TOWER_0: i16 = 0xF8;

/// `CONVEYOR_SPEED_DISABLED`, `CONVEYOR_SPEED_MAX` (`z64bgcheck.h`).
const CONVEYOR_SPEED_DISABLED: u8 = 0;
const CONVEYOR_SPEED_MAX: u8 = 4;

/// `SCENE_DDAN_BOSS` (`scene_table.h`: 0x12).
const SCENE_DDAN_BOSS: u16 = 0x12;

/// `EnRiverSound_Draw`'s `soundEffects`, by kind (0 for the kinds that play something else).
const SOUND_EFFECTS: [u16; 22] = [
    0,
    NA_SE_EV_WATER_WALL - SFX_FLAG,
    NA_SE_EV_MAGMA_LEVEL - SFX_FLAG,
    NA_SE_EV_WATER_WALL_BIG - SFX_FLAG,
    0,
    0,
    NA_SE_EV_MAGMA_LEVEL_M - SFX_FLAG,
    NA_SE_EV_MAGMA_LEVEL_L - SFX_FLAG,
    NA_SE_EV_WATERDROP - SFX_FLAG,
    NA_SE_EV_FOUNTAIN - SFX_FLAG,
    NA_SE_EV_CROWD - SFX_FLAG,
    0,
    NA_SE_EV_SARIA_MELODY - SFX_FLAG,
    0,
    NA_SE_EV_SAND_STORM - SFX_FLAG,
    NA_SE_EV_WATER_BUBBLE - SFX_FLAG,
    NA_SE_EV_KENJA_ENVIROMENT_0 - SFX_FLAG,
    NA_SE_EV_KENJA_ENVIROMENT_1 - SFX_FLAG,
    NA_SE_EV_EARTHQUAKE - SFX_FLAG,
    0,
    NA_SE_EV_TORCH - SFX_FLAG,
    NA_SE_EV_COW_CRY_LV - SFX_FLAG,
];

/// `EnRiverSound_Draw`'s `sfxFreqs`, by the current (`CONVEYOR_SPEED_SLOW` ... `_FAST`, less 1).
const SFX_FREQS: [f32; (CONVEYOR_SPEED_MAX - 1) as usize] = [0.7, 1.0, 1.4];

pub struct EnRiverSound {
    pub actor: Actor,
    /// `playSfx`: false until its first draw.
    pub play_sfx: bool,
    pub sfx_freq_index: u8,
    pub path_index: i16,
    /// Its own handle, for the C's pointers into it (`&this->actor.projectedPos`,
    /// `&this->actor.home.pos`).
    me: Option<ActorHandle>,
}

impl EnRiverSound {
    /// `EnRiverSound_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let path_index = (actor.params >> 8) & 0xFF;
        actor.params &= 0xFF;
        if actor.params >= RS_GANON_TOWER_0 {
            // Each room of the climb plays NA_BGM_GANON_TOWER louder.
            play.audio.set_ganons_tower_bgm_volume_level((actor.params - RS_GANON_TOWER_0) as u8);
            actor.kill();
        } else if actor.params == RS_NATURE_AMBIENCE {
            play.audio.play_nature_ambience_sequence(NATURE_ID_KOKIRI_REGION);
            actor.kill();
        } else if actor.params == RS_LOST_WOODS_SARIAS_SONG {
            let quest = play.save.inventory.quest_items;
            if quest & (1 << QUEST_SONG_LULLABY) == 0 || quest & (1 << QUEST_SONG_SARIA) != 0 {
                actor.kill();
            }
        }
        Box::new(EnRiverSound { actor, play_sfx: false, sfx_freq_index: 0, path_index, me: None })
    }

    fn is_river(&self) -> bool {
        matches!(self.actor.params, RS_RIVER_DEFAULT_LOW_FREQ | RS_RIVER_DEFAULT_MEDIUM_FREQ | RS_RIVER_DEFAULT_HIGH_FREQ)
    }
}

/// `EnRiverSound_FindClosestPointOnLineSegment`: the point of the line through `a` and `b`
/// nearest `hear_pos`, if it lies between them.
fn find_closest_point_on_line_segment(a: Vec3, b: Vec3, hear_pos: Vec3) -> Option<Vec3> {
    let s0 = a - hear_pos;
    let s1 = b - hear_pos;
    let s2 = s1 - s0;
    let dot = |u: Vec3, v: Vec3| u.x * v.x + u.y * v.y + u.z * v.z;
    let mut temp = dot(s2, s0);
    if (dot(s2, s1) * temp) < 0.0 {
        temp = -temp / (s2.x * s2.x + s2.y * s2.y + s2.z * s2.z);
        return Some(Vec3::new((s2.x * temp) + a.x, (s2.y * temp) + a.y, (s2.z * temp) + a.z));
    }
    None
}

/// `EnRiverSound_GetSfxPos`: where along the path `points` the river sounds for `hear_pos`
/// (written to `sfx_pos`), unless every point is 10000 or more away.
fn get_sfx_pos(points: &[Vec3], hear_pos: Vec3, sfx_pos: &mut Vec3) -> bool {
    let mut closest_point_dist = 10000.0f32;
    let mut closest_point_idx = 0;
    for (i, &point) in points.iter().enumerate() {
        // Math_Vec3f_DistXYZ.
        let d = point - hear_pos;
        let dist = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
        if dist < closest_point_dist {
            closest_point_dist = dist;
            closest_point_idx = i;
        }
    }
    if closest_point_dist >= 10000.0 {
        return false;
    }
    let closest = points[closest_point_idx];
    let prev = if closest_point_idx != 0 { find_closest_point_on_line_segment(points[closest_point_idx - 1], closest, hear_pos) } else { None };
    let next = if closest_point_idx + 1 != points.len() { find_closest_point_on_line_segment(closest, points[closest_point_idx + 1], hear_pos) } else { None };
    *sfx_pos = match (prev, next) {
        (Some(p), Some(n)) => find_closest_point_on_line_segment(p, n, hear_pos).unwrap_or(Vec3::new((p.x + n.x) * 0.5, (p.y + n.y) * 0.5, (p.z + n.z) * 0.5)),
        (Some(p), None) => p,
        (None, Some(n)) => n,
        (None, None) => closest,
    };
    true
}

/// What `sSariaBgmPtr` reads: the actor's `projectedPos` or `home.pos` (this actor's from `own`,
/// it being out of the arena while it runs).
fn read_saria(actors: &ActorContext, me: Option<ActorHandle>, own: (Vec3, Vec3)) -> impl Fn(SariaPos) -> Vec3 + '_ {
    move |p| match p {
        SariaPos::Projected(h) if Some(h) == me => own.0,
        SariaPos::Home(h) if Some(h) == me => own.1,
        SariaPos::Projected(h) => actors.actor(h).map(|a| a.projected_pos).unwrap_or_default(),
        SariaPos::Home(h) => actors.actor(h).map(|a| a.home_pos).unwrap_or_default(),
    }
}

impl ActorImpl for EnRiverSound {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnRiverSound_Update`: a river finds where on its path it sounds, and the current there;
    /// the music makers keep Link's position relative to them in `home.pos`
    /// (`func_8002DBD0`); in Gohma's room, a cleared room ends it.
    fn update(&mut self, play: &mut PlayState) {
        self.me = play.cur_actor;
        let Some(player) = play.player.and_then(|h| play.actors.actor(h)) else { return };
        let player_pos = player.world_pos;
        if self.is_river() {
            let points: Vec<Vec3> = match play.setup_path_list().get(self.path_index as usize) {
                Some(path) => (0..path.count()).map(|i| path.point(i)).collect(),
                None => return,
            };
            let mut pos = self.actor.world_pos;
            if get_sfx_pos(&points, player_pos, &mut pos) {
                self.actor.world_pos = pos;
                // BgCheck_EntityRaycastDown4.
                let (y, poly) = play.col.entity_raycast_down(pos);
                self.actor.floor_poly = poly;
                self.sfx_freq_index = match poly {
                    Some(p) if y != eng_collision::bgcheck::BGCHECK_Y_MIN => play.col.conveyor_speed(p) as u8,
                    _ => CONVEYOR_SPEED_DISABLED,
                };
                if self.sfx_freq_index == CONVEYOR_SPEED_DISABLED {
                    self.sfx_freq_index = match self.actor.params {
                        RS_RIVER_DEFAULT_MEDIUM_FREQ => 0,
                        RS_RIVER_DEFAULT_LOW_FREQ => 1,
                        _ => 2,
                    };
                } else {
                    self.sfx_freq_index = self.sfx_freq_index.wrapping_sub(1).min(CONVEYOR_SPEED_MAX - 2);
                }
            }
        } else if self.actor.params == RS_GORON_CITY_SARIAS_SONG || self.actor.params == RS_GREAT_FAIRY {
            self.actor.home_pos = player.world_to_actor_coords(self.actor.world_pos);
        } else if play.scene_id == SCENE_DDAN_BOSS && play.flags.get_clear(self.actor.room) {
            self.actor.kill();
        }
    }

    /// `EnRiverSound_Draw`: nothing the first time; then the kind's sound or music.
    fn draw_sfx(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        if !self.play_sfx {
            self.play_sfx = true;
            return;
        }
        self.me = play.cur_actor;
        let me = self.me;
        let own = (self.actor.projected_pos, self.actor.home_pos);
        match self.actor.params {
            _ if self.is_river() => {
                let pos = me.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
                play.audio.play_sfx_river(pos, SFX_FREQS[(self.sfx_freq_index as usize).min(SFX_FREQS.len() - 1)]);
            }
            // The market's entrance and back alley: the main bgm down to 90.
            RS_LOWER_MAIN_BGM_VOLUME => play.audio.lower_main_bgm_volume(90),
            // Saria's Song at the Lost Woods' next right way, by the distance.
            RS_LOST_WOODS_SARIAS_SONG => {
                let Some(h) = me else { return };
                let read = read_saria(&play.actors, me, own);
                play.audio.func_800f4e30(SariaPos::Projected(h), self.actor.xz_dist_to_player, &read);
            }
            RS_GORON_CITY_SARIAS_SONG | RS_GREAT_FAIRY => {
                let Some(h) = me else { return };
                let (seq_id, dist_max) = if self.actor.params == RS_GORON_CITY_SARIAS_SONG { (NA_BGM_SARIA_THEME, 1000) } else { (NA_BGM_GREAT_FAIRY, 800) };
                let read = read_saria(&play.actors, me, own);
                play.audio.play_saria_bgm(SariaPos::Home(h), seq_id, dist_max, &read);
            }
            RS_SANDSTORM | RS_CHAMBER_OF_SAGES_1 | RS_CHAMBER_OF_SAGES_2 | RS_RUMBLING => {
                play.audio.func_800788cc(SOUND_EFFECTS.get(self.actor.params as usize).copied().unwrap_or(0));
            }
            p => audio_play_actor_sfx2(play, SOUND_EFFECTS.get(p as usize).copied().unwrap_or(0)),
        }
    }

    /// `EnRiverSound_Destroy`: the music it played lets go of it.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.actor.params == RS_LOST_WOODS_SARIAS_SONG {
            if let Some(h) = self.me {
                play.audio.clear_saria_bgm_at_pos(SariaPos::Projected(h));
            }
        } else if self.actor.params == RS_GORON_CITY_SARIAS_SONG {
            play.audio.clear_saria_bgm2();
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
