//!
//! Qgis plugin manager
//!

use clap::Parser;
use std::path::Path;

mod catalog;
mod cli;
mod config;
mod download;
mod echo;
mod errors;
mod logger;
mod plugins;
mod statics;

use cli::{Cli, Commands};
use echo::{INFO, OK};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    use Commands::*;

    logger::init(args.verbose);

    let conf_dir = args.config.unwrap_or_else(|| {
        let mut p = std::env::current_dir().unwrap();
        p.push(".yapt");
        p
    });

    let cache_dir = args.cache_dir;

    let mut conf = config::Config::load_from(&conf_dir)?;

    match *args.command {
        Source { command } => {
            cmd_source(
                &mut conf,
                &cache_dir.unwrap_or(conf_dir.join("cache")),
                command,
            )?;
        }
        List(args) => {
            todo!();
        }
        Install(args) => {
            todo!();
        }
        Synchronize(args) => {
            todo!();
        }
        Search(args) => {
            todo!();
        }
    }
    Ok(())
}

fn cmd_source(
    conf: &mut config::Config,
    cache_dir: &Path,
    command: cli::SourceCommand,
) -> anyhow::Result<()> {
    use cli::SourceCommand::*;
    use config::Source;

    match command {
        Add { name, url, rest } => {
            conf.add_source(name.clone(), Source::new(url, rest))?
                .save()?;
            eprintln!("{OK}Source '{name}' added{OK:#}");
        }
        Remove { name } => {
            conf.remove_source(&name)?.save()?;
            eprintln!("{OK}Source '{name}' Removed{OK:#}");
        }
        Rename { old, new } => {
            conf.rename_source(&old, &new)?.save()?;
            eprintln!("{OK}'{old}':  Renamed to {new}{OK:#}");
        }
        List => {
            for (name, source) in conf.iter_sources() {
                println!("{name:20}{}", source.url);
            }
        }
        Fetch { source, refresh } => {
            download::download_sources(conf, cache_dir, refresh, source)?;
        }
    }
    Ok(())
}
