# PLATEAU Tile Worker

Cloudflare Worker that serves tiles from R2 cache, falling back to PLATEAU Tile Server on cache miss.

## Architecture

```
Client Request
     ↓
Cloudflare Worker
     ↓
┌───────────────────┐
│ Cached in R2?     │
│   ├─ Yes → Return from R2 (HIT)
│   └─ No  → Forward to Tile Server (MISS)
└───────────────────┘
     ↓
PLATEAU Tile Server (generates tile + writes to R2)
```

## Setup

```bash
cd tile/worker
npm install
```

## Development

```bash
npm run dev
```

## Deployment

```bash
# Staging
npm run deploy:staging

# Production
npm run deploy:production
```

## Environment Variables

| Name | Default | Description |
|------|---------|-------------|
| `ORIGIN_URL` | (required) | PLATEAU Tile Server URL |
| `CORS_ORIGINS` | `*` | CORS allowed origins (comma-separated, or `*` for all) |
| `CACHE_CONTROL` | `public, max-age=31536000, immutable` | Cache-Control header for responses |

## R2 Bindings

| Binding | Description |
|---------|-------------|
| `CACHE` | R2 bucket for tile cache |

## Response Headers

| Header | Description |
|--------|-------------|
| `X-Cache` | `HIT` (from R2) or `MISS` (from origin) |
| `ETag` | Full ETag from R2 metadata (for HTTP caching) |
| `X-R2-Version` | R2 object version ID (for debugging) |
| `X-Etag-Hash` | ETag hash from R2 metadata (for debugging) |

## Conditional Requests

The Worker supports HTTP conditional requests:

- **If-None-Match**: If the request includes this header and it matches the stored ETag, the Worker returns `304 Not Modified` without the response body, saving bandwidth.
