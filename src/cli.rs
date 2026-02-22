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
    /// Do no synchronize sources
    #[arg(global = true, long, help_heading = "Config options", env = EnvVars::YAPT_NO_SYNC)]
    pub no_sync: bool,
    /// The QGIS version
    ///
    /// If the qgis version is not specified, the version
    /// will be determined from the local QGIS installation
    /// if available.
    #[arg(global = true, long, help_heading = "Config options", env = EnvVars::QGIS_VERSION)]
    pub qgis_version: Option<String>,
    /// Plugin installation directory
    ///
    /// Default to local directory
    #[arg(
        global = true,
        long, short = 'd',
        help_heading = "Install options",
        env = EnvVars::QGIS_PLUGINPATH,
    )]
    pub install_dir: Option<PathBuf>,

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
    /// Display concise help
    #[arg(global = true, short, long, action = ArgAction::HelpShort, help_heading = "Global options")]
    pub help: Option<bool>,
    /// Display the program version
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
    /// Find plugins
    ///
    /// Display the list of installable plugins matching version
    /// specifiers. Without specifier, always display the latest version
    /// available for each source.
    Find(FindArgs),
    /// Install plugin(s)
    Install(InstallArgs),
    /// Upgrade installed plugins with remote sources
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
    /// Use only the specified source when searching for latest version
    #[arg(long)]
    pub source: Option<String>,
    /// Include pre-release, development and experimental versions.
    #[arg(long, env = EnvVars::QGIS_PLUGIN_INCLUDE_PRERELEASE)]
    pub pre: bool,
    // /// Select the output format
    //#[arg(long)]
    //pub format: OutputFormat,
}

#[derive(Debug, Default, Copy, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    /// Output as table
    Table,
    /// Output as JSon
    Json,
}

#[derive(Args, Debug)]
pub struct FindArgs {
    /// List of plugins with optional version specifiers.
    #[arg(name = "NAME", required = true)]
    pub names: Vec<String>,

    #[command(flatten)]
    pub resolver: ResolverArgs,
}

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Plugins to install
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
    #[arg(name = "NAME", required = true)]
    pub names: Vec<String>,

    #[command(flatten)]
    pub resolver: ResolverArgs,
    #[command(flatten)]
    pub installer: InstallerArgs,

    /// Upgrade plugin to latest version, if `--pre` is specified, the update will update
    /// to the latest experimental version if any.
    #[arg(long, short = 'U')]
    pub upgrade: bool,
    /// Only show what would be installed
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct UpgradeArgs {
    #[command(flatten)]
    pub resolver: ResolverArgs,
    #[command(flatten)]
    pub installer: InstallerArgs,
    /// Only show what would be installed
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct InstallerArgs {
    /// Force (re)installing
    #[arg(long)]
    pub reinstall: bool,
    /// Set files permissions to 0644
    #[arg(long)]
    pub fix_permissions: bool,
}

#[derive(Args, Debug)]
pub struct ResolverArgs {
    /// Include pre-release, development and experimental versions.
    #[arg(long, env = EnvVars::QGIS_PLUGIN_INCLUDE_PRERELEASE)]
    pub pre: bool,
    /// Include deprecated versions
    #[arg(long)]
    pub deprecated: bool,
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

    /// Consider only server plugins
    #[arg(long)]
    pub server: bool,

    /// Return all versions of plugins
    #[arg(long)]
    pub all: bool,

    #[command(flatten)]
    pub resolver: ResolverArgs,
}

// Clap style

use clap::builder::styling;

const CLAP_STYLE: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());
