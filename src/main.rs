//!
//! Qgis plugin manager
//!

use clap::Parser;
use std::path::Path;

mod catalog;
mod cli;
mod config;
mod context;
mod echo;
mod errors;
mod install;
mod logger;
mod plugins;
mod statics;
mod version;

use cli::{Cli, Commands};
use echo::{INFO, OK};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    use Commands::*;

    logger::init(args.verbose);

    let context = context::RunContext::new(args.config, args.cache_dir)?;

    match *args.command {
        Source { command } => {
            cmd_source(context, command)?;
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

fn cmd_source(context: context::RunContext, command: cli::SourceCommand) -> anyhow::Result<()> {
    use cli::SourceCommand::*;
    use config::Source;

    match command {
        Add { name, url, rest } => {
            context
                .config_mut()
                .add_source(name.clone(), Source::new(url, rest))?
                .save()?;
            eprintln!("{OK}Source '{name}' added{OK:#}");
        }
        Remove { name } => {
            context.config_mut().remove_source(&name)?.save()?;
            eprintln!("{OK}Source '{name}' Removed{OK:#}");
        }
        Rename { old, new } => {
            context.config_mut().rename_source(&old, &new)?.save()?;
            eprintln!("{OK}'{old}':  Renamed to {new}{OK:#}");
        }
        List => {
            for (name, source) in context.config().iter_sources() {
                println!("{name:20}{}", source.url);
            }
        }
        Fetch { source, refresh } => {
            context.refresh_sources(refresh, source)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
