//!
//! Run context
//!
use std::cell::{OnceCell, Ref, RefCell, RefMut};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use anyhow::Context;
use futures::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::catalog::{Catalog, CatalogImpl};
use crate::config::{Config, Source};
use crate::echo::{RefreshStyle, SearchProgress};
use crate::plugins::Plugin;
use crate::statics::EnvVars;
use crate::version::{Match, SemVer};

pub use crate::catalog::Select;

const USER_AGENT: &str = "Yapt manager";

pub struct RunContext {
    conf_dir: PathBuf,
    cache_dir: PathBuf,
    config: RefCell<Config>,
    client: OnceCell<Result<reqwest::Client, reqwest::Error>>,
    qgis_version: SemVer,
}

impl RunContext {
    pub fn install_dir(install_dir: Option<PathBuf>) -> PathBuf {
        install_dir.unwrap_or_else(|| Path::new("./").to_path_buf())
    }

    pub fn new(conf_dir: Option<PathBuf>, cache_dir: Option<PathBuf>) -> anyhow::Result<Self> {
        let conf_dir = conf_dir.unwrap_or_else(|| {
            let mut p = std::env::current_dir().unwrap();
            p.push(".yapt");
            p
        });
        let cache_dir = cache_dir.unwrap_or(conf_dir.join("cache"));
        let config = RefCell::new(Config::load_from(&conf_dir)?);

        Ok(Self {
            conf_dir,
            cache_dir,
            config,
            qgis_version: SemVer::None,
            client: OnceCell::new(),
        })
    }

    /// get configuration
    pub fn config(&self) -> Ref<'_, Config> {
        self.config.borrow()
    }

    /// get mutable configuration
    pub fn config_mut(&self) -> RefMut<'_, Config> {
        self.config.borrow_mut()
    }

    #[inline]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    #[inline]
    pub fn conf_dir(&self) -> &Path {
        &self.conf_dir
    }

    /// Return an http client
    pub fn client(&self) -> anyhow::Result<reqwest::Client> {
        self.client
            .get_or_init(|| {
                reqwest::Client::builder()
                    .user_agent(match &self.qgis_version {
                        SemVer::None => format!("{USER_AGENT}/{}", clap::crate_version!()),
                        // See https://github.com/3liz/qgis-plugin-manager/issues/66
                        // See https://lists.osgeo.org/pipermail/qgis-user/2024-May/054439.html
                        version => format!(
                            "{USER_AGENT}/{} QGIS/{version}/{}",
                            clap::crate_version!(),
                            std::env::consts::OS,
                        ),
                    })
                    .build()
            })
            .as_ref()
            .map_err(|err| anyhow::anyhow!(format!("{err}")))
            .cloned()
    }

    pub fn catalog(
        &self,
        name: &str,
        source: &Source,
        create: bool,
    ) -> anyhow::Result<CatalogImpl> {
        let path = self.cache_dir().join(name);
        if !path.exists() {
            if create {
                std::fs::create_dir(&path)
                    .with_context(|| format!("Failed to create cache dir {path:?}"))?;
            } else {
                return Err(anyhow::anyhow!(format!("Source no configured: {name}")));
            }
        }

        CatalogImpl::new(
            &path,
            source
                .try_url(&self.qgis_version)
                .with_context(|| format!("Qgis version required for {name}"))?
                .into_owned(),
        )
        .with_context(|| format!("Failed to load cache from {path:?}"))
    }

    /// Find a plugin
    pub async fn find(
        &mut self,
        plugin_name: &str,
        request: Match<'_>,
        source: Option<&String>,
    ) -> anyhow::Result<Option<Plugin>> {
        let rt = Self::create_runtime();

        let progress = SearchProgress::new()?;
        let conf = self.config();

        if let Some(name) = source {
            let source = conf.try_get_source(name)?;
            let catalog = self.catalog(name, source, false)?;
            progress.set_message(name.clone());
            rt.block_on(catalog.find(self, plugin_name, &request))
        } else if conf.num_sources() == 1 {
            let (name, source) = conf.iter_sources().next().unwrap();
            let catalog = self.catalog(name, source, false)?;
            progress.set_message(name.clone());
            rt.block_on(catalog.find(self, plugin_name, &request))
        } else {
            rt.block_on(async {
                for (name, source) in conf.iter_sources() {
                    let catalog = self.catalog(name, source, false)?;
                    progress.set_message(name.clone());
                    if let Some(plugin) = catalog.find(self, plugin_name, &request).await? {
                        return Ok(Some(plugin));
                    }
                }
                Ok(None)
            })
        }
    }

    /// Search for plugins
    pub fn search(
        &self,
        mut query: Select<'_>,
        source: Option<&String>,
        all: bool,
    ) -> anyhow::Result<Vec<SearchItem>> {
        let rt = Self::create_runtime();

        let progress = SearchProgress::new()?;
        let conf = self.config();

        // Search only for qgis_version if set
        query.qgis_version = self.qgis_version.clone();

        fn into_search(name: &str, v: Vec<Plugin>) -> Vec<SearchItem> {
            let source: std::rc::Rc<str> = name.into();
            v.into_iter()
                .map(|plugin| SearchItem {
                    plugin,
                    source: source.clone(),
                })
                .collect()
        }

        let mut plugins = if let Some(name) = source {
            let source = conf.try_get_source(name)?;
            let catalog = self.catalog(name, source, false)?;
            progress.set_message(name.clone());
            into_search(
                name,
                rt.block_on(catalog.search_with_options(self, &query, all))?,
            )
        } else if conf.num_sources() == 1 {
            let (name, source) = conf.iter_sources().next().unwrap();
            let catalog = self.catalog(name, source, false)?;
            progress.set_message(name.clone());
            into_search(
                name,
                rt.block_on(catalog.search_with_options(self, &query, all))?,
            )
        } else {
            rt.block_on(join_all(conf.iter_sources().map(|(name, source)| async {
                let catalog = self.catalog(name, source, false)?;
                progress.set_message(name.clone());
                catalog
                    .search_with_options(self, &query, all)
                    .await
                    .map(|v| into_search(name, v))
            })))
            .into_iter()
            .collect::<anyhow::Result<Vec<Vec<SearchItem>>>>()?
            .into_iter()
            .flatten()
            .collect()
        };
        if query.by_name {
            // Sort by version in decreasing order
            plugins.sort_by(|a, b| b.version.partial_cmp(&a.version).unwrap());
        }
        progress.finish_and_clear();
        Ok(plugins)
    }

    /// Check source for update
    pub fn check_sources(&self, source: Option<&String>) -> anyhow::Result<()> {
        let rt = Self::create_runtime();

        let m = MultiProgress::new();

        // Create catalog
        let create_catalog =
            |name: &str, source: &Source| -> anyhow::Result<(CatalogImpl, ProgressBar)> {
                let catalog = self.catalog(name, source, true)?;
                let progress = m.add(ProgressBar::no_length());
                progress.set_style(RefreshStyle::progress(name)?);
                Ok((catalog, progress))
            };

        let conf = self.config();

        if let Some(name) = source {
            let source = conf.try_get_source(name)?;
            let (mut catalog, progress) = create_catalog(name, source)?;
            rt.block_on(catalog.check_for_update(self, progress))
        } else if conf.num_sources() == 1 {
            let (name, source) = conf.iter_sources().next().unwrap();
            let (mut catalog, progress) = create_catalog(name, source)?;
            rt.block_on(catalog.check_for_update(self, progress))
        } else {
            rt.block_on(join_all(conf.iter_sources().map(|(name, source)| async {
                let (mut catalog, progress) = create_catalog(name, source)?;
                catalog.check_for_update(self, progress).await
            })))
            .into_iter()
            .try_for_each(|res| res)
        }
    }

    /// Helper for syncing sources
    pub fn sync(&mut self, no_sync: bool, source: Option<&String>) -> anyhow::Result<&mut Self> {
        if !no_sync {
            self.refresh_sources(false, source)?;
        }
        Ok(self)
    }

    /// Refresh source's caches
    pub fn refresh_sources(&self, force: bool, source: Option<&String>) -> anyhow::Result<()> {
        let rt = Self::create_runtime();

        let m = MultiProgress::new();

        // Create catalog
        let create_catalog =
            |name: &str, source: &Source| -> anyhow::Result<(CatalogImpl, ProgressBar)> {
                let catalog = self.catalog(name, source, true)?;
                let progress = m.add(ProgressBar::no_length());
                progress.set_style(RefreshStyle::progress(name)?);
                Ok((catalog, progress))
            };

        let conf = self.config();

        if let Some(name) = source {
            let source = conf.try_get_source(name)?;
            let (mut catalog, progress) = create_catalog(name, source)?;
            rt.block_on(catalog.refresh(self, progress, force))
        } else {
            rt.block_on(join_all(conf.iter_sources().map(|(name, source)| async {
                let (mut catalog, progress) = create_catalog(name, source)?;
                catalog.refresh(self, progress, force).await
            })))
            .into_iter()
            .try_for_each(|res| res)
        }
    }

    // Set the QGIS version or get the
    // installed QGIS version if available
    pub fn qgis_version(&mut self, version: Option<String>) -> anyhow::Result<&mut Self> {
        if let Some(version) = version {
            self.qgis_version = version.into();
        } else {
            self.qgis_version = Self::installed_qgis_version().map_or(SemVer::None, SemVer::from)
        }
        Ok(self)
    }

    /// Try to determine the current installed QGIS version
    pub fn installed_qgis_version() -> Option<String> {
        const VERSION_SCRIPT: &str = include_str!("version.py");

        let output = match Command::new(EnvVars::python_executable())
            .args(["-c", VERSION_SCRIPT])
            .output()
        {
            Ok(output) => output,
            Err(err) => {
                log::error!("Cannot run python executable: {err}");
                return None;
            }
        };

        if !output.status.success() {
            log::error!(
                "Cannot determine QGIS version: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            None
        } else {
            let version = String::from_utf8_lossy(&output.stdout).into_owned();
            if version.is_empty() {
                log::warn!("Cannot check QGIS version because QGIS is not installed");
                None
            } else {
                log::debug!("Found installed QGIS version: {version}");
                Some(version)
            }
        }
    }

    // Create tokio runtime
    pub(crate) fn create_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .expect("Failed to create tokio runtime")
    }
}

// Search results
pub struct SearchItem {
    pub(crate) plugin: Plugin,
    pub(crate) source: Rc<str>,
}

impl SearchItem {
    #[inline]
    pub fn source(&self) -> &str {
        &self.source[..]
    }
    pub fn status(&self) -> String {
        let mut st = [b'-'; 4];
        let p = &self.plugin;
        if p.server {
            st[0] = b'S'
        }
        if p.experimental {
            st[1] = b'X'
        }
        if p.trusted {
            st[2] = b'T'
        }
        if p.deprecated {
            st[3] = b'D'
        }
        str::from_utf8(&st).unwrap().to_string()
    }
}

impl Deref for SearchItem {
    type Target = Plugin;

    fn deref(&self) -> &Self::Target {
        &self.plugin
    }
}

impl AsRef<Plugin> for SearchItem {
    fn as_ref(&self) -> &Plugin {
        &self.plugin
    }
}
