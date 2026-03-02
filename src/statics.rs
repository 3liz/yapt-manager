//!
//! Static definitions
//!
use std::ffi::{OsStr, OsString};

pub struct EnvVars;

impl EnvVars {
    /// Add prerelease versions
    pub const QGIS_PLUGIN_INCLUDE_PRERELEASE: &'static str = "QGIS_PLUGIN_INCLUDE_PRERELEASE";

    /// The QGIS version to consider for pulling plugins
    pub const QGIS_VERSION: &'static str = "QGIS_VERSION";

    /// The plugin installation path
    /// This is the same variable used with QGIS/QJAZZ
    pub const QGIS_PLUGINPATH: &'static str = "QGIS_PLUGINPATH";

    /// Path to store the Yapt configuration (default to currentdir)
    pub const YAPT_CONF_DIR: &'static str = "YAPT_CONF_DIR";

    /// Path to cache indexes (default to `YAPT_CONF_DIR`)
    pub const YAPT_CACHE_DIR: &'static str = "YAPT_CACHE_DIR";

    /// Do not synchronize sources
    pub const YAPT_NO_SYNC: &'static str = "YAPT_NO_SYNC";

    /// User agent
    pub const YAPT_USER_AGENT: &'static str = "YAPT_USER_AGENT";

    /// Hide progress outputs
    pub const YAPT_NO_PROGRESS: &'static str = "YAPT_NO_PROGRESS";

    /// Python executable
    pub const PYTHON_EXECUTABLE: &'static str = "PYTHON_EXECUTABLE";
}

impl EnvVars {
    pub fn python_executable() -> OsString {
        std::env::var_os(EnvVars::PYTHON_EXECUTABLE)
            .unwrap_or_else(|| OsStr::new("python3").to_os_string())
    }
}
