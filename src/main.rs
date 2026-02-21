//!
//! Qgis plugin manager
//!

use clap::Parser;

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

use cli::{Cli, Commands, FindArgs, ListArgs, SearchArgs};
use display::{HEAD, column, print_table};
use echo::{INFO, NOTE, OK, TABINF};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    use Commands::*;

    logger::init(args.verbose);

    let mut context = context::RunContext::new(args.config, args.cache_dir)?;
    let no_sync = args.no_sync;
    let qgis_version = args.qgis_version;

    let install_dir = context::RunContext::install_dir(args.install_dir);

    match *args.command {
        Source { command } => {
            cmd_source(context, command, qgis_version)?;
        }

        Find(FindArgs { names, resolver }) => {
            use context::SearchItem;
            let source = resolver.source;
            context
                .qgis_version(qgis_version)?
                .sync(no_sync, source.as_ref())?;

            for name in names {
                let (name, request) = version::parse_requirement(&name)?;
                let candidates = context.find(
                    name,
                    &request,
                    resolver.pre,
                    resolver.deprecated,
                    source.as_ref(),
                )?;
                println!("{HEAD}{name}{HEAD:#}:");
                if candidates.is_empty() {
                    println!("{INFO}No matches{INFO:#}");
                } else {
                    print_table(
                        candidates.iter(),
                        (
                            column("NAME", |p: &SearchItem| p.name.as_str().into()),
                            column("VERSION", |p: &SearchItem| p.version.as_str().into()),
                            column("QGIS", |p: &SearchItem| {
                                p.qgis_minimum_version.as_str().into()
                            }),
                            column("SOURCE", |p: &SearchItem| p.source().into()),
                        ),
                    );
                }
            }
        }

        List(ListArgs {
            outdated,
            source,
            pre,
            //format,
        }) => {
            use install::InstallItem;
            let items = install::Installer::list(
                context
                    .qgis_version(qgis_version)?
                    .sync(no_sync, source.as_ref())?,
                &install_dir,
                pre,
                source.as_ref(),
            )?;
            if items.is_empty() {
                eprintln!("{INFO}No plugins found...{INFO:#}");
            } else {
                print_table(
                    items.iter().filter(|p| !outdated || p.outdated),
                    (
                        column("NAME", |p: &InstallItem| p.name.as_str().into()),
                        column("VERSION", |p: &InstallItem| p.version.as_str().into()),
                        column("QGIS\u{002a}", |p: &InstallItem| {
                            p.qgis_minimum_version.as_str().into()
                        }),
                        column("SOURCE", |p: &InstallItem| p.source().unwrap_or("-").into()),
                        column("LATEST", |p: &InstallItem| {
                            p.latest().map(|v| v.as_str()).unwrap_or("-").into()
                        }),
                        column("FOLDER", |p: &InstallItem| {
                            p.folder.display().to_string().into()
                        }),
                    ),
                );
                eprintln!("{NOTE}(\u{002a}) Minimum QGIS versions supported{NOTE:#}");
            }
        }

        Install(args) => {
            todo!();
        }
        Upgrade(args) => {
            todo!();
        }
        Search(SearchArgs {
            mut name,
            by_name,
            all,
            resolver,
        }) => {
            use context::SearchItem;

            name.make_ascii_lowercase();

            let source = resolver.source;
            let mut items = context
                .qgis_version(qgis_version)?
                .sync(no_sync, source.as_ref())?
                .search(
                    catalog::Select {
                        key: name.into(),
                        by_name,
                        server: resolver.server,
                        experimental: resolver.pre,
                        deprecated: resolver.deprecated,
                        ..Default::default()
                    },
                    source.as_ref(),
                    all,
                )?;
            if items.is_empty() {
                eprintln!("{INFO}No plugins found...{INFO:#}");
            } else {
                items.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());
                eprint!(" {TABINF} S {TABINF:#} Server");
                eprint!(" {TABINF} X {TABINF:#} Experimental");
                eprint!(" {TABINF} T {TABINF:#} Trusted");
                eprint!(" {TABINF} D {TABINF:#} Deprecated");
                eprintln!();
                print_table(
                    items.iter(),
                    (
                        column("NAME", |p: &SearchItem| p.name.as_str().into()),
                        column("VERSION", |p: &SearchItem| p.version.as_str().into()),
                        column("QGIS", |p: &SearchItem| {
                            p.qgis_minimum_version.as_str().into()
                        }),
                        column("STATUS", |p: &SearchItem| p.status().into()),
                        column("SOURCE", |p: &SearchItem| p.source().into()),
                    ),
                );
            }
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
