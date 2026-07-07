/**
 * PLATEAU tiles Worker.
 *
 * A single host (tiles.plateau.city) serves several datasets, split by the
 * first path segment, each backed by an R2 bucket:
 *   tiles.plateau.city/terrain/<key> -> plateau-terrain  (quantized-mesh + terrain COGs)
 *   tiles.plateau.city/ortho/<key>   -> plateau-ortho    (ortho-imagery COGs)
 *
 * The segment -> bucket mapping is config (PATH_BUCKETS in wrangler.toml), not
 * code: adding a dataset is a bucket binding + map entry, no edits here.
 *
 * Per request (the first segment is stripped to form the R2 key):
 *   - a key path (/terrain/base/dem10/3036.tif) -> serve that object
 *   - a directory path (/terrain/, /terrain/2024/) -> live JSON listing
 *   - the root (/)                                 -> index of available datasets
 */

import { corsHeaders, listPrefix, serveObject, tileConfig } from "./r2";

/** Read PATH_BUCKETS, tolerating a missing/malformed binding (fail closed). */
function pathBuckets(env: Env): Record<string, string> {
  const map = env.PATH_BUCKETS as unknown;
  return map && typeof map === "object" ? (map as Record<string, string>) : {};
}

/** Resolve the R2 bucket for a dataset segment (e.g. "terrain"), or null. */
function bucketForDataset(env: Env, segment: string): { bucket: R2Bucket; name: string } | null {
  const bindingName = pathBuckets(env)[segment];
  if (!bindingName) return null;
  // Config-driven binding lookup: env is indexable, the binding is an R2Bucket.
  const bucket = (env as unknown as Record<string, R2Bucket | undefined>)[bindingName];
  if (!bucket) return null;
  return { bucket, name: segment };
}

/** Root index: advertise the configured dataset prefixes as directories. */
function datasetsIndex(env: Env, cors: Headers): Response {
  const datasets = Object.keys(pathBuckets(env));
  const headers = new Headers(cors);
  headers.set("content-type", "application/json; charset=utf-8");
  headers.set("cache-control", "public, max-age=300");
  const body = { datasets: datasets.map((d) => `${d}/`) };
  return new Response(JSON.stringify(body, null, 2), { headers });
}

export default {
  async fetch(request: Request, env: Env, _ctx: ExecutionContext): Promise<Response> {
    const cors = corsHeaders(env.CORS_ALLOW_ORIGIN ?? "*");

    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: cors });
    }
    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("Method Not Allowed", { status: 405, headers: cors });
    }

    const url = new URL(request.url);
    // R2 keys have no leading slash. Decoding turns a percent-encoded pathname
    // back into the literal key; malformed encoding is a client error (400), not
    // a 500.
    let path: string;
    try {
      path = decodeURIComponent(url.pathname.replace(/^\/+/, ""));
    } catch {
      return new Response("Bad Request", { status: 400, headers: cors });
    }

    // Root -> list the datasets this host serves.
    if (path === "") {
      return datasetsIndex(env, cors);
    }

    // The first segment selects the dataset/bucket; the rest is the R2 key.
    const slash = path.indexOf("/");
    const segment = slash === -1 ? path : path.slice(0, slash);
    const key = slash === -1 ? "" : path.slice(slash + 1);

    const resolved = bucketForDataset(env, segment);
    if (!resolved) {
      return new Response("Unknown dataset", { status: 404, headers: cors });
    }

    // `/<dataset>/config.json` emits a PLATEAU /tile config describing this
    // dataset's COGs as `cog` sources (for the tile server's CONFIG_URL). Special
    // key, checked before object serving so it isn't looked up as an R2 object.
    if (key === "config.json") {
      return tileConfig(resolved.bucket, resolved.name, url.origin, url.searchParams, cors);
    }

    // A bare prefix (`/terrain`, `/terrain/`) or a trailing slash lists that
    // prefix; anything else is an object key.
    if (key === "" || key.endsWith("/")) {
      const cursor = url.searchParams.get("cursor");
      return listPrefix(resolved.bucket, resolved.name, key, cursor, cors);
    }

    return serveObject(resolved.bucket, key, request, cors);
  },
} satisfies ExportedHandler<Env>;
