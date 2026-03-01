# YAPT-Manager

QGIS plugin manager

A cli tool for handling QGIS plugins written in Rust.

Yapt manager is essentially designed to work with the YAPT plugin server REST api, it supports
managing plugins from qgis.org but with some limitations (see note below).

* Support for multiple sources
* List, install, upgrade and search for plugins across multiple sources
* Handle XML plugin repository and REST plugin api.

### Notes:

Yapt-manager handle plugins from plugin list (XML or JSon) returned by remote source, that
means that you won't be able to download plugins which are not listed in returned list.
This implies that from Qgis.org source, you will only be enable to download and list only latest
versions (stable and experimental) of plugins - this is the same behavior as QGIS desktop.
