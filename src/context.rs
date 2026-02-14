//!
//! Run context
//!
use std::cell::{OnceCell, Ref, RefCell, RefMut};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::FromUtf8Error;

use anyhow::Context;
use futures::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::catalog::{Catalog, CatalogImpl};
use crate::config::{Config, Source};
use crate::echo::RefreshStyle;
use crate::statics::EnvVars;

const USER_AGENT: &str = "Yapt manager";

pub struct RunContext {
    conf_dir: PathBuf,
    cache_dir: PathBuf,
    config: RefCell<Config>,
    client: OnceCell<Result<reqwest::Client, reqwest::Error>>,
    qgis_version: RefCell<Option<semver::Version>>,
}

impl RunContext {
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
            qgis_version: RefCell::new(None),
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
                    .user_agent(match self.qgis_version.borrow().deref() {
                        // See https://github.com/3liz/qgis-plugin-manager/issues/66
                        // See https://lists.osgeo.org/pipermail/qgis-user/2024-May/054439.html
                        Some(version) => format!(
                            "{USER_AGENT}/{} QGIS/{version}/{}",
                            clap::crate_version!(),
                            std::env::consts::OS,
                        ),
                        None => format!("{USER_AGENT}/{}", clap::crate_version!()),
                    })
                    .build()
            })
            .as_ref()
            .map_err(|err| anyhow::anyhow!(format!("{err}")))
            .cloned()
    }

    /// Refresh source'ss caches
    pub fn refresh_sources(&self, force: bool, source: Option<String>) -> anyhow::Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .expect("Failed to create tokio runtime");

        let m = MultiProgress::new();

        // Create catalog
        let create_catalog =
            |name: &str, source: &Source| -> anyhow::Result<(CatalogImpl, ProgressBar)> {
                let path = self.cache_dir().join(name);
                if !path.exists() {
                    std::fs::create_dir(&path)
                        .with_context(|| format!("Failed to create cache dir {path:?}"))?;
                }

                let progress = m.add(ProgressBar::no_length());
                progress.set_style(RefreshStyle::progress(name)?);

                let catalog = CatalogImpl::new(&path, source.url.clone())
                    .with_context(|| format!("Failed to load cache from {path:?}"))?;
                Ok((catalog, progress))
            };

        let conf = self.config();

        if let Some(name) = source {
            let source = conf.try_get_source(&name)?;
            let (mut catalog, progress) = create_catalog(&name, source)?;
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

    // Set the QGIS version
    pub fn qgis_version(&mut self, version: &str) -> anyhow::Result<&mut Self> {
        self.qgis_version.replace(Some(
            semver::Version::parse(version).context("Invalid QGIS version")?,
        ));
        Ok(self)
    }

    // Get the QGIS version if configured, otherwise returns the
    // installed QGIS version if available.
    pub fn get_qgis_version(&self) -> Ref<'_, Option<semver::Version>> {
        if self.qgis_version.borrow().is_none() {
            self.qgis_version
                .replace(Self::installed_qgis_version().and_then(|s| {
                    semver::Version::parse(&s)
                        .inspect_err(|_| {
                            log::error!("Invalid QGIS version");
                        })
                        .ok()
                }));
        }
        self.qgis_version.borrow()
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
                Some(version)
            }
        }
    }
}
