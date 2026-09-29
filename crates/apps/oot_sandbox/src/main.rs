//! Dev sandbox: the spikes' play mode. Link from the ROM on the synthetic test course or a
//! real scene, with the debug views, and the scripted headless runs that are the behaviour
//! regression suite (`scripts/golden.py`): contact sheets (`--sheet`), per-frame JSON traces
//! (`--trace`) and screenshots (`--screenshot`), all written to the git-ignored `out/`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use eng_input::pad::{BTN_A, BTN_B, BTN_Z, PadState};
use eng_render::{Renderer, Target};
use glam::Vec3;
use oot::{Assets, Options, SceneGfx, draw_frame, load_assets, new_play, new_play_at};
use oot_actors::PlayExt;
use oot_game::play::{PlayState, RenderFrame, scripted_input};

#[derive(Parser)]
#[command(about = "Dev sandbox: the test course or a scene, with scripted headless runs (contact sheets, traces, screenshots)")]
struct Cli {
    /// Load this scene from the ROM instead of the test course (e.g. spot04 = Kokiri
    /// Forest, spot00 = Hyrule Field, ydan = Deku Tree): rooms, lights, fog and collision.
    #[arg(long)]
    scene: Option<String>,
    /// Spawn point (index into the scene's spawn list).
    #[arg(long, default_value_t = 0)]
    spawn: usize,
    /// Time of day as HH:MM (`gSaveContext.dayTime`; a new save starts at 10:00).
    #[arg(long, default_value = "10:00")]
    time: String,
    /// Draw the scene's collision (coloured by surface class) instead of its rooms.
    #[arg(long)]
    collision: bool,
    /// Headless: fixed camera `eye_x,eye_y,eye_z,at_x,at_y,at_z` instead of the game camera.
    #[arg(long, value_delimiter = ',', allow_negative_numbers = true)]
    view: Vec<f32>,
    /// Use spike 03's follow camera instead of z_camera.c's (F3 toggles in the window).
    #[arg(long)]
    follow_camera: bool,
    /// Turn off foot IK (func_8008F87C) to compare (F4 toggles in the window).
    #[arg(long)]
    no_foot_ik: bool,
    /// Place a dummy Z-target this many units in front of the spawn (0 = none).
    #[arg(long, default_value_t = 0.0)]
    target: f32,
    /// Place Link at `x,y,z,yaw` (yaw in binary angle units) instead of the spawn or the
    /// script's start; works in scenes too.
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    at: Vec<f32>,
    /// Headless: contact sheet frame stride.
    #[arg(long, default_value_t = 2)]
    step: usize,
    /// Headless: run exactly this many game frames of the script (padding with idle frames or
    /// cutting it short; 0 = the script's own length).
    #[arg(long, default_value_t = 0)]
    frames: usize,
    /// Start as child Link.
    #[arg(long)]
    child: bool,
    /// Headless: run a built-in script and write a contact sheet PNG.
    #[arg(long)]
    sheet: Option<PathBuf>,
    /// Headless: run a built-in script and write a per-frame JSON trace.
    #[arg(long)]
    trace: Option<PathBuf>,
    /// Script for --sheet / --trace: run-roll, ledge, pit, stairs, walls, turn, idle, still,
    /// forward, tour, climb50, climb70, climb100, hang, ramp-stand, target, parallel, sword,
    /// swim, tread, platform, cup (C-Up: a house's viewpoint toggle), door (walk to a door and
    /// press A), open (press A where Link stands); with --entrance ENTR_SPOT04_3 also `house` (steers Link into his house and back
    /// out through the exits); with --entrance ENTR_LINK_HOME_0 --child --preset deku-tree-open
    /// also `playthrough` (GAME-02's scripted run from Link's bed into the Deku Tree), and with
    /// --entrance ENTR_LINK_HOME_0 --child (a new save) `sword-chest` (GAME-03's run through the
    /// crawlspace and past the boulder to the Kokiri Sword's chest) and `mido-shop` (on to 40
    /// rupees, the Deku Shield from the Kokiri shop, both worn, and past Mido).
    #[arg(long, default_value = "run-roll")]
    script: String,
    /// Headless: one screenshot after the script, from the chase camera.
    #[arg(long)]
    screenshot: Option<PathBuf>,
    /// Headless: draw the collision wireframe over the mesh.
    #[arg(long)]
    wire: bool,
    /// A pack file or loose folder to use instead of the default pack.
    #[arg(long)]
    pack: Option<PathBuf>,
    /// Enter by `Play_Init`: an `ENTR_*` name or index, or with no value the entrance to
    /// --scene at --spawn. Every placement spawns (ported or a placeholder), the room context
    /// loads and changes rooms, exits lead to other scenes. Without it a scene is the spikes'
    /// view: every room drawn and Player alone.
    #[arg(long, num_args = 0..=1, default_missing_value = "")]
    entrance: Option<String>,
    /// Draw markers where unported actors (placeholders) are (P toggles in the window).
    #[arg(long)]
    placeholders: bool,
    /// A debug save preset for --entrance: deku-tree-open (the Deku Tree met and his mouth
    /// open), deku-tree-dead (also the tree dead, with the Kokiri Emerald), or
    /// sword-and-40-rupees (the Kokiri Sword worn and 40 rupees, for the shop).
    #[arg(long)]
    preset: Option<String>,
    /// Headless: also a screenshot after each of these frames, next to --screenshot
    /// (`<name>_<frame>.png`).
    #[arg(long, value_delimiter = ',')]
    shots_at: Vec<usize>,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
}

fn options(cli: &Cli) -> Options {
    Options {
        scene: cli.scene.clone(),
        spawn: cli.spawn,
        time: cli.time.clone(),
        collision: cli.collision,
        view: cli.view.clone(),
        follow_camera: cli.follow_camera,
        no_foot_ik: cli.no_foot_ik,
        // The target script places a dummy 200 ahead unless --target says otherwise.
        target: if cli.target == 0.0 && cli.script == "target" { 200.0 } else { cli.target },
        at: cli.at.clone(),
        child: cli.child,
        pack: cli.pack.clone(),
        entrance: cli.entrance.clone(),
        placeholders: cli.placeholders,
        preset: cli.preset.clone(),
        room: None,
    }
}

// ---------------------------------------------------------------------------------------
// Scripts for the headless modes.

fn stick(x: i8, y: i8) -> PadState {
    PadState { button: 0, stick_x: x, stick_y: y }
}

fn script(name: &str) -> Result<(Vec<PadState>, Option<(Vec3, i16)>)> {
    let rep = |p: PadState, n: usize| vec![p; n];
    let mut s = Vec::new();
    let start;
    match name {
        // The brief's example: stand, full stick forward for 40 frames, then A.
        "run-roll" => {
            s.extend(rep(stick(0, 0), 6));
            s.extend(rep(stick(0, 80), 40));
            s.push(PadState { button: BTN_A, stick_x: 0, stick_y: 80 });
            s.extend(rep(stick(0, 80), 14));
            s.extend(rep(stick(0, 0), 20));
            start = Some((Vec3::new(-300.0, 0.0, 800.0), -0x8000));
        }
        "ledge" => {
            s.extend(rep(stick(0, 0), 4));
            s.extend(rep(stick(0, 80), 26));
            s.extend(rep(stick(0, 0), 30));
            start = Some((Vec3::new(-700.0, 150.0, -700.0), 0x4000));
        }
        "pit" => {
            s.extend(rep(stick(0, 80), 14));
            s.extend(rep(stick(0, 0), 46));
            start = Some((Vec3::new(600.0, 0.0, -300.0), -0x8000));
        }
        "stairs" => {
            s.extend(rep(stick(0, 80), 50));
            start = Some((Vec3::new(150.0, 0.0, -300.0), -0x8000));
        }
        "walls" => {
            s.extend(rep(stick(0, 80), 60));
            start = Some((Vec3::new(-300.0, 0.0, 600.0), -0x8000));
        }
        "turn" => {
            s.extend(rep(stick(-26, 0), 12));
            s.extend(rep(stick(80, 0), 12));
            s.extend(rep(stick(0, -80), 16));
            start = Some((Vec3::new(-300.0, 0.0, 900.0), -0x8000));
        }
        "idle" => {
            s.extend(rep(stick(0, 0), 120));
            start = None;
        }
        // Run into the 50 / 70 / 100 high ledges (climb classes 2, 3, 4).
        "climb50" | "climb70" | "climb100" => {
            s.extend(rep(stick(0, 80), 45));
            s.extend(rep(stick(0, 0), 15));
            let z = match name { "climb50" => 550.0, "climb70" => 700.0, _ => 850.0 };
            start = Some((Vec3::new(560.0, 0.0, z), 0x4000));
        }
        // Walk slowly off the plateau, hang, then climb back up.
        "hang" => {
            s.extend(rep(stick(0, 30), 32));
            s.extend(rep(stick(0, 0), 16));
            s.extend(rep(stick(0, 80), 40));
            start = Some((Vec3::new(-540.0, 150.0, -700.0), 0x4000));
        }
        // Z-targeting a dummy 200 ahead: lock on, sidestep left, side hop, backflip, let go.
        "target" => {
            let z = |p: PadState| PadState { button: p.button | BTN_Z, ..p };
            s.push(stick(0, 0));
            s.push(z(stick(0, 0)));
            s.extend(rep(stick(0, 0), 12));
            s.extend(rep(stick(-80, 0), 24));
            s.push(PadState { button: BTN_A, stick_x: -80, stick_y: 0 });
            s.extend(rep(stick(0, 0), 16));
            s.push(stick(0, -80));
            s.push(PadState { button: BTN_A, stick_x: 0, stick_y: -80 });
            s.extend(rep(stick(0, 0), 20));
            s.push(z(stick(0, 0)));
            s.extend(rep(stick(0, 0), 6));
            start = None;
        }
        // Parallel mode: hold Z with nothing to target, strafe left, walk back, release.
        "parallel" => {
            let z = |p: PadState| PadState { button: p.button | BTN_Z, ..p };
            s.push(stick(0, 0));
            s.extend(rep(z(stick(0, 0)), 4));
            s.extend(rep(z(stick(-80, 0)), 16));
            s.extend(rep(z(stick(0, -80)), 16));
            s.extend(rep(stick(0, 0), 8));
            start = None;
        }
        // B three times (draw the sword and slash, slash, combo finisher), then A puts it away.
        "sword" => {
            let b = PadState { button: BTN_B, stick_x: 0, stick_y: 0 };
            s.extend(rep(stick(0, 0), 2));
            for _ in 0..3 {
                s.push(b);
                s.extend(rep(stick(0, 0), 14));
            }
            s.extend(rep(stick(0, 0), 10));
            s.push(PadState { button: BTN_A, stick_x: 0, stick_y: 0 });
            s.extend(rep(stick(0, 0), 16));
            start = None;
        }
        // Run down the pool's ramp into deep water, tread water, dive holding A, rise and
        // surface, then swim back up the ramp and out.
        "swim" => {
            let a = PadState { button: BTN_A, stick_x: 0, stick_y: 0 };
            s.extend(rep(stick(0, 80), 30));
            s.extend(rep(stick(0, 0), 36));
            s.extend(rep(a, 50));
            s.extend(rep(stick(0, 0), 70));
            s.extend(rep(stick(0, -80), 80));
            start = Some((Vec3::new(-450.0, 0.0, 800.0), -0x4000));
        }
        // Ride the moving platform (Bg_Ydan_Hasi) across the channel, then run off its far
        // edge and jump to the bank.
        "platform" => {
            s.extend(rep(stick(0, 0), 44));
            s.extend(rep(stick(0, 80), 24));
            s.extend(rep(stick(0, 0), 12));
            start = Some((Vec3::new(650.0, 10.0, -200.0), 0x4000));
        }
        // Tread water, then swim a lap (for scenes, with --at).
        "tread" => {
            s.extend(rep(stick(0, 0), 60));
            s.extend(rep(stick(0, 80), 40));
            s.extend(rep(stick(80, 40), 20));
            start = None;
        }
        // Standing across ramp A (20.6°), for foot IK.
        "ramp-stand" => {
            s.extend(rep(stick(0, 0), 20));
            start = Some((Vec3::new(-700.0, 80.0, -300.0), 0x4000));
        }
        // A few frames standing at the spawn (for screenshots).
        "still" => {
            s.extend(rep(stick(0, 0), 8));
            start = None;
        }
        // Stand, press C-Up (a house's fixed / pivot camera toggle), stand.
        "cup" => {
            s.extend(rep(stick(0, 0), 30));
            s.push(PadState { button: eng_input::pad::BTN_CUP, stick_x: 0, stick_y: 0 });
            s.extend(rep(stick(0, 0), 20));
            start = None;
        }
        // Stand (facing a door), press A, and wait for the door and its camera.
        "open" => {
            s.extend(rep(stick(0, 0), 30));
            s.push(PadState { button: BTN_A, stick_x: 0, stick_y: 0 });
            s.extend(rep(stick(0, 0), 100));
            start = None;
        }
        // Walk forward (to a door), press A, and wait for the door and its camera.
        "door" => {
            s.extend(rep(stick(0, 60), 12));
            s.extend(rep(stick(0, 0), 2));
            s.push(PadState { button: BTN_A, stick_x: 0, stick_y: 0 });
            s.extend(rep(stick(0, 0), 90));
            start = None;
        }
        // Run, curve left, curve right, then stop and let the camera recentre.
        "tour" => {
            s.extend(rep(stick(0, 80), 20));
            s.extend(rep(stick(-60, 60), 25));
            s.extend(rep(stick(60, 60), 25));
            s.extend(rep(stick(0, 0), 50));
            start = None;
        }
        // Walk forward from the spawn, then stand.
        "forward" => {
            s.extend(rep(stick(0, 80), 40));
            s.extend(rep(stick(0, 0), 10));
            start = None;
        }
        other => anyhow::bail!("unknown script {other}"),
    }
    Ok((s, start))
}

/// The `house` script: from Link's porch (`ENTR_SPOT04_3`), in through the door and back out,
/// steering at each frame like a player would.
struct HouseWalk {
    phase: usize,
    wait: usize,
}

impl HouseWalk {
    /// The next frame's pad, or `None` when done.
    fn next(&mut self, w: &PlayState) -> Option<PadState> {
        use oot_game::transition::{TRANS_MODE_OFF, TRANS_TRIGGER_OFF};
        let idle = PadState::default();
        let settled = w.transition.trigger == TRANS_TRIGGER_OFF && w.transition.mode == TRANS_MODE_OFF && format!("{:?}", w.player().action) != "ExitWalk";
        let target = |name: &str| oot_actors::script::exit_to(w, name).map(|(_, c)| oot_actors::script::stick_towards(w, c, 60.0));
        loop {
            match self.phase {
                // Arrive: the fade in and the start mode's walk.
                0 | 2 => {
                    if settled {
                        self.wait += 1;
                        if self.wait > 10 {
                            self.phase += 1;
                            self.wait = 0;
                            continue;
                        }
                    }
                    return Some(idle);
                }
                // Walk into the exit.
                1 | 3 => {
                    if w.transition.trigger != TRANS_TRIGGER_OFF {
                        self.phase += 1;
                        self.wait = w.scene_changes as usize;
                        continue;
                    }
                    let name = if self.phase == 1 { "ENTR_LINK_HOME_1" } else { "ENTR_SPOT04_3" };
                    return Some(target(name).unwrap_or(idle));
                }
                // Wait for the scene change, then settle.
                4 => {
                    if w.scene_changes as usize > self.wait {
                        self.phase = 5;
                        self.wait = 0;
                        continue;
                    }
                    return Some(idle);
                }
                5 => {
                    if settled {
                        self.wait += 1;
                        if self.wait > 10 {
                            return None;
                        }
                    }
                    return Some(idle);
                }
                _ => return None,
            }
        }
    }
}

/// The script's play state: the script's start, or `--at`, or the spawn or entrance.
fn script_play(a: &Assets, cli: &Cli) -> Result<PlayState> {
    let route = oot_actors::playthrough::Route::from_script(&cli.script);
    if let Some(r) = route
        && (a.entrance.is_none() || cli.preset.as_deref() != r.preset() || !cli.child)
    {
        let preset = r.preset().map(|p| format!(" --preset {p}")).unwrap_or_default();
        anyhow::bail!("the {} script needs --entrance {} --child{preset}", r.script(), r.entrance());
    }
    let start = if cli.script == "house" || route.is_some() { None } else { script(&cli.script)?.1 };
    let mut w = new_play(a, cli.child);
    if let (Some((p, y)), None) = (start, &cli.scene) {
        w = new_play_at(a, cli.child, p, y, true);
    }
    if let [x, y, z, yaw] = cli.at[..] {
        if a.entrance.is_some() {
            // Entered by Play_Init: every actor stays; Player moves.
            w.place_player(Vec3::new(x, y, z), yaw as i32 as i16);
        } else {
            // (The spikes placed no targets with --at.)
            w = new_play_at(a, cli.child, Vec3::new(x, y, z), yaw as i32 as i16, false);
        }
    }
    Ok(w)
}

fn run_script(mut w: PlayState, cli: &Cli, on_frame: &mut dyn FnMut(&PlayState, usize, &RenderFrame) -> Result<()>) -> Result<(PlayState, Vec<RenderFrame>, Vec<serde_json::Value>)> {
    let house = cli.script == "house";
    // The playthroughs steer themselves (oot_actors::playthrough) and mark each step in the trace.
    let mut playthrough = oot_actors::playthrough::Route::from_script(&cli.script).map(oot_actors::playthrough::Playthrough::for_route);
    let mut s = if house || playthrough.is_some() { Vec::new() } else { script(&cli.script)?.0 };
    if cli.frames > 0 && !house && playthrough.is_none() {
        s.resize(cli.frames, stick(0, 0));
    }
    let mut snaps = Vec::new();
    let mut trace: Vec<serde_json::Value> = Vec::new();
    let mut prev = PadState::default();
    let mut walk = HouseWalk { phase: 0, wait: 0 };
    // Scene changes by frame, for the trace; the house walk and its phases ends the run.
    for i in 0.. {
        let cur = if let Some(run) = &mut playthrough {
            if cli.frames > 0 && i >= cli.frames {
                break;
            }
            let pad = run.next(&w);
            // A step is done on the state after the last frame.
            if let (Some(step), Some(t)) = (run.take_done(), trace.last_mut()) {
                t["step"] = serde_json::json!(step.name());
            }
            match pad {
                Some(p) => p,
                None => break,
            }
        } else if house {
            if i >= 2000 {
                anyhow::bail!("the house walk didn't finish (phase {})", walk.phase);
            }
            if cli.frames > 0 && i >= cli.frames {
                break;
            }
            match walk.next(&w) {
                Some(p) => p,
                None => break,
            }
        } else {
            match s.get(i) {
                Some(p) => *p,
                None => break,
            }
        };
        let cur = &cur;
        w.tick_with(scripted_input(prev, *cur));
        prev = *cur;
        let targets = w.targets();
        let p = w.player();
        trace.push(serde_json::json!({
            "frame": i + 1,
            "input": { "x": cur.stick_x, "y": cur.stick_y, "buttons": cur.names() },
            "action": format!("{:?}", p.action),
            "decomp_action": p.action.decomp_name(),
            "pos": [p.actor.world_pos.x, p.actor.world_pos.y, p.actor.world_pos.z],
            "linear_velocity": p.linear_velocity,
            "velocity_y": p.actor.velocity.y,
            "current_yaw": p.current_yaw,
            "shape_yaw": p.actor.shape_rot.y,
            "grounded": p.grounded(),
            "anim": w.data.anim_name(p.skel.animation),
            "anim_frame": p.skel.cur_frame,
            "walk_phase": p.unk_868,
            // The index of the locked-on target among the dummy targets, as the spikes wrote it.
            "target": p.unk_664.and_then(|h| targets.iter().position(|&t| t == h)),
            "parallel": p.state1 & oot_actors::player::STATE1_17 != 0,
            "locked_on": p.state1 & oot_actors::player::STATE1_4 != 0,
            "root_joint": p.skel.joint[0],
            "root_rot": p.skel.joint[1],
            "y_offset": p.actor.shape_y_offset,
            "camera": { "eye": w.game_camera.eye.to_array(), "at": w.game_camera.at.to_array(), "dist": w.game_camera.dist, "fov": w.game_camera.fov, "input_yaw": w.game_camera.input_dir_yaw() },
        }));
        if w.assets.is_some() {
            // Play_Init runs: the scene, the rooms and the transition.
            let t = trace.last_mut().unwrap();
            t["scene"] = serde_json::json!(w.scene.as_ref().map(|s| s.short_name().to_string()));
            t["room"] = serde_json::json!([w.room_ctx.cur.num, w.room_ctx.prev.num]);
            t["transition"] = serde_json::json!({ "trigger": w.transition.trigger, "mode": w.transition.mode, "type": w.transition.ty, "next_entrance": w.transition.next_entrance_index, "fill": w.screen_fill() });
            t["entrance"] = serde_json::json!(w.save.entrance_index);
            t["actors"] = serde_json::json!(w.actors.total());
            let c = &w.game_camera;
            t["camera"]["setting"] = serde_json::json!(c.setting);
            t["camera"]["mode"] = serde_json::json!(c.mode);
            t["camera"]["bg_cam"] = serde_json::json!(c.bg_cam_index);
            t["viewpoint"] = serde_json::json!(w.viewpoint);
        }
        let snap = w.current_frame();
        on_frame(&w, i + 1, &snap)?;
        snaps.push(snap);
    }
    if let Some(run) = &playthrough {
        if let Some(f) = &run.failure {
            anyhow::bail!("the playthrough stopped: {f}");
        }
        let steps: Vec<String> = run.steps.iter().map(|(s, f)| format!("{} at frame {f}", s.name())).collect();
        println!("{}: {}; texts {:x?}; drop {:?}; boulder waits {:?}", run.route.script(), steps.join(", "), run.texts, run.drop, run.boulder_waits);
    }
    Ok((w, snaps, trace))
}

fn headless(cli: &Cli) -> Result<()> {
    let mut a = load_assets(&options(cli))?;
    let (device, queue) = eng_render::headless_device()?;
    let mut r = Renderer::new(&device, &queue);
    let mut scene = SceneGfx::new(&a);
    // Screenshots during the run (the play state is only whole then: scenes change).
    let shot_path = |frame: usize| -> Option<PathBuf> {
        let p = cli.screenshot.as_ref()?;
        let stem = p.file_stem()?.to_string_lossy().to_string();
        Some(p.with_file_name(format!("{stem}_{frame}.png")))
    };
    let mut shots: Vec<(usize, PathBuf)> = cli.shots_at.iter().filter_map(|&f| shot_path(f).map(|p| (f, p))).collect();
    let (w, snaps, trace) = {
        let w = script_play(&a, cli)?;
        run_script(w, cli, &mut |w, frame, snap| {
            if let Some(k) = shots.iter().position(|(f, _)| *f == frame) {
                let (_, p) = shots.remove(k);
                let t = Target::new(&device, cli.width, cli.height);
                draw_frame(&mut r, &device, &queue, &t, &mut a, &mut scene, w, snap, cli.wire);
                let mut px = t.read_rgba(&device, &queue)?;
                if let Some(f) = w.screen_fill() {
                    oot::rooms::apply_fill(&mut px, f);
                }
                eng_app::save_png(&p, cli.width, cli.height, &px)?;
                println!("{}", p.display());
            }
            Ok(())
        })?
    };
    if let Some(p) = &cli.trace {
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, serde_json::to_string_pretty(&serde_json::json!({ "script": cli.script, "place": a.place, "frames": trace }))?)?;
        println!("{} ({} frames)", p.display(), trace.len());
    }
    if let Some(p) = &cli.screenshot {
        let t = Target::new(&device, cli.width, cli.height);
        draw_frame(&mut r, &device, &queue, &t, &mut a, &mut scene, &w, snaps.last().unwrap(), cli.wire);
        let mut px = t.read_rgba(&device, &queue)?;
        if let Some(f) = w.screen_fill() {
            oot::rooms::apply_fill(&mut px, f);
        }
        eng_app::save_png(p, cli.width, cli.height, &px)?;
        println!("{}", p.display());
    }
    if let Some(p) = &cli.sheet {
        // Every `step` frames, 6 columns.
        let picks: Vec<usize> = (0..snaps.len()).step_by(cli.step.max(1)).collect();
        let (tw, th, cols) = (360u32, 270u32, 6u32);
        let rows = (picks.len() as u32).div_ceil(cols);
        let (sw, sh) = (tw * cols, th * rows);
        let mut sheet = vec![0u8; (sw * sh * 4) as usize];
        let t = Target::new(&device, tw, th);
        for (k, &i) in picks.iter().enumerate() {
            draw_frame(&mut r, &device, &queue, &t, &mut a, &mut scene, &w, &snaps[i], cli.wire);
            let px = t.read_rgba(&device, &queue)?;
            let (cx, cy) = (k as u32 % cols, k as u32 / cols);
            for y in 0..th {
                let dst = (((cy * th + y) * sw + cx * tw) * 4) as usize;
                let src = (y * tw * 4) as usize;
                sheet[dst..dst + (tw * 4) as usize].copy_from_slice(&px[src..src + (tw * 4) as usize]);
            }
        }
        eng_app::save_png(p, sw, sh, &sheet)?;
        let labels: Vec<String> = picks.iter().map(|&i| format!("{}:{}", i + 1, trace[i]["action"].as_str().unwrap_or(""))).collect();
        println!("{} ({} cells, left to right): {}", p.display(), picks.len(), labels.join(" "));
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    if cli.sheet.is_some() || cli.trace.is_some() || cli.screenshot.is_some() {
        return headless(&cli);
    }
    oot::run_window(&options(&cli), "OoT clone: sandbox", cli.width, cli.height)
}
