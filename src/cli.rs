//!
//! Cli arguments
//!
use clap::{ArgAction, Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::statics::EnvVars;

#[derive(Parser)]
#[command(version, author, about, long_about=None)]
#[command(
    disable_help_flag = true,
    disable_help_subcommand = false,
    disable_version_flag = true
)]
#[command(styles = CLAP_STYLE)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Box<Commands>,

    // Config options
    /// The configuration directory path (default to current dir)
    #[arg(
        global = true,
        long, short = 'C',
        env = EnvVars::YAPT_CONF_DIR,
        help_heading = "Config options",
    )]
    pub config: Option<PathBuf>,
    /// The cache directory (default to config dir)
    #[arg(global = true, long, help_heading = "Config options", env = EnvVars::YAPT_CACHE_DIR)]
    pub cache_dir: Option<PathBuf>,
    #[arg(global = true, long, help_heading = "Config options", env = EnvVars::YAPT_NO_SYNC)]
    pub no_sync: bool,
    /// The QGIS version
    ///
    /// If the qgis version is not specified, the version
    /// will be determined from the local QGIS installation
    /// if available.
    #[arg(global = true, long, help_heading = "Config options", env = EnvVars::QGIS_VERSION)]
    pub qgis_version: Option<String>,

    // Global options
    /// Increase log verbosity
    #[arg(
        global = true,
        short,
        long,
        action = ArgAction::Count,
        help_heading = "Global options",
    )]
    pub verbose: u8,
    #[arg(global = true, short, long, action = ArgAction::HelpShort, help_heading = "Global options")]
    pub help: Option<bool>,
    #[arg(short = 'V', long, action = ArgAction::Version, help_heading = "Global options")]
    pub version: Option<bool>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage sources
    ///
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    /// List installed plugins
    List(ListArgs),
    /// Install plugin(s)
    Install(InstallArgs),
    /// Upgrade installed plugins with remote sources
    #[command(alias = "sync")]
    Upgrade(UpgradeArgs),
    /// Search for plugins
    Search(SearchArgs),
}

#[derive(Subcommand)]
pub enum SourceCommand {
    /// Add remote source
    Add {
        #[arg()]
        name: String,
        #[arg()]
        url: String,
        /// Indicates that the remote source implements
        /// the yapt REST api.
        #[arg(long)]
        rest: bool,
    },
    /// Remove remote source
    Remove {
        #[arg()]
        name: String,
    },
    /// Rename source
    Rename {
        #[arg()]
        old: String,
        #[arg()]
        new: String,
    },
    /// List sources
    List,
    /// Fetch sources
    ///
    /// Download does not always occurs: it checks for ETag and last-modifiied action
    /// from the server headers.
    /// If the server does not provides these headers, then the source will not be
    /// be synced when issuing commands..
    Update {
        /// Fetch only the specified source
        #[arg()]
        source: Option<String>,
        /// Refresh cached data
        #[arg(long)]
        refresh: bool,
    },
    /// Check for update
    Check {
        /// Check only the specified source
        #[arg()]
        source: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// List outdated plugins
    #[arg(long, short)]
    pub outdated: bool,

    #[command(flatten)]
    pub resolve_args: ResolverArgs,

    /// Select the output format
    #[arg(long)]
    pub format: OutputFormat,
}

#[derive(Debug, Default, Copy, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    /// Output as table
    Table,
    /// Output as frozen version list
    List,
    /// Output as JSon
    Json,
}

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Install all listed plugins
    ///
    /// The version can be specified using comparison specifiers:
    ///
    /// ex: "name>=1.2.3, <1.8", "name=1.8.1"
    ///
    /// If the version has a prerelease tag, then it will only matches
    /// if at least one comparator with same major.nimor.patch has also
    /// a prerelease tag.
    ///
    /// i.e:
    ///
    /// * matching '>1.2.0' and '1.2.1-alpha.1' is always false
    /// * matching '>1.2.1-alpha.0' and '1.2.1-alpha.1' is true
    ///
    /// If the comparison operator is '==' then the version will be check
    /// as an exact match. This may be useful is the plugin version
    /// does not follow semantic versioning.
    ///
    /// ex: "name==release"
    ///
    #[arg(name = "NAME")]
    pub names: Vec<String>,

    #[command(flatten)]
    pub resolve_args: ResolverArgs,
    #[command(flatten)]
    pub install_args: InstallerArgs,
}

#[derive(Args, Debug)]
pub struct UpgradeArgs {
    #[command(flatten)]
    pub resolve_args: ResolverArgs,
    #[command(flatten)]
    pub install_args: InstallerArgs,
}

#[derive(Args, Debug)]
pub struct InstallerArgs {
    /// Upgrade plugin to latest version, if `--pre` is specified, the update will update
    /// to the latest experimental version if any.
    #[arg(long, short = 'U')]
    pub upgrade: bool,
    /// Force (re)installing
    #[arg(long)]
    pub reinstall: bool,
    /// Set files permissions to 0644
    #[arg(long)]
    pub fix_permissions: bool,
    /// Plugin destination folder
    #[arg(long, short, env = EnvVars::QGIS_PLUGINPATH)]
    pub destination: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ResolverArgs {
    /// Include pre-release, development and experimental versions.
    #[arg(long, env = EnvVars::QGIS_PLUGIN_INCLUDE_PRERELEASE)]
    pub pre: bool,
    /// Include deprecated versions
    #[arg(long)]
    pub deprecated: bool,
    /// Consider only server plugins
    #[arg(long)]
    pub server: bool,
    /// Use only the specified source
    #[arg(long)]
    pub source: Option<String>,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg()]
    pub name: String,

    /// Only search by plugin name
    #[arg(long)]
    pub by_name: bool,

    /// Return all versions of plugins
    #[arg(long)]
    pub all: bool,

    #[command(flatten)]
    pub resolve_args: ResolverArgs,
}

// Clap style

use clap::builder::styling;

const CLAP_STYLE: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());
