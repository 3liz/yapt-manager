//!
//! Plugins metadata
//!

mod metadata;
mod xmlparse;

use crate::version::{SemVer, Version};

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Plugin {
    pub name: String,
    pub description: String,
    #[serde(
        serialize_with = "serialize_version",
        deserialize_with = "deserialize_version"
    )]
    pub version: Version<'static>,
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

impl Plugin {
    pub fn matches_qgis_version(&self, version: &SemVer) -> bool {
        SemVer::new(&self.qgis_minimum_version) <= *version
            && if !self.qgis_maximum_version.is_empty() {
                SemVer::new(&self.qgis_maximum_version) >= *version
            } else {
                true
            }
    }
}

// Serializer/Deserializer Version
fn serialize_version<S>(value: &Version<'static>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(value.as_str())
}

fn deserialize_version<'de, D>(d: D) -> Result<Version<'static>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    Ok(Version::from(String::deserialize(d)?))
}
