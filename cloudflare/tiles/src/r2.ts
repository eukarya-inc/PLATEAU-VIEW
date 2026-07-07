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

  // Advertise byte-range support on every response; COG readers probe for it.
  headers.set("accept-ranges", "bytes");

  // A failed conditional (If-None-Match / If-Modified-Since) yields a bodyless
  // R2Object -> 304 Not Modified.
  if (!("body" in object) || !object.body) {
    return new Response(null, { status: 304, headers });
  }

  // R2 sets `range` when it honored a Range request, but writeHttpMetadata does
  // NOT emit Content-Range -- build it ourselves and return 206 Partial Content.
  // Without this a satisfied range would wrongly read as a full 200.
  // Only a client-sent Range makes this a partial response. (R2/Miniflare may
  // populate `object.range` regardless, so gate on the actual request header.)
  let status = 200;
  if (request.headers.has("range") && object.range) {
    const resolved = resolveRange(object.range, object.size);
    if (resolved) {
      headers.set("content-range", `bytes ${resolved.start}-${resolved.end}/${object.size}`);
      headers.set("content-length", String(resolved.end - resolved.start + 1));
      status = 206;
    }
  }

  return new Response(object.body, {
    status,
    headers,
    encodeBody: isTerrain ? "manual" : "automatic",
  });
}

/**
 * Resolve an R2Range to an inclusive byte [start, end], or null if it doesn't
 * describe a valid sub-range. `suffix`/`offset`/`length` are read by value (not
 * `"key" in range`) because some runtimes attach all three keys as undefined.
 */
function resolveRange(range: R2Range, size: number): { start: number; end: number } | null {
  const suffix = (range as { suffix?: number }).suffix;
  const offset = (range as { offset?: number }).offset;
  const length = (range as { length?: number }).length;

  let start: number;
  let end: number;
  if (typeof suffix === "number") {
    start = size - Math.min(suffix, size);
    end = size - 1;
  } else {
    start = typeof offset === "number" ? offset : 0;
    end = typeof length === "number" ? start + length - 1 : size - 1;
  }

  if (!Number.isFinite(start) || !Number.isFinite(end) || start < 0 || end < start) {
    return null;
  }
  return { start, end };
}

interface TileCogLayer {
  type: "cog";
  url: string;
  order?: number;
}

interface TileSource {
  description: string;
  layers: TileCogLayer[];
}

interface TileConfig {
  version: string;
  sources: Record<string, TileSource>;
}

/**
 * Emit a PLATEAU `/tile` config.json describing this dataset's COGs as `cog`
 * sources, so the tile server can be pointed at
 * `https://<host>/<dataset>/config.json` (as one entry in its comma-separated
 * `CONFIG_URL`) and render the R2 COGs on demand. The COGs never move — the tile
 * server reads them straight from this Worker over HTTP Range.
 *
 * COG keys are `<group>/<file>.tif` (for ortho, group = acquisition year).
 * Default: one source per group, named `<dataset>-<group>` (e.g. `ortho-2024`),
 * each a footprint mosaic of that group's COGs. Pass `?source=<name>` to stack
 * every COG into a single source instead; numeric groups become the layer
 * `order` so newer groups render on top.
 *
 * `version` is a cheap FNV-1a hash of every key+etag, so the tile server's
 * config revalidation picks up any added / changed / removed COG.
 */
export async function tileConfig(
  bucket: R2Bucket,
  dataset: string,
  origin: string,
  params: URLSearchParams,
  cors: Headers,
): Promise<Response> {
  // Enumerate every COG in the bucket (paginated — no delimiter, full recurse).
  const cogs: { key: string; etag: string }[] = [];
  let cursor: string | undefined;
  do {
    const res = await bucket.list({ cursor, limit: 1000 });
    for (const o of res.objects) {
      if (o.key.endsWith(".tif") || o.key.endsWith(".tiff")) {
        cogs.push({ key: o.key, etag: o.etag });
      }
    }
    cursor = res.truncated ? (res.cursor ?? undefined) : undefined;
  } while (cursor);

  cogs.sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));

  const layerFor = (key: string): TileCogLayer => ({
    type: "cog",
    url: `${origin}/${dataset}/${key}`,
  });

  const sources: Record<string, TileSource> = {};
  const single = params.get("source");
  if (single) {
    sources[single] = {
      description: `${dataset} COG mosaic (${cogs.length} COGs)`,
      layers: cogs.map((o) => {
        const layer = layerFor(o.key);
        const group = Number(o.key.split("/")[0]);
        if (Number.isFinite(group)) layer.order = group; // newer group on top
        return layer;
      }),
    };
  } else {
    for (const o of cogs) {
      const slash = o.key.indexOf("/");
      const group = slash === -1 ? "" : o.key.slice(0, slash);
      const name = group ? `${dataset}-${group}` : dataset;
      (sources[name] ??= {
        description: group ? `${dataset} ${group} COG mosaic` : `${dataset} COG mosaic`,
        layers: [],
      }).layers.push(layerFor(o.key));
    }
  }

  // FNV-1a over key+etag pairs; Math.imul keeps the mix 32-bit.
  let h = 0x811c9dc5;
  for (const o of cogs) {
    const s = `${o.key}:${o.etag};`;
    for (let i = 0; i < s.length; i++) {
      h = Math.imul(h ^ s.charCodeAt(i), 0x01000193);
    }
  }
  const version = `${dataset}-${cogs.length}-${(h >>> 0).toString(16)}`;

  const config: TileConfig = { version, sources };
  const headers = new Headers(cors);
  headers.set("content-type", "application/json; charset=utf-8");
  headers.set("cache-control", "public, max-age=60");
  return new Response(JSON.stringify(config, null, 2), { headers });
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
