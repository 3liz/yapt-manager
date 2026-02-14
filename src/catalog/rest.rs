//!
//! REST api support
//!
use std::io::Read;

use crate::plugins::Plugin;

#[derive(serde::Deserialize)]
struct Plugins {
    plugins: Vec<Plugin>,
}

pub fn read_catalog<R: Read>(reader: &mut R) -> anyhow::Result<Vec<Plugin>> {
    Ok(serde_json::from_reader::<&mut R, Plugins>(reader)?.plugins)
}
