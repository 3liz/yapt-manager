# YAPT-Manager

QGIS plugin manager

A cli tool for handling QGIS plugins written in Rust.

Yapt manager is essentially designed to work with the YAPT plugin server REST api, it supports
managing plugins from qgis.org but with some limitations (see note below).

* Support for multiple sources
* List, install, upgrade and search for plugins across multiple sources
* Handle XML plugin repository and REST plugin api.

The manager will not handle plugin's directory that are symlinked, you may take advantage of 
this to exclude some installed plugins from beeing managed.

### Notes:

Yapt-manager handle plugins from plugin list (XML or JSon) returned by remote source, that
means that you won't be able to download plugins which are not listed in returned list.
This implies that from Qgis.org source, you will only be enable to download and list only latest
versions (stable and experimental) of plugins - this is the same behavior as QGIS desktop.

## Usage

Use `yapt-mngr --help ` for short help or `yapt-mngr <command>` for detailled help. 


### 1. Add sources

```
yapt-mngr source add qgis.org 'https://plugins.qgis.org/plugins/plugins.xml?qgis={VERSION}'"
```


### 2. Search plugins

```
yapt-mngr search "lizmap,server" --qgis-version=3.40
```

### 3. Install plugins

```
yapt-mngr install "Lizmap server" --qgis-version=3.40
```
