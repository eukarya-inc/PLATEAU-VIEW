/**
 * R2 object serving + directory listing.
 *
 * Buckets hold static assets (pre-gzipped quantized-mesh `.terrain` tiles and
 * Cloud Optimized GeoTIFF `.tif` COGs). We serve them verbatim with Range /
 * conditional / gzip handling, and expose the bucket contents as a live JSON
 * listing (no static catalog, so newly uploaded COGs appear automatically).
 */

export function corsHeaders(allowOrigin: string): Headers {
  return new Headers({
    "access-control-allow-origin": allowOrigin,
    "access-control-allow-methods": "GET, HEAD, OPTIONS",
    // Allow the request headers COG/quantized-mesh clients send (Range reads,
    // conditional revalidation), and expose the response headers they read back
    // (byte-range progress, cache validators, range support).
    "access-control-allow-headers": "Range, If-None-Match, If-Modified-Since",
    "access-control-expose-headers": "ETag, Content-Range, Content-Length, Accept-Ranges",
    "access-control-max-age": "86400",
  });
}

function contentTypeFor(key: string): string {
  if (key.endsWith(".terrain")) return "application/vnd.quantized-mesh";
  if (key.endsWith(".tif") || key.endsWith(".tiff")) return "image/tiff";
  if (key.endsWith(".json")) return "application/json";
  if (key.endsWith(".png")) return "image/png";
  if (key.endsWith(".webp")) return "image/webp";
  return "application/octet-stream";
}

/**
 * Serve a single object from `bucket` at `key`, honoring Range and conditional
 * (If-None-Match / If-Modified-Since) requests.
 */
export async function serveObject(
  bucket: R2Bucket,
  key: string,
  request: Request,
  cors: Headers,
): Promise<Response> {
  const object = await bucket.get(key, {
    onlyIf: request.headers,
    range: request.headers,
  });

  if (!object) {
    return new Response("Not Found", { status: 404, headers: cors });
  }

  const headers = new Headers(cors);
  object.writeHttpMetadata(headers);
  headers.set("etag", object.httpEtag);
  if (!headers.has("content-type")) {
    headers.set("content-type", contentTypeFor(key));
  }
  if (!headers.has("cache-control")) {
    headers.set("cache-control", "public, max-age=86400, immutable");
  }

  // `.terrain` tiles are stored pre-gzipped (raw bytes). Declare the encoding so
  // clients (Cesium) decode once. `encodeBody: "manual"` is required — otherwise
  // the runtime treats Content-Encoding as a hint and re-encodes, double-gzipping
  // the body. COGs are stored uncompressed, so they use automatic encoding.
  const isTerrain = key.endsWith(".terrain");
  if (isTerrain && !headers.has("content-encoding")) {
    headers.set("content-encoding", "gzip");
  }

  // `get` returns R2ObjectBody (with body) only on success; a failed conditional
  // yields a bodyless R2Object -> 304.
  const body = "body" in object ? object.body : null;
  const status = body ? (headers.has("content-range") ? 206 : 200) : 304;

  return new Response(body, {
    status,
    headers,
    encodeBody: isTerrain ? "manual" : "automatic",
  });
}

interface ListingEntry {
  key: string;
  size: number;
  uploaded: string;
  etag: string;
}

interface Listing {
  dataset: string;
  prefix: string;
  directories: string[];
  files: ListingEntry[];
  truncated: boolean;
  cursor: string | null;
}

/**
 * Directory-style listing of `bucket` under `prefix`, using `delimiter: "/"` so
 * sub-prefixes come back as `directories` and immediate objects as `files`.
 * Paginated via `?cursor=`. Bounded to one R2 `list` call per request.
 *
 * `dataset` is the URL path segment this bucket is served under (e.g. "terrain")
 * — `directories`/`files` keys are bucket-relative, so a full URL is
 * `/<dataset>/<key>`.
 */
export async function listPrefix(
  bucket: R2Bucket,
  dataset: string,
  prefix: string,
  cursor: string | null,
  cors: Headers,
): Promise<Response> {
  const result = await bucket.list({
    prefix,
    delimiter: "/",
    cursor: cursor ?? undefined,
    limit: 1000,
  });

  const nextCursor = result.truncated ? (result.cursor ?? null) : null;
  const listing: Listing = {
    dataset,
    prefix,
    directories: result.delimitedPrefixes,
    files: result.objects.map((o) => ({
      key: o.key,
      size: o.size,
      uploaded: o.uploaded.toISOString(),
      etag: o.etag,
    })),
    truncated: result.truncated,
    cursor: nextCursor,
  };

  const headers = new Headers(cors);
  headers.set("content-type", "application/json; charset=utf-8");
  headers.set("cache-control", "public, max-age=60");
  return new Response(JSON.stringify(listing, null, 2), { headers });
}
