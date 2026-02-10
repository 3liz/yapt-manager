//!
//! Handle downloads
//!
use anyhow::Context;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

use futures::future::join_all;

use crate::cache::Cache;
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

    // Create cache directories
    let create_cache_pb = |name: &String| -> anyhow::Result<(Cache, ProgressBar)> {
        let path = cache_dir.join(name);
        if !path.exists() {
            std::fs::create_dir(&path)
                .with_context(|| format!("Failed to create cache dir {path:?}"))?;
        }

        let pb = m.add(ProgressBar::no_length());
        pb.set_style(ProgressStyle::with_template(&format!(
            "{name:.<25} {{msg:.blue:}}"
        ))?);

        let cache = Cache::load_from(&path)
            .with_context(|| format!("Failed to load cache from {path:?}"))?;
        Ok((cache, pb))
    };

    let client = reqwest::Client::builder()
        .user_agent("yapt-manager")
        .build()?;

    if let Some(name) = source {
        let source = conf.try_get_source(&name)?;
        let (mut cache, pb) = create_cache_pb(&name)?;
        rt.block_on(cache.update_with_progress(&client, source.url(), pb, force))
    } else {
        rt.block_on(join_all(conf.iter_sources().map(|(name, source)| async {
            let (mut cache, pb) = create_cache_pb(name)?;
            cache
                .update_with_progress(&client, source.url(), pb, force)
                .await
        })))
        .into_iter()
        .try_for_each(|res| res)
    }
}
