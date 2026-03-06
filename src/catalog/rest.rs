//!
//! REST api support
//!
use reqwest::StatusCode;

use crate::catalog::Catalog;
use crate::context::RunContext;
use crate::errors::Error;
use crate::plugins::Plugin;
use crate::printer::CacheProgress;

use super::Select;

pub struct Rest {
    url: String,
}

impl Catalog for Rest {
    /// Search for plugins
    async fn search(
        &self,
        context: &RunContext,
        query: &Select<'_>,
    ) -> anyhow::Result<Vec<Plugin>> {
        let client = context.client()?;

        let url = self.build_url(query);

        self.fetch(client.get(url)).await
    }

    /// Search for all versions of plugins
    async fn search_all(
        &self,
        context: &RunContext,
        query: &Select<'_>,
    ) -> anyhow::Result<Vec<Plugin>> {
        let client = context.client()?;

        let mut url = self.build_url(query);
        url.push_str("&all=true");
        log::debug!("Fetch REST resources at: {url}");

        self.fetch(client.get(url)).await
    }

    /// Refresh;
    async fn refresh(
        &mut self,
        context: &RunContext,
        bar: CacheProgress,
        force: bool,
    ) -> anyhow::Result<()> {
        bar.finish_with_success();
        Ok(())
    }

    /// Check for update
    async fn check_for_update(
        &mut self,
        context: &RunContext,
        bar: CacheProgress,
    ) -> anyhow::Result<()> {
        bar.finish_with_success();
        Ok(())
    }
}

impl Rest {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    fn build_url(&self, opts: &Select<'_>) -> String {
        let mut url = String::with_capacity(128usize);
        if let Some((base, query)) = self.url.rsplit_once('?') {
            url.push_str(base.trim_end_matches('/'));
            if opts.by_name {
                url.push_str("/plugins/");
                url.push_str(&opts.key);
            }
            url.push_str("/plugins.json?");
            url.push_str(query);
        } else {
            url.push_str(self.url.trim_end_matches('/'));
            url.push_str("/plugins.json");
            url.push_str("?qgis=");
            url.push_str(&opts.qgis_version.to_string());
        }
        if opts.server {
            url.push_str("&server=true");
        }
        if opts.deprecated {
            url.push_str("&deprecated=true");
        }
        if opts.experimental {
            url.push_str("&pre=true");
        }
        if !opts.by_name && !opts.key.is_empty() {
            url.push_str("&tags=");
            url.push_str(&opts.key);
        }

        url
    }

    /// Fetch remote source
    async fn fetch(&self, builder: reqwest::RequestBuilder) -> anyhow::Result<Vec<Plugin>> {
        let res = builder.send().await?;
        match res.status() {
            StatusCode::OK => {
                #[derive(serde::Deserialize)]
                struct Plugins {
                    plugins: Vec<Plugin>,
                }

                Ok(res.json::<Plugins>().await?.plugins)
            }
            code => Err(anyhow::anyhow!(
                "Failed to get plugins from REST api: code {}, {}",
                Error::SourceRequestFailure(code),
                res.text().await?,
            )),
        }
    }
}
