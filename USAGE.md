# YAPT-Manager Usage Guide

## Overview

YAPT-Manager is a CLI tool for managing QGIS plugins from remote sources. It supports both XML plugin repositories (like qgis.org) and REST APIs.

### Key Concepts

- **Sources**: Remote plugin repositories (XML or REST)
- **Cache**: Local metadata cache of available plugins (stored in `.yapt/cache/`)
- **Config**: Configuration file (`config.json`) storing source definitions
- **Installation directory**: Where plugins are installed (defaults to current directory)

### Global Options

| Option | Description |
|--------|-------------|
| `-C, --config <PATH>` | Configuration directory (default: `.yapt/`) |
| `--cache-dir <PATH>` | Cache directory (default: `<config>/cache`) |
| `-d, --install-dir <PATH>` | Plugin installation directory |
| `--qgis-version <VERSION>` | Target QGIS version (auto-detected if not set) |
| `--no-sync` | Skip source synchronization |
| `--no-progress` | Hide progress bars |
| `-v, -vv` | Increase verbosity |
| `-h` | Show help |
| `-V` | Show version |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `YAPT_CONF_DIR` | Configuration directory |
| `YAPT_CACHE_DIR` | Cache directory |
| `YAPT_NO_SYNC` | Disable source sync |
| `YAPT_NO_PROGRESS` | Hide progress |
| `QGIS_VERSION` | QGIS version |
| `QGIS_PLUGINPATH` | Plugin installation path |
| `QGIS_PLUGIN_INCLUDE_PRERELEASE` | Include experimental versions |
| `PYTHON_EXECUTABLE` | Python executable for QGIS detection |

---

## Commands

### Source Management

Manage remote plugin sources.

#### Add a source

```bash
# Add qgis.org official repository
yapt-mngr source add qgis.org 'https://plugins.qgis.org/plugins/plugins.xml?qgis={VERSION}'

# Add a REST API source
yapt-mngr source add myserver 'https://my.server/api/plugins?qgis={VERSION}' --rest
```

#### List sources

```bash
yapt-mngr source list
```

#### Update/refresh source cache

```bash
# Refresh all sources
yapt-mngr source update

# Refresh a specific source
yapt-mngr source update qgis.org

# Force refresh (ignore cache)
yapt-mngr source update --refresh
```

#### Check for updates

```bash
# Check all sources
yapt-mngr source check

# Check specific source
yapt-mngr source check qgis.org
```

#### Remove a source

```bash
yapt-mngr source remove qgis.org
```

#### Rename a source

```bash
yapt-mngr source rename old-name new-name
```

---

### Search Plugins

Search for plugins by name or tags using fuzzy matching.

```bash
# Basic search
yapt-mngr search lizmap

# Search by exact name
yapt-mngr search lizmap --by-name

# Search server plugins only
yapt-mngr search server --server

# Include experimental versions
yapt-mngr search lizmap --pre

# Include deprecated plugins
yapt-mngr search old-plugin --deprecated

# Search in a specific source
yapt-mngr search lizmap --source qgis.org

# Show all versions
yapt-mngr search lizmap --all
```

---

### Find Plugins

Find plugins matching version requirements.

```bash
# Find latest version
yapt-mngr find lizmap

# Find specific version
yapt-mngr find "lizmap=3.5"

# Find version range
yapt-mngr find "lizmap>=2.0, <3.0"

# Exact match (for non-semver versions)
yapt-mngr find "plugin==release"

# Include experimental
yapt-mngr find lizmap --pre

# Search in specific source
yapt-mngr find lizmap --source 3liz
```

**Version specifiers:**
- `name=1.2.3` or `name>=1.2.3` — version constraints
- `name==exact-version` — exact string match (for non-semver versions)

---

### Install Plugins

```bash
# Install a plugin
yapt-mngr install lizmap

# Install multiple plugins
yapt-mngr install lizmap quickmap

# Install specific version
yapt-mngr install "lizmap=3.5"

# Upgrade to latest if already installed
yapt-mngr install lizmap --upgrade

# Dry run (show what would be installed)
yapt-mngr install lizmap --dry-run

# Include experimental versions
yapt-mngr install lizmap --pre

# Install from specific source
yapt-mngr install lizmap --source qgis.org
```

---

### List Installed Plugins

```bash
# List all installed plugins
yapt-mngr list

# List only outdated plugins
yapt-mngr list --outdated

# Include experimental in latest version check
yapt-mngr list --pre

# Check against specific source
yapt-mngr list --source qgis.org
```

---

### Upgrade Plugins

```bash
# Upgrade all installed plugins
yapt-mngr upgrade

# Show what would be upgraded (dry run)
yapt-mngr upgrade --dry-run

# Include experimental versions
yapt-mngr upgrade --pre

# Reinstall currently installed versions
yapt-mngr upgrade --reinstall
```

---

### Remove Plugins

```bash
# Remove a plugin
yapt-mngr remove lizmap

# Remove multiple plugins
yapt-mngr remove lizmap quickmap
```

**Note**: Symlinked plugin directories are not managed by YAPT-Manager and will be skipped.

---

## Examples

### Complete Workflow

```bash
# 1. Add official QGIS repository
yapt-mngr source add qgis.org 'https://plugins.qgis.org/plugins/plugins.xml?qgis={VERSION}'

# 2. Search for a plugin
yapt-mngr search lizmap

# 3. Install the plugin
yapt-mngr install lizmap --qgis-version 3.40

# 4. List installed plugins
yapt-mngr list

# 5. Check for updates
yapt-mngr upgrade --dry-run

# 6. Apply updates
yapt-mngr upgrade
```

### Using with QGIS Version

```bash
# Let YAPT auto-detect QGIS version
yapt-mngr install lizmap

# Or specify explicitly
yapt-mngr install lizmap --qgis-version 3.40
yapt-mngr search lizmap --qgis-version 3.40
```

### Managing Multiple Sources

```bash
# Add multiple sources
yapt-mngr source add qgis.org 'https://plugins.qgis.org/plugins/plugins.xml?qgis={VERSION}'
yapt-mngr source add MyRepo 'https://qgis-plugins.mydomain.org/' --rest

# Search in specific source
yapt-mngr search lizmap --source myrepo

# Install from specific source
yapt-mngr install lizmap --source myrepo
```
