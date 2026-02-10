//!
//! Static definitions
//!

pub struct EnvVars;

impl EnvVars {
    /// Add prerelease versions
    pub const QGIS_PLUGIN_INCLUDE_PRERELEASE: &'static str = "QGIS_PLUGIN_INCLUDE_PRERELEASE";

    /// The QGIS version to consider for pulling plugins
    pub const QGIS_VERSION: &'static str = "QGIS_VERSION";

    /// The plugin installation path(s)
    /// This is the same variable used with QGIS/QJAZZ
    pub const QGIS_PLUGIN_PATH: &'static str = "QGIS_PLUGIN_PATH";

    /// Path to store the Yapt configuration (default to currentdir)
    pub const YAPT_CONF_DIR: &'static str = "YAPT_CONF_DIR";

    /// Path to cache indexes (default to `YAPT_CONF_DIR`)
    pub const YAPT_CACHE_DIR: &'static str = "YAPT_CACHE_DIR";

    /// Do not synchronize sources
    pub const YAPT_NO_SYNC: &'static str = "YAPT_NO_SYNC";

    /// User agent
    pub const YAPT_USER_AGENT: &'static str = "YAPT_USER_AGENT";
}
