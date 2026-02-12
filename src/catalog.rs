//!
//! Catalog
//!
use std::borrow::Cow;
use std::path::Path;

use indicatif::ProgressBar;

use crate::plugins::Plugin;

mod cached;

pub struct Select<'a> {
    /// Key: plugin name or tag
    pub key: Option<Cow<'a, str>>,
    /// Request only server plugin
    pub server: bool,
    /// Request only trusted plugins
    pub trusted: bool,
    /// Include experimental plugins
    pub experimental: bool,
    /// Select only by name
    pub by_name: bool,
}

/// Catalog implementation
pub enum CatalogImpl {
    Cached(cached::Cached),
    Rest,
}

impl CatalogImpl {
    pub fn new(cache_dir: &Path, _rest: bool) -> anyhow::Result<CatalogImpl> {
        Ok(CatalogImpl::Cached(cached::Cached::load_from(cache_dir)?))
    }
}

impl Catalog for CatalogImpl {
    async fn find<F: FnMut(&Plugin)>(
        &self,
        uri: &str,
        query: &Select<'_>,
        f: F,
    ) -> anyhow::Result<()> {
        match self {
            Self::Cached(cat) => cat.find(uri, query, f).await,
            Self::Rest => {
                todo!();
            }
        }
    }

    async fn refresh(
        &mut self,
        client: &reqwest::Client,
        uri: &str,
        bar: ProgressBar,
        force: bool,
    ) -> anyhow::Result<()> {
        match self {
            Self::Cached(cat) => cat.refresh(client, uri, bar, force).await,
            Self::Rest => {
                todo!();
            }
        }
    }
}

pub trait Catalog {
    /// Search for plugins
    async fn find<F: FnMut(&Plugin)>(&self, uri: &str, query: &Select, f: F) -> anyhow::Result<()>;

    /// Refresh;
    async fn refresh(
        &mut self,
        client: &reqwest::Client,
        uri: &str,
        bar: ProgressBar,
        force: bool,
    ) -> anyhow::Result<()>;
}
