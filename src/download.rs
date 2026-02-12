//!
//! Handle downloads
//!
use anyhow::Context;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

use futures::future::join_all;

use crate::catalog::{Catalog, CatalogImpl};
use crate::config::{Config, Source};

pub fn download_sources(
    conf: &Config,
    cache_dir: &Path,
    force: bool,
    source: Option<String>,
) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .expect("Failed to create tokio runtime");

    let m = MultiProgress::new();

    // Create catalog
    let create_catalog =
        |name: &str, source: &Source| -> anyhow::Result<(CatalogImpl, ProgressBar)> {
            let path = cache_dir.join(name);
            if !path.exists() {
                std::fs::create_dir(&path)
                    .with_context(|| format!("Failed to create cache dir {path:?}"))?;
            }

            let progress = m.add(ProgressBar::no_length());
            progress.set_style(ProgressStyle::with_template(&format!(
                "{name:.<25} {{msg:.blue:}}"
            ))?);

            let catalog = CatalogImpl::new(&path, source.rest)
                .with_context(|| format!("Failed to load cache from {path:?}"))?;
            Ok((catalog, progress))
        };

    let client = reqwest::Client::builder()
        .user_agent("yapt-manager")
        .build()?;

    if let Some(name) = source {
        let source = conf.try_get_source(&name)?;
        let (mut catalog, progress) = create_catalog(&name, source)?;
        rt.block_on(catalog.refresh(&client, &source.url, progress, force))
    } else {
        rt.block_on(join_all(conf.iter_sources().map(|(name, source)| async {
            let (mut catalog, progress) = create_catalog(name, source)?;
            catalog.refresh(&client, &source.url, progress, force).await
        })))
        .into_iter()
        .try_for_each(|res| res)
    }
}
