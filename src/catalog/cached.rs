//!
//! Handle sources
//!
//!
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::echo::{ALERT, CHECK, CROSS, OK};
use crate::errors::Error;
use crate::plugins::Plugin;

use anyhow::Context;
use indicatif::ProgressBar;

use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use time::OffsetDateTime;

use reqwest::{Response, StatusCode, header};

use super::{Catalog, Select};

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
}

impl Catalog for Cached {
    async fn find<F: FnMut(&Plugin)>(
        &self,
        uri: &str,
        query: &Select<'_>,
        f: F,
    ) -> anyhow::Result<()> {
        todo!();
    }

    async fn refresh(
        &mut self,
        client: &reqwest::Client,
        uri: &str,
        bar: ProgressBar,
        force: bool,
    ) -> anyhow::Result<()> {
        bar.set_message("Checking update");

        match self.stream(client, uri, force).await {
            Ok(stream) => {
                if let Some(stream) = stream {
                    bar.set_message("Updating...");
                    self.download(stream).await.inspect_err(|err| {
                        bar.finish_with_message(format!("{ALERT}{CROSS} ERROR: {err}{ALERT:#}"));
                    })?;
                }
                bar.finish_with_message(format!("{OK}{CHECK} Up to date{OK:#}"));
                Ok(())
            }
            Err(err) => {
                bar.finish_with_message(format!("{ALERT}{CROSS} ERROR: {err}{ALERT:#}"));
                Err(err)
            }
        }
    }
}

pub trait ByteStream: Stream<Item = reqwest::Result<Bytes>> + std::marker::Unpin {}
impl<S: Stream<Item = reqwest::Result<Bytes>> + std::marker::Unpin> ByteStream for S {}

impl Cached {
    const CACHE_FILE: &'static str = "cache.json";
    const PLUGINS_FILE: &'static str = "plugins.json";

    // Create a new Cache
    pub fn load_from(cache_dir: &Path) -> anyhow::Result<Self> {
        let path = cache_dir.join(Self::CACHE_FILE);
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

    async fn fetch(
        &self,
        client: &reqwest::Client,
        uri: &str,
    ) -> anyhow::Result<Option<reqwest::RequestBuilder>> {
        let mut builder = client.get(uri);
        if let Some(etag) = &self.etag {
            //
            // We have a ETag, use it
            //
            builder = builder.header(header::IF_NONE_MATCH, etag)
        } else if let Some(last_update) = self.last_update {
            //
            // Compare update date with last-modified header
            //
            if let Some(last_modified) = Self::get_last_modified(uri, client).await? {
                if last_update >= last_modified {
                    // Source is up to date, nothing to do
                    return Ok(None);
                }
            } else {
                // No cache info, bail out
                return Err(anyhow::anyhow!(Error::SourceManualUpdateRequired));
            }
        }
        Ok(Some(builder))
    }

    /// Fetch remote source
    pub async fn stream(
        &mut self,
        client: &reqwest::Client,
        uri: &str,
        force: bool,
    ) -> anyhow::Result<Option<impl ByteStream + use<>>> {
        let builder = if force {
            client.get(uri)
        } else {
            match self.fetch(client, uri).await? {
                Some(builder) => builder,
                None => return Ok(None),
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
    pub async fn download<S: ByteStream>(&mut self, mut stream: S) -> anyhow::Result<()> {
        let mut tmpfile = tempfile::NamedTempFile::new_in(self.cache_dir())
            .context("Failed to create temporary file")?;

        // Download file to temporary
        {
            let mut file = std::io::BufWriter::new(tmpfile.as_file_mut());
            while let Some(chunk) = stream.next().await {
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
            Plugin::read_catalog(&mut body)?
        } else {
            Plugin::read_catalog_xml(&mut body)?
        };

        serde_json::to_writer_pretty(
            fs::File::create(self.cache_dir().join(Self::PLUGINS_FILE))?,
            &catalog,
        )?;
        Ok(())
    }

    /// Save the cache metadata to disk
    pub fn save_update(&mut self) -> anyhow::Result<()> {
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
