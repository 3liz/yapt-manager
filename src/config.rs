//!
//! YAPT manager configuration
//!
use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::Error;
use crate::version::SemVer;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Source {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub rest: bool,
    #[serde(default)]
    pub template: bool,
}

impl Source {
    pub fn new(name: String, url: String, rest: bool) -> Self {
        let template = url.contains("{VERSION}");
        Self {
            name,
            url,
            rest,
            template,
        }
    }

    #[inline]
    pub fn is(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }

    pub fn try_url(&self, qgis_version: &SemVer) -> Result<Cow<'_, str>, Error> {
        if self.template {
            if qgis_version.is_none() {
                Err(Error::QgisVersionRequired)
            } else {
                Ok(self
                    .url
                    .replace("{VERSION}", &qgis_version.to_string())
                    .into())
            }
        } else {
            Ok(self.url.as_str().into())
        }
    }
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    sources: Vec<Source>,

    #[serde(skip)]
    path: PathBuf,
}

impl Config {
    const CONF_FILE: &'static str = "config.json";

    pub fn add_source(&mut self, source: Source) -> Result<&mut Self, Error> {
        if self.sources.iter().any(|s| s.is(&source.name)) {
            Err(Error::SourceExists(source.name))
        } else {
            self.sources.push(source);
            Ok(self)
        }
    }

    pub fn remove_source(&mut self, name: &str) -> Result<&mut Self, Error> {
        if self.sources.extract_if(.., |s| s.is(name)).count() == 0 {
            Err(Error::SourceNotExists(name.to_string()))
        } else {
            Ok(self)
        }
    }

    pub fn rename_source(&mut self, old: &str, new: &str) -> Result<&mut Self, Error> {
        if self.get_source(new).is_some() {
            Err(Error::SourceExists(new.to_string()))
        } else if let Some(source) = self.sources.iter_mut().find(|s| s.is(old)) {
            source.name = new.to_string();
            Ok(self)
        } else {
            Err(Error::SourceNotExists(old.to_string()))
        }
    }

    #[inline]
    pub fn num_sources(&self) -> usize {
        self.sources.len()
    }

    #[inline]
    pub fn iter_sources(&self) -> impl Iterator<Item = &Source> {
        self.sources.iter()
    }

    pub fn load_from(conf_dir: &Path) -> anyhow::Result<Self> {
        let path = conf_dir.join(Self::CONF_FILE);
        if path.exists() {
            let file = fs::File::open(&path)?;
            Ok(Self {
                path,
                ..serde_json::from_reader(file)?
            })
        } else {
            Ok(Self {
                path,
                ..Default::default()
            })
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let conf_dir = self.path.parent().unwrap();
        if !conf_dir.exists() {
            fs::create_dir(conf_dir)?;
        }
        serde_json::to_writer_pretty(fs::File::create(&self.path)?, self)?;
        Ok(())
    }

    #[inline]
    pub fn get_source(&self, name: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.is(name))
    }

    #[inline]
    pub fn try_get_source(&self, name: &str) -> Result<&Source, Error> {
        self.get_source(name)
            .ok_or_else(|| Error::SourceNotExists(name.to_string()))
    }
}
