//! Controller probe: lists devices, shows live raw input and the mapped N64 pad, and walks
//! through a calibration that prints a `[pad]` snippet for `oot.toml`.

use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use eng_input::device::{PadConfig, Pads};
use eng_input::pad::BUTTON_NAMES;

#[derive(Parser)]
#[command(about = "N64 controller probe for the OoT clone")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// List connected controllers with ids, gilrs mapping source and the profile used.
    List,
    /// Print raw events and the mapped N64 state for a few seconds.
    Watch {
        #[arg(default_value_t = 10)]
        seconds: u64,
    },
    /// Press each N64 button when asked; prints a [pad.buttons] table.
    Calibrate,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    let cfg = PadConfig::find(&std::env::current_dir()?, "oot.toml");
    let mut pads = Pads::new(cfg)?;
    // Let the backend enumerate.
    let t = Instant::now();
    while t.elapsed() < Duration::from_millis(400) {
        pads.update();
        std::thread::sleep(Duration::from_millis(10));
    }
    match cli.cmd.unwrap_or(Cmd::List) {
        Cmd::List => {
            let devs = pads.devices();
            if devs.is_empty() {
                println!("no controllers found (keyboard only)");
            }
            for d in devs {
                println!(
                    "#{} \"{}\" (os: \"{}\") vid={} pid={} mapping={} connected={}\n    profile: {}",
                    d.id,
                    d.name,
                    d.os_name,
                    d.vid.map_or("?".into(), |v| format!("{v:04x}")),
                    d.pid.map_or("?".into(), |v| format!("{v:04x}")),
                    d.mapping,
                    d.connected,
                    d.profile
                );
            }
            if let Some(s) = pads.state() {
                println!("idle state: stick ({:+}, {:+}) buttons [{}]", s.stick_x, s.stick_y, s.names());
            }
        }
        Cmd::Watch { seconds } => {
            println!("watching {} for {seconds}s", pads.active_name().unwrap_or("nothing".into()));
            let t = Instant::now();
            let mut last = None;
            while t.elapsed() < Duration::from_secs(seconds) {
                pads.update();
                for e in pads.take_events() {
                    println!("  raw: {e}");
                }
                let s = pads.state();
                if s != last {
                    if let Some(s) = s {
                        println!("  N64: stick ({:+4}, {:+4}) [{}] raw held {:?}", s.stick_x, s.stick_y, s.names(), pads.raw_held());
                    }
                    last = s;
                }
                std::thread::sleep(Duration::from_millis(8));
            }
        }
        Cmd::Calibrate => {
            println!("calibrating {}", pads.active_name().unwrap_or("nothing".into()));
            let mut out = Vec::new();
            for (_, name) in BUTTON_NAMES.iter().take(10) {
                println!("press {name} (10 s, or wait to skip)");
                let t = Instant::now();
                let mut got = None;
                pads.take_events();
                while t.elapsed() < Duration::from_secs(10) && got.is_none() {
                    pads.update();
                    for e in pads.take_events() {
                        if let Some(code) = e.strip_prefix("button b").and_then(|r| r.split(' ').next()).and_then(|c| c.parse::<u32>().ok()) {
                            if e.ends_with("pressed") {
                                got = Some(format!("b{code}"));
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(8));
                }
                match got {
                    Some(b) => {
                        println!("  {name} = {b}");
                        out.push((name.to_string(), b));
                        // Wait for release.
                        std::thread::sleep(Duration::from_millis(300));
                        pads.update();
                        pads.take_events();
                    }
                    None => println!("  skipped"),
                }
            }
            println!("\n[pad.buttons]");
            for (n, b) in out {
                println!("\"{n}\" = \"{b}\"");
            }
        }
    }
    Ok(())
}
