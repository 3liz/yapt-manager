//!
//! Handle plugin installation
//!
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::Context;
use futures::stream::StreamExt;
use reqwest::StatusCode;

use crate::context::{RunContext, SearchItem, Select};
use crate::plugins::Plugin;
use crate::printer::InstallProgress;
use crate::version::Version;

pub enum InstallAction {
    Install(SearchItem),
    Upgrade(SearchItem, PathBuf),
    Unchanged(SearchItem, PathBuf),
    NotFound(String),
}

pub struct OutdatedItem {
    plugin: Plugin,
    pub outdated: bool,
    pub folder: PathBuf,
    pub latest: Option<SearchItem>,
}

impl OutdatedItem {
    #[inline]
    pub fn source(&self) -> Option<&str> {
        self.latest.as_ref().map(|s| s.source())
    }
    #[inline]
    pub fn latest(&self) -> Option<&SearchItem> {
        self.latest.as_ref()
    }
}

impl Deref for OutdatedItem {
    type Target = Plugin;

    fn deref(&self) -> &Self::Target {
        &self.plugin
    }
}

impl AsRef<Plugin> for OutdatedItem {
    fn as_ref(&self) -> &Plugin {
        &self.plugin
    }
}

pub struct Installer;

impl Installer {
    /// List installed plugins
    pub fn installed_plugins(
        install_dir: &Path,
    ) -> anyhow::Result<impl Iterator<Item = anyhow::Result<(Plugin, PathBuf)>>> {
        // Look for installed plugins (no-recurse)
        let globexpr = install_dir.join("*/metadata.txt");
        Ok(
            glob::glob(&format!("{}", globexpr.display()))?.map(|entry| {
                let path = entry?;
                log::debug!("Found plugin metadata: {}", path.display());
                Plugin::from_metadata(&mut fs::File::open(&path)?)
                    .map(|p| (p, path.parent().unwrap().to_path_buf()))
            }),
        )
    }

    /// Return plugin install info
    ///
    /// Search for plugin remote sources, return an error
    /// if the plugin  is not found.
    pub fn check_outdated_plugin(
        context: &RunContext,
        plugin: Plugin,
        folder: PathBuf,
        pre: bool,
        source: Option<&String>,
    ) -> anyhow::Result<OutdatedItem> {
        Ok(
            if let Some(latest) = context
                .search(
                    Select {
                        key: plugin.name.as_str().into(),
                        by_name: true,
                        server: plugin.server,
                        experimental: pre,
                        deprecated: plugin.deprecated,
                        ..Default::default()
                    },
                    source,
                    false, // Search only latests
                )?
                .into_iter()
                .next()
            {
                let outdated = latest.version > plugin.version;
                OutdatedItem {
                    plugin,
                    folder,
                    latest: Some(latest),
                    outdated,
                }
            } else {
                OutdatedItem {
                    plugin,
                    folder,
                    latest: None,
                    outdated: false,
                }
            },
        )
    }

    /// Download a plugin
    pub async fn download_plugin(
        context: &RunContext,
        source: &str,
        plugin: &Plugin,
        progress: &InstallProgress,
    ) -> anyhow::Result<PathBuf> {
        let mut tmpfile = tempfile::NamedTempFile::new_in(context.cache_dir().join(source))
            .context("Failed to create temporary file")?;

        let builder = context.client()?.get(&plugin.download_url);

        log::debug!("Downloading plugin {}", plugin.download_url);
        let res = builder.send().await?;

        if let Some(size) = res.content_length() {
            progress.set_length(&plugin.name, size)?;
        };

        let mut stream = match res.status() {
            StatusCode::OK => res.bytes_stream(),
            code => {
                return Err(anyhow::anyhow!(
                    "Failed to download plugin: http error {code}"
                ));
            }
        };

        {
            let mut file = std::io::BufWriter::new(tmpfile.as_file_mut());
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                file.write_all(&chunk)?;
                progress.inc(chunk.len());
            }
            file.flush()?;
        }
        Self::install_archive(tmpfile.path(), context.install_dir())
    }

    // Install from plugin archive
    fn install_archive(archive: &Path, dest: &Path) -> anyhow::Result<PathBuf> {
        // Get the name of the root folder in archive

        let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
        // Make sure that there is a root directory
        let root = zip
            .root_dir(zip::read::root_dir_common_filter)
            .context("Cannot find root dir in archive")?
            .ok_or(anyhow::anyhow!("No root dir in archive"))?;

        // Backup actual installation
        let installed = dest.join(root.file_name().unwrap());
        let backup = installed.with_added_extension("bak");
        if installed.exists() {
            log::debug!("Creating backup of installed plugin {backup:?}");
            fs::rename(&installed, &backup)?;
        }

        // Extract the plugin
        // In case of failure, remove residual and restore backup
        log::debug!("Extracting archive {archive:?}");
        if let Err(err) = zip.extract(dest).context("Failed to extract archive") {
            // Restore backup dir
            if installed.exists() {
                fs::remove_dir_all(&installed)?;
            }
            fs::rename(backup, &installed)?;
            return Err(anyhow::anyhow!(err));
        }

        // Remove backup
        if backup.exists() {
            log::debug!("Removing backup {backup:?}");
            if let Err(err) = fs::remove_dir_all(&backup) {
                log::error!("Cannot remove director {}: {err}", backup.display());
            }
        }
        Ok(installed)
    }

    /// Determine install actions from
    /// requirements list
    pub fn install_actions(
        context: &RunContext,
        requirements: Vec<String>,
        pre: bool,
        deprecated: bool,
        source: Option<&String>,
        upgrade: bool,
        reinstall: bool,
    ) -> anyhow::Result<impl Iterator<Item = anyhow::Result<InstallAction>>> {
        use InstallAction::*;

        let mut installed: HashMap<String, (Version, PathBuf)> =
            Self::installed_plugins(context.install_dir())?
                .map(|v| v.map(|(plugin, path)| (plugin.name, (plugin.version, path))))
                .collect::<anyhow::Result<_>>()?;

        Ok(requirements.into_iter().map(move |s| {
            let (name, request) = crate::version::parse_requirement(&s)?;
            Ok(
                if let Some(candidate) = context
                    .find(name, &request, pre, deprecated, source)?
                    .into_iter()
                    .next()
                {
                    if let Some((installed_version, path)) = installed.remove(&candidate.name) {
                        if path.is_symlink() {
                            // Skip symlink
                            log::warn!("Skipping {path:?} because it is a symlink");
                            Unchanged(candidate, path)
                        } else if reinstall {
                            Install(candidate)
                        } else if upgrade && installed_version < candidate.version {
                            Upgrade(candidate, path)
                        } else {
                            Unchanged(candidate, path)
                        }
                    } else {
                        Install(candidate)
                    }
                } else {
                    NotFound(s.to_string())
                },
            )
        }))
    }
}
