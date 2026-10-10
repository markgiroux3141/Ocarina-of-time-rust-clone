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
    /// After entering, set these switch flags (hex, comma-separated: --switch 0x27), as if
    /// their switches had just been pressed (a debug start, e.g. room 0's golden torches lit).
    #[arg(long, value_delimiter = ',')]
    switch: Vec<String>,
    /// After entering, set these rooms cleared (comma-separated: --clear 9), as if their
    /// enemies or puzzles were beaten (a debug start, e.g. room 9's door to room 11 open).
    #[arg(long, value_delimiter = ',')]
    clear: Vec<i8>,
    /// With --entrance: start in a cutscene layer, as this debug ROM's map select can: the save's
    /// cutscene index (0xFFF0 is layer 4, 0xFFF1 layer 5, ...; e.g. --entrance
    /// ENTR_KOKIRI_FOREST_0 --cutscene 0xFFF2: the Kokiri Emerald and the Deku Tree's death).
    #[arg(long)]
    cutscene: Option<String>,
    /// A dummy target (the sandbox's stand-in enemy) this many units in front of --at.
    #[arg(long, default_value_t = 0.0)]
    target: f32,
    /// The dummy hurts Link when he touches it: none (a plain hit), fire, ice, electric or
    /// knockback.
    #[arg(long)]
    target_hurts: Option<String>,
    /// A debug save preset: deku-tree-open (the Deku Tree met and his mouth open),
    /// deku-tree-dead (also the tree dead, with the Kokiri Emerald), deku-tree-inside (its intro
    /// seen), deku-tree-sticks (and ten Deku Sticks on C-Left), deku-tree-slingshot (and ten
    /// Deku nuts on C-Down and the Fairy Slingshot on C-Right), deku-tree-slingshot-owned (the
    /// slingshot on no button, for the pause menu to equip), or sword-and-40-rupees (the
    /// Kokiri Sword worn and 40 rupees, for the shop).
    #[arg(long)]
    preset: Option<String>,
    /// Start a new file as the file select does: Link's house with the opening (the Deku
    /// Tree's narration, the nightmare, Navi sent, and her waking Link). Start skips a scene.
    #[arg(long)]
    new_file: bool,
    /// Play file N (1 to 3) of the save file (`out/saves/<ROM SHA-1>.sra`, or $OOT_SAVE_DIR), as
    /// the file select loads it; with --new-file, a new file made in an empty one. File 1 goes
    /// through this debug ROM's map select: it enters by --entrance. Saves (the pause menu's B)
    /// write it back; F5 is the console's reset.
    #[arg(long)]
    file: Option<usize>,
    /// The SRAM image to play on instead of the save file.
    #[arg(long)]
    sram: Option<PathBuf>,
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
        switches: cli.switch.iter().map(|s| oot::parse_switch_flag(s)).collect::<anyhow::Result<_>>()?,
        clears: cli.clear.clone(),
        cutscene: cli.cutscene.as_deref().map(oot::parse_cutscene).transpose()?,
        target: cli.target,
        target_hurts: cli.target_hurts.as_deref().map(oot::parse_hit_effect).transpose()?,
        audio: !cli.no_audio,
        music: cli.music,
        audio_log: false,
        file: cli.file,
        sram: cli.sram,
        ..Default::default()
    };
    oot::run_window(&opts, "OoT clone", cli.width, cli.height)
}
