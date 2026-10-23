//! `Demo_6K` (`ovl_Demo_6K/z_demo_6k.c`): the cutscenes' balls of light, by params (0 to 19).
//!
//! **Types 14 to 19, the sages' lights** (`func_80967410`; Temple of Time room 0's 0x10, Death
//! Mountain Trail's layer 8 and Lon Lon Ranch's layer 10 place them): each follows the cue on
//! channel `params - 14` (`func_809691BC`: from the cue's start to its end by the script's frame,
//! `Environment_LerpWeight`), at scale 0.2, its `timer2` counting up. While the cue's id is 3 it's
//! drawn and throws a sparkle every frame (`func_80967244`: `EffectSsKiraKira_SpawnFocused` 20
//! out at a random angle, its colour `sEnvColors[params - 14]`, scale 6000 in the Temple of Time,
//! else 6000 before script frame 419 and 18000 after, 20 frames); otherwise, and with no cue or
//! no script, hidden (`flags` bit 0). Drawn (`func_8096865C`, `params > 8`: `gEffFlash1DL`) as
//! two flashes facing the camera turning opposite ways by `timer2 * 6`°, prim (0x80, 0x80) white,
//! env its colour. Not culled when drawn (`ACTOR_FLAG_DRAW_CULLING_DISABLED`).
//!
//! The shared code ported with them: `Demo6K_Init` (its switch: types 14 to 19 whole, the others
//! below), `Demo6K_Destroy` and the point light every type inserts (white, radius 100, at its
//! placement; types 14 to 19 never move it), `Demo6K_WaitForObject` (their object is
//! `gameplay_keep`'s, slot 0: the first update shows them and picks their action),
//! `func_809691BC`, `func_80967244`, `func_8096865C`'s branch for them, `Demo6K_Update`.
//!
//! **Not ported (the user's call): the other types, placeholders chosen at init** with a
//! `log::info!`: 0 (the rainbow bridge's lights, outside Ganon's Castle's layers 0 to 4: Phase
//! 19), 1 (the sages' light gathering, the Chamber of the Sages after Ganon, layer 6, then type 2:
//! Phase 19), 2 (the seal's flash, `object_demo_6k`, `Eff_Dust`: Phase 19), 3 to 8 (the six sages'
//! balls, Inside Ganon's Castle: Phase 19), 9 to 11 (the trials' barriers' balls: Phase 19), 12
//! and 13 (Ganon's magic, `object_gnd_magic`: Phase 19; 12 is `En_Viewer`'s, from the Ganondorf
//! cutscene, and it stays a placeholder). Rauru's (`En_Rl` type 2) spawns type 5.
//!
//! Params: `params` whole, the type (0 to 19; 14 to 19 ported, their cue channel `params - 14`
//! and colour `sEnvColors[params - 14]`: red, green, yellow-green, purple, orange, blue). Any
//! other value is `Demo6K_Init`'s `ASSERT`.
//! Flags: none.

use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind, ObjectNeed};
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::lights::{LightInfo, LightNode};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

use crate::demo_effect::BAKE_FLASH;

/// `ACTOR_DEMO_6K` (`actor_table.h`: 0x00F5).
pub const ACTOR_DEMO_6K: i16 = 0x00F5;
/// `OBJECT_GAMEPLAY_KEEP` (`object_table.h`: 0x0001), the profile's.
pub const OBJECT: &str = "gameplay_keep";
/// `SCENE_TEMPLE_OF_TIME` (`scene_table.h`: 0x43).
const SCENE_TEMPLE_OF_TIME: u16 = 0x43;

/// `Demo_6K_Profile`: `ACTORCAT_PROP`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`, no draw (the init's
/// `drawFunc` is set by `Demo6K_WaitForObject`).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_DEMO_6K, name: "Demo_6K", category: ACTORCAT_PROP, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params` (`Demo6K_Init`'s switch, `sObjectIds[params]`): which light.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the rainbow bridge's lights (outside Ganon's Castle)"),
        (1, "the sages' light gathering (the Chamber of the Sages after Ganon)"),
        (2, "the seal's flash (type 1's)"),
        (3, "a sage's ball: red (Inside Ganon's Castle)"),
        (4, "a sage's ball: green"),
        (5, "a sage's ball: yellow-green (Rauru's)"),
        (6, "a sage's ball: purple"),
        (7, "a sage's ball: orange"),
        (8, "a sage's ball: blue"),
        (9, "a trial's barrier ball: red"),
        (10, "a trial's barrier ball: blue"),
        (11, "a trial's barrier ball vanishing: orange"),
        (12, "Ganon's magic (En_Viewer's)"),
        (13, "Ganon's magic ball (type 12's)"),
        (14, "a sage's light on cue channel 0: red"),
        (15, "a sage's light on cue channel 1: green"),
        (16, "a sage's light on cue channel 2: yellow-green (the Temple of Time's)"),
        (17, "a sage's light on cue channel 3: purple"),
        (18, "a sage's light on cue channel 4: orange"),
        (19, "a sage's light on cue channel 5: blue"),
    ]),
    "which light (14 to 19 ported, the others placeholders)",
);

/// Demo_6K's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[],
    todo: false,
    // sObjectIds: Demo6K_Init's Object_GetSlot for these types (the rest are gameplay_keep's).
    objects: &[
        ObjectNeed::new("object_demo_6k").when("params", &[1, 2]).note("the seal's model (not ported)"),
        ObjectNeed::new("object_gnd_magic").when("params", &[12]).note("Ganon's magic's model (not ported)"),
    ],
    texts: &[],
};

/// `sEnvColors`.
pub const ENV_COLORS: [[u8; 3]; 6] = [[255, 50, 0], [0, 200, 0], [200, 255, 0], [200, 50, 255], [255, 150, 0], [0, 150, 255]];

/// `Demo_Effect`'s flash bake's colour segment (`gEffFlash1DL` with its prim and env dynamic).
const SEG_COLOR: u8 = 0x0A;

/// `Demo6KActionFunc` (the ported ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Demo6KAction {
    /// `Demo6K_WaitForObject`.
    WaitForObject,
    /// `func_80967410`.
    SageLight,
}

pub struct Demo6K {
    pub actor: Actor,
    /// `initActionFunc`.
    pub init_action: Demo6KAction,
    pub action: Demo6KAction,
    pub light_info: LightInfo,
    pub light_node: Option<LightNode>,
    /// `flags`: bit 0 hides the draw.
    pub flags: u16,
    pub timer1: u16,
    pub timer2: u16,
    pub required_object_slot: u8,
    pub unk_293: u8,
    /// `this->actor.draw` set (`drawFunc`, by `Demo6K_WaitForObject`).
    pub drawn: bool,
}

/// `DEG_TO_RAD`.
fn deg_to_rad(d: f32) -> f32 {
    (d as f64 * (std::f64::consts::PI / 180.0)) as f32
}

impl Demo6K {
    /// `Demo6K_Init`: the action and draw for `params`, the point light; the types not ported are
    /// placeholders.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut actor = actor;
        let params = TYPE.get(actor.params);
        log::debug!("no = {params}");
        let what = match params {
            14..=19 => None,
            0 => Some("func_80967AD0: the rainbow bridge's lights, outside Ganon's Castle (Phase 19)"),
            1 => Some("func_80966E04: the sages' light gathering, the Chamber of the Sages after Ganon (Phase 19)"),
            2 => Some("func_80966F84: the seal's flash (Phase 19)"),
            3..=8 => Some("func_8096712C: a sage's ball, Inside Ganon's Castle (Phase 19)"),
            9 | 10 => Some("func_809674E0: a trial's barrier ball (Phase 19)"),
            11 => Some("func_8096784C: a trial's barrier ball vanishing (Phase 19)"),
            12 => Some("func_80967F10: Ganon's magic (Phase 19)"),
            13 => Some("func_80967DBC: Ganon's magic ball (Phase 19)"),
            _ => Some("Demo6K_Init's ASSERT: no such type"),
        };
        if let Some(what) = what {
            log::info!("Demo_6K: type {params}, {what}, isn't ported: a placeholder");
            return oot_game::spawn::Placeholder::init(actor, play);
        }
        // sObjectIds[14..=19]: OBJECT_GAMEPLAY_KEEP, slot 0.
        let required_object_slot = 0;
        log::debug!("bank_ID = {required_object_slot}");
        // case 14 .. 19.
        actor.flags |= ACTOR_FLAG_DRAW_CULLING_DISABLED;
        actor.scale = Vec3::splat(0.2);
        let p = actor.world_pos;
        let light_info = LightInfo::point_no_glow(p.x as i32 as i16, p.y as i32 as i16, p.z as i32 as i16, [255, 255, 255], 100);
        let light_node = play.light_ctx.insert_light(light_info);
        Box::new(Demo6K {
            actor,
            init_action: Demo6KAction::SageLight,
            action: Demo6KAction::WaitForObject,
            light_info,
            light_node,
            flags: 1,
            timer1: 0,
            timer2: 0,
            required_object_slot,
            unk_293: (params - 14) as u8,
            drawn: false,
        })
    }

    /// `Demo6K_WaitForObject`: the object in (`gameplay_keep` always is), the draw and the action.
    fn wait_for_object(&mut self, play: &PlayState) {
        if play.object_ctx.is_loaded(self.required_object_slot as usize) {
            self.actor.obj_bank_index = Some(self.required_object_slot as usize);
            self.drawn = true;
            self.action = self.init_action;
        }
    }

    /// `func_809691BC`: along the cue on `channel` by the script's frame.
    fn func_809691bc(&mut self, play: &PlayState, channel: usize) {
        let Some(cue) = play.cs_ctx.npc_actions[channel] else { return };
        let start = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let end = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        let t = oot_game::env::lerp_weight(cue.end_frame, cue.start_frame, play.cs_ctx.frames);
        self.actor.world_pos = Vec3::new(start.x + (end.x - start.x) * t, start.y + (end.y - start.y) * t, start.z + (end.z - start.z) * t);
    }

    /// `func_80967244`: a sparkle thrown 20 out at a random angle, in the light's colour.
    fn func_80967244(&mut self, play: &mut PlayState) {
        let pos = self.actor.world_pos;
        // (s16)Rand_ZeroFloat(0xFFFF): through an s32, wrapping.
        let rand1 = play.rand.zero_float(65535.0) as i32 as i16;
        let rand2 = play.rand.zero_float(65535.0) as i32 as i16;
        let velocity = Vec3::new(
            eng_math::sin_s(rand2) * eng_math::cos_s(rand1) * 20.0,
            eng_math::sin_s(rand1) * 20.0,
            eng_math::cos_s(rand2) * eng_math::cos_s(rand1) * 20.0,
        );
        let accel = Vec3::ZERO;
        let c = ENV_COLORS[self.unk_293 as usize];
        let env = [c[0], c[1], c[2], 0];
        let scale = if play.scene_id == SCENE_TEMPLE_OF_TIME || play.cs_ctx.frames < 419 { 6000 } else { 18000 };
        play.ss().0.kirakira_spawn_focused(pos, velocity, accel, [255, 255, 255, 0], env, scale, 20);
    }

    /// `func_80967410`: types 14 to 19: the cue's place, shown with sparkles on its id 3.
    fn func_80967410(&mut self, play: &mut PlayState) {
        let channel = (TYPE.get(self.actor.params) - 14) as usize;
        self.timer2 = self.timer2.wrapping_add(1);
        self.actor.scale = Vec3::splat(0.2);
        let cue = if play.cs_ctx.state != CS_STATE_IDLE { play.cs_ctx.npc_actions[channel] } else { None };
        match cue {
            Some(cue) => {
                self.func_809691bc(play, channel);
                if cue.action == 3 {
                    self.flags &= !1;
                    self.func_80967244(play);
                } else {
                    self.flags |= 1;
                }
            }
            None => self.flags |= 1,
        }
    }
}

impl ActorImpl for Demo6K {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `Demo6K_Update`: the action.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Demo6KAction::WaitForObject => self.wait_for_object(play),
            Demo6KAction::SageLight => self.func_80967410(play),
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.switches = vec![self.drawn as u32, self.flags as u32, self.unk_293 as u32, self.timer2 as u32];
        rs
    }

    /// `func_8096865C` (`params > 8`: `gEffFlash1DL`): unless hidden, two flashes facing the
    /// camera, turned `timer2 * 6`° each way.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < 4 || rs.switches[0] == 0 || rs.switches[1] & 1 != 0 {
            return;
        }
        let c = ENV_COLORS[(rs.switches[2] as usize).min(ENV_COLORS.len() - 1)];
        let timer2 = rs.switches[3] as u16;
        let m = actor_draw_matrix(rs) * view.billboard;
        let rot = deg_to_rad((timer2 as i32 * 6) as f32);
        let neg = deg_to_rad(-((timer2 as i32 * 6) as f32));
        for b in [m * Mat4::from_rotation_z(rot), m * Mat4::from_rotation_z(neg)] {
            let mut sv = SegmentValues::default();
            sv.prim[SEG_COLOR as usize] = Some([255, 255, 255, 255]);
            sv.env[SEG_COLOR as usize] = Some([c[0], c[1], c[2], 255]);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_FLASH)), transform: b, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    /// `Demo6K_Destroy`: the light out.
    fn destroy(&mut self, play: &mut PlayState) {
        play.light_ctx.remove_light(self.light_node.take());
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
