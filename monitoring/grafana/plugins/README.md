# ScyllaDB Plugin for Grafana

## Installation

To install the ScyllaDB plugin for Grafana:

1. Download the plugin from the Grafana plugins directory or build it from source
2. Extract it to the plugins directory: `/var/lib/grafana/plugins/`
3. The plugin is already configured in docker-compose.yml with the environment variable:
   `GF_PLUGINS_ALLOW_LOADING_UNSIGNED_PLUGINS=scylladb-scylla-datasource`

## Manual Installation

If you need to install manually:

```bash
# Option 1: Download from releases
wget https://github.com/scylladb/grafana-scylla-datasource/releases/latest/download/scylladb-scylla-datasource.zip
unzip scylladb-scylla-datasource.zip -d monitoring/grafana/plugins/

# Option 2: Git clone and build
git clone https://github.com/scylladb/grafana-scylla-datasource.git
cd grafana-scylla-datasource
npm install
npm run build
cp -r dist/ ../monitoring/grafana/plugins/scylladb-scylla-datasource/
```

## Configuration

The datasource is pre-configured in `datasources.yml` with:
- Host: scylladb:9042
- Datacenter: datacenter1
- Keyspace: posts

## Dashboards

ScyllaDB dashboards are located in:
- `monitoring/grafana/dashboards/ver_2025.2/`

These include:
- scylla-overview.2025.2.json
- scylla-detailed.2025.2.json
- scylla-cql.2025.2.json
- scylla-advanced.2025.2.json
- scylla-ks.2025.2.json
- scylla-os.2025.2.json
- alternator.2025.2.json
