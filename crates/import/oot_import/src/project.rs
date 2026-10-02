//! Opens the user's ROM and decomp checkout as configured in `oot.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::rom::{Rom, decomp_file_names};
use crate::symbols::SymbolIndex;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub rom: PathBuf,
    pub decomp: PathBuf,
}

impl Config {
    /// Looks for `oot.toml` in `start` and its ancestors.
    pub fn find(start: &Path) -> Result<Config> {
        let mut dir = Some(start);
        while let Some(d) = dir {
            let p = d.join("oot.toml");
            if p.exists() {
                let text = std::fs::read_to_string(&p)?;
                return toml::from_str(&text).with_context(|| format!("parsing {}", p.display()));
            }
            dir = d.parent();
        }
        anyhow::bail!("no oot.toml found (copy oot.example.toml to oot.toml and set your paths)")
    }
}

pub struct Project {
    pub config: Config,
    pub rom: Rom,
    pub symbols: SymbolIndex,
    pub names_applied: bool,
}

impl Project {
    pub fn open(config: Config) -> Result<Project> {
        let mut rom = Rom::open(&config.rom)?;
        let names = decomp_file_names(&config.decomp)?;
        let names_applied = rom.set_names(names);
        let symbols = SymbolIndex::load(&config.decomp)?;
        Ok(Project { config, rom, symbols, names_applied })
    }

    pub fn open_default() -> Result<Project> {
        Self::open(Config::find(&std::env::current_dir()?)?)
    }
}
