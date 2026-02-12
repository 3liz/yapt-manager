//!
//! YAPT manager configuration
//!
use std::collections::{HashMap, hash_map};
use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::Error;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Source {
    pub url: String,
    #[serde(default)]
    pub rest: bool,
}

impl Source {
    pub fn new(url: String, rest: bool) -> Self {
        Self { url, rest }
    }
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    sources: HashMap<String, Source>,

    #[serde(skip)]
    path: PathBuf,
}

impl Config {
    const CONF_FILE: &'static str = "config.json";

    pub fn add_source(&mut self, name: String, source: Source) -> Result<&mut Self, Error> {
        match self.sources.entry(name) {
            hash_map::Entry::Occupied(e) => Err(Error::SourceExists(e.key().clone())),
            hash_map::Entry::Vacant(e) => {
                e.insert(source);
                Ok(self)
            }
        }
    }

    pub fn remove_source(&mut self, name: &str) -> Result<&mut Self, Error> {
        if self.sources.remove(name).is_none() {
            Err(Error::SourceNotExists(name.to_string()))
        } else {
            Ok(self)
        }
    }

    pub fn rename_source(&mut self, old: &str, new: &str) -> Result<&mut Self, Error> {
        if self.sources.contains_key(new) {
            Err(Error::SourceExists(new.to_string()))
        } else {
            let source = self
                .sources
                .remove(old)
                .ok_or_else(|| Error::SourceNotExists(old.to_string()))?;

            self.sources.insert(new.to_string(), source);
            Ok(self)
        }
    }

    pub fn iter_sources(&self) -> impl Iterator<Item = (&String, &Source)> {
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

    pub fn try_get_source(&self, name: &str) -> Result<&Source, Error> {
        self.sources
            .get(name)
            .ok_or_else(|| Error::SourceNotExists(name.to_string()))
    }
}
