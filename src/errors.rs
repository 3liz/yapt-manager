//!
//! Crate errors
//!
use std::borrow::Cow;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Source requires manual update")]
    SourceManualUpdateRequired,
    #[error("Status code: {0}")]
    SourceRequestFailure(reqwest::StatusCode),
    #[error("Xml parse error: {0}")]
    XmlParse(Cow<'static, str>),
    #[error("Source exists: {0}")]
    SourceExists(String),
    #[error("No such source: {0}")]
    SourceNotExists(String),
    #[error("Invalid plugin metadata: {0}")]
    PluginMetadata(String),
}
