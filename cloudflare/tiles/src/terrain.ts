const CORS_HEADERS: Record<string, string> = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, HEAD",
};

export async function handleTerrain(
  request: Request,
  env: Env,
  _ctx: ExecutionContext,
  key: string,
): Promise<Response> {
  if (!key || key.endsWith("/")) {
    return new Response("Not Found", { status: 404, headers: CORS_HEADERS });
  }

  const object = await env.TERRAIN_BUCKET.get(key, {
    onlyIf: request.headers,
    range: request.headers,
  });

  if (!object) {
    return new Response("Not Found", { status: 404, headers: CORS_HEADERS });
  }

  const headers = new Headers(CORS_HEADERS);
  object.writeHttpMetadata(headers);
  headers.set("etag", object.httpEtag);

  if (!headers.has("content-type")) {
    headers.set("content-type", contentTypeFor(key));
  }
  if (!headers.has("cache-control")) {
    headers.set("cache-control", "public, max-age=86400, immutable");
  }
  // R2 stores .terrain files pre-gzipped (raw bytes). Declare the encoding so
  // clients (Cesium) decode it once. We must use `encodeBody: "manual"` —
  // otherwise the Workers runtime treats Content-Encoding as a hint and
  // re-encodes the body, producing double-gzip on the wire (or stripping the
  // header for clients that asked for identity).
  const isTerrain = key.endsWith(".terrain");
  if (isTerrain && !headers.has("content-encoding")) {
    headers.set("content-encoding", "gzip");
  }

  // `get` returns R2ObjectBody only when there's a body; otherwise it's a 304/precondition result.
  const body = "body" in object ? object.body : null;
  const status = body ? (headers.has("content-range") ? 206 : 200) : 304;

  return new Response(body, {
    status,
    headers,
    encodeBody: isTerrain ? "manual" : "automatic",
  });
}

function contentTypeFor(key: string): string {
  if (key.endsWith(".terrain")) return "application/vnd.quantized-mesh";
  if (key.endsWith(".json")) return "application/json";
  return "application/octet-stream";
}
