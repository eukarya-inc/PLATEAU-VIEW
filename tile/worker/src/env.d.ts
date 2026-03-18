// Environment variable types (set via Cloudflare dashboard or wrangler secret)
declare namespace Cloudflare {
  interface Env {
    /** PLATEAU Tile Server URL */
    ORIGIN_URL: string;
    /** CORS allowed origins (comma-separated, or "*" for all) */
    CORS_ORIGINS: string;
    /** Cache-Control header for responses */
    CACHE_CONTROL: string;
  }
}
