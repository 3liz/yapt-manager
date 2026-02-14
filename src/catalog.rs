//!
//! Catalog
//!
use std::borrow::Cow;
use std::path::Path;

use indicatif::ProgressBar;

use crate::context::RunContext;
use crate::plugins::Plugin;

mod cached;
mod rest;

pub struct Select<'a> {
    /// Key: plugin name fragment or tag
    pub key: Cow<'a, str>,
    /// Request only server plugin
    pub server: bool,
    /// Request only trusted plugins
    pub trusted: bool,
    /// Include experimental plugins
    pub experimental: bool,
    /// By plugin name
    pub by_name: bool,
}

impl<'a> Select<'a> {
    pub fn by_name(key: Cow<'a, str>, pre: bool) -> Self {
        Self {
            key,
            server: false,
            trusted: false,
            experimental: pre,
            by_name: true,
        }
    }
}

/// Catalog implementation
pub enum CatalogImpl {
    Cached(cached::Cached),
    Rest,
}

impl CatalogImpl {
    pub fn new(cache_dir: &Path, uri: String) -> anyhow::Result<CatalogImpl> {
        Ok(CatalogImpl::Cached(cached::Cached::load_from(
            cache_dir, uri,
        )?))
    }
}

impl Catalog for CatalogImpl {
    async fn search(
        &self,
        context: &RunContext,
        query: &Select<'_>,
    ) -> anyhow::Result<Vec<Plugin>> {
        match self {
            Self::Cached(cat) => cat.search(context, query).await,
            Self::Rest => {
                todo!();
            }
        }
    }

    async fn refresh(
        &mut self,
        context: &RunContext,
        bar: ProgressBar,
        force: bool,
    ) -> anyhow::Result<()> {
        match self {
            Self::Cached(cat) => cat.refresh(context, bar, force).await,
            Self::Rest => {
                todo!();
            }
        }
    }
}

pub trait Catalog {
    /// Search for plugins
    async fn search(&self, context: &RunContext, query: &Select<'_>)
    -> anyhow::Result<Vec<Plugin>>;

    /// Refresh;
    async fn refresh(
        &mut self,
        context: &RunContext,
        bar: ProgressBar,
        force: bool,
    ) -> anyhow::Result<()>;
}
