//!
//! Cache remote source data
//!
//! Plugins server like plugins.qgis.org don't have an api;
//! the (partial) content of available plugins must
//! be downloaded and cached for subsequent requests.
//!
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use crate::context::RunContext;
use crate::echo::{ALERT, CHECK, CROSS, OK, RefreshStyle};
use crate::errors::Error;
use crate::plugins::Plugin;
use crate::version::Match;

use anyhow::Context;
use indicatif::ProgressBar;

use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use time::OffsetDateTime;

use reqwest::{Response, StatusCode, header};
use std::collections::HashMap;

use super::{Catalog, Select, rest};

type PluginMap = HashMap<String, Vec<Plugin>>;

// Plugin cache
#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Plugins {
    plugins: PluginMap,
}

/// Cache builder
pub struct CacheBuilder {
    inner: PluginMap,
}

impl CacheBuilder {
    pub fn new() -> Self {
        Self {
            inner: PluginMap::new(),
        }
    }

    pub fn insert(&mut self, p: Plugin) {
        match self.inner.get_mut(&p.name) {
            Some(v) => v.push(p),
            None => {
                self.inner.insert(p.name.clone(), vec![p]);
            }
        }
    }

    fn build(mut self) -> Plugins {
        // Sort each buckets by version
        self.inner.values_mut().for_each(|v| {
            v.sort_by(|a, b| b.version.partial_cmp(&a.version).unwrap());
        });

        Plugins {
            plugins: self.inner,
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum UpdateStatus {
    NeedUpdate(reqwest::RequestBuilder),
    UpToDate,
    ManualUpdate,
}

/// Cache metadata
///
/// Store update time and HTTP cache informations
#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Cached {
    /// Last updated time
    last_update: Option<time::OffsetDateTime>,
    etag: Option<String>,
    content_type: Option<String>,
    #[serde(skip)]
    path: PathBuf,
    #[serde(skip)]
    uri: String,
}

type CacheMap = HashMap<PathBuf, Plugins>;

static SEARCH_CACHE: LazyLock<Mutex<CacheMap>> = LazyLock::new(|| Mutex::new(CacheMap::new()));

impl Catalog for Cached {
    async fn search_all(
        &self,
        context: &RunContext,
        query: &Select<'_>,
    ) -> anyhow::Result<Vec<Plugin>> {
        Ok(self
            .search_cache()?
            .plugins
            .drain()
            .filter_map(|(k, v)| query.matches_by_name(&k).then_some(v))
            .flatten()
            .filter(|p| query.matches(p))
            .collect())
    }

    /// Returns only the latest versions of each plugins
    async fn search(
        &self,
        context: &RunContext,
        query: &Select<'_>,
    ) -> anyhow::Result<Vec<Plugin>> {
        Ok(self
            .search_cache()?
            .plugins
            .drain()
            .filter_map(|(k, v)| query.matches_by_name(&k).then_some(v))
            .filter_map(|v| v.into_iter().filter(|p| query.matches(p)).take(1).next())
            .collect())
    }

    async fn refresh(
        &mut self,
        context: &RunContext,
        bar: ProgressBar,
        force: bool,
    ) -> anyhow::Result<()> {
        bar.set_message("Checking update");

        let client = context.client()?;

        match self.stream(&client, force).await {
            Ok(stream) => {
                if let Some(stream) = stream {
                    bar.set_message("Updating...");
                    self.download(stream, &bar).await.inspect_err(|_| {
                        bar.finish_with_message(RefreshStyle::error_msg());
                    })?;
                }
                bar.finish_with_message(RefreshStyle::ok_msg());
                Ok(())
            }
            Err(err) => {
                bar.finish_with_message(RefreshStyle::error_msg());
                Err(err)
            }
        }
    }

    async fn check_for_update(
        &mut self,
        context: &RunContext,
        bar: ProgressBar,
    ) -> anyhow::Result<()> {
        bar.set_message("Checking update");

        let client = context.client()?;

        if let Some(etag) = &self.etag {
            //
            // We have a ETag, use it
            //
            let res = client
                .head(&self.uri)
                .header(header::IF_NONE_MATCH, etag)
                .send()
                .await?;

            match res.status() {
                StatusCode::NOT_MODIFIED => bar.finish_with_message(RefreshStyle::ok_msg()),
                StatusCode::OK => {
                    bar.finish_with_message(RefreshStyle::warn_msg("Update required"))
                }
                code => return Err(anyhow::anyhow!(Error::SourceRequestFailure(code))),
            }
        } else {
            match self.fetch(&client).await? {
                UpdateStatus::UpToDate => {
                    bar.finish_with_message(RefreshStyle::ok_msg());
                }
                UpdateStatus::NeedUpdate(_) => {
                    bar.finish_with_message(RefreshStyle::warn_msg("Update required"));
                }
                UpdateStatus::ManualUpdate => {
                    bar.finish_with_message(RefreshStyle::warn_msg("Require manual update"));
                }
            }
        }
        Ok(())
    }

    /// Find plugin with version request
    async fn find<'a>(
        &self,
        context: &RunContext,
        name: &str,
        request: &Match<'a>,
    ) -> anyhow::Result<Option<Plugin>> {
        todo!();
    }
}

pub trait ByteStream: Stream<Item = reqwest::Result<Bytes>> + std::marker::Unpin {}
impl<S: Stream<Item = reqwest::Result<Bytes>> + std::marker::Unpin> ByteStream for S {}

impl Cached {
    const CACHE_FILE: &'static str = "cache.json";
    const PLUGINS_FILE: &'static str = "plugins.json";

    // Create a new Cache
    pub fn load_from(cache_dir: &Path, uri: String) -> anyhow::Result<Self> {
        let path = cache_dir.join(Self::CACHE_FILE);
        Ok(if path.exists() {
            let file = fs::File::open(&path)?;
            Self {
                path,
                uri,
                ..serde_json::from_reader(file)?
            }
        } else {
            Self {
                path,
                uri,
                ..Default::default()
            }
        })
    }

    /// Load cached catalog
    fn load_cache(&self) -> anyhow::Result<Plugins> {
        let cache_file = self.cache_dir().join(Self::PLUGINS_FILE);
        Ok(if cache_file.exists() {
            log::debug!("Loading cache data from {}", cache_file.display());
            serde_json::from_reader(fs::File::open(&cache_file)?)
                .inspect_err(|_| log::error!("Failed to load cache file at {cache_file:?}"))?
        } else {
            CacheBuilder::new().build()
        })
    }

    /// Get plugins from search cache
    fn search_cache(&self) -> anyhow::Result<Plugins> {
        let mut cache = SEARCH_CACHE.lock().unwrap();
        let cache_dir = self.cache_dir();
        if let Some(plugins) = cache.get(cache_dir) {
            log::debug!("Get cached plugins for {}", cache_dir.display());
            Ok(plugins.clone())
        } else {
            let plugins = self.load_cache()?;
            cache.insert(cache_dir.to_path_buf(), plugins.clone());
            Ok(plugins)
        }
    }

    async fn get_last_modified(
        uri: &str,
        client: &reqwest::Client,
    ) -> anyhow::Result<Option<OffsetDateTime>> {
        let res = client.head(uri).send().await?;

        if res.status() != reqwest::StatusCode::OK {
            Err(Error::SourceRequestFailure(res.status()))?;
        }

        Ok(
            if let Some(last_modified) = get_header_str(&res, &header::LAST_MODIFIED) {
                // Check last-modified
                OffsetDateTime::parse(
                    last_modified,
                    &time::format_description::well_known::Rfc2822,
                )
                .ok()
                .or_else(|| {
                    log::debug!("Invalid Last-Modified date: {:?}", last_modified);
                    None
                })
            } else {
                None
            },
        )
    }

    async fn fetch(&self, client: &reqwest::Client) -> anyhow::Result<UpdateStatus> {
        let mut builder = client.get(&self.uri);
        if let Some(etag) = &self.etag {
            //
            // We have a ETag, use it
            //
            builder = builder.header(header::IF_NONE_MATCH, etag)
        } else if let Some(last_update) = self.last_update {
            //
            // Compare update date with last-modified header
            //
            if let Some(last_modified) = Self::get_last_modified(&self.uri, client).await? {
                if last_update >= last_modified {
                    // Source is up to date, nothing to do
                    return Ok(UpdateStatus::UpToDate);
                }
            } else {
                // No cache info, bail out
                return Ok(UpdateStatus::ManualUpdate);
            }
        }
        Ok(UpdateStatus::NeedUpdate(builder))
    }

    /// Fetch remote source
    async fn stream(
        &mut self,
        client: &reqwest::Client,
        force: bool,
    ) -> anyhow::Result<Option<impl ByteStream + use<>>> {
        let builder = if force {
            client.get(&self.uri)
        } else {
            match self.fetch(client).await? {
                UpdateStatus::NeedUpdate(builder) => builder,
                UpdateStatus::UpToDate => return Ok(None),
                UpdateStatus::ManualUpdate => {
                    return Err(anyhow::anyhow!(Error::SourceManualUpdateRequired));
                }
            }
        };

        let res = builder.send().await?;
        match res.status() {
            StatusCode::NOT_MODIFIED => Ok(None),
            StatusCode::OK => {
                self.etag = get_header_str(&res, &header::ETAG).map(String::from);
                self.content_type = get_header_str(&res, &header::CONTENT_TYPE).map(String::from);
                Ok(Some(res.bytes_stream()))
            }
            code => Err(anyhow::anyhow!(Error::SourceRequestFailure(code))),
        }
    }

    /// Return the cache directory
    pub fn cache_dir(&self) -> &Path {
        self.path.parent().unwrap()
    }

    /// Download data from stream
    async fn download<S: ByteStream>(
        &mut self,
        mut stream: S,
        progress: &ProgressBar,
    ) -> anyhow::Result<()> {
        let mut tmpfile = tempfile::NamedTempFile::new_in(self.cache_dir())
            .context("Failed to create temporary file")?;

        // Download file to temporary
        {
            let mut file = std::io::BufWriter::new(tmpfile.as_file_mut());
            while let Some(chunk) = stream.next().await {
                progress.tick();
                file.write_all(&chunk?)?;
            }
            file.flush()?;
        }

        // Update the catalog
        self.parse_catalog(tmpfile.path())?;

        // Update the cache meta file
        self.save_update()?;

        Ok(())
    }

    fn parse_catalog(&self, path: &Path) -> anyhow::Result<()> {
        let mut body = &mut fs::File::open(path)?;

        // Parse the file and replace the actual cache file
        let catalog = if let Some(ref content_type) = self.content_type
            && content_type.as_str() == mime::APPLICATION_JSON
        {
            rest::read_catalog(&mut body)?
        } else {
            Plugin::read_catalog_xml(&mut body)?
        }
        .build();

        let cache_dir = self.cache_dir();

        // Write catalog to file system
        serde_json::to_writer_pretty(
            fs::File::create(cache_dir.join(Self::PLUGINS_FILE))?,
            &catalog,
        )?;

        // Add catalog to search cache
        SEARCH_CACHE
            .lock()
            .unwrap()
            .insert(cache_dir.to_path_buf(), catalog);

        Ok(())
    }

    /// Save the cache metadata to disk
    fn save_update(&mut self) -> anyhow::Result<()> {
        self.last_update = Some(time::OffsetDateTime::now_utc());
        serde_json::to_writer_pretty(fs::File::create(&self.path)?, self)?;
        Ok(())
    }
}

// Get header as str
fn get_header_str<K>(res: &Response, key: K) -> Option<&str>
where
    K: header::AsHeaderName,
{
    res.headers().get(key).and_then(|s| s.to_str().ok())
}
