//!
//! Handle plugin installation
//!
use std::fs;
use std::io::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Context;
use futures::stream::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::context::{RunContext, SearchItem, Select};
use crate::echo::InstallProgress;
use crate::plugins::Plugin;
use crate::version::{Match, Version};

pub struct InstallItem {
    plugin: Plugin,
    source: Option<Rc<str>>,
    pub outdated: bool,
    pub folder: PathBuf,
    pub latest: Option<Version<'static>>,
}

impl InstallItem {
    #[inline]
    pub fn source(&self) -> Option<&str> {
        self.source.as_ref().map(|s| &s[..])
    }
    #[inline]
    pub fn latest(&self) -> Option<&Version<'static>> {
        self.latest.as_ref()
    }
}

impl Deref for InstallItem {
    type Target = Plugin;

    fn deref(&self) -> &Self::Target {
        &self.plugin
    }
}

impl AsRef<Plugin> for InstallItem {
    fn as_ref(&self) -> &Plugin {
        &self.plugin
    }
}

pub struct Installer;

impl Installer {
    /// List installed plugins
    pub fn list(
        context: &RunContext,
        install_dir: &Path,
        pre: bool,
        source: Option<&String>,
    ) -> anyhow::Result<Vec<InstallItem>> {
        // Look for installed plugins
        let globexpr = install_dir.join("**/metadata.txt");
        glob::glob(&format!("{}", globexpr.display()))?
            .map(|entry| {
                let path = entry?;
                log::debug!("Found plugin metadata in {}", path.display());
                Self::update_from_metadata(context, &path, pre, source)
            })
            .collect() //::<anyhow::Result<Vec<SearchItem>>>()
    }

    /// Update from metadata
    ///
    /// Search for plugin remote sources, return an error
    /// if the plugin  is not found.
    fn update_from_metadata(
        context: &RunContext,
        path: &Path,
        pre: bool,
        source: Option<&String>,
    ) -> anyhow::Result<InstallItem> {
        // Read plugin info from metadata
        let plugin = Plugin::from_metadata(&mut fs::File::open(path)?)?;
        let folder = path.parent().unwrap().to_path_buf();
        Ok(
            if let Some(latest) = context
                .search(
                    Select {
                        key: plugin.name.as_str().into(),
                        by_name: true,
                        server: plugin.server,
                        experimental: pre || plugin.experimental,
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
                InstallItem {
                    plugin,
                    folder,
                    source: Some(latest.source.clone()),
                    latest: outdated.then_some(latest.plugin.version),
                    outdated: outdated,
                }
            } else {
                InstallItem {
                    plugin,
                    folder,
                    outdated: false,
                    source: None,
                    latest: None,
                }
            },
        )
    }

    // Download a plugin
    async fn download_plugin(
        context: &RunContext,
        source: String,
        plugin: &Plugin,
        dest: &Path,
        progress: InstallProgress<'_>,
    ) -> anyhow::Result<PathBuf> {
        let mut tmpfile = tempfile::NamedTempFile::new_in(context.cache_dir().join(source))
            .context("Failed to create temporary file")?;

        let builder = context.client()?.get(&plugin.download_url);

        let res = builder.send().await?;

        if let Some(size) = res.content_length() {
            progress.set_length(size)?;
        };

        let mut stream = match res.status() {
            StatusCode::OK => res.bytes_stream(),
            code => {
                return Err(anyhow::anyhow!(format!(
                    "Failed to download plugin: http error {code}"
                )));
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
        Self::install_archive(tmpfile.path(), dest, &progress)
    }

    // Install from plugin archive
    fn install_archive(
        archive: &Path,
        dest: &Path,
        progress: &InstallProgress<'_>,
    ) -> anyhow::Result<PathBuf> {
        // Get the name of the root folder in archive

        let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
        // Make sure that there is a root directory
        let root = zip
            .root_dir(zip::read::root_dir_common_filter)
            .context("Cannot find root dir in archive")?
            .ok_or(anyhow::anyhow!("No root dir in archive"))?;

        // Backup actual installation
        let installed = dest.join(root.file_name().unwrap());
        let backup = installed.with_added_extension(".bak");
        if installed.exists() {
            fs::rename(&installed, &backup)?;
        }

        // Extract the plugin
        // In case of failure, remove residual and restore backup
        match zip.extract(dest).context("Failed to extract archive") {
            Ok(()) => fs::remove_dir_all(backup),
            Err(err) => {
                // Restore backup dir
                if installed.exists() {
                    fs::remove_dir_all(&installed)?;
                }
                fs::rename(backup, &installed)
            }
        }?;
        Ok(installed)
    }
}
