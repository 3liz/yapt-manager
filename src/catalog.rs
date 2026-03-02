//!
//! Catalog
//!
use std::borrow::Cow;
use std::path::Path;

use strsim::jaro_winkler;

use crate::context::RunContext;
use crate::plugins::Plugin;
use crate::printer::CacheProgress;
use crate::version::SemVer;

pub(crate) mod cached;

mod rest;

#[derive(Default)]
pub struct Select<'a> {
    /// Key: plugin name fragment or tag
    pub key: Cow<'a, str>,
    /// Request only server plugin
    pub server: bool,
    /// Include experimental plugins
    pub experimental: bool,
    /// Include deprecated plugins
    pub deprecated: bool,
    /// By plugin name exact match
    pub by_name: bool,
    /// Qgis_version supported
    pub qgis_version: SemVer,
}

impl<'a> Select<'a> {
    /// Check if plugin matches selection
    pub fn matches(&self, p: &Plugin) -> bool {
        self.key(p)
            && self.server(p)
            && self.experimental(p)
            && self.deprecated(p)
            && self.qgis_version(p)
    }

    pub fn matches_by_name(&self, s: &str) -> bool {
        !self.by_name || self.key.eq_ignore_ascii_case(s)
    }

    pub fn key(&self, p: &Plugin) -> bool {
        if self.by_name {
            // Exact plugin name match
            self.key.eq_ignore_ascii_case(&p.name)
        } else {
            const MINMATCH: f64 = 0.8;
            // Use Jaro Winkler comparison
            jaro_winkler(&self.key, &p.slug) > MINMATCH
                || p.tags
                    .split(',')
                    .any(|tag| jaro_winkler(&self.key, tag.trim()) > MINMATCH)
        }
    }

    #[inline]
    pub fn server(&self, p: &Plugin) -> bool {
        p.server || !self.server
    }
    #[inline]
    pub fn experimental(&self, p: &Plugin) -> bool {
        !p.experimental || self.experimental
    }
    #[inline]
    pub fn deprecated(&self, p: &Plugin) -> bool {
        !p.deprecated || self.deprecated
    }
    #[inline]
    pub fn qgis_version(&self, p: &Plugin) -> bool {
        if !self.qgis_version.is_none() {
            p.matches_qgis_version(&self.qgis_version)
        } else {
            true
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

    async fn search_all(
        &self,
        context: &RunContext,
        query: &Select<'_>,
    ) -> anyhow::Result<Vec<Plugin>> {
        match self {
            Self::Cached(cat) => cat.search_all(context, query).await,
            Self::Rest => {
                todo!();
            }
        }
    }

    async fn refresh(
        &mut self,
        context: &RunContext,
        bar: CacheProgress,
        force: bool,
    ) -> anyhow::Result<()> {
        match self {
            Self::Cached(cat) => cat.refresh(context, bar, force).await,
            Self::Rest => {
                todo!();
            }
        }
    }

    /// Check for update
    async fn check_for_update(
        &mut self,
        context: &RunContext,
        bar: CacheProgress,
    ) -> anyhow::Result<()> {
        match self {
            Self::Cached(cat) => cat.check_for_update(context, bar).await,
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

    async fn search_all(
        &self,
        context: &RunContext,
        query: &Select<'_>,
    ) -> anyhow::Result<Vec<Plugin>>;

    async fn search_with_options(
        &self,
        context: &RunContext,
        query: &Select<'_>,
        all: bool,
    ) -> anyhow::Result<Vec<Plugin>> {
        if all {
            self.search_all(context, query).await
        } else {
            self.search(context, query).await
        }
    }

    /// Refresh;
    async fn refresh(
        &mut self,
        context: &RunContext,
        bar: CacheProgress,
        force: bool,
    ) -> anyhow::Result<()>;

    /// Check for update
    async fn check_for_update(
        &mut self,
        context: &RunContext,
        bar: CacheProgress,
    ) -> anyhow::Result<()>;
}
