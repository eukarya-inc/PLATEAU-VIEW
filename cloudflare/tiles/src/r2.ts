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
  /** `"dem"` marks a CompositeDemProvider stack; omitted for raster overlays. */
  type?: string;
  description: string;
  layers: TileCogLayer[];
}

interface TileConfig {
  version: string;
  sources: Record<string, TileSource>;
}

interface Cog {
  key: string;
  etag: string;
}

/** List every `.tif`/`.tiff` under `prefix` (paginated). Empty prefix = whole bucket. */
async function listCogs(bucket: R2Bucket, prefix: string): Promise<Cog[]> {
  const cogs: Cog[] = [];
  let cursor: string | undefined;
  do {
    const res = await bucket.list({ prefix: prefix || undefined, cursor, limit: 1000 });
    for (const o of res.objects) {
      if (o.key.endsWith(".tif") || o.key.endsWith(".tiff")) {
        cogs.push({ key: o.key, etag: o.etag });
      }
    }
    cursor = res.truncated ? (res.cursor ?? undefined) : undefined;
  } while (cursor);
  return cogs;
}

/**
 * Absolute COG URL against this Worker's own origin. Each `/`-delimited key
 * segment is percent-encoded (keys with spaces/unicode/# would otherwise produce
 * a broken URL); the tile server round-trips it back via `decodeURIComponent`.
 */
function cogUrl(origin: string, dataset: string, key: string): string {
  const path = key.split("/").map(encodeURIComponent).join("/");
  return `${origin}/${dataset}/${path}`;
}

/**
 * Bottom→top paint priority for a terrain DEM overlay, from its key. Coarser and
 * more general layers sit at the bottom; finer/more-specific win on top:
 * `sea/` < `base/dem10` < `base/dem5` < `base/dem1` < `patch/`.
 */
function demPriority(key: string): number {
  if (key.startsWith("sea/")) return 0;
  const m = key.match(/^base\/dem(\d+)\//);
  if (m) return 100 - Number(m[1]); // dem10 -> 90 (bottom) ... dem1 -> 99 (top of base)
  if (key.startsWith("patch/")) return 200; // finest, frontmost
  return 150;
}

/** Raster sources: one per group (`<dataset>-<group>`), or one stacked source with `?source=`. */
function rasterSources(
  cogs: Cog[],
  dataset: string,
  origin: string,
  single: string | null,
): Record<string, TileSource> {
  const sources: Record<string, TileSource> = {};
  if (single) {
    sources[single] = {
      description: `${dataset} COG mosaic (${cogs.length} COGs)`,
      layers: cogs.map((o) => {
        const layer: TileCogLayer = { type: "cog", url: cogUrl(origin, dataset, o.key) };
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
      }).layers.push({ type: "cog", url: cogUrl(origin, dataset, o.key) });
    }
  }
  return sources;
}

/**
 * A single `type: "dem"` source stacking every COG bottom→top (see `demPriority`).
 * No per-layer `nodata` — the tile server reads each COG's own NoData tag.
 */
function demSource(
  cogs: Cog[],
  dataset: string,
  origin: string,
  name: string,
): Record<string, TileSource> {
  const ordered = [...cogs].sort(
    (a, b) => demPriority(a.key) - demPriority(b.key) || (a.key < b.key ? -1 : 1),
  );
  return {
    [name]: {
      type: "dem",
      description: `${dataset} DEM overlay stack (${cogs.length} COGs, bottom->top)`,
      layers: ordered.map((o) => ({
        type: "cog",
        url: cogUrl(origin, dataset, o.key),
      })),
    },
  };
}

/**
 * Emit a PLATEAU `/tile` config.json describing this dataset's COGs, so the tile
 * server can list `https://<host>/<dataset>/config.json` in its (comma-separated)
 * `CONFIG_URL` and render the R2 COGs on demand. The COGs never move — the tile
 * server reads them straight from this Worker over HTTP Range.
 *
 * `isDem` (from the `DEM_DATASETS` config, e.g. terrain) selects the mode:
 * - **raster** (ortho) — one `cog` source per group (first key segment, e.g.
 *   acquisition year): `<dataset>-<group>` (e.g. `ortho-2024`), each a footprint
 *   mosaic. `?source=<name>` instead stacks all COGs into one source with layer
 *   `order` = numeric group (newer on top).
 * - **dem** (terrain) — a single `type: "dem"` source (name from `?name=`,
 *   default `dem` = the tile server's default DEM source) stacking every COG
 *   bottom→top via `demPriority` (`sea` < `base/dem10` < `dem5` < `dem1` <
 *   `patch`). No per-layer `nodata` — the tile server reads each COG's own tag.
 *
 * `?prefix=a/,b/` scopes enumeration to those key prefixes (comma-separated);
 * defaults to `base/,patch/,sea/` in dem mode (skips the quantized-mesh mirror)
 * and the whole bucket otherwise. `version` is an FNV-1a hash of every key+etag,
 * so the tile server's config revalidation picks up any COG add/change/remove.
 */
export async function tileConfig(
  bucket: R2Bucket,
  dataset: string,
  origin: string,
  params: URLSearchParams,
  cors: Headers,
  method: string,
  isDem: boolean,
): Promise<Response> {
  const headers = new Headers(cors);
  headers.set("content-type", "application/json; charset=utf-8");
  headers.set("cache-control", "public, max-age=60");
  // HEAD wants headers only — skip the (potentially whole-bucket) R2 listing.
  if (method === "HEAD") {
    return new Response(null, { status: 200, headers });
  }

  const prefixParam = params.get("prefix");
  const prefixes =
    prefixParam !== null
      ? prefixParam
          .split(",")
          .map((s) => s.trim())
          .filter((s) => s.length > 0)
      : isDem
        ? ["base/", "patch/", "sea/"]
        : [""];

  const cogs: Cog[] = [];
  for (const prefix of prefixes) {
    cogs.push(...(await listCogs(bucket, prefix)));
  }
  cogs.sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));

  const sources = isDem
    ? demSource(cogs, dataset, origin, params.get("name")?.trim() || "dem")
    : rasterSources(cogs, dataset, origin, params.get("source")?.trim() || null);

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
