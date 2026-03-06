//!
//! Qgis plugin manager
//!

use clap::Parser;

mod catalog;
mod cli;
mod config;
mod context;
mod errors;
mod install;
mod logger;
mod plugins;
mod printer;
mod statics;
mod table;
mod version;

use cli::{Cli, Commands, FindArgs, InstallArgs, ListArgs, RemoveArgs, SearchArgs, UpgradeArgs};
use printer::{
    ALERT, INFO, NOTE, OK, TABINF,
    glyph::{ARROW, CHECK, CROSS, WARN},
};
use table::{HEAD, column, print_table};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    use Commands::*;

    logger::init(args.verbose);

    let mut context = context::ContextBuilder {
        conf_dir: args.config,
        cache_dir: args.cache_dir,
        install_dir: args.install_dir,
        no_sync: args.no_sync,
        // If verbose is set to debug then hide progress
        // otherwise it gets interleaved with debug message
        no_progress: args.no_progress || args.verbose > 1,
    }
    .build()?;

    let qgis_version = args.qgis_version;

    match *args.command {
        Source { command } => {
            cmd_source(context, command, qgis_version)?;
        }
        //
        // Find command
        //
        Find(FindArgs { names, resolver }) => {
            use context::SearchItem;
            let source = resolver.source;
            context.qgis_version(qgis_version)?.sync(source.as_ref())?;

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
        //
        // List command
        //
        List(ListArgs {
            outdated,
            source,
            pre,
            //format,
        }) => {
            use context::OutdatedItem;
            let items = context
                .qgis_version(qgis_version)?
                .sync(source.as_ref())?
                .list(pre, source.as_ref())?;
            if items.is_empty() {
                println!("{INFO}No plugins found...{INFO:#}");
            } else {
                print_table(
                    items.iter().filter(|p| !outdated || p.outdated),
                    (
                        column("NAME", |p: &OutdatedItem| p.name.as_str().into()),
                        column("VERSION", |p: &OutdatedItem| p.version.as_str().into()),
                        column("QGIS\u{002a}", |p: &OutdatedItem| {
                            p.qgis_minimum_version.as_str().into()
                        }),
                        column("SOURCE", |p: &OutdatedItem| {
                            p.source().unwrap_or("-").into()
                        }),
                        column("LATEST", |p: &OutdatedItem| {
                            p.latest().map(|s| s.version.as_str()).unwrap_or("-").into()
                        }),
                        column("FOLDER", |p: &OutdatedItem| {
                            p.folder.display().to_string().into()
                        }),
                    ),
                );
                println!("{NOTE}(\u{002a}) Minimum QGIS versions supported{NOTE:#}");
            }
        }
        //
        // Install command
        //
        Install(InstallArgs {
            names,
            resolver,
            installer,
            upgrade,
            dry_run,
        }) => {
            use context::{InstallAction::*, InstallResult};

            let source = resolver.source;
            let candidates = context
                .qgis_version(qgis_version)?
                .sync(source.as_ref())?
                .install_candidates(
                    names,
                    resolver.pre,
                    resolver.deprecated,
                    source.as_ref(),
                    upgrade,
                    installer.reinstall,
                )?;

            if dry_run {
                candidates.iter().for_each(|action| match action {
                    Install(item) => {
                        println!(
                            "{INFO}\t{CHECK} {:<25} {:<12}\tInstallable from {}{INFO:#}",
                            item.name,
                            item.version,
                            item.source(),
                        );
                    }
                    Upgrade(item, _) => {
                        println!(
                            "{INFO}\t{CHECK} {:<25} {:<12}\tUpgradable from {}{INFO:#}",
                            item.name,
                            item.version,
                            item.source(),
                        );
                    }
                    Unchanged(item, _) => {
                        println!(
                            "{OK}\t{CHECK} {:<25} {:<12}\tUnchanged{OK:#}",
                            item.name, item.version,
                        );
                    }
                    NotFound(name) => {
                        println!("{ALERT}\t{WARN} {name:<25}\tNot found{ALERT:#}");
                    }
                })
            } else {
                context
                    .install_plugins(candidates.into_iter().filter_map(|action| match action {
                        Install(item) | Upgrade(item, _) => Some(item),
                        Unchanged(item, _) => {
                            println!(
                                "{OK}\t{CHECK} {:<25} {:<12}\tUnchanged{OK:#}",
                                item.name, item.version
                            );
                            None
                        }
                        NotFound(name) => {
                            println!("{ALERT}\t{WARN} {name:<25}\tNot found{ALERT:#}");
                            None
                        }
                    }))
                    .into_iter()
                    .for_each(|result| match result {
                        InstallResult::Ok(item, _) => {
                            println!(
                                "{OK}\t{CHECK} {:<25} {:<12}\tInstalled from {}{OK:#}",
                                item.name,
                                item.version,
                                item.source(),
                            );
                        }
                        InstallResult::Err(item, err) => {
                            println!(
                                "{ALERT}\t{CROSS} {:<25} {:<12}\tError{ALERT:#}",
                                item.name, item.version
                            );
                            println!("{ALERT}{err}{ALERT:#}");
                        }
                    })
            }
        }
        //
        // Upgrade command
        //
        Upgrade(UpgradeArgs {
            resolver,
            installer,
            dry_run,
        }) => {
            let source = resolver.source;
            let items = context
                .qgis_version(qgis_version)?
                .sync(source.as_ref())?
                .list(resolver.pre, source.as_ref())?;
            if items.is_empty() {
                println!("{INFO}No plugins found...{INFO:#}");
            } else if dry_run {
                items.iter().for_each(|item| {
                    if item.folder.is_symlink() {
                        println!(
                            "{INFO}\t{WARN} {:<25} {}\tSkipped because it is a symlink{INFO:#}",
                            item.name, item.version,
                        );
                    } else if item.latest().is_none() {
                        println!(
                            "{INFO}\t{WARN} {:<25} {}\tSkipped (no source){INFO:#}",
                            item.name, item.version,
                        );
                    } else if item.outdated {
                        println!(
                            "{INFO}\t{CHECK} {:<25} {} {ARROW} {}\tUpgradable from {} {INFO:#}",
                            item.name,
                            item.version,
                            item.latest().unwrap().version,
                            item.source().unwrap(),
                        );
                    } else if installer.reinstall {
                        println!(
                            "{INFO}\t{CHECK} {:<25} {}\tInstallable from {} {INFO:#}",
                            item.name,
                            item.latest().unwrap().version,
                            item.source().unwrap(),
                        );
                    } else {
                        println!(
                            "{INFO}\t{CHECK} {:<25} {}\tUnchanged {INFO:#}",
                            item.name, item.version,
                        );
                    }
                })
            } else {
                use context::InstallResult;
                context
                    .install_plugins(items.into_iter().filter_map(|item| {
                        if item.folder.is_symlink() {
                            println!(
                                "{INFO}\t{WARN} {:<25} {}\tSkipped because it is a symlink{INFO:#}",
                                item.name, item.version,
                            );
                            None
                        } else if item.latest().is_none() {
                            println!(
                                "{INFO}\t{WARN} {:<25} {}\tSkipped (no source){INFO:#}",
                                item.name, item.version,
                            );
                            None
                        } else if item.outdated || installer.reinstall {
                            item.latest
                        } else {
                            println!(
                                "{INFO}\t{CHECK} {:<25} {}\tUnchanged {INFO:#}",
                                item.name, item.version,
                            );
                            None
                        }
                    }))
                    .into_iter()
                    .for_each(|result| match result {
                        InstallResult::Ok(item, _) => {
                            println!(
                                "{OK}\t{CHECK} {:<25} {:<12}\tInstalled from {}{OK:#}",
                                item.name,
                                item.version,
                                item.source(),
                            );
                        }
                        InstallResult::Err(item, err) => {
                            println!(
                                "{ALERT}\t{CROSS} {:<25} {:<12}\tError{ALERT:#}",
                                item.name, item.version
                            );
                            println!("{ALERT}{err}{ALERT:#}");
                        }
                    })
            }
        }
        //
        // Search command
        //
        Search(SearchArgs {
            mut name,
            by_name,
            all,
            server,
            resolver,
        }) => {
            use context::SearchItem;

            name.make_ascii_lowercase();

            let source = resolver.source;
            let mut items = context
                .qgis_version(qgis_version)?
                .sync(source.as_ref())?
                .search(
                    catalog::Select {
                        key: name.into(),
                        by_name,
                        server,
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
                        column("QGIS\u{002a}", |p: &SearchItem| {
                            p.qgis_minimum_version.as_str().into()
                        }),
                        column("STATUS", |p: &SearchItem| p.status().into()),
                        column("SOURCE", |p: &SearchItem| p.source().into()),
                    ),
                );
                eprintln!("{NOTE}(\u{002a}) Minimum QGIS versions supported{NOTE:#}");
            }
        }
        //
        // Remove command
        //
        Remove(RemoveArgs { names }) => {
            let mut count = 0;
            context.remove(names)?.for_each(|res| {
                count += 1;
                match res {
                    Ok(plugin) => {
                        println!(
                            "{OK}\t{CHECK} {:<25} {:<12}\tRemoved{OK:#}",
                            plugin.name, plugin.version,
                        );
                    }
                    Err(err) => {
                        println!("{CROSS}{ALERT}\tError {err} {ALERT:#}");
                    }
                }
            });
            if count == 0 {
                eprintln!("{INFO}No plugins found{INFO:#}")
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
                .add_source(Source::new(name.clone(), url, rest))?
                .save()?;
            println!("{OK}Source '{name}' added{OK:#}");
        }
        Remove { name } => {
            context.config_mut().remove_source(&name)?.save()?;
            println!("{OK}Source '{name}' Removed{OK:#}");
        }
        Rename { old, new } => {
            context.config_mut().rename_source(&old, &new)?.save()?;
            println!("{OK}'{old}':  Renamed to {new}{OK:#}");
        }
        List => {
            for source in context.config().iter_sources() {
                println!(
                    "{:20}{}  {}",
                    source.name,
                    source.url,
                    if source.rest { "(rest)" } else { "" }
                );
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
