use std::io::Read;

use super::Plugin;
use crate::catalog::cached::CacheBuilder;
use crate::errors::Error;
use crate::version::Version;

impl Plugin {
    pub fn from_xml<R: Read>(
        name: String,
        version: String,
        parser: &mut xml::EventReader<R>,
    ) -> Result<Self, Error> {
        use xml::common::Position;
        use xml::reader::XmlEvent;

        let version = Version::from(version);
        let slug = name.to_ascii_lowercase();

        let mut this = Self {
            name,
            version,
            slug,
            ..Default::default()
        };

        fn text<R: Read>(
            parser: &mut xml::EventReader<R>,
            required: bool,
        ) -> Result<String, Error> {
            match parser.next() {
                Ok(XmlEvent::Characters(s)) | Ok(XmlEvent::CData(s)) => {
                    parser
                        .skip()
                        .map_err(|err| Error::XmlParse(err.to_string().into()))?;
                    Ok(s)
                }
                Ok(XmlEvent::EndElement { .. }) => {
                    if required {
                        Err(Error::XmlParse("Missing required property value".into()))
                    } else {
                        Ok(String::new())
                    }
                }
                Ok(_) => Err(Error::XmlParse("Expecting text node !".into())),
                Err(err) => Err(Error::XmlParse(format!("{err:?}").into())),
            }
        }

        fn parse_property<R: Read>(
            this: &mut Plugin,
            name: &str,
            parser: &mut xml::EventReader<R>,
        ) -> Result<(), Error> {
            match name {
                "description" => this.description = text(parser, false)?,
                "qgis_minimum_version" => this.qgis_minimum_version = text(parser, true)?,
                "qgis_maximum_version" => this.qgis_maximum_version = text(parser, false)?,
                "file_name" => this.file_name = text(parser, false)?,
                "slug" => this.slug = text(parser, false)?,
                "author_name" => this.author_name = text(parser, true)?,
                "download_url" => this.download_url = text(parser, true)?,
                "create_date" => this.create_date = text(parser, true)?,
                "update_date" => this.update_date = text(parser, true)?,
                "experimental" => {
                    this.experimental = parse_bool(&text(parser, false)?, name)?;
                }
                "deprecated" => {
                    this.deprecated = parse_bool(&text(parser, false)?, name)?;
                }
                "tags" => this.tags = text(parser, false)?,
                "server" => this.server = parse_bool(&text(parser, false)?, name)?,
                "trusted" => this.trusted = parse_bool(&text(parser, false)?, name)?,
                _ => {
                    parser
                        .skip()
                        .map_err(|err| Error::XmlParse(err.to_string().into()))?;
                }
            }
            Ok(())
        }

        loop {
            match parser.next() {
                Ok(XmlEvent::StartElement { name, .. }) => {
                    parse_property(&mut this, &name.local_name, parser).inspect_err(|_| {
                        let pos = parser.position();
                        log::error!(
                            "Xml Error while parsing element <{}> at line: {}, col: {}",
                            name.local_name,
                            pos.row(),
                            pos.column(),
                        );
                    })?;
                }
                Ok(XmlEvent::EndElement { name }) => {
                    if name.local_name == "pyqgis_plugin" {
                        break Ok(this);
                    } else {
                        let pos = parser.position();
                        break Err(Error::XmlParse(
                            format!(
                                "Unexpected end element {} at: {}  col: {}",
                                name.local_name,
                                pos.row(),
                                pos.column(),
                            )
                            .into(),
                        ));
                    }
                }
                Ok(XmlEvent::EndDocument) => {
                    break Err(Error::XmlParse("Unexpected end of document".into()));
                }
                Ok(_) => {}
                Err(err) => {
                    break Err(Error::XmlParse(format!("{err:?}").into()));
                }
            }
        }
    }

    pub fn read_catalog_xml<R: Read>(reader: &mut R) -> Result<CacheBuilder, Error> {
        use xml::common::Position;
        use xml::reader::XmlEvent;

        let mut builder = CacheBuilder::new();

        let mut parser = xml::EventReader::new(reader);
        let mut plugins = false;
        loop {
            match parser.next() {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    match name.local_name.as_str() {
                        "plugins" => plugins = true,
                        "pyqgis_plugin" => {
                            let mut plugin_name = String::new();
                            let mut plugin_version = String::new();
                            for attr in attributes.into_iter() {
                                match attr.name.local_name.as_str() {
                                    "name" => plugin_name = attr.value,
                                    "version" => plugin_version = attr.value,
                                    _ => {}
                                }
                            }
                            if plugin_name.is_empty() {
                                break Err(Error::XmlParse(
                                    "Missing plugin 'name' attribute".into(),
                                ));
                            }
                            if plugin_version.is_empty() {
                                break Err(Error::XmlParse(
                                    "Missing plugin 'version' attribute".into(),
                                ));
                            }

                            // Register plugin
                            match Plugin::from_xml(plugin_name, plugin_version, &mut parser) {
                                Ok(p) => builder.insert(p),
                                Err(err) => {
                                    let pos = parser.position();
                                    log::error!(
                                        "Invalid plugin xml at line: {}, col: {}\n{}",
                                        err,
                                        pos.row(),
                                        pos.column(),
                                    );
                                }
                            }
                        }
                        _ => {
                            parser
                                .skip()
                                .map_err(|err| Error::XmlParse(err.to_string().into()))?;
                        }
                    }
                }
                Ok(XmlEvent::EndElement { name }) => {
                    if name.local_name == "plugins" {
                        break if plugins {
                            Ok(())
                        } else {
                            let pos = parser.position();
                            Err(Error::XmlParse(
                                format!(
                                    "Unexpected end of <plugins> element at line: {} col: {}",
                                    pos.row(),
                                    pos.column(),
                                )
                                .into(),
                            ))
                        };
                    }
                }
                Ok(XmlEvent::EndDocument) => {
                    break Err(Error::XmlParse("Unexpected end of document".into()));
                }
                Err(err) => {
                    break Err(Error::XmlParse(format!("{err:?}").into()));
                }
                _ => {}
            }
        }?;

        Ok(builder)
    }
}

fn parse_bool(v: &str, k: &str) -> Result<bool, Error> {
    match v {
        "" => Ok(false),
        _ if v.eq_ignore_ascii_case("true") => Ok(true),
        _ if v.eq_ignore_ascii_case("false") => Ok(false),
        _ if v.eq_ignore_ascii_case("None") => Ok(false),
        _ => Err(Error::XmlParse(
            format!("Invalid boolean value for {k}: {v}").into(),
        )),
    }
}
