//!
//! REST api support
//!
use std::io::Read;

use crate::catalog::cached::CacheBuilder;
use crate::plugins::Plugin;

#[derive(serde::Deserialize)]
struct Plugins {
    plugins: Vec<Plugin>,
}

pub fn read_catalog<R: Read>(reader: &mut R) -> anyhow::Result<CacheBuilder> {
    let mut builder = CacheBuilder::new();
    serde_json::from_reader::<&mut R, Plugins>(reader)?
        .plugins
        .into_iter()
        .for_each(|p| builder.insert(p));

    Ok(builder)
}
