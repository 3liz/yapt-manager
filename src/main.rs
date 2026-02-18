//!
//! Qgis plugin manager
//!

use clap::Parser;
use std::path::Path;

mod catalog;
mod cli;
mod config;
mod context;
mod display;
mod echo;
mod errors;
mod install;
mod logger;
mod plugins;
mod statics;
mod version;

use cli::{Cli, Commands};
use display::{Table, column, print_table};
use echo::{INFO, OK, TABINF};
use plugins::Plugin;

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    use Commands::*;

    logger::init(args.verbose);

    let mut context = context::RunContext::new(args.config, args.cache_dir)?;
    let no_sync = args.no_sync;
    let qgis_version = args.qgis_version;

    match *args.command {
        Source { command } => {
            cmd_source(context, command, qgis_version)?;
        }
        List(args) => {
            todo!();
        }
        Install(args) => {
            todo!();
        }
        Upgrade(args) => {
            todo!();
        }
        Search(mut args) => {
            use context::SearchItem;

            args.name.make_ascii_lowercase();

            let resolver = args.resolve_args;
            let source = resolver.source;
            let mut plugins = context
                .qgis_version(qgis_version)?
                .sync(no_sync, source.as_ref())?
                .search(
                    catalog::Select {
                        key: args.name.into(),
                        by_name: args.by_name,
                        server: resolver.server,
                        experimental: resolver.pre,
                        deprecated: resolver.deprecated,
                        ..Default::default()
                    },
                    source.as_ref(),
                    args.all,
                )?;
            plugins.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());
            eprint!(" {TABINF} S {TABINF:#} Server");
            eprint!(" {TABINF} X {TABINF:#} Experimental");
            eprint!(" {TABINF} T {TABINF:#} Trusted");
            eprint!(" {TABINF} D {TABINF:#} Deprecated");
            eprintln!();
            print_table(
                plugins.iter(),
                (
                    column("NAME", |p: &SearchItem| p.name.as_str().into()),
                    column("VERSION", |p: &SearchItem| p.version.as_str().into()),
                    column("QGIS MIN", |p: &SearchItem| {
                        p.qgis_minimum_version.as_str().into()
                    }),
                    column("STATUS", |p: &SearchItem| SearchItem::status(p).into()),
                    column("SOURCE", |p: &SearchItem| SearchItem::source(p).into()),
                ),
            );
        }
    }
    Ok(())
}

fn cmd_source(
    mut context: context::RunContext,
    command: cli::SourceCommand,
    qgis_version: Option<String>,
) -> anyhow::Result<()> {
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
        Update { source, refresh } => {
            context
                .qgis_version(qgis_version)?
                .refresh_sources(refresh, source.as_ref())?;
        }
        Check { source } => {
            context
                .qgis_version(qgis_version)?
                .check_sources(source.as_ref())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
