//!
//! Plugins metadata
//!

mod metadata;
mod xmlparse;

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub version: String,
    pub qgis_minimum_version: String,
    pub qgis_maximum_version: String,
    pub file_name: String,
    pub slug: String,
    pub author_name: String,
    pub download_url: String,
    pub create_date: String,
    pub update_date: String,
    pub experimental: bool,
    pub deprecated: bool,
    pub tags: String,
    pub server: bool,
    pub trusted: bool,
}

impl Plugin {}
