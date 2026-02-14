//!
//! Handle plugin installation
//!
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use futures::stream::StreamExt;
use indicatif::ProgressBar;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::context::RunContext;
use crate::echo::InstallProgress;
use crate::plugins::Plugin;

#[derive(Debug, Deserialize, Serialize)]
pub struct LockEntry {
    pub name: String,
    pub source: String,
    pub version: String,
    pub folder: PathBuf,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Installer {
    locked: Vec<LockEntry>,
    #[serde(skip)]
    path: PathBuf,
}

impl Installer {
    const LOCK_FILE: &'static str = "yapt.lock";

    pub fn read_from(path: &Path) -> anyhow::Result<Self> {
        let path = path.join(Self::LOCK_FILE);
        Ok(if path.exists() {
            let file = fs::File::open(&path)?;
            Self {
                path,
                ..serde_json::from_reader(file)?
            }
        } else {
            Self {
                path,
                ..Default::default()
            }
        })
    }

    /// Save lock file
    pub fn save(&self) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(fs::File::create(&self.path)?, self)?;
        Ok(())
    }

    /// Iter lock entries
    pub fn iter(&self) -> impl Iterator<Item = &LockEntry> {
        self.locked.iter()
    }

    // Update a lock entry
    pub fn update(&mut self, source: String, plugin: &Plugin, folder: PathBuf) -> &mut Self {
        if let Some(lock) = self.locked.iter_mut().find(|l| l.name == plugin.name) {
            lock.version = plugin.version.clone();
            lock.source = source;
        } else {
            self.locked.push(LockEntry {
                name: plugin.name.clone(),
                version: plugin.version.clone(),
                source,
                folder,
            })
        }
        self
    }

    // Download a plugin
    pub async fn download_plugin(
        &self,
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
        self.install_archive(context, tmpfile.path(), dest, &progress)
    }

    // Install from plugin archive
    pub fn install_archive(
        &self,
        context: &RunContext,
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
