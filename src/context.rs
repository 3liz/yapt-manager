//!
//! Run context
//!
use std::cell::{OnceCell, Ref, RefCell, RefMut};
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use anyhow::Context;
use futures::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};

use crate::catalog::{Catalog, CatalogImpl};
use crate::config::{Config, Source};
use crate::echo::{CacheProgress, InstallProgress, SearchProgress};
use crate::install::Installer;
use crate::plugins::Plugin;
use crate::statics::EnvVars;
use crate::version::{Match, SemVer};

pub use crate::catalog::Select;
pub use crate::install::{InstallAction, OutdatedItem};

const USER_AGENT: &str = "Yapt manager";

pub struct ContextBuilder {
    pub conf_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
    pub install_dir: Option<PathBuf>,
    pub no_sync: bool,
    pub no_progress: bool,
}

impl ContextBuilder {
    pub fn build(self) -> anyhow::Result<RunContext> {
        let conf_dir = self.conf_dir.unwrap_or_else(|| {
            let mut p = std::env::current_dir().unwrap();
            p.push(".yapt");
            p
        });
        let install_dir = self
            .install_dir
            .unwrap_or_else(|| Path::new("./").to_path_buf());
        let cache_dir = self.cache_dir.unwrap_or(conf_dir.join("cache"));
        let config = RefCell::new(Config::load_from(&conf_dir)?);
        Ok(RunContext {
            cache_dir,
            install_dir,
            config,
            qgis_version: SemVer::None,
            client: OnceCell::new(),
            no_sync: self.no_sync,
            no_progress: self.no_progress,
        })
    }
}

pub struct RunContext {
    cache_dir: PathBuf,
    install_dir: PathBuf,
    config: RefCell<Config>,
    client: OnceCell<Result<reqwest::Client, reqwest::Error>>,
    qgis_version: SemVer,
    no_sync: bool,
    no_progress: bool,
}

impl RunContext {
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
    pub fn install_dir(&self) -> &Path {
        &self.install_dir
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
            .map_err(|err| anyhow::anyhow!("{err}"))
            .cloned()
    }

    pub fn catalog(&self, source: &Source, create: bool) -> anyhow::Result<CatalogImpl> {
        let name = &source.name;
        let path = self.cache_dir().join(name);
        if !path.exists() {
            if create {
                std::fs::create_dir(&path)
                    .with_context(|| format!("Failed to create cache dir {path:?}"))?;
            } else {
                return Err(anyhow::anyhow!("Source no configured: {name}"));
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
            let catalog = self.catalog(source, false)?;
            progress.set_message(name.clone());
            into_search(
                name,
                rt.block_on(catalog.search_with_options(self, &query, all))?,
            )
        } else if conf.num_sources() == 1 {
            let source = conf.iter_sources().next().unwrap();
            let catalog = self.catalog(source, false)?;
            progress.set_message(source.name.clone());
            into_search(
                &source.name,
                rt.block_on(catalog.search_with_options(self, &query, all))?,
            )
        } else {
            rt.block_on(join_all(conf.iter_sources().map(|source| async {
                let catalog = self.catalog(source, false)?;
                progress.set_message(source.name.clone());
                catalog
                    .search_with_options(self, &query, all)
                    .await
                    .map(|v| into_search(&source.name, v))
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

    /// Find plugin matching version request
    pub fn find(
        &self,
        name: &str,
        request: &Match<'_>,
        pre: bool,
        deprecated: bool,
        source: Option<&String>,
    ) -> anyhow::Result<Vec<SearchItem>> {
        let select = Select {
            key: name.into(),
            by_name: true,
            experimental: pre,
            deprecated,
            ..Default::default()
        };

        Ok(if request.matches_any() {
            self.search(select, source, false)?
        } else {
            self.search(select, source, true)?
                .into_iter()
                .filter(|p| request.matches(&p.version))
                .collect()
        })
    }

    /// Remove installed plugins
    pub fn remove(
        &self,
        names: Vec<String>,
    ) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Plugin>>> {
        Ok(
            Installer::installed_plugins(&self.install_dir)?.filter_map(move |result| {
                if let Ok((plugin, folder)) = result {
                    names
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(&plugin.name))
                        .then(|| {
                            fs::remove_dir_all(&folder)
                                .with_context(|| format!("{} {}: ", plugin.name, folder.display()))
                                .map(|()| plugin)
                        })
                } else {
                    log::error!("{}", result.unwrap_err());
                    None
                }
            }),
        )
    }

    /// List installed plugins
    pub fn list(&self, pre: bool, source: Option<&String>) -> anyhow::Result<Vec<OutdatedItem>> {
        Installer::installed_plugins(&self.install_dir)?
            .map(|result| {
                result.and_then(|(plugin, folder)| {
                    Installer::check_outdated_plugin(self, plugin, folder, pre, source)
                })
            })
            .collect()
    }

    pub fn install_candidates(
        &self,
        requirements: Vec<String>,
        pre: bool,
        deprecated: bool,
        source: Option<&String>,
        upgrade: bool,
        reinstall: bool,
    ) -> anyhow::Result<Vec<InstallAction>> {
        if reinstall {
            Installer::install_actions(self, requirements, pre, deprecated, source)?.collect()
        } else {
            Installer::upgrade_actions(self, requirements, pre, deprecated, source, upgrade)?
                .collect()
        }
    }

    pub fn install_plugins(&self, plugins: impl Iterator<Item = SearchItem>) -> Vec<InstallResult> {
        let rt = Self::create_runtime();

        let m = self.progress_printer();
        rt.block_on(join_all(plugins.map(|item| {
            let bar = m.add(ProgressBar::no_length());
            async move {
                let progress = InstallProgress::new(&item.name, bar).unwrap();
                let result =
                    match Installer::download_plugin(self, item.source(), item.as_ref(), &progress)
                        .await
                    {
                        Ok(path) => InstallResult::Ok(item, path),
                        Err(err) => InstallResult::Err(item, err),
                    };
                progress.finish();
                result
            }
        })))
    }

    /// Check source for update
    pub fn check_sources(&self, source: Option<&String>) -> anyhow::Result<()> {
        let rt = Self::create_runtime();

        let m = self.progress_printer();

        // Create catalog
        let create_catalog = |source: &Source| -> anyhow::Result<(CatalogImpl, CacheProgress)> {
            let catalog = self.catalog(source, true)?;
            let progress = CacheProgress::new(&source.name, m.add(ProgressBar::no_length()))?;
            Ok((catalog, progress))
        };

        let conf = self.config();

        if let Some(name) = source {
            let source = conf.try_get_source(name)?;
            let (mut catalog, progress) = create_catalog(source)?;
            rt.block_on(catalog.check_for_update(self, progress))
        } else if conf.num_sources() == 1 {
            let source = conf.iter_sources().next().unwrap();
            let (mut catalog, progress) = create_catalog(source)?;
            rt.block_on(catalog.check_for_update(self, progress))
        } else {
            rt.block_on(join_all(conf.iter_sources().map(|source| async {
                let (mut catalog, progress) = create_catalog(source)?;
                catalog.check_for_update(self, progress).await
            })))
            .into_iter()
            .try_for_each(|res| res)
        }
    }

    /// Helper for syncing sources
    pub fn sync(&mut self, source: Option<&String>) -> anyhow::Result<&mut Self> {
        if !self.no_sync {
            self.refresh_sources(false, source)?;
        }
        Ok(self)
    }

    /// Refresh source's caches
    pub fn refresh_sources(&self, force: bool, source: Option<&String>) -> anyhow::Result<()> {
        let rt = Self::create_runtime();

        let m = self.progress_printer();

        // Create catalog
        let create_catalog = |source: &Source| -> anyhow::Result<(CatalogImpl, CacheProgress)> {
            let catalog = self.catalog(source, true)?;
            let progress = CacheProgress::new(&source.name, m.add(ProgressBar::no_length()))?;
            Ok((catalog, progress))
        };

        let conf = self.config();

        if let Some(name) = source {
            let source = conf.try_get_source(name)?;
            let (mut catalog, progress) = create_catalog(source)?;
            rt.block_on(catalog.refresh(self, progress, force))
        } else {
            rt.block_on(join_all(conf.iter_sources().map(|source| async {
                let (mut catalog, progress) = create_catalog(source)?;
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

    fn progress_printer(&self) -> MultiProgress {
        MultiProgress::with_draw_target(if self.no_progress {
            ProgressDrawTarget::hidden()
        } else {
            ProgressDrawTarget::stderr()
        })
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

pub enum InstallResult {
    Ok(SearchItem, PathBuf),
    Err(SearchItem, anyhow::Error),
}
