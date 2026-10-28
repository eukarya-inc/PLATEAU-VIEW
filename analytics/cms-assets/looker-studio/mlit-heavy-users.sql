-- PLATEAU CMS asset access per day (JST) by heavy user, for the MLIT report's heavy users page.
-- Reads the raw logs (kept 90 days). Heavy users are the IP addresses among the top 300 of the last 90 days
-- by accesses or by bytes; the rest are summed into "(上位以外の合計)". No IP addresses or cost: each heavy
-- user is shown as a number ranked by bytes over the last 90 days, which is renumbered on every refresh.
-- Counts client traffic only: not the CDN's own revalidation requests and not our own servers.
-- requests is the number of accesses (successful GETs; the 206 parts of a file count once per IP and
-- day), downloads the number of files sent (distinct IP and path per day).
WITH req AS (
  SELECT
    DATE(l.timestamp, "Asia/Tokyo") AS day,
    l.timestamp AS ts,
    l.http_request.remote_ip AS ip,
    l.http_request.request_method AS method,
    l.http_request.status AS status,
    -- The path without the query, so that the parts of one download share a key.
    REGEXP_EXTRACT(l.http_request.request_url, r"^https?://[^/]+([^?#]*)") AS path,
    NET.HOST(l.http_request.referer) AS referer_host,
    NULLIF(l.http_request.user_agent, "") AS user_agent,
    -- "partial": served from cache but part of the object was filled from the origin.
    CASE
      WHEN l.http_request.cache_hit AND l.http_request.cache_fill_bytes IS NOT NULL THEN "partial"
      WHEN l.http_request.cache_hit THEN "hit"
      WHEN l.http_request.cache_fill_bytes IS NOT NULL THEN "miss"
      ELSE "uncached"
    END AS cache_status,
    IFNULL(l.http_request.cache_lookup, FALSE) AS cache_lookup,
    IFNULL(CAST(l.http_request.response_size AS INT64), 0) AS bytes,
    IFNULL(CAST(l.http_request.cache_fill_bytes AS INT64), 0) AS fill_bytes
  FROM `reearth-plateau.cms_assets_requests_analytics._AllLogs` l
  WHERE l.timestamp >= TIMESTAMP(DATE_SUB(CURRENT_DATE("Asia/Tokyo"), INTERVAL 89 DAY), "Asia/Tokyo")
    -- Whole days only, like the aggregated tables. Logs start on 2026-10-12 at 14:40 JST.
    AND l.timestamp < TIMESTAMP(CURRENT_DATE("Asia/Tokyo"), "Asia/Tokyo")
    AND l.timestamp >= TIMESTAMP("2026-10-13", "Asia/Tokyo")
    -- The CDN's own revalidation of a client request; it carries the client's IP.
    AND NOT STARTS_WITH(IFNULL(l.http_request.user_agent, ""), "Cloud-CDN-Google")
),
ipv4 AS (SELECT DISTINCT ip, NET.IPV4_TO_INT64(NET.IP_FROM_STRING(ip)) AS n FROM req WHERE STRPOS(ip, ":") = 0),
geo AS (
  SELECT i.ip, ANY_VALUE(c.country) AS country
  FROM ipv4 i
  JOIN `reearth-plateau.cms_assets_analytics.ipv4_country` c
    ON c.bucket = DIV(i.n, 65536) AND i.n BETWEEN c.ip_from AND c.ip_to
  GROUP BY i.ip
),
asn AS (
  -- Network operators by DB-IP (https://db-ip.com), CC BY 4.0.
  SELECT i.ip, ANY_VALUE(a.asn) AS asn, ANY_VALUE(CONCAT("AS", a.asn, " ", a.org)) AS asn_org
  FROM ipv4 i
  JOIN `reearth-plateau.cms_assets_analytics.ipv4_asn` a
    ON a.bucket = DIV(i.n, 65536) AND i.n BETWEEN a.ip_from AND a.ip_to
  GROUP BY i.ip
),
price AS (SELECT item, region, unit_price_jpy FROM `reearth-plateau.cms_assets_analytics.unit_price`),
labeled AS (
  SELECT
    r.*,
    IFNULL(geo.country, "ZZ") AS country,
    IFNULL(asn.asn_org, "(不明)") AS asn_org,
    -- The same rules as traffic_class, client_type and source in the daily_cost view.
    asn.asn IN (396982, 15169)
      AND (r.user_agent IS NULL OR STARTS_WITH(r.user_agent, "Go-http-client/") OR STARTS_WITH(r.user_agent, "object_store/"))
      AS operator,
    CASE
      WHEN r.user_agent IS NULL THEN "(なし)"
      WHEN REGEXP_CONTAINS(LOWER(r.user_agent), r"bot|crawler|spider|slurp") THEN "ボット・クローラ"
      WHEN REGEXP_CONTAINS(r.user_agent, r"HeadlessChrome|Headless") THEN "自動操作のブラウザ"
      WHEN REGEXP_CONTAINS(r.user_agent, r"^(curl|Wget|wget)/") THEN "curl・wget"
      WHEN REGEXP_CONTAINS(LOWER(r.user_agent), r"python|aiohttp|httpx|urllib") THEN "Python"
      WHEN STARTS_WITH(r.user_agent, "Go-http-client") THEN "Go"
      WHEN REGEXP_CONTAINS(r.user_agent, r"^(node|undici|Bun/|axios/|node-fetch|okhttp)|PowerShell") THEN "その他のプログラム"
      WHEN REGEXP_CONTAINS(r.user_agent, r"QGIS") THEN "QGIS"
      -- libplateau (PLATEAU SDK for Unity/Unreal) sends the cpp-httplib default UA.
      WHEN STARTS_WITH(r.user_agent, "cpp-httplib/") THEN "PLATEAU SDK"
      WHEN REGEXP_CONTAINS(r.user_agent, r"Unity|UnrealEngine|Cesium") THEN "3D エンジン（Unity・Unreal・Cesium）"
      WHEN STARTS_WITH(r.user_agent, "Mozilla/") THEN "ブラウザ"
      ELSE "その他"
    END AS client_type,
    CASE
      WHEN STARTS_WITH(r.user_agent, "cpp-httplib/") THEN "PLATEAU SDK"
      WHEN r.referer_host IS NULL THEN "(なし)"
      WHEN r.referer_host = "plateauview.mlit.go.jp" THEN "PLATEAU VIEW"
      WHEN r.referer_host = "geospatial.jp" OR ENDS_WITH(r.referer_host, ".geospatial.jp") THEN "G空間情報センター"
      WHEN ENDS_WITH(r.referer_host, ".plateauview.mlit.go.jp") OR r.referer_host = "plateau.reearth.io"
        OR ENDS_WITH(r.referer_host, ".plateau.reearth.io") THEN "PLATEAU の関連サービス"
      WHEN r.referer_host = "mlit.go.jp" OR ENDS_WITH(r.referer_host, ".mlit.go.jp") THEN "国土交通省のサイト"
      WHEN r.referer_host IN ("localhost", "127.0.0.1", "[::1]", "0.0.0.0") OR ENDS_WITH(r.referer_host, ".localhost")
        OR REGEXP_CONTAINS(r.referer_host, r"^(10\.|192\.168\.|172\.(1[6-9]|2[0-9]|3[01])\.)") THEN "ローカル（自分のPC・社内ネットワーク）"
      ELSE "その他のサイト"
    END AS source,
    -- An access is a successful GET; the 206 parts of one file count once per IP and day. A download
    -- is a file sent to an IP in the day. Each sits on the first matching request so they add up.
    r.method = "GET" AND r.status IN (200, 206, 304)
      AND (r.status != 206
        OR ROW_NUMBER() OVER (PARTITION BY r.day, r.ip, r.path, r.method = "GET" AND r.status = 206 ORDER BY r.ts) = 1)
      AS access,
    r.method = "GET" AND r.status IN (200, 206)
      AND ROW_NUMBER() OVER (PARTITION BY r.day, r.ip, r.path, r.method = "GET" AND r.status IN (200, 206) ORDER BY r.ts) = 1
      AS download,
    -- Same estimate as the daily_cost view.
    IF(r.cache_status != "uncached", r.bytes / POW(1024, 3) * IFNULL(pe.unit_price_jpy, pe_asia.unit_price_jpy), 0)
      + IF(r.cache_status = "uncached", r.bytes / POW(1024, 3) * IFNULL(pi.unit_price_jpy, pi_apac.unit_price_jpy), 0)
      + r.fill_bytes / POW(1024, 3) * IFNULL(pf.unit_price_jpy, pf_asia.unit_price_jpy)
      + IF(r.cache_lookup, pl.unit_price_jpy, 0)
      + IF(r.cache_status != "hit", po.unit_price_jpy, 0) AS est_cost_jpy
  FROM req r
  LEFT JOIN geo USING (ip)
  LEFT JOIN asn USING (ip)
  LEFT JOIN `reearth-plateau.cms_assets_analytics.country_region` cr ON cr.country = IFNULL(geo.country, "ZZ")
  LEFT JOIN price pe ON pe.item = "cdn_egress" AND pe.region = cr.cdn_region
  LEFT JOIN price pe_asia ON pe_asia.item = "cdn_egress" AND pe_asia.region = "Asia"
  LEFT JOIN price pi ON pi.item = "internet_egress" AND pi.region = cr.egress_region
  LEFT JOIN price pi_apac ON pi_apac.item = "internet_egress" AND pi_apac.region = "Apac"
  LEFT JOIN price pf ON pf.item = "cache_fill" AND pf.region = cr.cdn_region
  LEFT JOIN price pf_asia ON pf_asia.item = "cache_fill" AND pf_asia.region = "Asia"
  LEFT JOIN price pl ON pl.item = "cache_lookup"
  LEFT JOIN price po ON po.item = "gcs_class_b"
),
-- Client traffic only: our own servers on Google Cloud are left out.
client AS (SELECT * FROM labeled WHERE NOT operator),
top AS (
  SELECT ip, SUM(bytes) AS total_bytes
  FROM client
  GROUP BY ip
  HAVING COUNTIF(access) > 0
  QUALIFY ROW_NUMBER() OVER (ORDER BY COUNTIF(access) DESC) <= 300
    OR ROW_NUMBER() OVER (ORDER BY SUM(bytes) DESC) <= 300
),
ranked AS (SELECT ip, ROW_NUMBER() OVER (ORDER BY total_bytes DESC) AS user_rank FROM top)
SELECT
  c.day,
  IF(r.ip IS NULL, "(上位以外の合計)", FORMAT("利用者%04d", r.user_rank)) AS user,
  IF(r.ip IS NULL, "(上位以外の合計)", c.asn_org) AS asn_org,
  IF(r.ip IS NULL, "(上位以外の合計)", c.country) AS country,
  c.client_type,
  c.source,
  COUNTIF(c.access) AS requests,
  COUNTIF(c.download) AS downloads,
  COUNT(DISTINCT IF(c.access, c.ip, NULL)) AS ips,
  SUM(c.bytes) / POW(1024, 3) AS gib
FROM client c
LEFT JOIN ranked r USING (ip)
GROUP BY day, user, asn_org, country, client_type, source
