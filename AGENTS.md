# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

PLATEAU VIEW is a comprehensive system for managing and visualizing urban 3D models and geographic data. The project consists of:

- **PLATEAU CMS**: Data management and distribution
- **PLATEAU Editor**: Web application for creating and publishing viewers
- **PLATEAU Flow**: Workflow system for data transformation
- **PLATEAU VIEW**: Web application for data visualization

## Repository Structure

This is a monorepo containing multiple interconnected services:

```
/server         # Go-based server providing PLATEAU API
/editor         # PLATEAU Editor application
  /core         # @reearth/core - Map engine abstraction library
  /web          # Main editor web application
  /server       # Editor backend server
/extension      # PLATEAU VIEW extension widgets
/geo            # NestJS geo service for address search
/tile           # Rust-based high-performance tile server (XYZ proxy + COG rendering)
/worker         # Background worker applications for PLATEAU API server
/terraform      # Infrastructure as Code (AWS/GCP)
/tools          # CLI tools for data migration
/.github        # GitHub Actions workflows and scripts
```

## Common Development Commands

### Server (Go) - /server
```bash
go build                # Build the server
go test ./...          # Run tests
make gql               # Generate GraphQL code for plateauapi
go run main.go         # Run the server
```

### Editor Web - /editor/web
```bash
yarn                   # Install dependencies
yarn start             # Development server
yarn build             # Production build
yarn test              # Run tests (Vitest)
yarn lint              # Lint code
yarn fix               # Fix lint issues
yarn type              # TypeScript check
yarn check             # Run type, lint, and coverage
yarn gql               # Generate GraphQL types
yarn storybook         # Component development
```

### Extension - /extension
```bash
yarn                   # Install dependencies
yarn dev               # Development mode (port 5001)
yarn build             # Production build
yarn test              # Run tests
yarn lint              # Lint code
yarn gql               # Generate GraphQL types (geo + plateau)
```

### Geo Service - /geo
```bash
yarn                   # Install dependencies
yarn start:dev         # Development server
yarn build             # Production build
yarn test              # Run tests (Jest)
yarn lint              # Lint code
```

### Tile Server (Rust) - /tile
```bash
cargo build                              # Build
cargo test                               # Run tests
cargo fmt                                # Format code
cargo clippy --all-targets -- -D warnings # Lint (CI treats warnings as errors)
CONFIG_URL=file://config.json cargo run  # Run dev server
```

## High-Level Architecture

### Map Abstraction Layer (editor/core)
The core provides an engine-agnostic map abstraction:
- **Visualizer** → **Map** → **Engine** (Cesium) → **Features**
- Supports runtime engine switching through dependency injection
- Extensive use of React refs for imperative API access

### Widget/Plugin System (extension)
- Widget-based architecture where each feature is self-contained
- Context-based state sharing between widgets
- Property-based configuration system
- Widgets include: Toolbar, Search, Inspector, Timeline, etc.

### Server Architecture
- Service-oriented design with modular features
- Echo framework for HTTP routing
- GraphQL API using gqlgen
- Integration with external services (CMS, FME, tile servers)

### State Management
- Frontend uses Jotai for atomic state management
- Custom patterns for storage persistence and URL-based sharing
- Server maintains stateless request handling

### API Structure
- **PLATEAU API**: GraphQL API for data catalog (`/server/datacatalog/plateauapi/`)
- **Geo API**: GraphQL service for geographic queries (`/geo/`)
- **REST APIs**: Standard endpoints for CRUD operations
- Generated type-safe clients on both frontend and backend

### Data Flow
1. Frontend components use typed hooks to fetch data
2. Requests go through Apollo Client (GraphQL) or fetch (REST)
3. Backend services handle authentication and data processing
4. External integrations (CMS, FME) for data management
5. Results cached and returned to frontend

## Key Development Patterns

### Error Handling
- **Go**: Use wrapped errors with context: `fmt.Errorf("context: %w", err)`
- **TypeScript**: Promise-based error handling with proper catch blocks

### GraphQL Development
- Schema changes require regenerating code:
  - Server: `make gql` or `go generate ./...`
  - Client: `yarn gql` in respective directories
- Use fragments for reusable query parts
- Follow Relay-style Node interface patterns

### Component Development
- Use custom hooks for complex logic
- Implement proper TypeScript types
- Follow existing patterns for state management
- Use Storybook for component development

### Testing
- Write tests alongside code changes
- Go: Use testify for assertions
- TypeScript: Use Vitest depending on project
- Run tests before committing: `go test ./...` or `yarn test`

## Important Notes
- Node.js version requirements vary by project (check package.json)
- All JavaScript/TypeScript projects use Yarn as package manager
- Mock authentication available for local development
- Environment variables configured in `.env` files (not committed)
- **When investigating or modifying `/server`**: Always read `/server/CLAUDE.md` or `/server/AGENTS.md` first before using the Explore tool or making changes. These files contain critical server-specific architecture, patterns, and development guidelines.
- **When investigating or modifying `/tile`**: Always read `/tile/CLAUDE.md` first. This contains Rust-specific development guidelines, environment variables, and layer type documentation.

## Deployment Scripts

Deployment scripts are located in `/.github/scripts/`:

- **watch-and-deploy-prod.sh**: Watches CI and dev deploy workflows, then triggers production deployment. Supports server, worker, and tile targets.
  ```bash
  .github/scripts/watch-and-deploy-prod.sh server  # Deploy PLATEAU Server
  .github/scripts/watch-and-deploy-prod.sh worker  # Deploy PLATEAU Worker
  .github/scripts/watch-and-deploy-prod.sh tile    # Deploy PLATEAU Tile
  ```
