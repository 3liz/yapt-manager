//!
//! Parse metadata
//!
use std::io::Read;
use std::str::FromStr;

use anyhow::Context;

use crate::errors::Error;

use super::Plugin;

impl Plugin {
    /// Parse metadata
    ///
    /// This is a partial initialisation  since we do not need
    /// all the metadata.
    pub fn from_metadata<R: Read>(r: &mut R) -> anyhow::Result<Self> {
        // Read metadata as ini file
        let ini_parser = ini::Ini::read_from_opt(
            r,
            ini::ParseOption {
                enabled_indented_multiline_value: true,
                ..Default::default()
            },
        )
        .context("Failed to read plugin metadata")?;

        let metadata = ini_parser
            .section(Some("general"))
            .ok_or_else(|| Error::PluginMetadata("Missing 'general' section".into()))?;

        Ok(Self {
            name: metadata.get_string("name", true)?,
            description: metadata.get_string("description", true)?,
            version: metadata.get_string("version", true)?,
            qgis_minimum_version: metadata.get_string("qgisMinimumVersion", true)?,
            qgis_maximum_version: metadata.get_string("qgisMaximumVersion", false)?,
            author_name: metadata.get_string("author", true)?,
            experimental: metadata.get_bool("experimental", false)?,
            deprecated: metadata.get_bool("deprecated", false)?,
            server: metadata.get_bool("server", false)?,
            tags: metadata.get_string("tags", false)?,
            ..Default::default()
        })
    }
}

trait Metadata {
    fn get_metadata(&self, k: &'static str, required: bool) -> Result<&str, Error>;

    fn get_string(&self, k: &'static str, required: bool) -> Result<String, Error> {
        self.get_metadata(k, required).map(Into::<String>::into)
    }
    fn get_bool(&self, k: &'static str, required: bool) -> Result<bool, Error> {
        self.get_metadata(k, required)
            .and_then(|v| parse_bool(v, k))
    }
}

fn parse_bool(v: &str, k: &str) -> Result<bool, Error> {
    match v {
        "" => Ok(false),
        _ if v.eq_ignore_ascii_case("true") => Ok(true),
        _ if v.eq_ignore_ascii_case("false") => Ok(false),
        _ => Err(Error::PluginMetadata(format!(
            "Invalid boolean value for {k}"
        ))),
    }
}

impl Metadata for ini::Properties {
    fn get_metadata(&self, k: &'static str, required: bool) -> Result<&str, Error> {
        self.get(k)
            .or(if required { None } else { Some("") })
            .ok_or_else(|| Error::PluginMetadata(format!("Missing required '{k}' property")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::rootdir;
    use std::fs::File;

    #[test]
    fn test_parse_metadata() {
        let mut input = File::open(rootdir().join("fixtures/metadata.txt")).unwrap();

        let plugin = Plugin::from_metadata(&mut input).unwrap();

        assert_eq!(plugin.name, "Edigeo-processing");
        assert!(plugin.experimental);
    }
}
