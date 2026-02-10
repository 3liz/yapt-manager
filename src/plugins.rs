//!
//! Plugins metadata
//!
use std::io::Read;

mod xmlparse;

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Plugin {
    name: String,
    description: String,
    version: String,
    qgis_minimum_version: String,
    qgis_maximum_version: String,
    file_name: String,
    slug: String,
    author_name: String,
    download_url: String,
    create_date: String,
    update_date: String,
    experimental: bool,
    deprecated: bool,
    tags: String,
    server: bool,
    trusted: bool,
}

impl Plugin {
    /// Read catalog as JSON
    pub fn read_catalog<R: Read>(reader: &mut R) -> anyhow::Result<Vec<Plugin>> {
        #[derive(serde::Deserialize)]
        struct Data {
            plugins: Vec<Plugin>,
        }

        Ok(serde_json::from_reader::<&mut R, Data>(reader)?.plugins)
    }
}
