# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This is a Go-based sidecar server that provides APIs and integration services for PLATEAU VIEW. It acts as a bridge between frontend applications and various backend services including Re:Earth CMS, FME (Feature Manipulation Engine), and tile servers.

## Architecture

### Service Registration Pattern
The server uses a modular service architecture. Services are registered in `service.go`:
```go
type Service struct {
    Name           string
    Echo           func(g *echo.Group) error  
    Webhook        cmswebhook.Handler
    DisableNoCache bool
}
```

### Available Services
- **proxy**: CORS proxy for external resources with round-robin support
- **openapi**: API documentation endpoint
- **cmsintegration**: Re:Earth CMS integration with v2/v3 compatibility
- **sdkapi**: SDK endpoints for external integrations
- **opinion**: User feedback collection service
- **sidebar**: UI sidebar data management and sharing
- **datacatalog**: Core data catalog API with GraphQL
- **govpolygon**: Government polygon data service
- **tiles**: Map tile serving with Chiitiler integration
- **embed**: Static file delivery
- **citygml**: CityGML data processing and conversion

## Development Commands

```bash
# Run the server
go run .                    # Run the server in development
go test ./...              # Run all tests
go test -v ./path/...      # Run tests with verbose output

# Generate code
make gql                   # Generate GraphQL resolvers for plateauapi
go generate ./...          # Run all code generation

# Code formatting and linting
go fmt ./...               # Format all Go code
golangci-lint run ./...    # Run linting checks

# Testing specific packages
go test ./datacatalog/...  # Test datacatalog
go test ./cmsintegration/... # Test CMS integration
```

## Server Startup and Shutdown

### Starting the Server

```bash
# Method 1: Run in foreground (recommended for development)
go run .

# Method 2: Run in background with logs
go run . > server.log 2>&1 &

# Method 3: Build binary (only needed for production)
go build
./server
```

### Stopping the Server

```bash
# Method 1: If running in foreground
# Press Ctrl+C

# Method 2: If running in background
# Find the process
lsof -i :8080
# or
ps aux | grep server

# Kill the process
kill <PID>

# Method 3: Kill all processes on port 8080
lsof -ti :8080 | xargs kill
```

### Important Notes

- The server runs on port 8080 by default (configurable via PORT env var)
- Check if old processes are running before starting: `lsof -i :8080`
- If you get "address already in use" error, kill the existing process first
- Server logs startup information including configuration and port

## Configuration

Environment variables (can be set in `.env` file):
```bash
# Server
PORT=8080                           # Server port
HOST=localhost                      # Server host

# CMS Integration
CMS_BASEURL=                       # Re:Earth CMS URL
CMS_TOKEN=                         # CMS authentication token
CMS_WEBHOOK_SECRET=                # Webhook HMAC secret
PLATEAUVIEW_CMS_BASEURL=           # PLATEAU VIEW CMS URL
PLATEAUVIEW_CMS_SYSTEMPROJECT=     # System project ID

# External Services
FME_BASEURL=                       # FME server URL
FME_TOKEN=                         # FME authentication token
FME_RESULTURL=                     # FME result callback URL
FME_MOCK=true                      # Use FME mock for development

# GCP Integration
GCP_PROJECT=                       # GCP project ID
GCP_REGION=                        # GCP region
GOOGLE_CLOUD_PROJECT=              # Alternative GCP project ID

# Other Services
TILES_HOST=                        # Tile server host
CKAN_BASEURL=                      # CKAN integration URL
CKAN_TOKEN=                        # CKAN authentication token
```

## Key Patterns

### Error Handling
```go
// Always wrap errors with context
if err != nil {
    return fmt.Errorf("failed to process: %w", err)
}

// Use rerror for domain errors
return rerror.ErrNotFound

// Use echo HTTP errors for API responses
return echo.NewHTTPError(http.StatusBadRequest, "invalid request")
```

### Context Usage
```go
// Always propagate context for cancellation
ctx := c.Request().Context()
result, err := service.Process(ctx, data)

// Use context for timeouts
ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
defer cancel()
```

### Middleware Stack
1. **Recovery**: Panic recovery
2. **RequestID**: Request tracing
3. **AccessLog**: Request logging
4. **CORS**: Cross-origin support
5. **Cache/NoCache**: Cache control
6. **LastModified**: HTTP caching

### Caching Strategy
The server implements multi-level caching:
```go
// Memory cache for hot data
memCache := NewMemoryCache(1000) // 1000 items

// Disk cache for larger datasets
diskCache := NewDiskCache("./cache", 100*1024*1024) // 100MB

// Cache with TTL
cache.Set(key, value, 5*time.Minute)
```

## CMS Integration

### Webhook Processing
1. Receive webhook from CMS
2. Verify HMAC signature
3. Process event (asset, item changes)
4. Trigger downstream operations
5. Update status in CMS

### Task Runners
- **GCP Cloud Build**: For container-based tasks
- **Cloud Run Jobs**: For serverless jobs
- **Cloud Batch**: For batch processing

## GraphQL API

### Schema Location
- Schema: `/datacatalog/plateauapi/schema.graphql`
- Resolvers: `/datacatalog/plateauapi/resolvers/`
- Generated: `/datacatalog/plateauapi/gql/`

### Key Types
```graphql
type Area {
  code: AreaCode!
  name: String!
  datasets: [Dataset!]!
}

type Dataset {
  id: ID!
  name: String!
  year: Int!
  items: [DatasetItem!]!
}
```

### Adding New Resolvers
1. Update `schema.graphql`
2. Run `make gql`
3. Implement resolver in `/resolvers/`
4. Add tests

## Testing

### Test Structure
```go
func TestService(t *testing.T) {
    // Setup
    httpmock.Activate()
    defer httpmock.DeactivateAndReset()
    
    // Mock external calls
    httpmock.RegisterResponder("GET", "https://api.example.com",
        httpmock.NewStringResponder(200, `{"status":"ok"}`))
    
    // Test
    service := NewService(config)
    result, err := service.Method()
    
    // Assert
    assert.NoError(t, err)
    assert.Equal(t, expected, result)
}
```

### Mock Services
- **FME Mock**: Set `FME_MOCK=true`
- **HTTP Mock**: Use `jarcoal/httpmock`
- **CMS Mock**: Use test client

## Common Tasks

### Adding a New Service
1. Create package in appropriate directory
2. Implement service interface:
```go
func ServiceName() *Service {
    return &Service{
        Name: "servicename",
        Echo: func(g *echo.Group) error {
            g.GET("/path", handler)
            return nil
        },
    }
}
```
3. Register in `service.go`
4. Add tests

### Implementing a New Endpoint
```go
func handler(c echo.Context) error {
    ctx := c.Request().Context()
    
    // Parse request
    var req Request
    if err := c.Bind(&req); err != nil {
        return echo.NewHTTPError(http.StatusBadRequest, err.Error())
    }
    
    // Process
    result, err := process(ctx, req)
    if err != nil {
        return err
    }
    
    // Response
    return c.JSON(http.StatusOK, result)
}
```

### Performance Optimization
- Use goroutines for parallel processing
- Implement request deduplication with singleflight
- Use sync.Map for concurrent cache access
- Profile with pprof: `go tool pprof http://localhost:8080/debug/pprof/profile`

## Important Libraries
- **Echo v4**: Web framework
- **gqlgen**: GraphQL code generation
- **samber/lo**: Functional utilities
- **reearth/reearthx**: Common utilities
- **go-playground/validator**: Request validation
- **joho/godotenv**: Environment file loading

## Code Quality

### Before Committing
Always run these commands before committing code:
```bash
# Format code
go fmt ./...

# Run linting
golangci-lint run ./...

# Run tests
go test ./...
```

### Linting
The project uses `golangci-lint` with default settings. Common issues to watch for:
- `errcheck`: Always handle errors, even for deferred Close() calls
- `staticcheck`: Various Go best practices
- Proper error messages (lowercase, no punctuation at end)

### Code Formatting
- Always use `go fmt` - it's the standard Go formatter
- The project follows standard Go conventions
- No manual formatting needed if you run `go fmt`

## Debugging Tips
- Use `pp.Println()` for complex struct debugging
- Check request IDs in logs for tracing
- Enable Echo debug mode: `e.Debug = true`
- Use `httputil.DumpRequest` for HTTP debugging

## Documentation

### Detailed Data Flow Documentation
For comprehensive documentation about the PLATEAU VIEW data flow, architecture, and troubleshooting, see:
- **[docs/plateau-view-data-flow.md](docs/plateau-view-data-flow.md)** - Complete guide covering:
  - System architecture and data flow
  - CMS integration and data conversion
  - GraphQL API implementation
  - Caching mechanisms
  - Asset management and metadata extraction
  - Comprehensive troubleshooting guide with real examples (Section 10)