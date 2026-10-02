//! The game. Enters a scene from the asset pack by `Play_Init` with the ported game logic:
//! every actor placed (ported, or a placeholder for the rest; P shows them), rooms loaded and
//! changed as Link moves, exits leading to other scenes. Starts as child Link in Kokiri Forest
//! (the entrance to spawn 0), at 10:00 like a new save.
//!
//! `oot import` builds the pack from your ROM (with the decomp's help), into the per-user data
//! folder. The first launch does it by itself if there's no pack yet. For the test course,
//! scripted runs and the debug views, use `oot_sandbox`.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use oot::Options;

#[derive(Parser)]
#[command(about = "OoT clone: plays a scene from your ROM's asset pack")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Scene (e.g. spot04 = Kokiri Forest, spot00 = Hyrule Field, ydan = Deku Tree), entered by
    /// the first entrance to its spawn --spawn.
    #[arg(long, default_value = "spot04")]
    scene: String,
    /// Spawn point (index into the scene's spawn list).
    #[arg(long, default_value_t = 0)]
    spawn: usize,
    /// An entrance instead (`ENTR_*` name or index, e.g. ENTR_KOKIRI_FOREST_3: outside Link's house).
    #[arg(long)]
    entrance: Option<String>,
    /// Draw markers where unported actors are.
    #[arg(long)]
    placeholders: bool,
    /// Time of day as HH:MM (a new save starts at 10:00).
    #[arg(long, default_value = "10:00")]
    time: String,
    /// Start as adult Link.
    #[arg(long)]
    adult: bool,
    /// After entering, change to this room as walking into it would (a debug start, e.g.
    /// --room 2 for Kokiri Forest's sword chest).
    #[arg(long)]
    room: Option<i8>,
    /// Then put Link at `x,y,z,yaw` (yaw in binary angle units).
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    at: Vec<f32>,
    /// A debug save preset: deku-tree-open (the Deku Tree met and his mouth open),
    /// deku-tree-dead (also the tree dead, with the Kokiri Emerald), or sword-and-40-rupees (the
    /// Kokiri Sword worn and 40 rupees, for the shop).
    #[arg(long)]
    preset: Option<String>,
    /// Start a new file as the file select does: Link's house with the opening (the Deku
    /// Tree's narration, the nightmare, Navi sent, and her waking Link). Start skips a scene.
    #[arg(long)]
    new_file: bool,
    /// A pack file or loose folder to use instead of the default pack.
    #[arg(long)]
    pack: Option<PathBuf>,
    /// A sequence to play instead of the first scene's own (`gSequenceTable`'s index: 60 =
    /// Kokiri Forest, 30 = the title theme): `Environment_ForcePlaySequence`.
    #[arg(long)]
    music: Option<u8>,
    /// No sound.
    #[arg(long)]
    no_audio: bool,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
}

#[derive(Subcommand)]
enum Cmd {
    /// Build the asset pack from the ROM and decomp named in oot.toml (or given here), and
    /// make it the one the game uses.
    Import {
        /// The ROM (default: `rom` in oot.toml).
        #[arg(long)]
        rom: Option<PathBuf>,
        /// The decomp checkout (default: `decomp` in oot.toml).
        #[arg(long)]
        decomp: Option<PathBuf>,
        /// Write loose records to this folder instead (the dev mode; play it with --pack).
        #[arg(long)]
        loose: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    if let Some(Cmd::Import { rom, decomp, loose }) = cli.cmd {
        let mut config = oot_import::project::Config::find(&std::env::current_dir()?).ok();
        if let (Some(r), Some(d)) = (&rom, &decomp) {
            config = Some(oot_import::project::Config { rom: r.clone(), decomp: d.clone() });
        }
        let mut config = config.ok_or_else(|| anyhow::anyhow!("no oot.toml: pass --rom and --decomp"))?;
        if let Some(r) = rom {
            config.rom = r;
        }
        if let Some(d) = decomp {
            config.decomp = d;
        }
        let project = oot_import::project::Project::open(config)?;
        let (report, path) = oot_import::pack::import_to_default(&project, loose.as_deref())?;
        print!("{}", oot_import::pack::summary(&report, &path));
        return Ok(());
    }
    let opts = Options {
        scene: Some(cli.scene),
        spawn: cli.spawn,
        time: cli.time,
        child: !cli.adult,
        pack: cli.pack,
        entrance: Some(cli.entrance.unwrap_or_default()),
        placeholders: cli.placeholders,
        preset: cli.preset,
        new_file: cli.new_file,
        room: cli.room,
        at: cli.at,
        audio: !cli.no_audio,
        music: cli.music,
        audio_log: false,
        ..Default::default()
    };
    oot::run_window(&opts, "OoT clone", cli.width, cli.height)
}
