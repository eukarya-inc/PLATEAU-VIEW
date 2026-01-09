/**
 * PLATEAU Tile Worker
 *
 * Cloudflare Worker that serves tiles from R2 cache,
 * falling back to PLATEAU Tile Server on cache miss.
 *
 * Flow:
 * 1. Request comes in for /tiles/{source}/{z}/{x}/{y}.{format}
 * 2. Check R2 for cached tile
 * 3. If hit, return from R2
 * 4. If miss, forward to PLATEAU Tile Server
 * 5. Tile Server generates tile and writes to R2 (write-only mode)
 */


/** Tile path pattern: /tiles/{source}/{z}/{x}/{y}.{format} */
const TILE_PATH_REGEX = /^\/tiles\/([^/]+)\/(\d+)\/(\d+)\/(\d+)\.(png|webp|avif)$/;

/** Default Cache-Control header */
const DEFAULT_CACHE_CONTROL = "public, max-age=31536000, immutable";

/** Check if origin is allowed for CORS */
function isOriginAllowed(origin: string | null, allowedOrigins: string | undefined): string | null {
  if (!origin) return null;
  if (!allowedOrigins || allowedOrigins === "*") return "*";

  const allowed = allowedOrigins.split(",").map((o) => o.trim());
  if (allowed.includes(origin)) {
    return origin;
  }
  return null;
}

/** Add CORS headers to response */
function addCorsHeaders(headers: Headers, allowedOrigin: string | null): void {
  if (allowedOrigin) {
    headers.set("Access-Control-Allow-Origin", allowedOrigin);
    headers.set("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS");
    headers.set("Access-Control-Max-Age", "86400");
  }
}

/** Convert tile URL path to R2 cache key */
function pathToCacheKey(path: string): string | null {
  const match = path.match(TILE_PATH_REGEX);
  if (!match) return null;

  const [, source, z, x, y, format] = match;
  // Cache key format: {source}/{format}/{z}/{x}/{y}.{format}
  return `${source}/${format}/${z}/${x}/${y}.${format}`;
}

/** Get content type for tile format */
function getContentType(format: string): string {
  switch (format) {
    case "png":
      return "image/png";
    case "webp":
      return "image/webp";
    case "avif":
      return "image/avif";
    default:
      return "application/octet-stream";
  }
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);
    const cacheControl = env.CACHE_CONTROL || DEFAULT_CACHE_CONTROL;
    const allowedOrigin = isOriginAllowed(request.headers.get("Origin"), env.CORS_ORIGINS);

    // Handle CORS preflight
    if (request.method === "OPTIONS") {
      const headers = new Headers();
      addCorsHeaders(headers, allowedOrigin);
      return new Response(null, { status: 204, headers });
    }

    // Only handle tile requests
    const cacheKey = pathToCacheKey(url.pathname);

    if (cacheKey) {
      // Try to get from R2 cache
      const cached = await env.CACHE.get(cacheKey);

      if (cached) {
        // Cache hit - return from R2
        console.log(`R2 HIT: ${cacheKey}`);
        const format = cacheKey.split(".").pop() || "png";

        // Get stored ETag from metadata
        const storedEtag = cached.customMetadata?.etag;

        // Check If-None-Match header for 304 response
        const ifNoneMatch = request.headers.get("If-None-Match");
        if (storedEtag && ifNoneMatch) {
          // Handle multiple ETags (comma-separated) and "*"
          const matches =
            ifNoneMatch === "*" ||
            ifNoneMatch.split(",").some((v) => v.trim() === storedEtag);
          if (matches) {
            console.log(`R2 HIT (304): ${cacheKey}`);
            const headers = new Headers({
              ETag: storedEtag,
              "Cache-Control": cacheControl,
              "X-Cache": "HIT",
            });
            addCorsHeaders(headers, allowedOrigin);
            return new Response(null, { status: 304, headers });
          }
        }

        const headers = new Headers({
          "Content-Type": getContentType(format),
          "Cache-Control": cacheControl,
          "X-Cache": "HIT",
        });

        // Set ETag header if available
        if (storedEtag) {
          headers.set("ETag", storedEtag);
        }

        // Copy R2 version ID (for debugging)
        if (cached.version) {
          headers.set("X-R2-Version", cached.version);
        }

        // Copy custom metadata if present (for debugging)
        if (cached.customMetadata?.etag_hash) {
          headers.set("X-Etag-Hash", cached.customMetadata.etag_hash);
        }

        addCorsHeaders(headers, allowedOrigin);
        return new Response(cached.body, { headers });
      }

      console.log(`R2 MISS: ${cacheKey}`);
    }

    // Cache miss or non-tile request - forward to origin
    const originUrl = new URL(url.pathname + url.search, env.ORIGIN_URL);
    console.log(`Forwarding to origin: ${originUrl}`);

    const originResponse = await fetch(originUrl.toString(), {
      method: request.method,
      headers: request.headers,
    });

    // Clone response to return
    const headers = new Headers(originResponse.headers);
    addCorsHeaders(headers, allowedOrigin);

    // Add cache miss header for tile requests
    if (cacheKey && originResponse.ok) {
      headers.set("X-Cache", "MISS");
    }

    return new Response(originResponse.body, {
      status: originResponse.status,
      statusText: originResponse.statusText,
      headers,
    });
  },
};
