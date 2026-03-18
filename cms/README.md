# PLATEAU CMS

This directory contains the PLATEAU CMS (Content Management System), which was originally based on Re:Earth CMS and has been integrated into the PLATEAU-VIEW-3.0 monorepo.

## Overview

PLATEAU CMS provides data management and distribution capabilities for PLATEAU VIEW. It consists of:

- **Web Application** (`/web`): React-based admin interface for managing content
- **Server** (`/server`): Go-based GraphQL API server
- **Worker** (`/worker`): Background processing workers for async tasks

## Directory Structure

```
cms/
├── web/              # Web admin interface
├── server/           # GraphQL API server
├── worker/           # Background workers (decompressor, copier, etc.)
├── go.work           # Go workspace configuration
├── docker-compose.yml # Local development setup
└── README.md         # This file
```

## Development

### Server

```bash
cd server
go run main.go
```

See [server/README.md](server/README.md) for more details.

### Web

```bash
cd web
yarn install
yarn start
```

See [web/README.md](web/README.md) for more details.

### Worker

```bash
cd worker
go run main.go
```

See [worker/README.md](worker/README.md) for more details.

## Package Names

The packages have been renamed to fit within the PLATEAU-VIEW-3.0 namespace:

- Go modules: `github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/{server,worker}`
- NPM package: `@plateau-view-3.0/cms`

## CI/CD

Workflows for building and testing CMS components are located in `.github/workflows/`:

- `build-cms-server.yml` - Build and push server Docker images
- `build-cms-web.yml` - Build and push web Docker images
- `build-cms-worker.yml` - Build and push worker Docker images
- `ci-cms-server.yml` - Server linting and tests
- `ci-cms-web.yml` - Web linting and tests
- `ci-cms-worker.yml` - Worker linting and tests

## Integration with PLATEAU Server

The PLATEAU server (located at `/server` in the root) integrates with this CMS via the reearth-cms-api. Future work will migrate to using this local CMS codebase directly instead of the external dependency.

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

## Original Source

This code was originally developed as part of Re:Earth CMS:
https://github.com/reearth/reearth-cms
